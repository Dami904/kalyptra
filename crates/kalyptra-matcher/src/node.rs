//! One matcher node: its bulletin board and its round state machine.
//!
//! ```text
//!  Open ──close()──> Closed ──t matching commit votes──> Committed ──clear()──> Cleared
//!                       └──────t votes to void (too few orders)──────> Voided (never opened)
//! ```
//!
//! The rules that protect sealed orders are enforced here, not left to the operator:
//!
//! * Only envelopes whose sealer proof verifies **for this round** are accepted and receipted.
//! * A node releases partial decryptions **only** once `t` nodes have signed the same final
//!   committed set, and **never** for a voided round. Before that, `release_partials` refuses.
//! * Statements from other nodes are accepted only with a valid signature from the roster; a
//!   node that signs two different commit or record votes for a round is reported as
//!   equivocating.
//!
//! This module is transport-agnostic: callers move [`Receipt`]s, [`CommitVote`]s, partials and
//! [`RecordVote`]s between nodes however they like.

use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use kalyptra_tse::{
    open, partial_decrypt, verify_envelope, verify_partial, Envelope, GroupKey, NodeKey,
    PartialDecryption,
};
use rand_core::{CryptoRng, RngCore};

use crate::auction::{self, AuctionOrder};
use crate::checker::PublicRound;
use crate::order::{round_aad, Order};
use crate::round::{
    short, CommitVote, Hash, PublicOrder, Receipt, RecordVote, Roster, RoundConfig, RoundRecord,
    MAX_EPOCH,
};
use crate::MatcherError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Open,
    Closed,
    Committed,
    Voided,
    Cleared,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Open => "open",
            Phase::Closed => "closed",
            Phase::Committed => "committed",
            Phase::Voided => "voided",
            Phase::Cleared => "cleared",
        }
    }
}

struct RoundState {
    cfg: RoundConfig,
    aad: Vec<u8>,
    phase: Phase,
    envelopes: BTreeMap<Hash, (Vec<u8>, Envelope)>,
    receipts: BTreeMap<Hash, BTreeMap<u16, Receipt>>,
    /// By (epoch, node): a node may vote once in each epoch.
    commit_votes: BTreeMap<(u32, u16), CommitVote>,
    committed: Option<Vec<Hash>>,
    partials: BTreeMap<Hash, BTreeMap<u16, PartialDecryption>>,
    record_votes: BTreeMap<u16, RecordVote>,
    my_record: Option<RoundRecord>,
    final_record: Option<Hash>,
}

/// One matcher node.
pub struct MatcherNode {
    id: u16,
    key: SigningKey,
    tse: NodeKey,
    group: GroupKey,
    roster: Roster,
    rounds: BTreeMap<u64, RoundState>,
}

impl MatcherNode {
    /// `key` signs statements; `tse` is this node's decryption share. They must be different
    /// keys, and the roster must list this node's signing key.
    pub fn new(
        id: u16,
        key: SigningKey,
        tse: NodeKey,
        group: GroupKey,
        roster: Roster,
    ) -> Result<Self, MatcherError> {
        if tse.id() != id {
            return Err(MatcherError::Config(
                "decryption share belongs to another node",
            ));
        }
        if roster.threshold != group.threshold() as usize {
            return Err(MatcherError::Config(
                "roster and group key disagree on the threshold",
            ));
        }
        if roster.key(id)? != &key.verifying_key() {
            return Err(MatcherError::Config(
                "roster lists a different signing key for this node",
            ));
        }
        if group.verifying_share(id).is_none() {
            return Err(MatcherError::Config("node is not part of the group key"));
        }
        // With 2t <= n, two different commits could each gather t votes and both be final.
        if roster.nodes.len() >= 2 * roster.threshold {
            return Err(MatcherError::Config(
                "the threshold must be more than half the nodes, or two conflicting commits could both be final",
            ));
        }
        Ok(MatcherNode {
            id,
            key,
            tse,
            group,
            roster,
            rounds: BTreeMap::new(),
        })
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn group(&self) -> &GroupKey {
        &self.group
    }

    fn round(&self, id: u64) -> Result<&RoundState, MatcherError> {
        self.rounds.get(&id).ok_or(MatcherError::UnknownRound(id))
    }

    fn round_mut(&mut self, id: u64) -> Result<&mut RoundState, MatcherError> {
        self.rounds
            .get_mut(&id)
            .ok_or(MatcherError::UnknownRound(id))
    }

    pub fn phase(&self, round_id: u64) -> Result<Phase, MatcherError> {
        Ok(self.round(round_id)?.phase)
    }

    /// Every round this node knows: its configuration, phase and how many envelopes it holds.
    pub fn rounds(&self) -> Vec<(RoundConfig, Phase, usize)> {
        self.rounds
            .values()
            .map(|r| (r.cfg.clone(), r.phase, r.envelopes.len()))
            .collect()
    }

    pub fn open_round(&mut self, cfg: RoundConfig) -> Result<(), MatcherError> {
        if self.rounds.contains_key(&cfg.round_id) {
            return Err(MatcherError::RoundExists(cfg.round_id));
        }
        if cfg.market.is_empty() || cfg.market.len() > crate::order::MARKET_MAX {
            return Err(MatcherError::Config("market name length"));
        }
        if cfg.min_orders == 0 {
            return Err(MatcherError::Config("min_orders must be at least 1"));
        }
        let aad = round_aad(cfg.round_id, &cfg.market);
        self.rounds.insert(
            cfg.round_id,
            RoundState {
                cfg,
                aad,
                phase: Phase::Open,
                envelopes: BTreeMap::new(),
                receipts: BTreeMap::new(),
                commit_votes: BTreeMap::new(),
                committed: None,
                partials: BTreeMap::new(),
                record_votes: BTreeMap::new(),
                my_record: None,
                final_record: None,
            },
        );
        Ok(())
    }

    fn check_envelope(
        &self,
        round_id: u64,
        bytes: &[u8],
    ) -> Result<(Hash, Envelope), MatcherError> {
        let r = self.round(round_id)?;
        let env = Envelope::from_bytes(bytes).map_err(MatcherError::BadEnvelope)?;
        verify_envelope(self.group.public_key(), &env, &r.aad)
            .map_err(MatcherError::BadEnvelope)?;
        Ok((crate::round::envelope_hash(bytes), env))
    }

    /// An investor submits a sealed order. Accepted only while the round is open and only if
    /// the envelope was sealed for this round. Returns this node's signed receipt.
    pub fn submit(&mut self, round_id: u64, bytes: &[u8]) -> Result<Receipt, MatcherError> {
        let phase = self.round(round_id)?.phase;
        if phase != Phase::Open {
            return Err(MatcherError::WrongPhase {
                round: round_id,
                phase: phase.name(),
            });
        }
        let (h, env) = self.check_envelope(round_id, bytes)?;
        let receipt = Receipt::new(&self.key, self.id, round_id, h);
        let r = self.round_mut(round_id)?;
        r.envelopes.insert(h, (bytes.to_vec(), env));
        r.receipts
            .entry(h)
            .or_default()
            .insert(receipt.node, receipt.clone());
        Ok(receipt)
    }

    /// Learn an envelope another node accepted (so this node can decrypt it later). Checked
    /// exactly like a submission, but issues no receipt.
    pub fn learn_envelope(&mut self, round_id: u64, bytes: &[u8]) -> Result<(), MatcherError> {
        let (h, env) = self.check_envelope(round_id, bytes)?;
        self.round_mut(round_id)?
            .envelopes
            .entry(h)
            .or_insert((bytes.to_vec(), env));
        Ok(())
    }

    pub fn learn_receipt(&mut self, receipt: &Receipt) -> Result<(), MatcherError> {
        receipt.verify(&self.roster)?;
        self.round_mut(receipt.round_id)?
            .receipts
            .entry(receipt.envelope)
            .or_default()
            .insert(receipt.node, receipt.clone());
        Ok(())
    }

    /// Stop accepting orders and vote on the committed set: every envelope that `t` nodes
    /// receipted. If that set is smaller than the round's minimum, vote to void the round.
    pub fn close(&mut self, round_id: u64) -> Result<CommitVote, MatcherError> {
        let r = self.round(round_id)?;
        if r.phase != Phase::Open {
            return Err(MatcherError::WrongPhase {
                round: round_id,
                phase: r.phase.name(),
            });
        }
        let vote = self.vote_on_receipts(round_id, 0)?;
        self.round_mut(round_id)?.phase = Phase::Closed;
        self.learn_commit_vote(&vote)?;
        Ok(vote)
    }

    /// A signed vote on every envelope that `t` nodes are known to have receipted.
    fn vote_on_receipts(&self, round_id: u64, epoch: u32) -> Result<CommitVote, MatcherError> {
        let t = self.roster.threshold;
        let r = self.round(round_id)?;
        let set: Vec<Hash> = r
            .receipts
            .iter()
            .filter(|(_, by)| by.len() >= t)
            .map(|(h, _)| *h)
            .collect();
        let voided = set.len() < r.cfg.min_orders;
        Ok(CommitVote::in_epoch(
            &self.key,
            self.id,
            round_id,
            epoch,
            set,
            r.cfg.min_orders,
            voided,
        ))
    }

    /// If the commit votes so far can no longer give any set `t` matching votes, vote again in
    /// the next epoch, on the receipts this node knows by now.
    ///
    /// This is what un-sticks a round whose nodes saw different receipts when they closed. It
    /// is safe because a vote is only ever repeated once the earlier epoch is **provably dead**:
    /// the largest group of matching votes plus every vote not yet seen is still below `t`.
    /// Votes only ever add up, so a dead epoch stays dead, and two different sets can never
    /// both become final. Returns `None` when it is not time (or the round is already decided).
    pub fn revote(&mut self, round_id: u64) -> Result<Option<CommitVote>, MatcherError> {
        let t = self.roster.threshold;
        let n = self.roster.nodes.len();
        let id = self.id;
        let r = self.round(round_id)?;
        if r.phase != Phase::Closed || r.committed.is_some() {
            return Ok(None);
        }
        let Some(mine) = r
            .commit_votes
            .keys()
            .filter(|(_, v)| *v == id)
            .map(|(e, _)| *e)
            .max()
        else {
            return Ok(None);
        };
        if mine >= MAX_EPOCH || !epoch_is_dead(r, mine, t, n) {
            return Ok(None);
        }
        let vote = self.vote_on_receipts(round_id, mine + 1)?;
        self.learn_commit_vote(&vote)?;
        Ok(Some(vote))
    }

    /// Learn a commit vote. When `t` nodes have signed the same set (or the same decision to
    /// void), the commit is final.
    pub fn learn_commit_vote(&mut self, vote: &CommitVote) -> Result<(), MatcherError> {
        vote.verify(&self.roster)?;
        vote.well_formed()?;
        let t = self.roster.threshold;
        let r = self.round_mut(vote.round_id)?;
        if vote.min_orders as usize != r.cfg.min_orders {
            return Err(MatcherError::BadCommit(
                "the vote signs a different minimum number of orders than this round has",
            ));
        }
        if !vote.void_is_consistent() {
            return Err(MatcherError::BadCommit(
                "the vote's decision to void (or not) does not follow from its set and the minimum",
            ));
        }
        if let Some(prev) = r.commit_votes.get(&(vote.epoch, vote.node)) {
            if prev != vote {
                return Err(MatcherError::Equivocation(vote.node));
            }
            return Ok(());
        }
        r.commit_votes.insert((vote.epoch, vote.node), vote.clone());
        if r.committed.is_some() {
            return Ok(());
        }
        // The first epoch (lowest number) in which `t` nodes signed the same thing decides.
        let epochs: Vec<u32> = r.commit_votes.keys().map(|(e, _)| *e).collect();
        for epoch in epochs {
            let mut tally: BTreeMap<(Hash, bool, u32), usize> = BTreeMap::new();
            for v in r.commit_votes.values().filter(|v| v.epoch == epoch) {
                *tally.entry(v.key()).or_default() += 1;
            }
            if let Some((key, _)) = tally.into_iter().find(|(_, c)| *c >= t) {
                let winner = r
                    .commit_votes
                    .values()
                    .find(|v| v.epoch == epoch && v.key() == key)
                    .expect("a vote with this key exists");
                r.committed = Some(winner.set.clone());
                r.phase = if key.1 {
                    Phase::Voided
                } else {
                    Phase::Committed
                };
                break;
            }
        }
        Ok(())
    }

    /// Release this node's partial decryptions for every envelope in the final committed set.
    ///
    /// Refuses before the commit is final and, always, for a voided round. Also refuses if this
    /// node is missing a committed envelope (learn it from a peer first).
    pub fn release_partials<R: RngCore + CryptoRng>(
        &mut self,
        round_id: u64,
        rng: &mut R,
    ) -> Result<Vec<(Hash, PartialDecryption)>, MatcherError> {
        let r = self.round(round_id)?;
        match r.phase {
            Phase::Committed | Phase::Cleared => {}
            Phase::Voided => {
                let have = r.committed.as_ref().map_or(0, |s| s.len());
                return Err(MatcherError::Voided {
                    have,
                    need: r.cfg.min_orders,
                });
            }
            Phase::Open | Phase::Closed => return Err(MatcherError::NotCommitted(round_id)),
        }
        let set = r.committed.clone().expect("committed phase has a set");
        let mut out = Vec::with_capacity(set.len());
        for h in &set {
            let (_, env) = r
                .envelopes
                .get(h)
                .ok_or_else(|| MatcherError::MissingEnvelope(short(h)))?;
            let p = partial_decrypt(&self.tse, &self.group, env, &r.aad, rng)
                .map_err(MatcherError::BadEnvelope)?;
            out.push((*h, p));
        }
        let id = self.id;
        let r = self.round_mut(round_id)?;
        for (h, p) in &out {
            r.partials.entry(*h).or_default().insert(id, *p);
        }
        Ok(out)
    }

    /// Learn another node's partial decryption. It must verify against that node's public
    /// verifying share, and only partials for committed envelopes are kept.
    pub fn learn_partial(
        &mut self,
        round_id: u64,
        envelope: Hash,
        partial: &PartialDecryption,
    ) -> Result<(), MatcherError> {
        let group = self.group.clone();
        let r = self.round_mut(round_id)?;
        let in_set = r.committed.as_ref().is_some_and(|s| s.contains(&envelope));
        if !in_set {
            return Err(MatcherError::NotCommitted(round_id));
        }
        let (_, env) = r
            .envelopes
            .get(&envelope)
            .ok_or_else(|| MatcherError::MissingEnvelope(short(&envelope)))?;
        verify_partial(&group, env, partial)
            .map_err(|_| MatcherError::BadPartial(partial.node_id))?;
        r.partials
            .entry(envelope)
            .or_default()
            .insert(partial.node_id, *partial);
        Ok(())
    }

    /// Open every committed envelope, run the auction and sign the round record.
    pub fn clear(&mut self, round_id: u64) -> Result<RecordVote, MatcherError> {
        let r = self.round(round_id)?;
        if r.phase != Phase::Committed {
            return Err(MatcherError::WrongPhase {
                round: round_id,
                phase: r.phase.name(),
            });
        }
        let set = r.committed.clone().expect("committed phase has a set");
        let mut plaintexts = Vec::with_capacity(set.len());
        for h in &set {
            let (_, env) = r
                .envelopes
                .get(h)
                .ok_or_else(|| MatcherError::MissingEnvelope(short(h)))?;
            let ps: Vec<PartialDecryption> = r
                .partials
                .get(h)
                .map(|m| m.values().copied().collect())
                .unwrap_or_default();
            let opened =
                open(&self.group, env, &r.aad, &ps).map_err(|e| MatcherError::CannotOpen {
                    envelope: short(h),
                    source: e,
                })?;
            plaintexts.push(opened.plaintext);
        }
        let record = build_record(&r.cfg, &set, &plaintexts);
        let vote = RecordVote::new(&self.key, self.id, &record);
        let r = self.round_mut(round_id)?;
        r.my_record = Some(record);
        r.phase = Phase::Cleared;
        self.learn_record_vote(&vote)?;
        Ok(vote)
    }

    pub fn learn_record_vote(&mut self, vote: &RecordVote) -> Result<(), MatcherError> {
        vote.verify(&self.roster)?;
        let t = self.roster.threshold;
        let r = self.round_mut(vote.round_id)?;
        if let Some(prev) = r.record_votes.get(&vote.node) {
            if prev != vote {
                return Err(MatcherError::Equivocation(vote.node));
            }
            return Ok(());
        }
        r.record_votes.insert(vote.node, vote.clone());
        if r.final_record.is_none() {
            let mut tally: BTreeMap<Hash, usize> = BTreeMap::new();
            for v in r.record_votes.values() {
                *tally.entry(v.record).or_default() += 1;
            }
            r.final_record = tally.into_iter().find(|(_, c)| *c >= t).map(|(h, _)| h);
        }
        Ok(())
    }

    /// The round record, once `t` nodes have signed it **and** it matches this node's own
    /// computation. `None` otherwise.
    pub fn final_record(&self, round_id: u64) -> Option<&RoundRecord> {
        let r = self.rounds.get(&round_id)?;
        let fin = r.final_record?;
        r.my_record.as_ref().filter(|rec| rec.hash() == fin)
    }

    /// Everything a checker needs, as this node sees it.
    pub fn export(&self, round_id: u64) -> Result<PublicRound, MatcherError> {
        let r = self.round(round_id)?;
        Ok(PublicRound {
            config: r.cfg.clone(),
            envelopes: r.envelopes.values().map(|(b, _)| b.clone()).collect(),
            receipts: r
                .receipts
                .values()
                .flat_map(|m| m.values().cloned())
                .collect(),
            commit_votes: r.commit_votes.values().cloned().collect(),
            partials: r
                .partials
                .iter()
                .flat_map(|(h, m)| m.values().map(move |p| (*h, *p)))
                .collect(),
            record: r.my_record.clone(),
            record_votes: r.record_votes.values().cloned().collect(),
        })
    }
}

/// Whether no set can still get `t` matching votes in `epoch`, given the votes seen so far.
fn epoch_is_dead(r: &RoundState, epoch: u32, t: usize, n: usize) -> bool {
    let mut tally: BTreeMap<(Hash, bool, u32), usize> = BTreeMap::new();
    let mut seen = 0;
    for v in r.commit_votes.values().filter(|v| v.epoch == epoch) {
        *tally.entry(v.key()).or_default() += 1;
        seen += 1;
    }
    let best = tally.values().copied().max().unwrap_or(0);
    best + n.saturating_sub(seen) < t
}

/// Turn opened plaintexts (in committed-set order) into the round record. Shared by nodes and
/// the checker so both apply exactly the same rules.
pub(crate) fn build_record(cfg: &RoundConfig, set: &[Hash], plaintexts: &[Vec<u8>]) -> RoundRecord {
    let mut orders = Vec::new();
    let mut invalid = Vec::new();
    let mut auction_orders = Vec::new();
    for (i, pt) in plaintexts.iter().enumerate() {
        let i = i as u32;
        match Order::parse_signed(pt) {
            Ok(o) if o.round_id == cfg.round_id && o.market == cfg.market => {
                orders.push((
                    i,
                    PublicOrder {
                        side: o.side,
                        size: o.size,
                        limit: o.limit,
                        owner: o.investor,
                    },
                ));
                auction_orders.push(AuctionOrder {
                    id: i,
                    side: o.side,
                    size: o.size,
                    limit: o.limit,
                });
            }
            _ => invalid.push(i),
        }
    }
    let c = auction::clear(&auction_orders);
    let mut set = set.to_vec();
    set.sort();
    RoundRecord {
        round_id: cfg.round_id,
        market: cfg.market.clone(),
        set,
        orders,
        invalid,
        price: c.price,
        volume: c.volume,
        fills: c.fills,
    }
}

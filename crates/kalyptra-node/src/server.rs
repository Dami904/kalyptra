//! A matcher node as an HTTP server.
//!
//! On first start a node runs the distributed key generation with its peers over HTTP. Round 1
//! messages are signed broadcasts; round 2 messages hold secret shares, so each is sealed to
//! its recipient's key and signed. The resulting share is saved, so later starts skip this.
//!
//! Then the node serves rounds. It moves a round forward by itself: once `t` nodes have
//! committed, it releases its partial decryptions; once it holds `t` valid partials for every
//! committed envelope, it clears the round and signs the record. Everything it learns from
//! peers is checked by `kalyptra-matcher` before use. Messages that arrive too early (a partial
//! before the commit, a receipt before the round is open here) are held and retried.
//!
//! After a round clears, the node collects each filled order's settlement part (an action
//! group its owner built and signed for). The round's leader checks every part against the
//! record's terms, assembles one swap transaction and mines it, through its own wallet.
//!
//! Limits: plain HTTP without TLS, for a local demo. Gossip and key-generation messages are
//! signed or self-authenticating, but anyone who can reach a node can call the operator
//! endpoints (open and close a round).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use kalyptra_matcher::settlement::{leader, legs, market_assets, verify_part, Leg};
use kalyptra_matcher::{MatcherError, MatcherNode, Phase, Roster, RoundRecord};
use kalyptra_tse::{open_single, seal, Envelope, GroupKey, Participant};
use rand_core::OsRng;
use serde::Serialize;
use serde_json::Value;
use tiny_http::{Header, Method, Request, Response};

use crate::files::{
    listings_path, load_share, read_json_or_default, save_share, share_path, write_json,
    NodeIdentity,
};
use crate::wallet::{Assembled, Wallet};
use crate::wire::{
    hx, listing_msg, point, unhex, unhex_n, valid_symbol, AssetsJ, CommitVoteJ, ConfigJ, Dkg1J,
    Dkg2J, ErrorJ, Gossip, GroupJ, LegJ, ListingJ, ListingSigJ, PartialJ, PublicRoundJ, ReceiptJ,
    RecordJ, RecordVoteJ, RosterJ, RoundStatusJ, RoundSummaryJ, SettleJ, SettlementJ, StatusJ,
    SubmitJ,
};

pub struct NodeOptions {
    pub identity: NodeIdentity,
    pub roster: RosterJ,
    pub data_dir: PathBuf,
    /// Print what the node does.
    pub verbose: bool,
    /// Needed only to lead settlement. A node without it still collects and checks parts.
    pub settle: Option<SettleOptions>,
}

/// What a node needs to act as a round's settlement leader.
#[derive(Clone, Debug)]
pub struct SettleOptions {
    pub wallet: Wallet,
    pub assets: AssetsJ,
    /// If set, the assets are re-read from this file before each settlement, so a company
    /// listed after the node started can settle too.
    pub assets_file: Option<PathBuf>,
    /// How long the next node in turn waits, once it holds every part, before checking that
    /// the round's leader has taken the settlement up (and taking it over if not).
    pub takeover_after: Duration,
}

#[derive(Clone, Debug, Default)]
enum SettleStatus {
    #[default]
    Collecting,
    Assembling,
    Settled {
        txid: String,
        height: u32,
    },
    Failed {
        error: String,
        bad: Vec<u32>,
    },
    /// The wallet tool gave no result: the transaction may or may not have been sent. A retry
    /// is safe, because the chain refuses to spend the same notes twice; if the first attempt
    /// landed, the retry fails with a duplicate-nullifier rejection and nothing moves.
    Unknown {
        error: String,
    },
}

#[derive(Default)]
struct Settlement {
    /// Verified parts by order position.
    parts: BTreeMap<u32, Vec<u8>>,
    status: SettleStatus,
    /// An earlier attempt for this round ended unknown: it may have landed. A later rejection
    /// (for example a duplicate nullifier) then does not prove that nothing moved.
    was_unknown: bool,
}

/// An assembly to run once the state lock is released (it takes tens of seconds).
struct Job {
    round: u64,
    wallet: Wallet,
    dir: PathBuf,
    /// (file name, bytes, plan entry)
    parts: Vec<(String, Vec<u8>, Value)>,
}

struct DkgState {
    participant: Participant,
    own_r1: Vec<u8>,
    r1: BTreeMap<u16, Vec<u8>>,
    r2_in: BTreeMap<u16, Vec<u8>>,
    r2_sent: bool,
}

struct State {
    identity: NodeIdentity,
    roster_j: RosterJ,
    roster: Roster,
    data_dir: PathBuf,
    verbose: bool,
    dkg: Option<DkgState>,
    node: Option<MatcherNode>,
    group: Option<GroupKey>,
    released: BTreeSet<u64>,
    /// Company listings this node has accepted and signed: symbol to asset id.
    listings: AssetsJ,
    pending: Vec<Gossip>,
    /// Messages to send once the lock is released: (url, body).
    outbox: Vec<(String, Value)>,
    settle: Option<SettleOptions>,
    settlements: BTreeMap<u64, Settlement>,
    jobs: Vec<Job>,
    /// Rounds this node settles in place of a leader that did not take them up.
    acting: BTreeSet<u64>,
    /// Rounds whose leader this node (next in turn) is to check on, once the lock is released.
    watches: Vec<u64>,
    watched: BTreeSet<u64>,
}

pub struct NodeHandle {
    pub addr: SocketAddr,
    server: Arc<tiny_http::Server>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NodeHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.server.unblock();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

type HttpResult = Result<Value, (u16, String)>;

fn bad(msg: impl ToString) -> (u16, String) {
    (400, msg.to_string())
}

fn json<T: Serialize>(v: &T) -> HttpResult {
    serde_json::to_value(v).map_err(|e| (500, e.to_string()))
}

fn dkg1_msg(from: u16, payload: &[u8]) -> Vec<u8> {
    [b"kalyptra/v1/dkg1".as_slice(), &from.to_le_bytes(), payload].concat()
}

fn dkg2_msg(from: u16, to: u16, envelope: &[u8]) -> Vec<u8> {
    [
        b"kalyptra/v1/dkg2".as_slice(),
        &from.to_le_bytes(),
        &to.to_le_bytes(),
        envelope,
    ]
    .concat()
}

fn dkg2_aad(from: u16, to: u16) -> Vec<u8> {
    format!("kalyptra/v1/dkg2/{from}/{to}").into_bytes()
}

/// A matcher error that means "too early", not "wrong": keep the message and retry later.
fn retriable(e: &MatcherError) -> bool {
    matches!(
        e,
        MatcherError::UnknownRound(_)
            | MatcherError::NotCommitted(_)
            | MatcherError::MissingEnvelope(_)
    )
}

impl State {
    fn log(&self, msg: impl AsRef<str>) {
        if self.verbose {
            eprintln!("[node {}] {}", self.identity.id, msg.as_ref());
        }
    }

    fn peers(&self) -> Vec<(u16, String)> {
        self.roster_j
            .nodes
            .iter()
            .filter(|n| n.id != self.identity.id)
            .map(|n| (n.id, n.url.clone()))
            .collect()
    }

    fn broadcast(&mut self, path: &str, body: Value) {
        for (_, url) in self.peers() {
            self.outbox.push((format!("{url}{path}"), body.clone()));
        }
    }

    fn gossip(&mut self, g: Gossip) {
        let body = serde_json::to_value(&g).expect("gossip serializes");
        self.broadcast("/v1/gossip", body);
    }

    fn signer_key(&self, node: u16) -> Result<VerifyingKey, (u16, String)> {
        self.roster.key(node).copied().map_err(bad)
    }

    // ---- key generation ----------------------------------------------------------------------

    fn start_dkg(&mut self) -> Result<(), String> {
        let n = self.roster_j.nodes.len() as u16;
        let t = self.roster_j.threshold;
        let mut participant =
            Participant::new(self.identity.id, n, t).map_err(|e| e.to_string())?;
        let own_r1 = participant.round1(&mut OsRng).map_err(|e| e.to_string())?;
        let sig = self
            .identity
            .sign
            .sign(&dkg1_msg(self.identity.id, &own_r1));
        let body = serde_json::to_value(Dkg1J {
            from: self.identity.id,
            msg: hx(&own_r1),
            sig: hx(&sig.to_bytes()),
        })
        .expect("serializes");
        self.broadcast("/v1/dkg/round1", body);
        self.dkg = Some(DkgState {
            participant,
            own_r1,
            r1: BTreeMap::new(),
            r2_in: BTreeMap::new(),
            r2_sent: false,
        });
        self.log("key generation: round 1 sent");
        Ok(())
    }

    fn on_dkg1(&mut self, j: Dkg1J) -> HttpResult {
        let payload = unhex(&j.msg).map_err(bad)?;
        let sig = Signature::from_bytes(&unhex_n::<64>(&j.sig).map_err(bad)?);
        self.signer_key(j.from)?
            .verify(&dkg1_msg(j.from, &payload), &sig)
            .map_err(|_| bad("round 1 signature does not verify"))?;
        let Some(d) = self.dkg.as_mut() else {
            return Ok(Value::Null); // already done
        };
        if let Some(prev) = d.r1.get(&j.from) {
            if prev != &payload {
                return Err(bad(format!(
                    "node {} sent two different round 1 messages",
                    j.from
                )));
            }
        }
        d.r1.insert(j.from, payload);
        self.dkg_progress().map_err(bad)?;
        Ok(Value::Null)
    }

    fn on_dkg2(&mut self, j: Dkg2J) -> HttpResult {
        if j.to != self.identity.id {
            return Err(bad("round 2 message addressed to another node"));
        }
        let env_bytes = unhex(&j.envelope).map_err(bad)?;
        let sig = Signature::from_bytes(&unhex_n::<64>(&j.sig).map_err(bad)?);
        self.signer_key(j.from)?
            .verify(&dkg2_msg(j.from, j.to, &env_bytes), &sig)
            .map_err(|_| bad("round 2 signature does not verify"))?;
        let env = Envelope::from_bytes(&env_bytes).map_err(bad)?;
        let plain = open_single(&self.identity.dkg, &env, &dkg2_aad(j.from, j.to)).map_err(bad)?;
        let Some(d) = self.dkg.as_mut() else {
            return Ok(Value::Null);
        };
        d.r2_in.insert(j.from, plain);
        self.dkg_progress().map_err(bad)?;
        Ok(Value::Null)
    }

    fn dkg_progress(&mut self) -> Result<(), String> {
        let n = self.roster_j.nodes.len();
        let me = self.identity.id;
        let Some(d) = self.dkg.as_mut() else {
            return Ok(());
        };
        let all_r1: Vec<Vec<u8>> = std::iter::once(d.own_r1.clone())
            .chain(d.r1.values().cloned())
            .collect();
        if !d.r2_sent && d.r1.len() == n - 1 {
            let msgs = d.participant.round2(&all_r1).map_err(|e| e.to_string())?;
            d.r2_sent = true;
            let mut out = Vec::new();
            for m in msgs {
                let peer = self.roster_j.node(m.to)?.clone();
                let pk = point(&peer.dkg_pk)?;
                let env = seal(&pk, &dkg2_aad(me, m.to), &m.bytes, &mut OsRng).to_bytes();
                let sig = self.identity.sign.sign(&dkg2_msg(me, m.to, &env));
                let body = serde_json::to_value(Dkg2J {
                    from: me,
                    to: m.to,
                    envelope: hx(&env),
                    sig: hx(&sig.to_bytes()),
                })
                .expect("serializes");
                out.push((format!("{}/v1/dkg/round2", peer.url), body));
            }
            self.outbox.extend(out);
            self.log("key generation: round 2 sent");
        }
        let Some(d) = self.dkg.as_mut() else {
            return Ok(());
        };
        if d.r2_sent && d.r2_in.len() == n - 1 {
            let r2: Vec<Vec<u8>> = d.r2_in.values().cloned().collect();
            let (key, group) = d
                .participant
                .round3(&all_r1, &r2)
                .map_err(|e| e.to_string())?;
            save_share(&share_path(&self.data_dir, me), &key, &group)?;
            let node = MatcherNode::new(
                me,
                self.identity.sign.clone(),
                key,
                group.clone(),
                self.roster.clone(),
            )
            .map_err(|e| e.to_string())?;
            self.log(format!(
                "key generation done; group key {}",
                &hx(group.public_key().compress().as_bytes())[..16]
            ));
            self.node = Some(node);
            self.group = Some(group);
            self.dkg = None;
        }
        Ok(())
    }

    // ---- rounds ----------------------------------------------------------------------------------

    fn node(&mut self) -> Result<&mut MatcherNode, (u16, String)> {
        self.node
            .as_mut()
            .ok_or((503, "key generation not finished".to_string()))
    }

    fn apply(&mut self, g: &Gossip) -> Result<u64, MatcherError> {
        let node = self
            .node
            .as_mut()
            .ok_or(MatcherError::Config("key generation not finished"))?;
        let bad_wire = |_| MatcherError::Config("malformed gossip");
        match g {
            Gossip::Envelope { round_id, envelope } => {
                node.learn_envelope(*round_id, &unhex(envelope).map_err(bad_wire)?)?;
                Ok(*round_id)
            }
            Gossip::Receipt(r) => {
                let r = r.receipt().map_err(bad_wire)?;
                node.learn_receipt(&r)?;
                Ok(r.round_id)
            }
            Gossip::Commit(v) => {
                let v = v.vote().map_err(bad_wire)?;
                node.learn_commit_vote(&v)?;
                Ok(v.round_id)
            }
            Gossip::Partial { round_id, partial } => {
                let (h, p) = partial.parts().map_err(bad_wire)?;
                node.learn_partial(*round_id, h, &p)?;
                Ok(*round_id)
            }
            Gossip::RecordVote(v) => {
                let v = v.vote().map_err(bad_wire)?;
                node.learn_record_vote(&v)?;
                Ok(v.round_id)
            }
        }
    }

    fn on_gossip(&mut self, g: Gossip) -> HttpResult {
        self.node()?;
        match self.apply(&g) {
            Ok(round) => {
                self.advance(round);
                Ok(Value::Null)
            }
            Err(e) if retriable(&e) => {
                self.pending.push(g);
                Ok(Value::Null)
            }
            Err(e) => Err(bad(e)),
        }
    }

    fn retry_pending(&mut self) {
        loop {
            let pending = std::mem::take(&mut self.pending);
            let before = pending.len();
            for g in pending {
                match self.apply(&g) {
                    Ok(_) => {}
                    Err(e) if retriable(&e) => self.pending.push(g),
                    Err(e) => self.log(format!("dropping gossip: {e}")),
                }
            }
            if self.pending.len() == before || self.pending.is_empty() {
                break;
            }
        }
    }

    /// Move a round forward as far as this node can on its own.
    fn advance(&mut self, round: u64) {
        self.retry_pending();
        let Some(node) = self.node.as_mut() else {
            return;
        };
        // Nodes that closed with different receipts: once no set can still win, vote again.
        match node.revote(round) {
            Ok(Some(vote)) => {
                self.log(format!(
                    "round {round}: the votes could not agree; voting again (epoch {}, {} orders)",
                    vote.epoch,
                    vote.set.len()
                ));
                self.gossip(Gossip::Commit(CommitVoteJ::from(&vote)));
            }
            Ok(None) => {}
            Err(e) => self.log(format!("round {round}: cannot vote again: {e}")),
        }
        let Some(node) = self.node.as_mut() else {
            return;
        };
        if node.phase(round) == Ok(Phase::Committed) && !self.released.contains(&round) {
            match node.release_partials(round, &mut OsRng) {
                Ok(ps) => {
                    self.released.insert(round);
                    self.log(format!(
                        "round {round}: committed; released {} partials",
                        ps.len()
                    ));
                    for (h, p) in ps {
                        self.gossip(Gossip::Partial {
                            round_id: round,
                            partial: PartialJ::new(&h, &p),
                        });
                    }
                }
                Err(MatcherError::MissingEnvelope(_)) => {}
                Err(e) => self.log(format!("round {round}: cannot release: {e}")),
            }
        }
        let Some(node) = self.node.as_mut() else {
            return;
        };
        if node.phase(round) == Ok(Phase::Committed) && self.released.contains(&round) {
            match node.clear(round) {
                Ok(vote) => {
                    self.log(format!("round {round}: cleared; record signed"));
                    self.gossip(Gossip::RecordVote(RecordVoteJ::from(&vote)));
                }
                Err(MatcherError::CannotOpen { .. }) => {}
                Err(e) => self.log(format!("round {round}: cannot clear: {e}")),
            }
        }
        if let Some(node) = self.node.as_ref() {
            if node.phase(round) == Ok(Phase::Voided) && !self.released.contains(&round) {
                self.released.insert(round);
                self.log(format!("round {round}: voided; nothing will be decrypted"));
            }
        }
    }

    // ---- settlement ------------------------------------------------------------------------------

    fn final_record(&mut self, round: u64) -> Result<(RoundRecord, Vec<Leg>), (u16, String)> {
        let rec = self
            .node()?
            .final_record(round)
            .cloned()
            .ok_or((409, format!("round {round} has no final record")))?;
        let ls = legs(&rec).map_err(|e| (500, e.to_string()))?;
        Ok((rec, ls))
    }

    fn leader_of(&self, round: u64) -> u16 {
        if self.acting.contains(&round) {
            return self.identity.id;
        }
        let ids: Vec<u16> = self.roster_j.nodes.iter().map(|n| n.id).collect();
        leader(&ids, round).expect("the roster is not empty")
    }

    /// The node that takes a round's settlement over when its leader does not: the next in
    /// turn after the leader.
    fn backup_of(&self, round: u64) -> u16 {
        let ids: Vec<u16> = self.roster_j.nodes.iter().map(|n| n.id).collect();
        leader(&ids, round.wrapping_add(1)).expect("the roster is not empty")
    }

    fn on_settle(&mut self, round: u64, j: SettleJ) -> HttpResult {
        let (rec, _) = self.final_record(round)?;
        let part = unhex(&j.part).map_err(bad)?;
        let sig = unhex_n::<64>(&j.sig).map_err(bad)?;
        let leg = verify_part(&rec, j.order, &part, &sig).map_err(|e| (403, e.to_string()))?;
        let s = self.settlements.entry(round).or_default();
        match s.status {
            SettleStatus::Assembling | SettleStatus::Settled { .. } => {
                return Err((
                    409,
                    format!("round {round} settlement is under way or done"),
                ))
            }
            SettleStatus::Failed { .. } | SettleStatus::Unknown { .. } => {
                s.status = SettleStatus::Collecting
            }
            SettleStatus::Collecting => {}
        }
        s.parts.insert(j.order, part);
        let have = s.parts.len();
        self.log(format!(
            "round {round}: settlement part for order {} verified ({have} held)",
            leg.order
        ));
        self.maybe_assemble(round);
        self.maybe_watch_leader(round);
        Ok(serde_json::json!({ "order": leg.order }))
    }

    /// As the round's leader, assemble again after a failed or unknown attempt, with the parts
    /// already held. Safe to repeat: the parts are bound to the signed record, and the chain
    /// refuses to spend the same notes twice, so at most one attempt can ever land.
    fn retry_settlement(&mut self, round: u64) -> HttpResult {
        self.final_record(round)?;
        if self.leader_of(round) != self.identity.id {
            return Err((
                409,
                format!(
                    "node {} is not the leader for round {round}; node {} is",
                    self.identity.id,
                    self.leader_of(round)
                ),
            ));
        }
        let s = self.settlements.entry(round).or_default();
        match s.status {
            SettleStatus::Failed { .. } | SettleStatus::Unknown { .. } => {
                s.status = SettleStatus::Collecting
            }
            _ => {
                return Err((
                    409,
                    format!("round {round} settlement has not failed; nothing to retry"),
                ))
            }
        }
        self.log(format!("round {round}: retrying the settlement"));
        self.maybe_assemble(round);
        self.settlement_view(round)
    }

    /// As the next node in turn, once every part is in, check on the leader later: a leader
    /// that was down when the round opened never learns it, and would hold it up for good.
    fn maybe_watch_leader(&mut self, round: u64) {
        if self.settle.is_none()
            || self.backup_of(round) != self.identity.id
            || self.leader_of(round) == self.identity.id
            || self.watched.contains(&round)
        {
            return;
        }
        let Ok((_, ls)) = self.final_record(round) else {
            return;
        };
        let s = self.settlements.entry(round).or_default();
        if ls.is_empty() || !ls.iter().all(|l| s.parts.contains_key(&l.order)) {
            return;
        }
        self.watched.insert(round);
        self.watches.push(round);
    }

    /// Take a round's settlement over from its leader, and assemble it if every part is in.
    fn take_over(&mut self, round: u64, why: &str) {
        let leader = self.leader_of(round);
        if !self.acting.insert(round) {
            return;
        }
        crate::log::event(
            "warn",
            &format!("node {}", self.identity.id),
            "settlement taken over",
            serde_json::json!({ "round": round, "leader": leader, "why": why }),
        );
        self.log(format!("round {round}: taking the settlement over ({why})"));
        self.maybe_assemble(round);
    }

    /// As the round's leader, once every leg has its part, queue the assembly.
    fn maybe_assemble(&mut self, round: u64) {
        if self.leader_of(round) != self.identity.id {
            return;
        }
        let Some(mut opts) = self.settle.clone() else {
            return;
        };
        if let Some(f) = &opts.assets_file {
            if let Ok(a) = crate::files::read_json::<AssetsJ>(f) {
                opts.assets = a;
            }
        }
        // what this node itself accepted and signed as a listing
        for (sym, id) in &self.listings {
            opts.assets.entry(sym.clone()).or_insert_with(|| id.clone());
        }
        let Ok((rec, ls)) = self.final_record(round) else {
            return;
        };
        let dir = self
            .data_dir
            .join(format!("settle-node{}-round{round}", self.identity.id));
        let s = self.settlements.entry(round).or_default();
        if ls.is_empty()
            || !matches!(s.status, SettleStatus::Collecting)
            || !ls.iter().all(|l| s.parts.contains_key(&l.order))
        {
            return;
        }
        let ids = market_assets(&rec.market)
            .and_then(|(b, q)| Some((opts.assets.get(b)?.clone(), opts.assets.get(q)?.clone())));
        let Some((base, quote)) = ids else {
            s.status = SettleStatus::Failed {
                error: format!("no asset ids for market {}", rec.market),
                bad: vec![],
            };
            return;
        };
        let parts = ls
            .iter()
            .map(|l| {
                let (sa, sn, ra, rn) = if l.sends_base() {
                    (&base, l.base, &quote, l.quote)
                } else {
                    (&quote, l.quote, &base, l.base)
                };
                let entry = serde_json::json!({
                    "label": format!("order {}", l.order),
                    "send_asset": sa, "send": sn, "recv_asset": ra, "recv": rn,
                });
                (
                    format!("order-{}.part", l.order),
                    s.parts[&l.order].clone(),
                    entry,
                )
            })
            .collect();
        s.status = SettleStatus::Assembling;
        self.jobs.push(Job {
            round,
            wallet: opts.wallet,
            dir,
            parts,
        });
        self.log(format!(
            "round {round}: every part is in; assembling the swap"
        ));
    }

    fn settlement_view(&mut self, round: u64) -> HttpResult {
        let (_, ls) = self.final_record(round)?;
        let leader = self.leader_of(round);
        let s = self.settlements.entry(round).or_default();
        let (status, txid, height, bad_orders, error) = match &s.status {
            _ if ls.is_empty() => ("nothing", None, None, vec![], None),
            SettleStatus::Collecting => ("collecting", None, None, vec![], None),
            SettleStatus::Assembling => ("assembling", None, None, vec![], None),
            SettleStatus::Settled { txid, height } => {
                ("settled", Some(txid.clone()), Some(*height), vec![], None)
            }
            SettleStatus::Failed { error, bad } => {
                ("failed", None, None, bad.clone(), Some(error.clone()))
            }
            SettleStatus::Unknown { error } => ("unknown", None, None, vec![], Some(error.clone())),
        };
        json(&SettlementJ {
            round_id: round,
            leader,
            legs: ls
                .iter()
                .map(|l| LegJ::new(l, s.parts.contains_key(&l.order)))
                .collect(),
            status: status.into(),
            txid,
            height,
            bad: bad_orders,
            error,
        })
    }

    // ---- company listings ---------------------------------------------------------------------

    fn sign_listing(&self, symbol: &str, asset_id: &str) -> Result<ListingSigJ, String> {
        let id = unhex_n::<32>(asset_id)?;
        let sig = self.identity.sign.sign(&listing_msg(symbol, &id));
        Ok(ListingSigJ {
            symbol: symbol.to_string(),
            asset_id: asset_id.to_string(),
            node: self.identity.id,
            sig: hx(&sig.to_bytes()),
        })
    }

    /// Accept a listing: a symbol that is well formed and an asset id that is not already
    /// taken by another symbol, nor the symbol by another id. First come, first served; the
    /// same listing again is answered with the same signature.
    fn on_listing(&mut self, l: ListingJ) -> HttpResult {
        if !valid_symbol(&l.symbol) {
            return Err(bad("a symbol is 1 to 12 letters or digits"));
        }
        unhex_n::<32>(&l.asset_id).map_err(bad)?;
        let asset_id = l.asset_id.to_lowercase();
        match self.listings.get(&l.symbol) {
            Some(have) if *have != asset_id => {
                return Err((
                    409,
                    format!("{} is already listed as another asset", l.symbol),
                ))
            }
            Some(_) => {}
            None => {
                if let Some((other, _)) = self.listings.iter().find(|(_, a)| **a == asset_id) {
                    return Err((409, format!("that asset is already listed as {other}")));
                }
                // the file first: a listing that could not be saved is not remembered, and so
                // not signed, rather than signed now and forgotten after a restart
                let mut next = self.listings.clone();
                next.insert(l.symbol.clone(), asset_id.clone());
                write_json(
                    &listings_path(&self.data_dir, self.identity.id),
                    &next,
                    false,
                )
                .map_err(|e| (500, e))?;
                self.listings = next;
                self.log(format!("listed {} as {}", l.symbol, short_hex(&asset_id)));
            }
        }
        json(&self.sign_listing(&l.symbol, &asset_id).map_err(bad)?)
    }

    fn route(&mut self, method: &Method, path: &str, body: &str) -> HttpResult {
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let parse =
            |s: &str| -> Result<u64, (u16, String)> { s.parse().map_err(|_| bad("bad round id")) };
        let from_body =
            |b: &str| -> Result<Value, (u16, String)> { serde_json::from_str(b).map_err(bad) };
        match (method, parts.as_slice()) {
            (Method::Get, ["v1", "status"]) => json(&StatusJ {
                id: self.identity.id,
                dkg_done: self.node.is_some(),
                group: self.group.as_ref().map(GroupJ::from_group),
            }),
            (Method::Post, ["v1", "dkg", "round1"]) => {
                self.on_dkg1(serde_json::from_value(from_body(body)?).map_err(bad)?)
            }
            (Method::Post, ["v1", "dkg", "round2"]) => {
                self.on_dkg2(serde_json::from_value(from_body(body)?).map_err(bad)?)
            }
            (Method::Post, ["v1", "gossip"]) => {
                self.on_gossip(serde_json::from_value(from_body(body)?).map_err(bad)?)
            }
            (Method::Post, ["v1", "rounds"]) => {
                let c: ConfigJ = serde_json::from_value(from_body(body)?).map_err(bad)?;
                self.node()?.open_round(c.config()).map_err(bad)?;
                self.log(format!("round {} opened ({})", c.round_id, c.market));
                self.advance(c.round_id);
                Ok(Value::Null)
            }
            (Method::Post, ["v1", "rounds", r, "submit"]) => {
                let round = parse(r)?;
                let s: SubmitJ = serde_json::from_value(from_body(body)?).map_err(bad)?;
                let bytes = unhex(&s.envelope).map_err(bad)?;
                let receipt = self.node()?.submit(round, &bytes).map_err(|e| match e {
                    MatcherError::WrongPhase { .. } => (409, e.to_string()),
                    MatcherError::UnknownRound(_) => (404, e.to_string()),
                    other => bad(other),
                })?;
                self.gossip(Gossip::Envelope {
                    round_id: round,
                    envelope: s.envelope.clone(),
                });
                let rj = ReceiptJ::from(&receipt);
                self.gossip(Gossip::Receipt(rj.clone()));
                json(&rj)
            }
            (Method::Post, ["v1", "rounds", r, "close"]) => {
                let round = parse(r)?;
                // Idempotent: a node may already be past "open" because `t` peers committed
                // before this request arrived. That is success, not an error.
                let phase = self
                    .node()?
                    .phase(round)
                    .map_err(|e| (404, e.to_string()))?;
                if phase != Phase::Open {
                    return json(&serde_json::json!({ "phase": phase.name() }));
                }
                let vote = self
                    .node()?
                    .close(round)
                    .map_err(|e| (409, e.to_string()))?;
                self.log(format!(
                    "round {round}: closed; voted {} ({} orders)",
                    if vote.voided { "to void" } else { "to commit" },
                    vote.set.len()
                ));
                self.gossip(Gossip::Commit(CommitVoteJ::from(&vote)));
                self.advance(round);
                Ok(Value::Null)
            }
            (Method::Get, ["v1", "rounds"]) => {
                let list: Vec<RoundSummaryJ> = self
                    .node()?
                    .rounds()
                    .into_iter()
                    .map(|(cfg, phase, orders)| RoundSummaryJ {
                        round_id: cfg.round_id,
                        market: cfg.market,
                        min_orders: cfg.min_orders,
                        phase: phase.name().into(),
                        orders,
                    })
                    .collect();
                json(&list)
            }
            (Method::Get, ["v1", "rounds", r]) => {
                let round = parse(r)?;
                let node = self.node()?;
                let phase = node.phase(round).map_err(|e| (404, e.to_string()))?;
                json(&RoundStatusJ {
                    round_id: round,
                    phase: phase.name().into(),
                    record: node.final_record(round).map(RecordJ::from),
                })
            }
            (Method::Get, ["v1", "rounds", r, "public"]) => {
                let round = parse(r)?;
                let p = self
                    .node()?
                    .export(round)
                    .map_err(|e| (404, e.to_string()))?;
                json(&PublicRoundJ::from(&p))
            }
            (Method::Post, ["v1", "rounds", r, "settle"]) => {
                let round = parse(r)?;
                self.on_settle(
                    round,
                    serde_json::from_value(from_body(body)?).map_err(bad)?,
                )
            }
            (Method::Get, ["v1", "rounds", r, "settlement"]) => {
                let round = parse(r)?;
                self.settlement_view(round)
            }
            (Method::Post, ["v1", "rounds", r, "settle", "retry"]) => {
                let round = parse(r)?;
                self.retry_settlement(round)
            }
            (Method::Post, ["v1", "listings"]) => {
                self.on_listing(serde_json::from_value(from_body(body)?).map_err(bad)?)
            }
            (Method::Get, ["v1", "listings"]) => {
                let all: Vec<ListingSigJ> = self
                    .listings
                    .iter()
                    .map(|(s, a)| self.sign_listing(s, a))
                    .collect::<Result<_, _>>()
                    .map_err(bad)?;
                json(&all)
            }
            _ => Err((404, "no such endpoint".into())),
        }
    }
}

/// Send each message in its own thread, retrying while the peer is unreachable (it may not be
/// up yet). A peer that answers with an error has rejected the message; that is not retried.
fn dispatch(id: u16, outbox: Vec<(String, Value)>) {
    for (url, body) in outbox {
        thread::spawn(move || {
            for _ in 0..480 {
                match ureq::post(&url)
                    .timeout(Duration::from_secs(10))
                    .send_json(body.clone())
                {
                    Ok(_) => return,
                    Err(ureq::Error::Status(code, resp)) => {
                        // a rejected message (a vote, a partial) can stall a round: always say so
                        let msg = resp.into_string().unwrap_or_default();
                        crate::log::event(
                            if code >= 500 { "error" } else { "warn" },
                            &format!("node {id}"),
                            "send a message to a peer",
                            serde_json::json!({ "url": url, "status": code, "error": msg }),
                        );
                        return;
                    }
                    Err(_) => thread::sleep(Duration::from_millis(250)),
                }
            }
            crate::log::event(
                "error",
                &format!("node {id}"),
                "send a message to a peer",
                serde_json::json!({ "url": url, "error": "gave up after 480 attempts; the peer never answered" }),
            );
        });
    }
}

/// Run one assembly, then record the outcome. Parts that did not match their legs are dropped
/// so their owners can hand in new ones; the others are kept.
/// Wait, then ask the round's leader how its settlement stands. Take it over if the leader
/// does not answer, does not have the round, or has not started although every part was
/// handed to every node. Two nodes assembling the same round cannot both land: the chain
/// refuses to spend the same notes twice.
fn watch_leader(st: Arc<Mutex<State>>, round: u64) {
    let (url, wait) = {
        let s = st.lock().expect("node state lock");
        let lead = s.leader_of(round);
        let Ok(node) = s.roster_j.node(lead) else {
            return;
        };
        let wait = s.settle.as_ref().map(|o| o.takeover_after);
        (format!("{}/v1/rounds/{round}/settlement", node.url), wait)
    };
    let Some(wait) = wait else { return };
    thread::sleep(wait);
    let why = match ureq::get(&url).timeout(Duration::from_secs(5)).call() {
        Ok(r) => match r.into_json::<SettlementJ>() {
            Ok(v) if v.status == "collecting" => Some("the leader has not started it".to_string()),
            Ok(_) => None,
            Err(e) => Some(format!("the leader's answer is unreadable: {e}")),
        },
        Err(ureq::Error::Status(code, _)) => Some(format!(
            "the leader answers {code}: it does not have the round"
        )),
        Err(e) => Some(format!("the leader does not answer: {e}")),
    };
    let Some(why) = why else { return };
    let jobs = {
        let mut s = st.lock().expect("node state lock");
        s.take_over(round, &why);
        std::mem::take(&mut s.jobs)
    };
    for job in jobs {
        let st = st.clone();
        thread::spawn(move || run_job(st, job));
    }
}

fn run_job(st: Arc<Mutex<State>>, job: Job) {
    let outcome = (|| -> Result<Assembled, String> {
        std::fs::create_dir_all(&job.dir).map_err(|e| e.to_string())?;
        let mut plan = Vec::new();
        for (file, bytes, mut entry) in job.parts {
            let path = job.dir.join(file);
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            entry["file"] = Value::String(path.display().to_string());
            plan.push(entry);
        }
        let plan_path = job.dir.join("plan.json");
        std::fs::write(&plan_path, serde_json::json!({ "legs": plan }).to_string())
            .map_err(|e| e.to_string())?;
        job.wallet.assemble(&plan_path)
    })();
    let mut s = st.lock().expect("node state lock");
    let round = job.round;
    let entry = s.settlements.entry(round).or_default();
    let msg = match outcome {
        Ok(Assembled::Settled { txid, height }) => {
            let m = format!("round {round}: SETTLED in one transaction {txid} (block {height})");
            entry.status = SettleStatus::Settled { txid, height };
            m
        }
        Ok(Assembled::BadParts(labels)) => {
            let bad: Vec<u32> = labels
                .iter()
                .filter_map(|l| l.strip_prefix("order ")?.parse().ok())
                .collect();
            for o in &bad {
                entry.parts.remove(o);
            }
            entry.status = SettleStatus::Failed {
                error: "some parts do not match their legs".into(),
                bad: bad.clone(),
            };
            format!("round {round}: settlement refused; parts that do not match: {bad:?}")
        }
        // After an unknown attempt, a rejection may just mean the first attempt already landed
        // (its notes are spent): still unknown, not "nothing moved".
        Ok(Assembled::Rejected(e)) if entry.was_unknown => {
            let error = format!(
                "a retry was rejected ({e}); an earlier attempt may already have settled: check balances"
            );
            entry.status = SettleStatus::Unknown {
                error: error.clone(),
            };
            format!("round {round}: settlement outcome UNKNOWN: {error}")
        }
        Ok(Assembled::Rejected(e)) => {
            entry.status = SettleStatus::Failed {
                error: e.clone(),
                bad: vec![],
            };
            format!("round {round}: settlement failed: {e}")
        }
        // No result from the tool: the transaction may or may not have gone out. Not "failed".
        Err(e) => {
            entry.was_unknown = true;
            entry.status = SettleStatus::Unknown { error: e.clone() };
            format!(
                "round {round}: settlement outcome UNKNOWN (check balances before retrying): {e}"
            )
        }
    };
    // a settlement's outcome is always recorded, verbose or not: confirmed, failed or unknown
    let (level, state) = match &entry.status {
        SettleStatus::Settled { .. } => ("info", "confirmed"),
        SettleStatus::Unknown { .. } => ("error", "unknown"),
        _ => ("error", "failed"),
    };
    crate::log::event(
        level,
        &format!("node {}", s.identity.id),
        "assemble the round's settlement",
        serde_json::json!({ "round": round, "outcome": state, "detail": msg }),
    );
}

fn respond(req: Request, result: HttpResult) {
    let (code, body) = match result {
        Ok(v) => (200, v.to_string()),
        Err((code, msg)) => (
            code,
            serde_json::to_string(&ErrorJ { error: msg }).unwrap_or_default(),
        ),
    };
    let header = Header::from_bytes("Content-Type", "application/json").expect("valid header");
    let _ = req.respond(
        Response::from_string(body)
            .with_status_code(code)
            .with_header(header),
    );
}

/// Start a node on an already-bound HTTP server (bind first so tests can use port 0).
pub fn spawn(server: tiny_http::Server, opts: NodeOptions) -> Result<NodeHandle, String> {
    let roster = opts.roster.roster()?;
    let entry = opts.roster.node(opts.identity.id)?.clone();
    if entry.sign_pk != hx(opts.identity.sign.verifying_key().as_bytes())
        || entry.dkg_pk != hx((G * opts.identity.dkg).compress().as_bytes())
    {
        return Err("this node's keys do not match its roster entry".into());
    }
    let addr = server
        .server_addr()
        .to_ip()
        .ok_or("server is not listening on an IP address")?;
    let id = opts.identity.id;
    let verbose = opts.verbose;
    let mut state = State {
        identity: opts.identity,
        roster_j: opts.roster,
        roster,
        data_dir: opts.data_dir,
        verbose,
        dkg: None,
        node: None,
        group: None,
        released: BTreeSet::new(),
        listings: AssetsJ::new(),
        pending: Vec::new(),
        outbox: Vec::new(),
        settle: opts.settle,
        settlements: BTreeMap::new(),
        jobs: Vec::new(),
        acting: BTreeSet::new(),
        watches: Vec::new(),
        watched: BTreeSet::new(),
    };
    // a missing file is "no listings yet"; one that exists but cannot be read stops the node,
    // or it would forget what it has signed and could sign a conflicting listing
    state.listings = read_json_or_default::<AssetsJ>(&listings_path(&state.data_dir, id))?;
    let share_file = share_path(&state.data_dir, id);
    if share_file.exists() {
        let (key, group) = load_share(&share_file)?;
        state.node = Some(
            MatcherNode::new(
                id,
                state.identity.sign.clone(),
                key,
                group.clone(),
                state.roster.clone(),
            )
            .map_err(|e| e.to_string())?,
        );
        state.group = Some(group);
        state.log("loaded key share from disk");
    } else {
        state.start_dkg()?;
    }
    let first = std::mem::take(&mut state.outbox);
    let shared = Arc::new(Mutex::new(state));
    let server = Arc::new(server);
    let stop = Arc::new(AtomicBool::new(false));
    dispatch(id, first);

    let (srv, stp, st) = (server.clone(), stop.clone(), shared.clone());
    let thread = thread::spawn(move || {
        while !stp.load(Ordering::SeqCst) {
            let Ok(mut req) = srv.recv() else { break };
            let mut body = String::new();
            if req
                .as_reader()
                .take(4 << 20)
                .read_to_string(&mut body)
                .is_err()
            {
                respond(req, Err(bad("could not read body")));
                continue;
            }
            let method = req.method().clone();
            let path = req.url().split('?').next().unwrap_or("").to_string();
            let (result, outbox, jobs, watches) = {
                let mut s = st.lock().expect("node state lock");
                let r = s.route(&method, &path, &body);
                (
                    r,
                    std::mem::take(&mut s.outbox),
                    std::mem::take(&mut s.jobs),
                    std::mem::take(&mut s.watches),
                )
            };
            dispatch(id, outbox);
            for job in jobs {
                let st = st.clone();
                thread::spawn(move || run_job(st, job));
            }
            for round in watches {
                let st = st.clone();
                thread::spawn(move || watch_leader(st, round));
            }
            respond(req, result);
        }
    });
    Ok(NodeHandle {
        addr,
        server,
        stop,
        thread: Some(thread),
    })
}

fn short_hex(s: &str) -> &str {
    &s[..s.len().min(8)]
}

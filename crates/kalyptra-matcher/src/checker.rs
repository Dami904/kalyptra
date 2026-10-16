//! The public checker: anyone can verify a round from its published data alone.
//!
//! It re-derives everything and trusts no matcher:
//!
//! 1. every receipt, commit vote and record vote carries a valid roster signature;
//! 2. `t` nodes signed the same final committed set (or the same decision to void);
//! 3. **no censorship:** every envelope with receipts from `t` nodes is in the committed set,
//!    and nothing in the set lacks `t` receipts;
//! 4. every committed envelope is published, its sealer proof verifies for this round, and its
//!    published partials open it;
//! 5. the auction, re-run on the opened orders, gives exactly the published record;
//! 6. `t` nodes signed that record;
//! 7. a voided round publishes no partials and no record.
//!
//! Privacy note: publishing partials lets anyone open the committed envelopes after the
//! round. That is by design (the price must be checkable), so **investors must use a fresh
//! signing key for every order**; then an opened order reveals its side, size and limit but
//! not who placed it.

use std::collections::{BTreeMap, BTreeSet};

use kalyptra_tse::{open, verify_envelope, Envelope, GroupKey, PartialDecryption};

use crate::node::build_record;
use crate::order::round_aad;
use crate::round::{
    envelope_hash, short, CommitVote, Hash, Receipt, RecordVote, Roster, RoundConfig, RoundRecord,
};

/// A round's published data.
#[derive(Clone, Debug)]
pub struct PublicRound {
    pub config: RoundConfig,
    pub envelopes: Vec<Vec<u8>>,
    pub receipts: Vec<Receipt>,
    pub commit_votes: Vec<CommitVote>,
    pub partials: Vec<(Hash, PartialDecryption)>,
    pub record: Option<RoundRecord>,
    pub record_votes: Vec<RecordVote>,
}

/// Something the checker found wrong. A clean round has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    BadSignature {
        node: u16,
        what: &'static str,
    },
    Equivocation {
        node: u16,
        what: &'static str,
    },
    NoFinalCommit,
    /// Receipts from `t` nodes prove this envelope was accepted, yet it is not committed.
    Censored {
        envelope: String,
        receipted_by: Vec<u16>,
    },
    /// In the committed set without receipts from `t` nodes (injected after the fact).
    Unreceipted {
        envelope: String,
    },
    MissingEnvelope {
        envelope: String,
    },
    BadEnvelope {
        envelope: String,
    },
    CannotOpen {
        envelope: String,
    },
    PartialsForVoidedRound,
    RecordForVoidedRound,
    NoRecord,
    /// The published record differs from the one recomputed from the opened orders.
    RecordMismatch {
        field: &'static str,
    },
    RecordNotFinal,
    /// A commit vote whose set is unsorted or repeats an envelope (it would count an order
    /// twice).
    MalformedCommit {
        node: u16,
    },
    /// Two different commits each gathered `t` votes. Possible only when `2t <= n`.
    ConflictingCommits,
    /// The committed round was voided though it had enough orders, or was not voided though
    /// it had too few.
    WrongVoidDecision {
        orders: usize,
        min_orders: u32,
        voided: bool,
    },
    /// The published round configuration differs from the one `t` nodes signed.
    ConfigMismatch,
    /// The final commit is from a later epoch, but an earlier epoch could still have reached
    /// `t` matching votes, so the nodes had no right to vote again.
    PrematureRevote {
        epoch: u32,
    },
}

#[derive(Debug)]
pub struct CheckReport {
    pub findings: Vec<Finding>,
    /// The record the checker computed itself, when it got that far.
    pub recomputed: Option<RoundRecord>,
    pub voided: bool,
}

impl CheckReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

pub fn check(public: &PublicRound, group: &GroupKey, roster: &Roster) -> CheckReport {
    let t = roster.threshold;
    let round = public.config.round_id;
    let aad = round_aad(round, &public.config.market);
    let mut f = Vec::new();
    let report =
        |findings: Vec<Finding>, recomputed: Option<RoundRecord>, voided: bool| CheckReport {
            findings,
            recomputed,
            voided,
        };

    // 1. signatures; one receipt per (node, envelope); one vote per node
    let mut receipts: BTreeMap<Hash, BTreeSet<u16>> = BTreeMap::new();
    for r in public.receipts.iter().filter(|r| r.round_id == round) {
        if r.verify(roster).is_ok() {
            receipts.entry(r.envelope).or_default().insert(r.node);
        } else {
            f.push(Finding::BadSignature {
                node: r.node,
                what: "receipt",
            });
        }
    }
    let mut commit_votes: BTreeMap<(u32, u16), &CommitVote> = BTreeMap::new();
    for v in public.commit_votes.iter().filter(|v| v.round_id == round) {
        if v.verify(roster).is_err() {
            f.push(Finding::BadSignature {
                node: v.node,
                what: "commit vote",
            });
            continue;
        }
        if v.well_formed().is_err() {
            f.push(Finding::MalformedCommit { node: v.node });
            continue;
        }
        if let Some(prev) = commit_votes.insert((v.epoch, v.node), v) {
            if prev != v {
                f.push(Finding::Equivocation {
                    node: v.node,
                    what: "commit vote",
                });
            }
        }
    }

    // 2. final commit
    // The first epoch in which `t` nodes signed the same thing decides; every earlier epoch must
    // have been unable to finish (see `MatcherNode::revote`).
    let n = roster.nodes.len();
    // every epoch up to the highest one voted in, including one nobody voted in: skipping an
    // epoch is as premature as leaving one that could still have won
    let epochs = 0..=commit_votes.keys().map(|(e, _)| *e).max().unwrap_or(0);
    let mut decided: Option<(u32, &CommitVote)> = None;
    for epoch in epochs {
        let mut tally: BTreeMap<(Hash, bool, u32), Vec<&CommitVote>> = BTreeMap::new();
        for v in commit_votes.values().filter(|v| v.epoch == epoch) {
            tally.entry(v.key()).or_default().push(v);
        }
        let mut winners = tally.values().filter(|vs| vs.len() >= t);
        if let Some(w) = winners.next() {
            if winners.next().is_some() {
                f.push(Finding::ConflictingCommits);
            }
            decided = Some((epoch, w[0]));
            break;
        }
        let best = tally.values().map(Vec::len).max().unwrap_or(0);
        let seen: usize = tally.values().map(Vec::len).sum();
        if best + n.saturating_sub(seen) >= t {
            // this epoch could still have finished, yet a later one was used
            f.push(Finding::PrematureRevote { epoch });
        }
    }
    // A later epoch that also gathered `t` votes for something else would be a second final
    // commit: the nodes that signed it had no right to vote again.
    if let Some((won, w)) = decided {
        for epoch in commit_votes.keys().map(|(e, _)| *e).filter(|e| *e > won) {
            let mut tally: BTreeMap<(Hash, bool, u32), usize> = BTreeMap::new();
            for v in commit_votes.values().filter(|v| v.epoch == epoch) {
                *tally.entry(v.key()).or_default() += 1;
            }
            if tally.iter().any(|(k, c)| *c >= t && *k != w.key()) {
                f.push(Finding::ConflictingCommits);
                break;
            }
        }
    }
    let Some((_, winner)) = decided else {
        // a premature-revote finding above is only meaningful with a decision after it
        f.retain(|x| !matches!(x, Finding::PrematureRevote { .. }));
        f.push(Finding::NoFinalCommit);
        return report(f, None, false);
    };
    if winner.min_orders as usize != public.config.min_orders {
        f.push(Finding::ConfigMismatch);
    }
    if !winner.void_is_consistent() {
        f.push(Finding::WrongVoidDecision {
            orders: winner.set.len(),
            min_orders: winner.min_orders,
            voided: winner.voided,
        });
    }
    let set: Vec<Hash> = winner.set.clone();
    let in_set: BTreeSet<Hash> = set.iter().copied().collect();

    // 3. censorship and injection
    for (h, by) in &receipts {
        if by.len() >= t && !in_set.contains(h) {
            f.push(Finding::Censored {
                envelope: short(h),
                receipted_by: by.iter().copied().collect(),
            });
        }
    }
    for h in &set {
        if receipts.get(h).map_or(0, |b| b.len()) < t {
            f.push(Finding::Unreceipted { envelope: short(h) });
        }
    }

    // 7. voided round
    if winner.voided {
        if !public.partials.is_empty() {
            f.push(Finding::PartialsForVoidedRound);
        }
        if public.record.is_some() {
            f.push(Finding::RecordForVoidedRound);
        }
        return report(f, None, true);
    }

    // 4. open every committed envelope
    let published: BTreeMap<Hash, &Vec<u8>> = public
        .envelopes
        .iter()
        .map(|b| (envelope_hash(b), b))
        .collect();
    let mut partials: BTreeMap<Hash, Vec<PartialDecryption>> = BTreeMap::new();
    for (h, p) in &public.partials {
        partials.entry(*h).or_default().push(*p);
    }
    let mut plaintexts = Vec::with_capacity(set.len());
    for h in &set {
        let Some(bytes) = published.get(h) else {
            f.push(Finding::MissingEnvelope { envelope: short(h) });
            return report(f, None, false);
        };
        let env = match Envelope::from_bytes(bytes) {
            Ok(e) if verify_envelope(group.public_key(), &e, &aad).is_ok() => e,
            _ => {
                f.push(Finding::BadEnvelope { envelope: short(h) });
                return report(f, None, false);
            }
        };
        match open(group, &env, &aad, partials.get(h).map_or(&[][..], |v| v)) {
            Ok(o) => plaintexts.push(o.plaintext),
            Err(_) => {
                f.push(Finding::CannotOpen { envelope: short(h) });
                return report(f, None, false);
            }
        }
    }

    // 5. recompute and compare
    let recomputed = build_record(&public.config, &set, &plaintexts);
    match &public.record {
        None => f.push(Finding::NoRecord),
        Some(rec) => {
            let fields: [(&'static str, bool); 6] = [
                ("set", rec.set == recomputed.set),
                ("orders", rec.orders == recomputed.orders),
                ("invalid", rec.invalid == recomputed.invalid),
                ("price", rec.price == recomputed.price),
                ("volume", rec.volume == recomputed.volume),
                ("fills", rec.fills == recomputed.fills),
            ];
            for (name, ok) in fields {
                if !ok {
                    f.push(Finding::RecordMismatch { field: name });
                }
            }
            if rec.round_id != round || rec.market != public.config.market {
                f.push(Finding::RecordMismatch { field: "round" });
            }
            // 6. t valid signatures on the published record
            let h = rec.hash();
            let mut signers = BTreeSet::new();
            let mut signed: BTreeMap<u16, Hash> = BTreeMap::new();
            for v in public.record_votes.iter().filter(|v| v.round_id == round) {
                if v.verify(roster).is_err() {
                    f.push(Finding::BadSignature {
                        node: v.node,
                        what: "record vote",
                    });
                    continue;
                }
                if let Some(prev) = signed.insert(v.node, v.record) {
                    if prev != v.record {
                        f.push(Finding::Equivocation {
                            node: v.node,
                            what: "record vote",
                        });
                    }
                }
                if v.record == h {
                    signers.insert(v.node);
                }
            }
            if signers.len() < t {
                f.push(Finding::RecordNotFinal);
            }
        }
    }
    report(f, Some(recomputed), false)
}

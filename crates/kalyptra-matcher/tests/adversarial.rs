//! Attacks by cheating nodes and tampered public data. Every test states what a cheater
//! tries and asserts it is refused or caught.
//!
//! `ATTACKS.md` maps each attack to the test that covers it. The two randomized tests at the
//! end check one invariant over hundreds of tampered rounds: **a clean check always means the
//! honest result.**

mod common;

use common::{cfg, Net, MARKET};
use ed25519_dalek::SigningKey;
use kalyptra_matcher::round::envelope_hash;
use kalyptra_matcher::{
    check, CommitVote, Finding, MatcherError, MatcherNode, Phase, PublicRound, RecordVote, Roster,
    RoundConfig, RoundRecord, Side,
};
use kalyptra_tse::{run_local_dkg, Envelope};
use rand_core::OsRng;

const R: u64 = 1;

/// Four orders that clear at 5, volume 14. Returns the sealed envelopes.
fn four(net: &mut Net) -> Vec<Vec<u8>> {
    let orders = vec![
        net.sealed(R, Side::Sell, 10, 4),
        net.sealed(R, Side::Sell, 4, 5),
        net.sealed(R, Side::Buy, 8, 6),
        net.sealed(R, Side::Buy, 6, 5),
    ];
    for o in &orders {
        net.submit_all(R, o);
    }
    orders
}

fn hashes(orders: &[Vec<u8>]) -> Vec<kalyptra_matcher::Hash> {
    orders.iter().map(|o| envelope_hash(o)).collect()
}

/// Nodes 1 and 2 collude: they hold `t = 2` signing keys.
fn colluders(net: &Net) -> [SigningKey; 2] {
    [net.keys[0].clone(), net.keys[1].clone()]
}

fn has(report: &kalyptra_matcher::CheckReport, f: &Finding) -> bool {
    report.findings.contains(f)
}

/// A cleared honest round: its public data and its record.
fn honest() -> (Net, PublicRound, RoundRecord) {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four(&mut net);
    net.run_all(R);
    let public = net.public(R);
    let record = public.record.clone().expect("record");
    assert!(check(&public, &net.group, &net.roster).is_clean());
    (net, public, record)
}

// --- committed-set attacks ---------------------------------------------------------------------

#[test]
fn a_duplicated_envelope_in_the_committed_set_is_refused_and_flagged() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = four(&mut net);
    let mut set = hashes(&orders);
    set.push(set[0]); // the first sell counted twice: double weight in the auction
    let [k1, k2] = colluders(&net);
    let votes = [
        CommitVote::new(&k1, 1, R, set.clone(), 2, false),
        CommitVote::new(&k2, 2, R, set, 2, false),
    ];
    for v in &votes {
        assert!(
            matches!(
                net.node(3).learn_commit_vote(v),
                Err(MatcherError::BadCommit(_))
            ),
            "node 3 accepted a set with a duplicate"
        );
    }
    let mut public = net.public(R);
    public.commit_votes = votes.to_vec();
    let report = check(&public, &net.group, &net.roster);
    assert!(has(&report, &Finding::MalformedCommit { node: 1 }));
    assert!(has(&report, &Finding::NoFinalCommit));
}

#[test]
fn an_unsorted_committed_set_is_refused_and_flagged() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = four(&mut net);
    // a validly signed vote whose set is then shuffled in transit
    let mut v = CommitVote::new(&net.keys[0], 1, R, hashes(&orders), 2, false);
    v.set.reverse();
    assert!(
        v.verify(&net.roster).is_ok(),
        "the signature covers the set, not its order"
    );
    assert!(matches!(
        net.node(3).learn_commit_vote(&v),
        Err(MatcherError::BadCommit(_))
    ));
    let mut public = net.public(R);
    public.commit_votes = vec![v];
    let report = check(&public, &net.group, &net.roster);
    assert!(has(&report, &Finding::MalformedCommit { node: 1 }));
}

#[test]
fn t_colluding_nodes_voiding_a_viable_round_are_caught() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = four(&mut net);
    let set = hashes(&orders);
    let [k1, k2] = colluders(&net);
    let votes = [
        CommitVote::new(&k1, 1, R, set.clone(), 2, true),
        CommitVote::new(&k2, 2, R, set, 2, true),
    ];
    // an honest node does not go along
    assert!(matches!(
        net.node(3).learn_commit_vote(&votes[0]),
        Err(MatcherError::BadCommit(_))
    ));
    // and in public data the lie is visible
    let mut public = net.public(R);
    public.commit_votes = votes.to_vec();
    let report = check(&public, &net.group, &net.roster);
    assert!(has(
        &report,
        &Finding::WrongVoidDecision {
            orders: 4,
            min_orders: 2,
            voided: true
        }
    ));
}

#[test]
fn t_colluding_nodes_committing_a_lone_order_are_caught() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = four(&mut net);
    let [k1, k2] = colluders(&net);
    let lone = vec![envelope_hash(&orders[0])];
    let votes = [
        CommitVote::new(&k1, 1, R, lone.clone(), 2, false),
        CommitVote::new(&k2, 2, R, lone, 2, false),
    ];
    assert!(matches!(
        net.node(3).learn_commit_vote(&votes[0]),
        Err(MatcherError::BadCommit(_))
    ));
    let mut public = net.public(R);
    public.commit_votes = votes.to_vec();
    let report = check(&public, &net.group, &net.roster);
    assert!(has(
        &report,
        &Finding::WrongVoidDecision {
            orders: 1,
            min_orders: 2,
            voided: false
        }
    ));
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::Censored { .. })),
        "the three dropped receipted orders are proven censored too"
    );
}

#[test]
fn a_commit_signing_a_different_minimum_is_refused() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = four(&mut net);
    // minimum 1 instead of 2: would let a single-order round open
    let v = CommitVote::new(&net.keys[0], 1, R, hashes(&orders), 1, false);
    assert!(matches!(
        net.node(3).learn_commit_vote(&v),
        Err(MatcherError::BadCommit(_))
    ));
}

#[test]
fn a_published_round_with_a_relabelled_minimum_is_flagged() {
    let (net, mut public, _) = honest();
    public.config.min_orders = 1; // the signed votes say 2
    let report = check(&public, &net.group, &net.roster);
    assert!(has(&report, &Finding::ConfigMismatch));
}

#[test]
fn two_conflicting_commits_cannot_both_be_final() {
    // 2-of-4 has no honest majority: nodes {1,2} and {3,4} could each be "t votes".
    let (shares, group) = run_local_dkg(4, 2, &mut OsRng).unwrap();
    let keys: Vec<SigningKey> = (0..4).map(|_| SigningKey::generate(&mut OsRng)).collect();
    let roster = Roster {
        threshold: 2,
        nodes: keys
            .iter()
            .enumerate()
            .map(|(i, k)| (i as u16 + 1, k.verifying_key()))
            .collect(),
    };
    let share = shares.into_iter().next().unwrap();
    let node = MatcherNode::new(
        share.id(),
        keys[0].clone(),
        share,
        group.clone(),
        roster.clone(),
    );
    assert!(
        matches!(node, Err(MatcherError::Config(_))),
        "a node refuses a threshold of half the nodes or less"
    );
    // and if such a roster were used anyway, the checker flags the split
    let a = vec![[1u8; 32]];
    let votes = vec![
        CommitVote::new(&keys[0], 1, R, a.clone(), 1, false),
        CommitVote::new(&keys[1], 2, R, a, 1, false),
        CommitVote::new(&keys[2], 3, R, vec![], 1, true),
        CommitVote::new(&keys[3], 4, R, vec![], 1, true),
    ];
    let public = PublicRound {
        config: RoundConfig {
            round_id: R,
            market: MARKET.into(),
            min_orders: 1,
        },
        envelopes: vec![],
        receipts: vec![],
        commit_votes: votes,
        partials: vec![],
        record: None,
        record_votes: vec![],
    };
    let report = check(&public, &group, &roster);
    assert!(has(&report, &Finding::ConflictingCommits));
}

// --- vote attacks ----------------------------------------------------------------------------------

#[test]
fn a_node_signing_two_different_records_is_flagged() {
    let (net, mut public, record) = honest();
    let mut other = record;
    other.volume += 1;
    public
        .record_votes
        .push(RecordVote::new(&net.keys[2], 3, &other));
    let report = check(&public, &net.group, &net.roster);
    assert!(has(
        &report,
        &Finding::Equivocation {
            node: 3,
            what: "record vote"
        }
    ));
}

#[test]
fn each_forged_record_field_is_caught_even_when_t_nodes_sign_it() {
    // The colluders sign the forged record, so the signature check cannot be what catches it:
    // only the checker's own recomputation can.
    type Forge = fn(&mut RoundRecord);
    let forgeries: [(&str, Forge); 6] = [
        ("price", |r| r.price = r.price.map(|p| p + 1)),
        ("volume", |r| r.volume += 1),
        ("fills", |r| r.fills[0].quantity += 1),
        ("orders", |r| r.orders[0].1.limit += 1),
        ("invalid", |r| r.invalid.push(99)),
        ("set", |r| r.set.reverse()),
    ];
    for (field, forge) in forgeries {
        let (net, mut public, record) = honest();
        let mut forged = record;
        forge(&mut forged);
        public.record = Some(forged.clone());
        public.record_votes = vec![
            RecordVote::new(&net.keys[0], 1, &forged),
            RecordVote::new(&net.keys[1], 2, &forged),
        ];
        let report = check(&public, &net.group, &net.roster);
        assert!(
            has(&report, &Finding::RecordMismatch { field }),
            "forged {field} passed: {:?}",
            report.findings
        );
        assert!(!has(&report, &Finding::RecordNotFinal));
    }
    // the same for a record that names another round or market
    for forge in [
        (|r: &mut RoundRecord| r.round_id += 1) as fn(&mut RoundRecord),
        |r| r.market.push('x'),
    ] {
        let (net, mut public, record) = honest();
        let mut forged = record;
        forge(&mut forged);
        public.record = Some(forged.clone());
        public.record_votes = vec![
            RecordVote::new(&net.keys[0], 1, &forged),
            RecordVote::new(&net.keys[1], 2, &forged),
        ];
        let report = check(&public, &net.group, &net.roster);
        assert!(has(&report, &Finding::RecordMismatch { field: "round" }));
    }
}

#[test]
fn a_checker_given_another_group_key_names_every_envelope_bad() {
    // Real envelopes were sealed to the real group key; their sealer proofs fail under any
    // other. A made-up key cannot make a round pass.
    let (net, public, _) = honest();
    let (_, other_group) = run_local_dkg(3, 2, &mut OsRng).unwrap();
    assert_ne!(other_group.public_key(), net.group.public_key());
    let report = check(&public, &other_group, &net.roster);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::BadEnvelope { .. })),
        "{:?}",
        report.findings
    );
    assert!(report.recomputed.is_none());
}

#[test]
fn a_record_published_for_a_voided_round_is_flagged() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 5));
    let o = net.sealed(R, Side::Buy, 1, 1);
    net.submit_all(R, &o);
    net.close_all(R);
    let mut public = net.public(R);
    assert!(check(&public, &net.group, &net.roster).voided);
    // a record appears for a round that was voided: nothing in it was ever opened
    public.record = Some(RoundRecord {
        round_id: R,
        market: MARKET.into(),
        set: vec![envelope_hash(&o)],
        orders: vec![],
        invalid: vec![],
        price: Some(3),
        volume: 1,
        fills: vec![],
    });
    let report = check(&public, &net.group, &net.roster);
    assert!(has(&report, &Finding::RecordForVoidedRound));
}

// --- stalling ----------------------------------------------------------------------------------------

#[test]
fn with_too_few_partials_the_round_stalls_visibly_and_reveals_nothing() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four(&mut net);
    net.close_all(R);
    // only node 1 releases: one partial, below t = 2
    net.release(R, &[1]);
    for id in [1, 2, 3] {
        assert!(
            matches!(net.node(id).clear(R), Err(MatcherError::CannotOpen { .. })),
            "node {id} cleared with a single node's partials"
        );
        assert_eq!(net.node(id).phase(R).unwrap(), Phase::Committed);
        assert!(net.node(id).final_record(R).is_none());
    }
    // the public checker says exactly why
    let public = net.public(R);
    let report = check(&public, &net.group, &net.roster);
    assert!(report
        .findings
        .iter()
        .any(|f| matches!(f, Finding::CannotOpen { .. })));
    assert!(report.recomputed.is_none(), "nothing was opened");
    // nothing leaked: the lone partials do not open any envelope
    for (h, _) in &public.partials {
        let bytes = public
            .envelopes
            .iter()
            .find(|e| envelope_hash(e) == *h)
            .unwrap();
        let env = Envelope::from_bytes(bytes).unwrap();
        let ps: Vec<_> = public
            .partials
            .iter()
            .filter(|(x, _)| x == h)
            .map(|(_, p)| *p)
            .collect();
        assert!(kalyptra_tse::open(
            &net.group,
            &env,
            &kalyptra_matcher::round_aad(R, MARKET),
            &ps
        )
        .is_err());
    }
    // liveness returns as soon as a second node releases
    net.release(R, &[2]);
    net.clear(R, &[1, 2, 3]);
    assert_eq!(net.node(1).final_record(R).unwrap().price, Some(5));
}

// --- replay ----------------------------------------------------------------------------------------------

#[test]
fn a_cleared_rounds_envelope_replayed_in_the_next_round_is_refused() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = four(&mut net);
    net.run_all(R);
    net.open_round(cfg(R + 1, 2));
    for n in 1..=3 {
        assert!(
            net.node(n).submit(R + 1, &orders[0]).is_err(),
            "node {n} accepted a replayed envelope"
        );
    }
}

// --- randomized tampering: a clean check means the honest result ---------------------------------

/// A tiny deterministic generator, so a failing case can be reproduced from its seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn assert_clean_means_honest(
    case: &str,
    seed: u64,
    report: &kalyptra_matcher::CheckReport,
    honest: &RoundRecord,
) {
    if report.is_clean() {
        assert_eq!(
            report.recomputed.as_ref(),
            Some(honest),
            "{case} (seed {seed}): the checker passed a result that differs from the honest one"
        );
        assert!(
            !report.voided,
            "{case} (seed {seed}): a clean check on a voided round"
        );
    }
}

#[test]
fn tampering_with_public_data_never_yields_a_clean_check_of_a_different_result() {
    let (net, public, record) = honest();
    let mut flagged = 0;
    let runs = 600;
    for seed in 1..=runs as u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut p = public.clone();
        // one to three random tamperings, applied together
        for _ in 0..1 + rng.below(3) {
            match rng.below(14) {
                0 if !p.receipts.is_empty() => {
                    let i = rng.below(p.receipts.len());
                    p.receipts.remove(i);
                }
                1 if !p.commit_votes.is_empty() => {
                    let i = rng.below(p.commit_votes.len());
                    p.commit_votes.remove(i);
                }
                2 if !p.partials.is_empty() => {
                    let i = rng.below(p.partials.len());
                    p.partials.remove(i);
                }
                3 if !p.record_votes.is_empty() => {
                    let i = rng.below(p.record_votes.len());
                    p.record_votes.remove(i);
                }
                4 if !p.envelopes.is_empty() => {
                    let i = rng.below(p.envelopes.len());
                    p.envelopes.remove(i);
                }
                5 if !p.envelopes.is_empty() => {
                    let i = rng.below(p.envelopes.len());
                    let e = p.envelopes[i].clone();
                    p.envelopes.push(e);
                }
                6 if !p.envelopes.is_empty() => {
                    let i = rng.below(p.envelopes.len());
                    let n = p.envelopes[i].len();
                    p.envelopes[i][rng.below(n)] ^= 1 << rng.below(8);
                }
                7 if p.partials.len() >= 2 => {
                    // two partials swap the envelopes they claim to be for
                    let (i, j) = (rng.below(p.partials.len()), rng.below(p.partials.len()));
                    let (hi, hj) = (p.partials[i].0, p.partials[j].0);
                    p.partials[i].0 = hj;
                    p.partials[j].0 = hi;
                }
                8 if !p.partials.is_empty() => {
                    let i = rng.below(p.partials.len());
                    let mut b = p.partials[i].1.to_bytes();
                    let n = b.len();
                    b[rng.below(n)] ^= 1 << rng.below(8);
                    if let Ok(x) = kalyptra_tse::PartialDecryption::from_bytes(&b) {
                        p.partials[i].1 = x;
                    }
                }
                9 => {
                    if let Some(r) = p.record.as_mut() {
                        match rng.below(6) {
                            0 => r.price = r.price.map(|x| x + 1 + rng.below(3) as u64),
                            1 => r.volume += 1,
                            2 if !r.fills.is_empty() => {
                                let i = rng.below(r.fills.len());
                                r.fills[i].quantity += 1;
                            }
                            3 if !r.orders.is_empty() => {
                                let i = rng.below(r.orders.len());
                                r.orders[i].1.limit += 1;
                            }
                            4 if !r.orders.is_empty() => {
                                let i = rng.below(r.orders.len());
                                r.orders[i].1.owner[0] ^= 1;
                            }
                            _ => r.orders.reverse(),
                        }
                    }
                }
                10 => p.record = None,
                11 => match rng.below(3) {
                    0 => p.config.min_orders += 1 + rng.below(3),
                    1 => p.config.market.push('x'),
                    _ => p.config.round_id += 1,
                },
                12 if !p.receipts.is_empty() => {
                    let i = rng.below(p.receipts.len());
                    p.receipts[i].envelope[rng.below(32)] ^= 1;
                }
                13 if !p.commit_votes.is_empty() => {
                    let i = rng.below(p.commit_votes.len());
                    match rng.below(3) {
                        0 => p.commit_votes[i].voided ^= true,
                        1 => p.commit_votes[i].min_orders += 1,
                        _ => {
                            let v = &mut p.commit_votes[i];
                            if !v.set.is_empty() {
                                let n = v.set.len();
                                v.set.remove(rng.below(n));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let report = check(&p, &net.group, &net.roster);
        if !report.is_clean() {
            flagged += 1;
        }
        assert_clean_means_honest("data tampering", seed, &report, &record);
    }
    // not vacuous: the tampering really did break most rounds
    assert!(
        flagged > runs / 2,
        "only {flagged} of {runs} tamperings were flagged"
    );
}

#[test]
fn t_colluding_nodes_signing_forged_votes_never_yield_a_clean_check_of_a_different_result() {
    let (net, public, record) = honest();
    let [k1, k2] = colluders(&net);
    let honest_set = record.set.clone();
    let mut flagged = 0;
    let runs = 600;
    for seed in 1..=runs as u64 {
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
        let mut p = public.clone();
        // Nodes 1 and 2 replace their honest commit votes with forged ones. Node 3's honest
        // vote stays in the public data.
        p.commit_votes.retain(|v| v.node == 3);
        let mut set: Vec<_> = honest_set
            .iter()
            .copied()
            .filter(|_| rng.below(4) != 0)
            .collect();
        match rng.below(4) {
            0 => {
                if let Some(h) = set.first().copied() {
                    set.push(h); // duplicate
                }
            }
            1 => set.push([rng.below(250) as u8 + 1; 32]), // an envelope nobody receipted
            _ => {}
        }
        let min_orders = 1 + rng.below(5);
        let voided = rng.below(2) == 0;
        p.commit_votes
            .push(CommitVote::new(&k1, 1, R, set.clone(), min_orders, voided));
        p.commit_votes
            .push(CommitVote::new(&k2, 2, R, set, min_orders, voided));
        // and, half the time, they also sign a forged record
        if rng.below(2) == 0 {
            let mut forged = record.clone();
            match rng.below(3) {
                0 => forged.price = Some(1 + rng.below(9) as u64),
                1 => forged.volume += 1 + rng.below(5) as u64,
                _ => forged.fills.clear(),
            }
            p.record = Some(forged.clone());
            p.record_votes.retain(|v| v.node == 3);
            p.record_votes.push(RecordVote::new(&k1, 1, &forged));
            p.record_votes.push(RecordVote::new(&k2, 2, &forged));
        }
        let report = check(&p, &net.group, &net.roster);
        if !report.is_clean() {
            flagged += 1;
        }
        assert_clean_means_honest("colluding forgery", seed, &report, &record);
    }
    assert!(
        flagged > runs / 2,
        "only {flagged} of {runs} forgeries were flagged"
    );
}

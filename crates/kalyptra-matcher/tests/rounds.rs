//! Whole rounds across three matcher nodes (2-of-3), honest and adversarial.

mod common;

use common::{cfg, Net, MARKET};
use kalyptra_matcher::{
    check, round_aad, CommitVote, Finding, MatcherError, Phase, Receipt, RecordVote, Side,
};
use kalyptra_tse::{partial_decrypt_unchecked_for_tests, Envelope};
use rand_core::OsRng;

const R: u64 = 1;

/// Four orders that clear at 5 with volume 14 (the G0 spike's round).
fn four_orders(net: &mut Net, round: u64) -> Vec<Vec<u8>> {
    let orders = vec![
        net.sealed(round, Side::Sell, 10, 4),
        net.sealed(round, Side::Sell, 4, 5),
        net.sealed(round, Side::Buy, 8, 6),
        net.sealed(round, Side::Buy, 6, 5),
    ];
    for o in &orders {
        net.submit_all(round, o);
    }
    orders
}

// --- the honest path ---------------------------------------------------------------------

#[test]
fn an_honest_round_clears_at_5_and_the_public_checker_agrees() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four_orders(&mut net, R);
    net.run_all(R);

    for n in &net.nodes {
        assert_eq!(n.phase(R).unwrap(), Phase::Cleared);
        let rec = n.final_record(R).expect("final record on every node");
        assert_eq!(rec.price, Some(5));
        assert_eq!(rec.volume, 14);
        assert_eq!(rec.orders.len(), 4);
        assert!(rec.invalid.is_empty());
        // every order is fully filled in this round
        for (pos, o) in &rec.orders {
            let q: u64 = rec
                .fills
                .iter()
                .filter(|f| f.id == *pos)
                .map(|f| f.quantity)
                .sum();
            assert_eq!(q, o.size);
        }
    }
    let report = check(&net.public(R), &net.group, &net.roster);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn every_node_reaches_the_same_record_hash() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four_orders(&mut net, R);
    net.run_all(R);
    let hashes: Vec<_> = net
        .nodes
        .iter()
        .map(|n| n.final_record(R).unwrap().hash())
        .collect();
    assert!(hashes.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn a_round_completes_with_one_node_offline() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let orders = vec![
        net.sealed(R, Side::Sell, 5, 3),
        net.sealed(R, Side::Buy, 5, 4),
    ];
    for o in &orders {
        net.submit_to(R, o, &[1, 2]); // node 3 is down
    }
    net.close(R, &[1, 2]);
    net.release(R, &[1, 2]);
    net.clear(R, &[1, 2]);
    assert_eq!(net.node(1).final_record(R).unwrap().price, Some(3));
    assert!(check(&net.public(R), &net.group, &net.roster).is_clean());
}

// --- sealed until close: no node releases anything early -----------------------------------

#[test]
fn no_partials_before_close_or_before_the_commit_is_final() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four_orders(&mut net, R);
    assert_eq!(
        net.node(1).release_partials(R, &mut OsRng),
        Err(MatcherError::NotCommitted(R))
    );
    // node 1 closes, but only its own vote exists: still not final
    net.node(1).close(R).unwrap();
    assert_eq!(net.node(1).phase(R).unwrap(), Phase::Closed);
    assert_eq!(
        net.node(1).release_partials(R, &mut OsRng),
        Err(MatcherError::NotCommitted(R))
    );
}

#[test]
fn a_voided_round_never_releases_partials_and_the_checker_confirms_it() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 3)); // needs 3 orders
    net.submit_all(R, &net.sealed(R, Side::Sell, 5, 3));
    net.submit_all(R, &net.sealed(R, Side::Buy, 5, 4));
    net.close_all(R);
    for id in 1..=3 {
        assert_eq!(net.node(id).phase(R).unwrap(), Phase::Voided);
        assert_eq!(
            net.node(id).release_partials(R, &mut OsRng),
            Err(MatcherError::Voided { have: 2, need: 3 })
        );
    }
    let report = check(&net.public(R), &net.group, &net.roster);
    assert!(report.voided);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn submissions_after_close_are_refused() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let o = net.sealed(R, Side::Buy, 1, 1);
    net.close_all(R);
    assert!(matches!(
        net.node(1).submit(R, &o),
        Err(MatcherError::WrongPhase { .. })
    ));
}

// --- what nodes refuse to accept -----------------------------------------------------------

#[test]
fn an_envelope_sealed_for_another_round_is_refused_at_the_door() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(1, 1));
    net.open_round(cfg(2, 1));
    let for_round_1 = net.sealed(1, Side::Buy, 5, 5);
    assert!(matches!(
        net.node(1).submit(2, &for_round_1),
        Err(MatcherError::BadEnvelope(_))
    ));
}

#[test]
fn a_lifted_ephemeral_point_is_refused_at_the_door() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(1, 1));
    net.open_round(cfg(2, 1));
    let victim = Envelope::from_bytes(&net.sealed(1, Side::Buy, 5, 5)).unwrap();
    let mine = Envelope::from_bytes(&net.sealed(2, Side::Buy, 1, 1)).unwrap();
    let forged = Envelope {
        ephemeral: victim.ephemeral,
        ..mine
    };
    assert!(matches!(
        net.node(1).submit(2, &forged.to_bytes()),
        Err(MatcherError::BadEnvelope(_))
    ));
}

#[test]
fn garbage_is_refused_at_the_door() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    assert!(net.node(1).submit(R, b"not an envelope").is_err());
    assert!(net.node(1).submit(R, &[0u8; 200]).is_err());
    let o = net.sealed(R, Side::Buy, 1, 1);
    assert!(net.node(1).submit(99, &o).is_err());
}

#[test]
fn an_order_naming_another_round_inside_is_committed_but_excluded_from_the_auction() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    // sealed correctly for round 1, but the signed order inside says round 9
    let odd = net.sealed_with(R, 9, Side::Buy, 100, 100);
    net.submit_all(R, &odd);
    net.submit_all(R, &net.sealed(R, Side::Sell, 5, 3));
    net.run_all(R);
    let rec = net.node(1).final_record(R).unwrap();
    assert_eq!(rec.invalid.len(), 1);
    assert_eq!(rec.orders.len(), 1);
    assert_eq!(rec.price, None, "the only valid order is a lone sell");
    assert!(check(&net.public(R), &net.group, &net.roster).is_clean());
}

#[test]
fn forged_or_unknown_signatures_are_rejected() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let o = net.sealed(R, Side::Buy, 1, 1);
    let mut r = net.node(1).submit(R, &o).unwrap();
    r.envelope[0] ^= 1;
    assert_eq!(
        net.node(2).learn_receipt(&r),
        Err(MatcherError::BadSignature(1))
    );
    let mut r = net.node(1).submit(R, &o).unwrap();
    r.node = 9;
    assert_eq!(
        net.node(2).learn_receipt(&r),
        Err(MatcherError::UnknownNode(9))
    );
}

// --- cheating nodes --------------------------------------------------------------------------

#[test]
fn a_node_sending_wrong_partials_is_rejected_and_the_round_still_clears() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four_orders(&mut net, R);
    net.close_all(R);
    net.release(R, &[1, 2]);
    // node 3 sends, for each envelope, a partial computed for a DIFFERENT envelope
    let mine = net.node(3).release_partials(R, &mut OsRng).unwrap();
    let rotated: Vec<_> = mine
        .iter()
        .zip(mine.iter().cycle().skip(1))
        .map(|((h, _), (_, p))| (*h, *p))
        .collect();
    for (h, p) in &rotated {
        assert_eq!(
            net.node(1).learn_partial(R, *h, p),
            Err(MatcherError::BadPartial(3))
        );
    }
    net.clear(R, &[1, 2, 3]);
    assert_eq!(net.node(1).final_record(R).unwrap().price, Some(5));
}

#[test]
fn equivocation_on_the_commit_is_detected() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let a = net.sealed(R, Side::Buy, 1, 1);
    net.submit_all(R, &a);
    let honest = net.node(3).close(R).unwrap();
    net.node(1).learn_commit_vote(&honest).unwrap();
    // a different but well-formed vote: void the round (an empty set is below the minimum of 1)
    let other = CommitVote::new(&net.keys[2], 3, R, vec![], 1, true);
    assert_eq!(
        net.node(1).learn_commit_vote(&other),
        Err(MatcherError::Equivocation(3))
    );
}

#[test]
fn a_single_node_refusing_to_receipt_cannot_censor_an_order() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let x = net.sealed(R, Side::Buy, 5, 4);
    net.submit_to(R, &x, &[1, 2]); // node 3 refuses
    net.submit_all(R, &net.sealed(R, Side::Sell, 5, 3));
    net.run_all(R);
    let rec = net.node(3).final_record(R).unwrap();
    assert_eq!(rec.orders.len(), 2, "node 3 could not keep the order out");
    assert_eq!(rec.price, Some(3));
}

/// Build a commit vote set by colluding nodes 1 and 2.
fn colluding_commit(net: &Net, set: Vec<kalyptra_matcher::Hash>) -> Vec<CommitVote> {
    vec![
        CommitVote::new(&net.keys[0], 1, R, set.clone(), 1, false),
        CommitVote::new(&net.keys[1], 2, R, set, 1, false),
    ]
}

#[test]
fn checker_proves_censorship_when_t_nodes_drop_a_receipted_order() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let x = net.sealed(R, Side::Buy, 5, 4);
    let y = net.sealed(R, Side::Sell, 5, 3);
    net.submit_all(R, &x);
    net.submit_all(R, &y);
    let mut public = net.public(R);
    // nodes 1 and 2 collude and commit only y
    public.commit_votes = colluding_commit(&net, vec![kalyptra_matcher::round::envelope_hash(&y)]);
    let report = check(&public, &net.group, &net.roster);
    assert!(
        report.findings.iter().any(
            |f| matches!(f, Finding::Censored { receipted_by, .. } if receipted_by.len() == 3)
        ),
        "{:?}",
        report.findings
    );
}

#[test]
fn checker_flags_an_order_injected_into_the_set_without_receipts() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let y = net.sealed(R, Side::Sell, 5, 3);
    net.submit_all(R, &y);
    let injected = net.sealed(R, Side::Buy, 5, 9); // never submitted, so never receipted
    let mut public = net.public(R);
    public.envelopes.push(injected.clone());
    public.commit_votes = colluding_commit(
        &net,
        vec![
            kalyptra_matcher::round::envelope_hash(&y),
            kalyptra_matcher::round::envelope_hash(&injected),
        ],
    );
    let report = check(&public, &net.group, &net.roster);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::Unreceipted { .. })),
        "{:?}",
        report.findings
    );
}

#[test]
fn checker_catches_a_wrong_price_even_when_t_nodes_sign_it() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four_orders(&mut net, R);
    net.run_all(R);
    let mut public = net.public(R);
    let mut forged = public.record.clone().unwrap();
    forged.price = Some(6);
    public.record = Some(forged.clone());
    // nodes 1 and 2 collude and sign the forged record
    public.record_votes = vec![
        RecordVote::new(&net.keys[0], 1, &forged),
        RecordVote::new(&net.keys[1], 2, &forged),
    ];
    let report = check(&public, &net.group, &net.roster);
    assert!(report
        .findings
        .contains(&Finding::RecordMismatch { field: "price" }));
    assert!(!report.findings.contains(&Finding::RecordNotFinal));
}

#[test]
fn checker_catches_a_record_signed_by_too_few_nodes() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    four_orders(&mut net, R);
    net.run_all(R);
    let mut public = net.public(R);
    public.record_votes.truncate(1);
    let report = check(&public, &net.group, &net.roster);
    assert_eq!(report.findings, vec![Finding::RecordNotFinal]);
}

#[test]
fn checker_catches_partials_published_for_a_voided_round() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 5));
    let o = net.sealed(R, Side::Buy, 1, 1);
    net.submit_all(R, &o);
    net.close_all(R);
    let mut public = net.public(R);
    // a misbehaving node decrypts anyway and publishes it
    let env = Envelope::from_bytes(&o).unwrap();
    let mut shares = kalyptra_tse::run_local_dkg(3, 2, &mut OsRng).unwrap().0; // unrelated key, any partial
    let p = partial_decrypt_unchecked_for_tests(&shares.remove(0), &env, &mut OsRng).unwrap();
    public
        .partials
        .push((kalyptra_matcher::round::envelope_hash(&o), p));
    let report = check(&public, &net.group, &net.roster);
    assert!(report.findings.contains(&Finding::PartialsForVoidedRound));
}

#[test]
fn checker_flags_a_bad_receipt_signature() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let o = net.sealed(R, Side::Buy, 1, 1);
    net.submit_all(R, &o);
    net.run_all(R);
    let mut public = net.public(R);
    let mut bad: Receipt = public.receipts[0].clone();
    bad.round_id = R;
    bad.envelope[1] ^= 1;
    public.receipts.push(bad);
    let report = check(&public, &net.group, &net.roster);
    assert!(report.findings.iter().any(|f| matches!(
        f,
        Finding::BadSignature {
            what: "receipt",
            ..
        }
    )));
}

#[test]
fn a_node_missing_an_envelope_must_learn_it_before_releasing() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let o = net.sealed(R, Side::Buy, 1, 1);
    // nodes 1 and 2 accept it; node 3 never sees the envelope, only the receipts
    let r1 = net.node(1).submit(R, &o).unwrap();
    let r2 = net.node(2).submit(R, &o).unwrap();
    for id in 1..=3 {
        net.node(id).learn_receipt(&r1).unwrap();
        net.node(id).learn_receipt(&r2).unwrap();
    }
    net.close_all(R);
    assert!(matches!(
        net.node(3).release_partials(R, &mut OsRng),
        Err(MatcherError::MissingEnvelope(_))
    ));
    net.node(3).learn_envelope(R, &o).unwrap();
    assert!(net.node(3).release_partials(R, &mut OsRng).is_ok());
}

#[test]
fn bad_node_configuration_is_rejected() {
    use ed25519_dalek::SigningKey;
    use kalyptra_matcher::MatcherNode;
    let net = Net::new(3, 2);
    let (mut shares, group) = kalyptra_tse::run_local_dkg(3, 2, &mut OsRng).unwrap();
    let share = shares.remove(0);
    // a signing key that is not the one in the roster
    let r = MatcherNode::new(
        1,
        SigningKey::generate(&mut OsRng),
        share,
        group,
        net.roster.clone(),
    );
    assert!(matches!(r, Err(MatcherError::Config(_))));
    assert_eq!(round_aad(1, MARKET), round_aad(1, MARKET));
}

// --- nodes that closed with different receipts ---------------------------------------------

/// Three orders, each receipted by two of the three nodes, with every node told only about the
/// order it is part of. So when the round closes, all three nodes vote on different sets.
fn divergent_views(net: &mut Net, min_orders: usize) -> Vec<Vec<u8>> {
    net.open_round(cfg(R, min_orders));
    let orders = vec![
        net.sealed(R, Side::Buy, 5, 6),
        net.sealed(R, Side::Sell, 5, 4),
        net.sealed(R, Side::Buy, 3, 7),
    ];
    // order i is receipted by nodes i and i+1 (wrapping); node i hears only of order i
    for (i, o) in orders.iter().enumerate() {
        let a = (i % 3) as u16 + 1;
        let b = ((i + 1) % 3) as u16 + 1;
        let ra = net.node(a).submit(R, o).unwrap();
        let rb = net.node(b).submit(R, o).unwrap();
        net.node(a).learn_receipt(&rb).unwrap();
        net.node(b).learn_receipt(&ra).unwrap();
    }
    // the third node of each order hears of it only later
    orders
}

/// Everything the nodes have seen, told to everyone (the gossip that was late).
fn catch_up(net: &mut Net) {
    let p = net.public(R);
    for n in &mut net.nodes {
        for e in &p.envelopes {
            n.learn_envelope(R, e).unwrap();
        }
        for r in &p.receipts {
            n.learn_receipt(r).unwrap();
        }
    }
}

#[test]
fn nodes_that_closed_on_different_receipts_vote_again_and_the_round_goes_through() {
    let mut net = Net::new(3, 2);
    divergent_views(&mut net, 2);
    // each node closes with only what it has seen: three different sets
    let votes = net.close_all(R);
    let sets: std::collections::BTreeSet<_> = votes.iter().map(|v| v.key()).collect();
    assert_eq!(sets.len(), 3, "the views really differ");
    for n in &net.nodes {
        assert_eq!(
            n.phase(R).unwrap(),
            Phase::Closed,
            "stuck at the first vote"
        );
    }

    // nobody votes again until the first vote cannot win: with a vote still missing it could
    let lone = net.node(1).revote(R).unwrap();
    assert!(
        lone.is_some(),
        "all three votes are in and none agree: the epoch is dead"
    );

    // the late gossip arrives, then everyone votes again, in epoch 1
    catch_up(&mut net);
    let mut again = vec![lone.unwrap()];
    for id in [2u16, 3] {
        again.push(
            net.node(id)
                .revote(R)
                .unwrap()
                .expect("epoch 0 is dead for every node"),
        );
    }
    assert!(again.iter().all(|v| v.epoch == 1));
    // the first node's second vote was made before it caught up, so it differs; the other two
    // agree and that is enough for t = 2
    for v in &again {
        for n in &mut net.nodes {
            let _ = n.learn_commit_vote(v);
        }
    }
    for n in &net.nodes {
        assert_ne!(
            n.phase(R).unwrap(),
            Phase::Closed,
            "node {} still stuck",
            n.id()
        );
    }
    let ids: Vec<u16> = net.nodes.iter().map(|n| n.id()).collect();
    net.release(R, &ids);
    net.clear(R, &ids);
    let rec = net.node(1).final_record(R).expect("a final record");
    assert!(rec.price.is_some());
    let report = check(&net.public(R), &net.group, &net.roster);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn a_node_does_not_vote_again_while_the_first_vote_could_still_win() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let a = net.sealed(R, Side::Buy, 1, 1);
    net.submit_all(R, &a);
    // only node 1 has voted: nodes 2 and 3 could still agree with it
    net.node(1).close(R).unwrap();
    assert_eq!(net.node(1).revote(R).unwrap(), None);
    // node 2 votes the same: that is a final commit, so there is nothing to vote again about
    let v2 = net.node(2).close(R).unwrap();
    net.node(1).learn_commit_vote(&v2).unwrap();
    assert_eq!(net.node(1).phase(R).unwrap(), Phase::Committed);
    assert_eq!(net.node(1).revote(R).unwrap(), None);
    // an open round has nothing to vote again about either
    assert_eq!(net.node(3).revote(R).unwrap(), None);
}

#[test]
fn a_second_epoch_cannot_overturn_a_first_epoch_commit() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let a = net.sealed(R, Side::Buy, 1, 1);
    net.submit_all(R, &a);
    let votes = net.close(R, &[1, 2]);
    assert_eq!(net.node(3).phase(R).unwrap(), Phase::Committed);
    let committed = net.node(3).export(R).unwrap().commit_votes.len();
    assert_eq!(committed, 2);
    // a faulty node pushes later-epoch votes for a different set; the first epoch still decides
    let late = [
        CommitVote::in_epoch(&net.keys[2], 3, R, 1, vec![], 1, true),
        CommitVote::in_epoch(&net.keys[0], 1, R, 1, vec![], 1, true),
    ];
    for v in &late {
        net.node(3).learn_commit_vote(v).unwrap();
    }
    assert_eq!(net.node(3).phase(R).unwrap(), Phase::Committed);
    assert_eq!(votes.len(), 2);
}

#[test]
fn a_vote_beyond_the_epoch_limit_is_refused() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let v = CommitVote::in_epoch(
        &net.keys[0],
        1,
        R,
        kalyptra_matcher::MAX_EPOCH + 1,
        vec![],
        1,
        true,
    );
    assert!(matches!(
        net.node(2).learn_commit_vote(&v),
        Err(MatcherError::BadCommit(_))
    ));
}

#[test]
fn checker_flags_a_later_epoch_used_while_the_first_could_still_have_won() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let x = net.sealed(R, Side::Buy, 5, 4);
    let y = net.sealed(R, Side::Sell, 5, 3);
    net.submit_all(R, &x);
    net.submit_all(R, &y);
    let hx = kalyptra_matcher::round::envelope_hash(&x);
    let hy = kalyptra_matcher::round::envelope_hash(&y);
    let mut public = net.public(R);
    // epoch 0: one vote only, so it could still have won; yet nodes 1 and 2 moved to epoch 1
    public.commit_votes = vec![
        CommitVote::in_epoch(&net.keys[0], 1, R, 0, vec![hx, hy], 1, false),
        CommitVote::in_epoch(&net.keys[0], 1, R, 1, vec![hy], 1, false),
        CommitVote::in_epoch(&net.keys[1], 2, R, 1, vec![hy], 1, false),
    ];
    let report = check(&public, &net.group, &net.roster);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::PrematureRevote { epoch: 0 })),
        "{:?}",
        report.findings
    );
}

#[test]
fn a_vote_cannot_be_relabelled_into_another_epoch() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let v = CommitVote::in_epoch(&net.keys[0], 1, R, 0, vec![], 1, true);
    let mut moved = v.clone();
    moved.epoch = 1;
    assert!(matches!(
        net.node(2).learn_commit_vote(&moved),
        Err(MatcherError::BadSignature(1))
    ));
    // the real one is accepted
    net.node(2).learn_commit_vote(&v).unwrap();
}

#[test]
fn checker_flags_a_second_final_commit_in_a_later_epoch() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let x = net.sealed(R, Side::Buy, 5, 4);
    let y = net.sealed(R, Side::Sell, 5, 3);
    net.submit_all(R, &x);
    net.submit_all(R, &y);
    let hx = kalyptra_matcher::round::envelope_hash(&x);
    let hy = kalyptra_matcher::round::envelope_hash(&y);
    let mut public = net.public(R);
    // nodes 1 and 2 agree in epoch 0, then also "agree" on something else in epoch 1
    public.commit_votes = vec![
        CommitVote::in_epoch(&net.keys[0], 1, R, 0, vec![hx, hy], 1, false),
        CommitVote::in_epoch(&net.keys[1], 2, R, 0, vec![hx, hy], 1, false),
        CommitVote::in_epoch(&net.keys[0], 1, R, 1, vec![hy], 1, false),
        CommitVote::in_epoch(&net.keys[1], 2, R, 1, vec![hy], 1, false),
    ];
    let report = check(&public, &net.group, &net.roster);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::ConflictingCommits)),
        "{:?}",
        report.findings
    );
}

#[test]
fn checker_flags_a_node_that_skips_an_epoch() {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 1));
    let x = net.sealed(R, Side::Buy, 5, 4);
    net.submit_all(R, &x);
    let hx = kalyptra_matcher::round::envelope_hash(&x);
    let mut public = net.public(R);
    // nobody voted in epochs 0 or 1, yet two nodes decide in epoch 2
    public.commit_votes = vec![
        CommitVote::in_epoch(&net.keys[0], 1, R, 2, vec![hx], 1, false),
        CommitVote::in_epoch(&net.keys[1], 2, R, 2, vec![hx], 1, false),
    ];
    let report = check(&public, &net.group, &net.roster);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::PrematureRevote { epoch: 0 })),
        "{:?}",
        report.findings
    );
}

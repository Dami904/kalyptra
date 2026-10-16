//! Behavioural tests for kalyptra-tse. Each test states the guarantee it checks.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::scalar::Scalar;
use kalyptra_tse::dleq;
use kalyptra_tse::{
    open, open_unchecked_for_tests, partial_decrypt, partial_decrypt_unchecked_for_tests,
    run_local_dkg, seal, seal_with_nonces, verify_envelope, verify_partial, Envelope, Error,
    GroupKey, NodeKey, PartialDecryption, SealProof,
};
use proptest::prelude::*;
use rand_core::OsRng;

const AAD: &[u8] = b"kalyptra/round-7";

fn setup(n: u16, t: u16) -> (Vec<NodeKey>, GroupKey) {
    run_local_dkg(n, t, &mut OsRng).expect("dkg")
}

fn node(nodes: &[NodeKey], id: u16) -> &NodeKey {
    nodes.iter().find(|n| n.id() == id).unwrap()
}

fn partials(
    nodes: &[NodeKey],
    group: &GroupKey,
    env: &Envelope,
    aad: &[u8],
    ids: &[u16],
) -> Vec<PartialDecryption> {
    ids.iter()
        .map(|id| partial_decrypt(node(nodes, *id), group, env, aad, &mut OsRng).unwrap())
        .collect()
}

/// All subsets of `items` of size `k`.
fn subsets(items: &[u16], k: usize) -> Vec<Vec<u16>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut out = Vec::new();
    for (i, first) in items.iter().enumerate() {
        for mut rest in subsets(&items[i + 1..], k - 1) {
            let mut v = vec![*first];
            v.append(&mut rest);
            out.push(v);
        }
    }
    out
}

fn shares_of(
    group: &GroupKey,
) -> std::collections::BTreeMap<u16, curve25519_dalek::RistrettoPoint> {
    group
        .node_ids()
        .into_iter()
        .map(|id| (id, *group.verifying_share(id).unwrap()))
        .collect()
}

// --- key generation ---------------------------------------------------------------------

#[test]
fn dkg_gives_every_node_a_share_matching_its_public_verifying_share() {
    let (nodes, group) = setup(5, 3);
    assert_eq!(nodes.len(), 5);
    assert_eq!(group.threshold(), 3);
    for n in &nodes {
        assert_eq!(group.verifying_share(n.id()), Some(n.verifying_share()));
    }
}

#[test]
fn dkg_verifying_shares_are_consistent_with_the_group_key() {
    let (_, group) = setup(5, 3);
    assert!(group.is_consistent());
}

#[test]
fn two_independent_dkg_runs_give_different_group_keys() {
    let (_, a) = setup(3, 2);
    let (_, b) = setup(3, 2);
    assert_ne!(a.public_key(), b.public_key());
}

#[test]
fn dkg_rejects_bad_thresholds() {
    assert!(run_local_dkg(3, 0, &mut OsRng).is_err());
    assert!(run_local_dkg(3, 4, &mut OsRng).is_err());
}

#[test]
fn group_key_with_tampered_verifying_share_is_rejected() {
    let (_, group) = setup(3, 2);
    let mut shares = shares_of(&group);
    shares.insert(1, G * Scalar::from(5u64)); // not the real share
    assert!(GroupKey::new(2, *group.public_key(), shares).is_err());
}

#[test]
fn a_group_cannot_claim_a_higher_threshold_than_it_was_generated_with() {
    // Found by review: a 2-of-5 key presented as 4-of-5 would make users believe 4 nodes must
    // collude when 2 suffice.
    let (_, group) = setup(5, 2);
    let shares = shares_of(&group);
    for fake_t in [3u16, 4, 5] {
        assert!(
            GroupKey::new(fake_t, *group.public_key(), shares.clone()).is_err(),
            "claimed threshold {fake_t} accepted for a 2-of-5 key"
        );
    }
    // and not a lower one either
    assert!(GroupKey::new(1, *group.public_key(), shares.clone()).is_err());
    // the true threshold is accepted
    assert!(GroupKey::new(2, *group.public_key(), shares).is_ok());
}

#[test]
fn identity_points_are_rejected_in_a_group_key() {
    use curve25519_dalek::traits::Identity;
    let (_, group) = setup(3, 2);
    let mut shares = shares_of(&group);
    let id = curve25519_dalek::RistrettoPoint::identity();
    assert_eq!(
        GroupKey::new(2, id, shares.clone()),
        Err(Error::InvalidPoint)
    );
    shares.insert(2, id);
    assert_eq!(
        GroupKey::new(2, *group.public_key(), shares),
        Err(Error::InvalidPoint)
    );
}

// --- the core guarantee: any t open it, fewer than t cannot -------------------------------

#[test]
fn any_t_of_n_nodes_can_open_the_envelope_2_of_3() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"BUY 10 ACME @ 5", &mut OsRng);
    for ids in subsets(&[1, 2, 3], 2) {
        let opened = open(
            &group,
            &env,
            AAD,
            &partials(&nodes, &group, &env, AAD, &ids),
        )
        .unwrap();
        assert_eq!(opened.plaintext, b"BUY 10 ACME @ 5");
        assert_eq!(opened.used, ids);
        assert!(opened.rejected.is_empty());
    }
}

#[test]
fn any_t_of_n_nodes_can_open_the_envelope_3_of_5_every_subset() {
    let (nodes, group) = setup(5, 3);
    let env = seal(group.public_key(), AAD, b"SELL 4 ACME @ 5", &mut OsRng);
    let all: Vec<u16> = (1..=5).collect();
    let mut count = 0;
    for k in 3..=5 {
        for ids in subsets(&all, k) {
            let opened = open(
                &group,
                &env,
                AAD,
                &partials(&nodes, &group, &env, AAD, &ids),
            )
            .unwrap();
            assert_eq!(opened.plaintext, b"SELL 4 ACME @ 5");
            count += 1;
        }
    }
    assert_eq!(count, 10 + 5 + 1); // C(5,3) + C(5,4) + C(5,5)
}

#[test]
fn fewer_than_t_partials_do_not_open_the_envelope() {
    let (nodes, group) = setup(5, 3);
    let env = seal(group.public_key(), AAD, b"secret order", &mut OsRng);
    for k in 0..3 {
        for ids in subsets(&[1, 2, 3, 4, 5], k) {
            let r = open(
                &group,
                &env,
                AAD,
                &partials(&nodes, &group, &env, AAD, &ids),
            );
            assert_eq!(r, Err(Error::NotEnoughPartials { have: k, need: 3 }));
        }
    }
}

#[test]
fn attacker_forcing_interpolation_with_t_minus_1_shares_gets_nothing() {
    // Even skipping every check and interpolating whatever shares exist, t-1 of them give a
    // wrong shared secret, so the AEAD key is wrong and decryption fails.
    let (nodes, group) = setup(5, 3);
    let env = seal(group.public_key(), AAD, b"secret order", &mut OsRng);
    for ids in subsets(&[1, 2, 3, 4, 5], 2) {
        let ps = partials(&nodes, &group, &env, AAD, &ids);
        assert_eq!(
            open_unchecked_for_tests(&group, &env, AAD, &ps),
            Err(Error::Decrypt)
        );
    }
    // sanity: the same unchecked path DOES work with t shares, so the test is meaningful
    let ps = partials(&nodes, &group, &env, AAD, &[1, 3, 5]);
    assert_eq!(
        open_unchecked_for_tests(&group, &env, AAD, &ps).unwrap(),
        b"secret order"
    );
}

#[test]
fn one_node_alone_cannot_open_even_with_its_own_partial() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"secret", &mut OsRng);
    let ps = partials(&nodes, &group, &env, AAD, &[2]);
    assert_eq!(
        open(&group, &env, AAD, &ps),
        Err(Error::NotEnoughPartials { have: 1, need: 2 })
    );
    assert_eq!(
        open_unchecked_for_tests(&group, &env, AAD, &ps),
        Err(Error::Decrypt)
    );
}

// --- the sealer's proof: nobody can make nodes decrypt someone else's envelope ------------

#[test]
fn an_honest_envelope_verifies_only_for_its_own_group_and_round() {
    let (_, group) = setup(3, 2);
    let (_, other) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    assert!(verify_envelope(group.public_key(), &env, AAD).is_ok());
    assert_eq!(
        verify_envelope(group.public_key(), &env, b"kalyptra/round-8"),
        Err(Error::InvalidProof)
    );
    assert_eq!(
        verify_envelope(other.public_key(), &env, AAD),
        Err(Error::InvalidProof)
    );
}

#[test]
fn copying_a_victims_ephemeral_point_into_a_new_envelope_gets_no_partials() {
    // Found in review (and confirmed by a proof of concept on the old version): an attacker
    // lifts R from a sealed order in a round that must never open (say a voided round) into
    // their own envelope in a live round. Without a binding, nodes would release s_i * R at
    // close and the attacker could open the victim's order. Now the attacker must prove it
    // knows r for that R, which it does not.
    let (nodes, group) = setup(3, 2);
    let victim = seal(
        group.public_key(),
        b"kalyptra/round-VOIDED",
        b"victim order",
        &mut OsRng,
    );
    let mine = seal(group.public_key(), AAD, b"junk", &mut OsRng);
    let forged = Envelope {
        ephemeral: victim.ephemeral,
        proof: mine.proof, // any proof the attacker can make is for a different r
        ciphertext: mine.ciphertext,
    };
    for n in &nodes {
        assert_eq!(
            partial_decrypt(n, &group, &forged, AAD, &mut OsRng),
            Err(Error::InvalidProof)
        );
    }
    // reusing the victim's own proof does not help either: it is bound to its round and
    // ciphertext
    let forged2 = Envelope {
        proof: victim.proof,
        ..forged
    };
    assert_eq!(
        partial_decrypt(&nodes[0], &group, &forged2, AAD, &mut OsRng),
        Err(Error::InvalidProof)
    );
}

#[test]
fn shifting_a_victims_ephemeral_point_by_a_known_amount_gets_no_partials() {
    // Found in review: submit R' = R + k*G for a k you know; each partial P'_i = P_i + k*Y_i
    // would reveal P_i after subtracting k*Y_i with the public verifying shares. Nodes now
    // refuse R' because nobody can prove knowledge of r + k without knowing r.
    let (nodes, group) = setup(3, 2);
    let victim = seal(
        group.public_key(),
        b"kalyptra/round-VOIDED",
        b"victim order",
        &mut OsRng,
    );
    let k = Scalar::random(&mut OsRng);
    let shifted = (victim.ephemeral.decompress().unwrap() + G * k).compress();
    let mine = seal(group.public_key(), AAD, b"junk", &mut OsRng);
    let forged = Envelope {
        ephemeral: shifted,
        proof: mine.proof,
        ciphertext: mine.ciphertext,
    };
    for n in &nodes {
        assert_eq!(
            partial_decrypt(n, &group, &forged, AAD, &mut OsRng),
            Err(Error::InvalidProof)
        );
    }
}

#[test]
fn an_envelope_cannot_be_moved_into_another_round() {
    let (nodes, group) = setup(3, 2);
    let env = seal(
        group.public_key(),
        b"kalyptra/round-1",
        b"order",
        &mut OsRng,
    );
    // nodes refuse to decrypt it as part of round 2
    assert_eq!(
        partial_decrypt(&nodes[0], &group, &env, b"kalyptra/round-2", &mut OsRng),
        Err(Error::InvalidProof)
    );
    // and even partials obtained honestly for round 1 do not open it as round 2
    let ps = partials(&nodes, &group, &env, b"kalyptra/round-1", &[1, 2]);
    assert_eq!(
        open(&group, &env, b"kalyptra/round-2", &ps),
        Err(Error::InvalidProof)
    );
    assert!(open(&group, &env, b"kalyptra/round-1", &ps).is_ok());
}

#[test]
fn every_part_of_the_sealer_proof_is_checked() {
    let (_, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let pk = group.public_key();
    let mut e = env.clone();
    e.proof.c += Scalar::ONE;
    assert_eq!(verify_envelope(pk, &e, AAD), Err(Error::InvalidProof));
    let mut e = env.clone();
    e.proof.z += Scalar::ONE;
    assert_eq!(verify_envelope(pk, &e, AAD), Err(Error::InvalidProof));
    let mut e = env.clone();
    e.ciphertext[0] ^= 1;
    assert_eq!(verify_envelope(pk, &e, AAD), Err(Error::InvalidProof));
    let mut e = env.clone();
    e.ephemeral = (G * Scalar::from(9u64)).compress();
    assert_eq!(verify_envelope(pk, &e, AAD), Err(Error::InvalidProof));
}

#[test]
fn even_if_a_node_skips_the_sealer_check_open_still_refuses_the_forged_envelope() {
    let (nodes, group) = setup(3, 2);
    let victim = seal(
        group.public_key(),
        b"kalyptra/round-VOIDED",
        b"victim",
        &mut OsRng,
    );
    let mine = seal(group.public_key(), AAD, b"junk", &mut OsRng);
    let forged = Envelope {
        ephemeral: victim.ephemeral,
        proof: mine.proof,
        ciphertext: mine.ciphertext,
    };
    let ps: Vec<_> = nodes[..2]
        .iter()
        .map(|n| partial_decrypt_unchecked_for_tests(n, &forged, &mut OsRng).unwrap())
        .collect();
    assert_eq!(open(&group, &forged, AAD, &ps), Err(Error::InvalidProof));
}

#[test]
fn an_envelope_sealed_with_r_zero_is_rejected() {
    // R = identity would make the envelope readable by anyone
    let (nodes, group) = setup(3, 2);
    let env = seal_with_nonces(
        group.public_key(),
        AAD,
        b"order",
        &Scalar::ZERO,
        &Scalar::from(7u64),
    );
    assert_eq!(
        verify_envelope(group.public_key(), &env, AAD),
        Err(Error::InvalidPoint)
    );
    assert_eq!(
        partial_decrypt(&nodes[0], &group, &env, AAD, &mut OsRng),
        Err(Error::InvalidPoint)
    );
}

// --- cheating nodes ---------------------------------------------------------------------

#[test]
fn a_wrong_partial_is_caught_and_excluded_and_the_honest_nodes_still_open_it() {
    let (nodes, group) = setup(5, 3);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let mut ps = partials(&nodes, &group, &env, AAD, &[1, 2, 3, 4]);
    // node 2 lies: a valid-looking partial made for a different envelope
    let other = seal(group.public_key(), AAD, b"x", &mut OsRng);
    ps[1] = partial_decrypt(&nodes[1], &group, &other, AAD, &mut OsRng).unwrap();
    assert_eq!(
        verify_partial(&group, &env, &ps[1]),
        Err(Error::InvalidProof)
    );
    let opened = open(&group, &env, AAD, &ps).unwrap();
    assert_eq!(opened.plaintext, b"order");
    assert_eq!(opened.rejected, vec![2]);
    assert_eq!(opened.used, vec![1, 3, 4]);
}

#[test]
fn a_cheater_can_stall_only_if_too_few_honest_nodes_remain() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let other = seal(group.public_key(), AAD, b"x", &mut OsRng);
    let mut ps = partials(&nodes, &group, &env, AAD, &[1, 2]);
    ps[1] = partial_decrypt(&nodes[1], &group, &other, AAD, &mut OsRng).unwrap();
    // only one honest partial left: refuses rather than returning garbage
    assert_eq!(
        open(&group, &env, AAD, &ps),
        Err(Error::NotEnoughPartials { have: 1, need: 2 })
    );
}

#[test]
fn a_partial_from_a_random_point_with_a_forged_proof_is_rejected() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let mut p = partial_decrypt(&nodes[0], &group, &env, AAD, &mut OsRng).unwrap();
    p.point = (G * Scalar::from(77u64)).compress();
    assert!(verify_partial(&group, &env, &p).is_err());
}

#[test]
fn a_proof_cannot_be_replayed_under_another_nodes_identity() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let mut p = partial_decrypt(&nodes[0], &group, &env, AAD, &mut OsRng).unwrap();
    p.node_id = 2; // claim to be node 2
    assert!(verify_partial(&group, &env, &p).is_err());
}

#[test]
fn a_partial_for_an_unknown_node_is_rejected() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let mut p = partial_decrypt(&nodes[0], &group, &env, AAD, &mut OsRng).unwrap();
    p.node_id = 9;
    assert_eq!(verify_partial(&group, &env, &p), Err(Error::UnknownNode(9)));
}

#[test]
fn duplicate_partials_do_not_count_twice() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let p1 = partial_decrypt(&nodes[0], &group, &env, AAD, &mut OsRng).unwrap();
    let r = open(&group, &env, AAD, &[p1, p1]);
    assert_eq!(r, Err(Error::NotEnoughPartials { have: 1, need: 2 }));
}

#[test]
fn a_partial_made_for_a_different_envelope_does_not_verify() {
    let (nodes, group) = setup(3, 2);
    let a = seal(group.public_key(), AAD, b"a", &mut OsRng);
    let b = seal(group.public_key(), AAD, b"b", &mut OsRng);
    let pa = partial_decrypt(&nodes[0], &group, &a, AAD, &mut OsRng).unwrap();
    assert!(verify_partial(&group, &a, &pa).is_ok());
    assert!(verify_partial(&group, &b, &pa).is_err());
}

// --- the envelope itself -----------------------------------------------------------------

#[test]
fn a_tampered_ciphertext_fails_to_open() {
    let (nodes, group) = setup(3, 2);
    let mut env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let ps = partials(&nodes, &group, &env, AAD, &[1, 2]);
    let last = env.ciphertext.len() - 1;
    env.ciphertext[last] ^= 1;
    assert!(open(&group, &env, AAD, &ps).is_err());
}

#[test]
fn an_envelope_sealed_to_one_group_does_not_open_under_another() {
    let (nodes_a, group_a) = setup(3, 2);
    let (_, group_b) = setup(3, 2);
    let env = seal(group_a.public_key(), AAD, b"order", &mut OsRng);
    let ps = partials(&nodes_a, &group_a, &env, AAD, &[1, 2]);
    assert!(open(&group_b, &env, AAD, &ps).is_err());
}

#[test]
fn two_seals_of_the_same_message_are_unlinkable() {
    let (_, group) = setup(3, 2);
    let a = seal(group.public_key(), AAD, b"same", &mut OsRng);
    let b = seal(group.public_key(), AAD, b"same", &mut OsRng);
    assert_ne!(a.ephemeral, b.ephemeral);
    assert_ne!(a.ciphertext, b.ciphertext);
}

#[test]
fn empty_and_large_plaintexts_round_trip() {
    let (nodes, group) = setup(3, 2);
    for msg in [vec![], vec![7u8; 1], vec![0xAB; 100_000]] {
        let env = seal(group.public_key(), AAD, &msg, &mut OsRng);
        let ps = partials(&nodes, &group, &env, AAD, &[1, 3]);
        assert_eq!(open(&group, &env, AAD, &ps).unwrap().plaintext, msg);
    }
}

// --- wire formats ------------------------------------------------------------------------

#[test]
fn envelope_and_partial_survive_a_byte_round_trip() {
    let (nodes, group) = setup(3, 2);
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let env2 = Envelope::from_bytes(&env.to_bytes()).unwrap();
    assert_eq!(env, env2);
    let p = partial_decrypt(&nodes[0], &group, &env, AAD, &mut OsRng).unwrap();
    let p2 = PartialDecryption::from_bytes(&p.to_bytes()).unwrap();
    assert_eq!(p, p2);
    assert_eq!(PartialDecryption::BYTES, 98);
}

#[test]
fn malformed_wire_input_is_rejected_not_trusted() {
    assert_eq!(Envelope::from_bytes(&[0u8; 10]), Err(Error::BadLength));
    assert_eq!(Envelope::from_bytes(&[0u8; 111]), Err(Error::BadLength));
    assert_eq!(
        Envelope::from_bytes(&[0xFFu8; 128]),
        Err(Error::InvalidScalar)
    );
    assert_eq!(
        PartialDecryption::from_bytes(&[0u8; 97]),
        Err(Error::BadLength)
    );
    // a non-canonical scalar (all 0xFF) in the proof
    let mut bad = [0u8; 98];
    bad[34..].fill(0xFF);
    assert_eq!(
        PartialDecryption::from_bytes(&bad),
        Err(Error::InvalidScalar)
    );
    // an envelope whose ephemeral is not a valid point cannot be decrypted or opened
    let (nodes, group) = setup(3, 2);
    let junk = Envelope {
        ephemeral: CompressedRistretto([0xFF; 32]),
        proof: SealProof {
            c: Scalar::ZERO,
            z: Scalar::ZERO,
        },
        ciphertext: vec![0; 16],
    };
    assert_eq!(
        partial_decrypt(&nodes[0], &group, &junk, AAD, &mut OsRng),
        Err(Error::InvalidPoint)
    );
    assert!(open(&group, &junk, AAD, &[]).is_err());
}

#[test]
fn node_key_debug_output_never_contains_the_share() {
    let (nodes, _) = setup(3, 2);
    let shown = format!("{:?}", nodes[0]);
    assert!(shown.contains("redacted"));
}

// --- the proof of equal discrete logs, on its own -----------------------------------------

#[test]
fn dleq_accepts_an_honest_proof_and_rejects_every_kind_of_tampering() {
    let s = Scalar::random(&mut OsRng);
    let y = G * s;
    let r = G * Scalar::random(&mut OsRng);
    let (p, proof) = dleq::prove(4, &s, &r, &mut OsRng);
    assert!(dleq::verify(4, &y, &r, &p, &proof));
    assert!(!dleq::verify(5, &y, &r, &p, &proof), "wrong node id");
    assert!(
        !dleq::verify(4, &(G * Scalar::random(&mut OsRng)), &r, &p, &proof),
        "wrong verifying share"
    );
    assert!(!dleq::verify(4, &y, &r, &(p + G), &proof), "wrong partial");
    let mut bad = proof;
    bad.z += Scalar::ONE;
    assert!(!dleq::verify(4, &y, &r, &p, &bad), "tampered response");
}

#[test]
fn dleq_proofs_of_the_same_statement_use_fresh_nonces() {
    let s = Scalar::random(&mut OsRng);
    let y = G * s;
    let r = G * Scalar::random(&mut OsRng);
    let (p1, a) = dleq::prove(1, &s, &r, &mut OsRng);
    let (p2, b) = dleq::prove(1, &s, &r, &mut OsRng);
    assert_eq!(p1, p2);
    assert_ne!(a, b, "two proofs of one statement must not repeat a nonce");
    assert!(dleq::verify(1, &y, &r, &p1, &a) && dleq::verify(1, &y, &r, &p2, &b));
}

// --- properties --------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

    #[test]
    fn prop_any_plaintext_any_aad_any_t_subset_round_trips_2_of_3(
        msg in proptest::collection::vec(any::<u8>(), 0..300),
        aad in proptest::collection::vec(any::<u8>(), 0..40),
        pick in 0usize..3,
    ) {
        let (nodes, group) = setup(3, 2);
        let env = seal(group.public_key(), &aad, &msg, &mut OsRng);
        let all = subsets(&[1, 2, 3], 2);
        let ids = &all[pick];
        let opened = open(&group, &env, &aad, &partials(&nodes, &group, &env, &aad, ids)).unwrap();
        prop_assert_eq!(opened.plaintext, msg);
    }

    #[test]
    fn prop_t_minus_1_never_opens_3_of_5(
        msg in proptest::collection::vec(any::<u8>(), 1..200),
        pick in 0usize..10,
    ) {
        let (nodes, group) = setup(5, 3);
        let env = seal(group.public_key(), AAD, &msg, &mut OsRng);
        let two = subsets(&[1, 2, 3, 4, 5], 2);
        let ps = partials(&nodes, &group, &env, AAD, &two[pick]);
        prop_assert!(open(&group, &env, AAD, &ps).is_err());
        prop_assert!(open_unchecked_for_tests(&group, &env, AAD, &ps).is_err());
    }

    #[test]
    fn prop_flipping_any_ciphertext_bit_breaks_opening(
        msg in proptest::collection::vec(any::<u8>(), 1..100),
        bit in 0usize..8,
        pos_seed in any::<usize>(),
    ) {
        let (nodes, group) = setup(3, 2);
        let mut env = seal(group.public_key(), AAD, &msg, &mut OsRng);
        let ps = partials(&nodes, &group, &env, AAD, &[1, 2]);
        let pos = pos_seed % env.ciphertext.len();
        env.ciphertext[pos] ^= 1 << bit;
        prop_assert!(open(&group, &env, AAD, &ps).is_err());
    }
}

// --- single-recipient envelopes and stored shares -------------------------------------------

#[test]
fn a_single_recipient_envelope_opens_only_with_that_recipients_secret() {
    use kalyptra_tse::open_single;
    let x = Scalar::random(&mut OsRng);
    let other = Scalar::random(&mut OsRng);
    let env = seal(&(G * x), b"dkg/1/2", b"share bytes", &mut OsRng);
    assert_eq!(open_single(&x, &env, b"dkg/1/2").unwrap(), b"share bytes");
    assert!(open_single(&other, &env, b"dkg/1/2").is_err());
    assert_eq!(open_single(&x, &env, b"dkg/1/3"), Err(Error::InvalidProof));
}

#[test]
fn a_stored_share_reloads_to_the_same_node_key() {
    let (nodes, group) = setup(3, 2);
    let reloaded: Vec<NodeKey> = nodes
        .iter()
        .map(|n| NodeKey::from_share_bytes(n.id(), &n.share_bytes()).unwrap())
        .collect();
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let ps = partials(&reloaded, &group, &env, AAD, &[1, 3]);
    assert_eq!(open(&group, &env, AAD, &ps).unwrap().plaintext, b"order");
    assert!(NodeKey::from_share_bytes(1, &[0u8; 32]).is_err());
    assert!(NodeKey::from_share_bytes(1, &[0xFF; 32]).is_err());
    assert!(NodeKey::from_share_bytes(0, &nodes[0].share_bytes()).is_err());
}

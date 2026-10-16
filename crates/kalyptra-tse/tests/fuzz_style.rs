//! Fuzz-style tests: hostile bytes must never panic, never decrypt, and never be trusted.
//!
//! These are property-based and exhaustive-mutation tests, not coverage-guided fuzzing
//! (no libFuzzer). They cover every function that accepts bytes from the network.

use std::sync::OnceLock;

use kalyptra_tse::{
    open, partial_decrypt, run_local_dkg, seal, verify_envelope, verify_partial, Envelope,
    GroupKey, NodeKey, PartialDecryption, Participant,
};
use proptest::prelude::*;
use rand_core::OsRng;

const AAD: &[u8] = b"fuzz-round";

fn fixture() -> &'static (Vec<NodeKey>, GroupKey) {
    static F: OnceLock<(Vec<NodeKey>, GroupKey)> = OnceLock::new();
    F.get_or_init(|| run_local_dkg(3, 2, &mut OsRng).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4000, ..ProptestConfig::default() })]

    #[test]
    fn envelope_parser_never_panics_and_round_trips_what_it_accepts(
        bytes in proptest::collection::vec(any::<u8>(), 0..260)
    ) {
        if let Ok(env) = Envelope::from_bytes(&bytes) {
            prop_assert_eq!(env.to_bytes(), bytes);
        }
    }

    #[test]
    fn partial_parser_never_panics_and_round_trips_what_it_accepts(
        bytes in proptest::collection::vec(any::<u8>(), 0..130)
    ) {
        if let Ok(p) = PartialDecryption::from_bytes(&bytes) {
            prop_assert_eq!(p.to_bytes().to_vec(), bytes);
        }
    }

    #[test]
    fn nodes_never_decrypt_random_envelopes(
        r in proptest::collection::vec(any::<u8>(), 32),
        body in proptest::collection::vec(any::<u8>(), 16..120),
    ) {
        let (nodes, group) = fixture();
        // build a parseable envelope: random point bytes, canonical (small) proof scalars,
        // random ciphertext
        let mut bytes = r.clone();
        bytes.extend_from_slice(&[1u8; 1]);
        bytes.extend_from_slice(&[0u8; 31]);
        bytes.extend_from_slice(&[2u8; 1]);
        bytes.extend_from_slice(&[0u8; 31]);
        bytes.extend_from_slice(&body);
        let env = Envelope::from_bytes(&bytes).unwrap();
        prop_assert!(verify_envelope(group.public_key(), &env, AAD).is_err());
        prop_assert!(partial_decrypt(&nodes[0], group, &env, AAD, &mut OsRng).is_err());
    }

    #[test]
    fn open_never_succeeds_on_random_envelopes_and_random_partials(
        env_bytes in proptest::collection::vec(any::<u8>(), 0..220),
        partial_bytes in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 98), 0..5),
    ) {
        let group = &fixture().1;
        if let Ok(env) = Envelope::from_bytes(&env_bytes) {
            let ps: Vec<PartialDecryption> = partial_bytes
                .iter()
                .filter_map(|b| PartialDecryption::from_bytes(b).ok())
                .collect();
            prop_assert!(open(group, &env, AAD, &ps).is_err());
            for p in &ps {
                prop_assert!(verify_partial(group, &env, p).is_err());
            }
        }
    }

    #[test]
    fn dkg_participants_reject_garbage_messages_without_panicking(
        msgs in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..120), 0..5)
    ) {
        let mut p = Participant::new(1, 3, 2).unwrap();
        p.round1(&mut OsRng).unwrap();
        prop_assert!(p.round2(&msgs).is_err());
        let mut q = Participant::new(2, 3, 2).unwrap();
        q.round1(&mut OsRng).unwrap();
        prop_assert!(q.round3(&msgs, &msgs).is_err());
    }
}

#[test]
fn flipping_any_single_bit_of_a_valid_partial_makes_it_fail() {
    let (nodes, group) = fixture();
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let good = partial_decrypt(&nodes[0], group, &env, AAD, &mut OsRng).unwrap();
    assert!(verify_partial(group, &env, &good).is_ok());
    let bytes = good.to_bytes();
    let mut checked = 0;
    for byte in 0..bytes.len() {
        for bit in 0..8 {
            let mut m = bytes;
            m[byte] ^= 1 << bit;
            let still_valid = PartialDecryption::from_bytes(&m)
                .map(|p| verify_partial(group, &env, &p).is_ok())
                .unwrap_or(false);
            assert!(
                !still_valid,
                "bit {bit} of byte {byte} could be flipped without detection"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 98 * 8);
}

#[test]
fn flipping_any_single_bit_of_a_sealed_envelope_stops_it_opening_and_being_decrypted() {
    let (nodes, group) = fixture();
    let env = seal(group.public_key(), AAD, b"BUY 10 ACME @ 5", &mut OsRng);
    let ps: Vec<_> = nodes[..2]
        .iter()
        .map(|n| partial_decrypt(n, group, &env, AAD, &mut OsRng).unwrap())
        .collect();
    assert!(open(group, &env, AAD, &ps).is_ok());
    let bytes = env.to_bytes();
    let mut checked = 0;
    for byte in 0..bytes.len() {
        for bit in 0..8 {
            let mut m = bytes.clone();
            m[byte] ^= 1 << bit;
            if let Ok(e) = Envelope::from_bytes(&m) {
                assert!(
                    open(group, &e, AAD, &ps).is_err(),
                    "bit {bit} of byte {byte}: tampered envelope still opened"
                );
                assert!(
                    partial_decrypt(&nodes[0], group, &e, AAD, &mut OsRng).is_err(),
                    "bit {bit} of byte {byte}: a node decrypted a tampered envelope"
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, bytes.len() * 8);
}

#[test]
fn truncating_or_extending_wire_bytes_is_rejected() {
    let (nodes, group) = fixture();
    let env = seal(group.public_key(), AAD, b"order", &mut OsRng);
    let p = partial_decrypt(&nodes[0], group, &env, AAD, &mut OsRng)
        .unwrap()
        .to_bytes();
    for len in 0..p.len() {
        assert!(PartialDecryption::from_bytes(&p[..len]).is_err());
    }
    let mut longer = p.to_vec();
    longer.push(0);
    assert!(PartialDecryption::from_bytes(&longer).is_err());
    // shorter than the header (point plus two proof scalars) plus the 16-byte tag is rejected
    assert!(Envelope::from_bytes(&env.to_bytes()[..111]).is_err());
    // a truncated ciphertext parses but no longer verifies
    let wire = env.to_bytes();
    let cut = Envelope::from_bytes(&wire[..wire.len() - 1]).unwrap();
    assert!(verify_envelope(group.public_key(), &cut, AAD).is_err());
}

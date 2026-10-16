//! Signed orders.

use ed25519_dalek::SigningKey;
use kalyptra_matcher::{MatcherError, Order, Side};
use rand_core::OsRng;

fn order(key: &SigningKey) -> Order {
    Order {
        side: Side::Buy,
        size: 10,
        limit: 5,
        round_id: 7,
        market: "ACME/vUSD".into(),
        salt: [9; 16],
        investor: key.verifying_key().to_bytes(),
    }
}

#[test]
fn a_signed_order_round_trips() {
    let key = SigningKey::generate(&mut OsRng);
    let o = order(&key);
    assert_eq!(Order::parse_signed(&o.sign(&key).unwrap()).unwrap(), o);
}

#[test]
fn flipping_any_bit_of_a_signed_order_is_rejected() {
    let key = SigningKey::generate(&mut OsRng);
    let bytes = order(&key).sign(&key).unwrap();
    for i in 0..bytes.len() {
        for bit in 0..8 {
            let mut m = bytes.clone();
            m[i] ^= 1 << bit;
            assert!(Order::parse_signed(&m).is_err(), "byte {i} bit {bit}");
        }
    }
}

#[test]
fn only_the_named_investor_can_sign() {
    let key = SigningKey::generate(&mut OsRng);
    let other = SigningKey::generate(&mut OsRng);
    assert!(order(&key).sign(&other).is_err());
}

#[test]
fn truncated_extended_and_invalid_orders_are_rejected() {
    let key = SigningKey::generate(&mut OsRng);
    let bytes = order(&key).sign(&key).unwrap();
    assert!(Order::parse_signed(&bytes[..bytes.len() - 1]).is_err());
    assert!(Order::parse_signed(&[bytes.clone(), vec![0]].concat()).is_err());
    assert!(Order::parse_signed(&[]).is_err());
    let mut zero = order(&key);
    zero.size = 0;
    assert_eq!(
        zero.sign(&key),
        Err(MatcherError::BadOrder("size and limit must be positive"))
    );
    let mut long = order(&key);
    long.market = "X".repeat(33);
    assert!(long.sign(&key).is_err());
}

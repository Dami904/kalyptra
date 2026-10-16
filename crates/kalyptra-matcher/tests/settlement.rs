//! Settlement terms from a final record, and owners' authorisation of their parts.

mod common;

use common::{cfg, Net};
use ed25519_dalek::SigningKey;
use kalyptra_matcher::round::envelope_hash;
use kalyptra_matcher::settlement::{leader, legs, market_assets, sign_part, verify_part, Leg};
use kalyptra_matcher::{MatcherError, RoundRecord, Side};
use rand_core::OsRng;

const R: u64 = 7;

/// The four-order round (clears at 5, volume 14). Returns the final record and each
/// order's one-time key, keyed by the order's position in the committed set.
fn cleared_round() -> (RoundRecord, Vec<(u32, SigningKey)>) {
    let mut net = Net::new(3, 2);
    net.open_round(cfg(R, 2));
    let mut keyed = Vec::new();
    for (side, size, limit) in [
        (Side::Sell, 10, 4),
        (Side::Sell, 4, 5),
        (Side::Buy, 8, 6),
        (Side::Buy, 6, 5),
    ] {
        let (env, key) = net.sealed_keyed(R, R, side, size, limit);
        net.submit_all(R, &env);
        keyed.push((env, key));
    }
    net.run_all(R);
    let record = net.nodes[0].final_record(R).unwrap().clone();
    let keys = keyed
        .into_iter()
        .map(|(env, key)| {
            let h = envelope_hash(&env);
            let pos = record.set.iter().position(|x| *x == h).unwrap() as u32;
            (pos, key)
        })
        .collect();
    (record, keys)
}

fn key_for(keys: &[(u32, SigningKey)], order: u32) -> &SigningKey {
    &keys.iter().find(|(p, _)| *p == order).unwrap().1
}

#[test]
fn every_filled_order_gets_a_leg_and_the_legs_balance() {
    let (record, _) = cleared_round();
    let ls = legs(&record).unwrap();
    assert_eq!(ls.len(), 4);
    for l in &ls {
        assert_eq!(l.quote, l.base * 5, "every leg at the one clearing price");
        let (_, o) = record.orders.iter().find(|(i, _)| *i == l.order).unwrap();
        assert_eq!(l.owner, o.owner);
        assert_eq!(l.side, o.side);
    }
    let total = |side: Side, f: fn(&Leg) -> u64| -> u64 {
        ls.iter().filter(|l| l.side == side).map(f).sum()
    };
    assert_eq!(total(Side::Sell, |l| l.base), 14);
    assert_eq!(total(Side::Buy, |l| l.base), 14);
    assert_eq!(total(Side::Sell, |l| l.quote), 70);
    assert_eq!(total(Side::Buy, |l| l.quote), 70);
    // what each side sends and receives
    let sell = ls.iter().find(|l| l.side == Side::Sell).unwrap();
    assert_eq!(
        (sell.send_amount(), sell.recv_amount()),
        (sell.base, sell.quote)
    );
    let buy = ls.iter().find(|l| l.side == Side::Buy).unwrap();
    assert_eq!(
        (buy.send_amount(), buy.recv_amount()),
        (buy.quote, buy.base)
    );
}

#[test]
fn only_the_orders_owner_can_authorise_its_part() {
    let (record, keys) = cleared_round();
    let part = b"an action group".to_vec();
    for l in legs(&record).unwrap() {
        let sig = sign_part(key_for(&keys, l.order), &record, l.order, &part);
        assert_eq!(verify_part(&record, l.order, &part, &sig).unwrap(), l);

        // someone else's key
        let stranger = SigningKey::generate(&mut OsRng);
        let forged = sign_part(&stranger, &record, l.order, &part);
        assert!(verify_part(&record, l.order, &part, &forged).is_err());

        // the owner's signature does not carry over to other bytes, or to another order
        assert!(verify_part(&record, l.order, b"another group", &sig).is_err());
        let other = (l.order + 1) % record.set.len() as u32;
        assert!(verify_part(&record, other, &part, &sig).is_err());
    }
}

#[test]
fn a_signature_is_bound_to_the_exact_record() {
    let (record, keys) = cleared_round();
    let l = legs(&record).unwrap()[0];
    let part = b"group".to_vec();
    let sig = sign_part(key_for(&keys, l.order), &record, l.order, &part);
    let mut other = record.clone();
    other.round_id += 1;
    assert!(verify_part(&other, l.order, &part, &sig).is_err());
}

#[test]
fn records_whose_fills_do_not_balance_are_refused() {
    let (record, _) = cleared_round();

    let mut more = record.clone();
    more.fills[0].quantity += 1;
    assert!(matches!(legs(&more), Err(MatcherError::Settlement(_))));

    let mut flipped = record.clone();
    flipped.fills[0].side = match flipped.fills[0].side {
        Side::Buy => Side::Sell,
        Side::Sell => Side::Buy,
    };
    assert!(legs(&flipped).is_err());

    let mut twice = record.clone();
    let dup = twice.fills[0];
    twice.fills.push(dup);
    assert!(legs(&twice).is_err());

    let mut stranger = record.clone();
    stranger.fills[0].id = 99;
    assert!(legs(&stranger).is_err());

    let mut overflow = record.clone();
    overflow.price = Some(u64::MAX);
    assert!(legs(&overflow).is_err());
}

#[test]
fn a_fill_outside_its_orders_limit_is_refused() {
    let (record, _) = cleared_round(); // clears at 5
    for (side, worse_limit) in [(Side::Buy, 4), (Side::Sell, 6)] {
        let mut r = record.clone();
        let i = r
            .orders
            .iter()
            .position(|(_, o)| o.side == side)
            .expect("an order on this side");
        // the order's own limit says it would not trade at 5
        r.orders[i].1.limit = worse_limit;
        let err = legs(&r).unwrap_err();
        assert!(
            matches!(err, MatcherError::Settlement(m) if m.contains("limit")),
            "{side:?}: {err:?}"
        );
    }
    // a limit exactly at the price is fine
    let mut r = record;
    for (_, o) in r.orders.iter_mut() {
        o.limit = 5;
    }
    assert!(legs(&r).is_ok());
}

#[test]
fn each_settlement_rule_is_enforced_on_its_own() {
    // Each case breaks exactly one rule and leaves the others satisfied, so a rule that
    // stopped being enforced would let its case through.
    let (record, _) = cleared_round();
    let nth = |r: &RoundRecord, side: Side, n: usize| {
        r.fills
            .iter()
            .enumerate()
            .filter(|(_, f)| f.side == side)
            .map(|(i, _)| i)
            .nth(n)
            .unwrap()
    };
    let message = |r: &RoundRecord| match legs(r).unwrap_err() {
        MatcherError::Settlement(m) => m,
        other => panic!("unexpected error {other:?}"),
    };

    // totals that do not balance, every fill within its order's size and limit
    let mut r = record.clone();
    let b = nth(&r, Side::Buy, 0);
    r.fills[b].quantity -= 1;
    assert!(message(&r).contains("balance"));

    // a fill above its order's size, with the totals still balanced
    let mut r = record.clone();
    let (a, b) = (nth(&r, Side::Sell, 0), nth(&r, Side::Sell, 1));
    r.fills[a].quantity += 1;
    r.fills[b].quantity -= 1;
    assert!(message(&r).contains("does not match"));

    // one order filled in two pieces, with the totals still balanced
    let mut r = record.clone();
    let a = nth(&r, Side::Sell, 0);
    let mut rest = r.fills[a];
    rest.quantity -= 1;
    r.fills[a].quantity = 1;
    r.fills.push(rest);
    assert!(message(&r).contains("twice"));
}

#[test]
fn a_round_without_a_price_settles_nothing() {
    let (mut record, _) = cleared_round();
    record.price = None;
    assert!(legs(&record).unwrap().is_empty());
}

#[test]
fn the_leader_rotates_through_the_roster() {
    assert_eq!(leader(&[3, 1, 2], 0), Some(1));
    assert_eq!(leader(&[3, 1, 2], 1), Some(2));
    assert_eq!(leader(&[3, 1, 2], 2), Some(3));
    assert_eq!(leader(&[3, 1, 2], 3), Some(1));
    assert_eq!(leader(&[], 3), None);
}

#[test]
fn market_names_split_into_base_and_quote() {
    assert_eq!(market_assets("ACME/vUSD"), Some(("ACME", "vUSD")));
    assert_eq!(market_assets("ACME"), None);
    assert_eq!(market_assets("/vUSD"), None);
    assert_eq!(market_assets("ACME/ACME"), None);
    assert_eq!(market_assets("A/B/C"), None);
}

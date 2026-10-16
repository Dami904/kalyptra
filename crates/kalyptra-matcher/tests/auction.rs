//! The uniform-price call auction.

use kalyptra_matcher::auction::{clear, AuctionOrder};
use kalyptra_matcher::Side;
use proptest::prelude::*;

fn o(id: u32, side: Side, size: u64, limit: u64) -> AuctionOrder {
    AuctionOrder {
        id,
        side,
        size,
        limit,
    }
}

fn filled(c: &kalyptra_matcher::auction::Clearing, id: u32) -> u64 {
    c.fills
        .iter()
        .filter(|f| f.id == id)
        .map(|f| f.quantity)
        .sum()
}

#[test]
fn the_g0_round_clears_at_5_with_volume_14() {
    // the round that settled as one atomic swap in the G0 spike
    let orders = [
        o(0, Side::Sell, 10, 4),
        o(1, Side::Sell, 4, 5),
        o(2, Side::Buy, 8, 6),
        o(3, Side::Buy, 6, 5),
    ];
    let c = clear(&orders);
    assert_eq!(c.price, Some(5));
    assert_eq!(c.volume, 14);
    for (id, q) in [(0, 10), (1, 4), (2, 8), (3, 6)] {
        assert_eq!(filled(&c, id), q);
    }
}

#[test]
fn nothing_trades_when_bids_do_not_reach_asks() {
    let c = clear(&[o(0, Side::Buy, 5, 4), o(1, Side::Sell, 5, 6)]);
    assert_eq!(c.price, None);
    assert_eq!(c.volume, 0);
    assert!(c.fills.is_empty());
}

#[test]
fn one_sided_or_empty_books_do_not_trade() {
    assert_eq!(clear(&[]).price, None);
    assert_eq!(
        clear(&[o(0, Side::Buy, 5, 9), o(1, Side::Buy, 2, 3)]).price,
        None
    );
    assert_eq!(clear(&[o(0, Side::Sell, 5, 1)]).price, None);
}

#[test]
fn the_marginal_order_is_partly_filled() {
    let orders = [
        o(0, Side::Buy, 10, 5),
        o(1, Side::Sell, 4, 4),
        o(2, Side::Sell, 4, 5),
    ];
    let c = clear(&orders);
    assert_eq!(c.price, Some(5));
    assert_eq!(c.volume, 8);
    assert_eq!(filled(&c, 0), 8);
}

#[test]
fn ties_go_to_the_least_imbalance_then_the_lowest_price() {
    // volume 5 at both 3 and 7, both balanced: the lower price wins
    let c = clear(&[o(0, Side::Buy, 5, 7), o(1, Side::Sell, 5, 3)]);
    assert_eq!(c.price, Some(3));
}

#[test]
fn better_priced_orders_fill_first() {
    // both buyers are in at the clearing price 8, but only 7 units are for sale: the higher
    // limit fills completely even though it comes later in the set
    let orders = [
        o(0, Side::Buy, 5, 8),
        o(1, Side::Buy, 5, 9),
        o(2, Side::Sell, 7, 8),
    ];
    let c = clear(&orders);
    assert_eq!(c.price, Some(8));
    assert_eq!(c.volume, 7);
    assert_eq!(filled(&c, 1), 5);
    assert_eq!(filled(&c, 0), 2);
}

#[test]
fn least_imbalance_can_raise_the_price_above_the_lowest_crossing_limit() {
    // at 6, 10 units are bid against 5 offered; at 9, 5 against 5. Same volume, so the
    // balanced price 9 wins (the documented rule).
    let orders = [
        o(0, Side::Buy, 5, 6),
        o(1, Side::Buy, 5, 9),
        o(2, Side::Sell, 5, 6),
    ];
    let c = clear(&orders);
    assert_eq!(c.price, Some(9));
    assert_eq!(filled(&c, 1), 5);
    assert_eq!(filled(&c, 0), 0);
}

#[test]
fn equal_limits_fill_by_position_in_the_committed_set() {
    let orders = [
        o(7, Side::Buy, 5, 6),
        o(3, Side::Buy, 5, 6),
        o(9, Side::Sell, 5, 6),
    ];
    let c = clear(&orders);
    assert_eq!(filled(&c, 3), 5);
    assert_eq!(filled(&c, 7), 0);
}

#[test]
fn huge_sizes_do_not_overflow() {
    let orders = [
        o(0, Side::Buy, u64::MAX / 2, u64::MAX),
        o(1, Side::Buy, u64::MAX / 2, u64::MAX),
        o(2, Side::Sell, u64::MAX, 1),
    ];
    let c = clear(&orders);
    assert_eq!(c.volume, (u64::MAX / 2) * 2);
}

fn arb_orders() -> impl Strategy<Value = Vec<AuctionOrder>> {
    proptest::collection::vec((any::<bool>(), 1u64..50, 1u64..20), 0..14).prop_map(|v| {
        v.into_iter()
            .enumerate()
            .map(|(i, (buy, size, limit))| {
                o(
                    i as u32,
                    if buy { Side::Buy } else { Side::Sell },
                    size,
                    limit,
                )
            })
            .collect()
    })
}

fn volume_at(orders: &[AuctionOrder], p: u64) -> u64 {
    let d: u64 = orders
        .iter()
        .filter(|x| x.side == Side::Buy && x.limit >= p)
        .map(|x| x.size)
        .sum();
    let s: u64 = orders
        .iter()
        .filter(|x| x.side == Side::Sell && x.limit <= p)
        .map(|x| x.size)
        .sum();
    d.min(s)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    #[test]
    fn prop_fills_are_consistent_and_respect_every_limit(orders in arb_orders()) {
        let c = clear(&orders);
        let buys: u64 = c.fills.iter().filter(|f| f.side == Side::Buy).map(|f| f.quantity).sum();
        let sells: u64 = c.fills.iter().filter(|f| f.side == Side::Sell).map(|f| f.quantity).sum();
        prop_assert_eq!(buys, c.volume);
        prop_assert_eq!(sells, c.volume);
        if let Some(p) = c.price {
            prop_assert!(orders.iter().any(|x| x.limit == p), "price is one of the limits");
            for f in &c.fills {
                let x = orders.iter().find(|x| x.id == f.id).unwrap();
                prop_assert!(f.quantity > 0 && f.quantity <= x.size);
                prop_assert_eq!(f.side, x.side);
                match x.side {
                    Side::Buy => prop_assert!(x.limit >= p, "a buyer never pays above its limit"),
                    Side::Sell => prop_assert!(x.limit <= p, "a seller never sells below its limit"),
                }
            }
        } else {
            prop_assert_eq!(c.volume, 0);
        }
    }

    #[test]
    fn prop_no_other_price_trades_more(orders in arb_orders()) {
        let c = clear(&orders);
        for p in 0..=21 {
            prop_assert!(volume_at(&orders, p) <= c.volume);
        }
    }

    #[test]
    fn prop_input_order_does_not_change_the_result(orders in arb_orders()) {
        let mut rev = orders.clone();
        rev.reverse();
        prop_assert_eq!(clear(&orders), clear(&rev));
    }
}

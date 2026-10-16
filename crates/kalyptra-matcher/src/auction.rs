//! Uniform-price call auction.
//!
//! Every order that trades, trades at one clearing price. The rules are fixed and public so
//! every matcher node, and anyone running the checker, computes the same result:
//!
//! 1. **Candidate prices** are the orders' limit prices.
//! 2. At a price `p`, **demand** is the total size of buy orders with limit `>= p`, and
//!    **supply** the total size of sell orders with limit `<= p`. The executable volume is
//!    `min(demand, supply)`.
//! 3. The clearing price maximises executable volume. Ties go to the price with the smallest
//!    imbalance `|demand - supply|`, then to the **lowest** such price.
//! 4. **Allocation** at that price: buyers by highest limit first, sellers by lowest limit
//!    first; equal limits by position in the committed order set (sorted by envelope hash).
//!    The marginal order may be partly filled.
//!
//! The envelope-hash tiebreak is deterministic and not chosen by any matcher, but an investor
//! can grind salts to get a favourable hash. With a pro-rata rule that would not help; this
//! is a known, documented limit of the demo.

use crate::order::Side;

/// What the auction needs from an order. `id` is its position in the committed set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuctionOrder {
    pub id: u32,
    pub side: Side,
    pub size: u64,
    pub limit: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Position of the order in the committed set.
    pub id: u32,
    pub side: Side,
    pub quantity: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clearing {
    /// `None` when nothing crosses.
    pub price: Option<u64>,
    pub volume: u64,
    pub fills: Vec<Fill>,
}

fn demand_supply(orders: &[AuctionOrder], p: u64) -> (u128, u128) {
    let mut d = 0u128;
    let mut s = 0u128;
    for o in orders {
        match o.side {
            Side::Buy if o.limit >= p => d += o.size as u128,
            Side::Sell if o.limit <= p => s += o.size as u128,
            _ => {}
        }
    }
    (d, s)
}

/// Clear a set of orders. Pure and deterministic.
pub fn clear(orders: &[AuctionOrder]) -> Clearing {
    let mut prices: Vec<u64> = orders.iter().map(|o| o.limit).collect();
    prices.sort_unstable();
    prices.dedup();

    // (volume, -imbalance, -price) is maximised: most volume, then least imbalance, then lowest price
    let mut best: Option<(u128, u128, u64)> = None;
    for &p in &prices {
        let (d, s) = demand_supply(orders, p);
        let vol = d.min(s);
        let imbalance = d.abs_diff(s);
        let better = match best {
            None => true,
            Some((bv, bi, bp)) => {
                vol > bv
                    || (vol == bv && imbalance < bi)
                    || (vol == bv && imbalance == bi && p < bp)
            }
        };
        if better {
            best = Some((vol, imbalance, p));
        }
    }

    let (volume, price) = match best {
        Some((v, _, p)) if v > 0 => (v, p),
        _ => {
            return Clearing {
                price: None,
                volume: 0,
                fills: Vec::new(),
            }
        }
    };
    let volume_u64 = u64::try_from(volume).expect("executable volume fits in u64");

    let mut buys: Vec<&AuctionOrder> = orders
        .iter()
        .filter(|o| o.side == Side::Buy && o.limit >= price)
        .collect();
    buys.sort_by(|a, b| b.limit.cmp(&a.limit).then(a.id.cmp(&b.id)));
    let mut sells: Vec<&AuctionOrder> = orders
        .iter()
        .filter(|o| o.side == Side::Sell && o.limit <= price)
        .collect();
    sells.sort_by(|a, b| a.limit.cmp(&b.limit).then(a.id.cmp(&b.id)));

    let mut fills = Vec::new();
    for side in [buys, sells] {
        let mut left = volume_u64;
        for o in side {
            if left == 0 {
                break;
            }
            let q = o.size.min(left);
            fills.push(Fill {
                id: o.id,
                side: o.side,
                quantity: q,
            });
            left -= q;
        }
    }
    fills.sort_by_key(|f| f.id);
    Clearing {
        price: Some(price),
        volume: volume_u64,
        fills,
    }
}

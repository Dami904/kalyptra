//! Settlement terms: what each filled order must send and receive, taken from the final
//! co-signed round record.
//!
//! A cleared round settles as **one** atomic swap transaction on Zcash Shielded Assets. Each
//! filled order's owner builds its own part (an action group) with its own wallet, and the
//! round's settlement leader checks every part against these terms and assembles them. Because
//! the terms come from the record, which `t` nodes signed and anyone can recompute, the leader
//! cannot change a price or a size: a part that does not match is rejected by name, and the
//! node rejects any transaction that does not balance.
//!
//! An owner proves a part is theirs by signing it with the order's one-time key (the `owner`
//! in the record), so nobody else can submit, or replace, the part for someone's order.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::order::Side;
use crate::round::{Hash, RoundRecord};
use crate::MatcherError;

/// One filled order's settlement terms, in base units (the equity) and quote units (vUSD).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leg {
    /// Position of the order in the committed set.
    pub order: u32,
    pub owner: [u8; 32],
    pub side: Side,
    /// Filled quantity of the base asset.
    pub base: u64,
    /// `base * price` of the quote asset.
    pub quote: u64,
}

impl Leg {
    /// True if this order sends the base asset (it sold) and receives the quote asset.
    pub fn sends_base(&self) -> bool {
        self.side == Side::Sell
    }

    pub fn send_amount(&self) -> u64 {
        if self.sends_base() {
            self.base
        } else {
            self.quote
        }
    }

    pub fn recv_amount(&self) -> u64 {
        if self.sends_base() {
            self.quote
        } else {
            self.base
        }
    }
}

/// The settlement legs of a final record, one per order with a non-zero fill, sorted by
/// order. Checks that they balance: sellers deliver exactly what buyers receive, in both
/// assets. A record with no price settles nothing.
pub fn legs(record: &RoundRecord) -> Result<Vec<Leg>, MatcherError> {
    let Some(price) = record.price else {
        return Ok(Vec::new());
    };
    let mut out: Vec<Leg> = Vec::new();
    for f in record.fills.iter().filter(|f| f.quantity > 0) {
        let (_, o) =
            record
                .orders
                .iter()
                .find(|(i, _)| *i == f.id)
                .ok_or(MatcherError::Settlement(
                    "a fill names an order not in the record",
                ))?;
        if o.side != f.side || f.quantity > o.size {
            return Err(MatcherError::Settlement("a fill does not match its order"));
        }
        // Nobody trades at a price worse than their own limit: a buyer pays at most its
        // limit, a seller receives at least its limit.
        let within_limit = match o.side {
            Side::Buy => price <= o.limit,
            Side::Sell => price >= o.limit,
        };
        if !within_limit {
            return Err(MatcherError::Settlement(
                "a fill is outside its order's limit price",
            ));
        }
        if out.iter().any(|l| l.order == f.id) {
            return Err(MatcherError::Settlement("an order is filled twice"));
        }
        let quote = f
            .quantity
            .checked_mul(price)
            .ok_or(MatcherError::Settlement("fill value overflows"))?;
        out.push(Leg {
            order: f.id,
            owner: o.owner,
            side: f.side,
            base: f.quantity,
            quote,
        });
    }
    out.sort_by_key(|l| l.order);
    let sum = |side: Side, f: fn(&Leg) -> u64| -> u128 {
        out.iter()
            .filter(|l| l.side == side)
            .map(|l| f(l) as u128)
            .sum()
    };
    if sum(Side::Sell, |l| l.base) != sum(Side::Buy, |l| l.base)
        || sum(Side::Sell, |l| l.quote) != sum(Side::Buy, |l| l.quote)
        || sum(Side::Sell, |l| l.base) != record.volume as u128
    {
        return Err(MatcherError::Settlement("the fills do not balance"));
    }
    Ok(out)
}

/// The settlement leader for a round: rotates through the roster's node ids.
pub fn leader(node_ids: &[u16], round_id: u64) -> Option<u16> {
    let mut ids = node_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return None;
    }
    Some(ids[(round_id % ids.len() as u64) as usize])
}

/// What an owner signs to hand over its part for one order: the round's record, the order,
/// and the exact bytes of the part.
pub fn settle_message(record: &RoundRecord, order: u32, part: &[u8]) -> Vec<u8> {
    let part_hash: Hash = Sha256::digest(part).into();
    let mut m = b"kalyptra/v1/settle".to_vec();
    m.extend_from_slice(&record.hash());
    m.extend_from_slice(&order.to_le_bytes());
    m.extend_from_slice(&part_hash);
    m
}

/// Sign a part with the order's one-time key.
pub fn sign_part(key: &SigningKey, record: &RoundRecord, order: u32, part: &[u8]) -> [u8; 64] {
    key.sign(&settle_message(record, order, part)).to_bytes()
}

/// Check that `sig` is the order owner's signature on this part, and return the order's leg.
pub fn verify_part(
    record: &RoundRecord,
    order: u32,
    part: &[u8],
    sig: &[u8; 64],
) -> Result<Leg, MatcherError> {
    let leg = legs(record)?
        .into_iter()
        .find(|l| l.order == order)
        .ok_or(MatcherError::Settlement("this order has nothing to settle"))?;
    let vk = VerifyingKey::from_bytes(&leg.owner)
        .map_err(|_| MatcherError::Settlement("the owner signature does not verify"))?;
    vk.verify_strict(
        &settle_message(record, order, part),
        &Signature::from_bytes(sig),
    )
    .map_err(|_| MatcherError::Settlement("the owner signature does not verify"))?;
    Ok(leg)
}

/// Split a market name such as `ACME/vUSD` into `(base, quote)`.
pub fn market_assets(market: &str) -> Option<(&str, &str)> {
    let (b, q) = market.split_once('/')?;
    (!b.is_empty() && !q.is_empty() && b != q && !q.contains('/')).then_some((b, q))
}

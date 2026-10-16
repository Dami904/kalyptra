//! Orders, as investors sign them and seal them.
//!
//! An order is signed by the investor's Ed25519 identity key, then sealed with `kalyptra-tse` to
//! the matchers' group key. The round id and the market are inside the signed order **and** in
//! the envelope's authenticated data, so an order cannot be replayed into another round or
//! another market.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::MatcherError;

/// Longest market name, such as `ACME/vUSD`.
pub const MARKET_MAX: usize = 32;
const ORDER_DOMAIN: &[u8] = b"kalyptra/v1/order";
const VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Buy,
    Sell,
}

/// A limit order for one round. `size` is in units of the equity, `limit` in units of the
/// quote asset per unit of equity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub side: Side,
    pub size: u64,
    pub limit: u64,
    pub round_id: u64,
    pub market: String,
    /// Fresh randomness so two identical orders produce different envelopes and hashes.
    pub salt: [u8; 16],
    /// The investor's Ed25519 public key.
    pub investor: [u8; 32],
}

/// The authenticated data every envelope of a round must be sealed under.
pub fn round_aad(round_id: u64, market: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + market.len());
    out.extend_from_slice(b"kalyptra/v1/round");
    out.extend_from_slice(&round_id.to_le_bytes());
    out.push(market.len() as u8);
    out.extend_from_slice(market.as_bytes());
    out
}

impl Order {
    fn body(&self) -> Result<Vec<u8>, MatcherError> {
        if self.market.is_empty() || self.market.len() > MARKET_MAX {
            return Err(MatcherError::BadOrder("market name length"));
        }
        if self.size == 0 || self.limit == 0 {
            return Err(MatcherError::BadOrder("size and limit must be positive"));
        }
        let mut b = Vec::with_capacity(96);
        b.push(VERSION);
        b.push(match self.side {
            Side::Buy => 0,
            Side::Sell => 1,
        });
        b.extend_from_slice(&self.size.to_le_bytes());
        b.extend_from_slice(&self.limit.to_le_bytes());
        b.extend_from_slice(&self.round_id.to_le_bytes());
        b.push(self.market.len() as u8);
        b.extend_from_slice(self.market.as_bytes());
        b.extend_from_slice(&self.salt);
        b.extend_from_slice(&self.investor);
        Ok(b)
    }

    fn signing_message(body: &[u8]) -> Vec<u8> {
        [ORDER_DOMAIN, body].concat()
    }

    /// Encode and sign. The key must be the investor's key named in the order.
    pub fn sign(&self, key: &SigningKey) -> Result<Vec<u8>, MatcherError> {
        if key.verifying_key().to_bytes() != self.investor {
            return Err(MatcherError::BadOrder(
                "signing key is not the order's investor",
            ));
        }
        let body = self.body()?;
        let sig = key.sign(&Self::signing_message(&body));
        Ok([body, sig.to_bytes().to_vec()].concat())
    }

    /// Parse a signed order and verify its signature against the investor key it names.
    pub fn parse_signed(bytes: &[u8]) -> Result<Order, MatcherError> {
        let bad = |m| MatcherError::BadOrder(m);
        if bytes.len() < 1 + 1 + 8 + 8 + 8 + 1 {
            return Err(bad("too short"));
        }
        if bytes[0] != VERSION {
            return Err(bad("unknown version"));
        }
        let side = match bytes[1] {
            0 => Side::Buy,
            1 => Side::Sell,
            _ => return Err(bad("unknown side")),
        };
        let u64_at = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().expect("8 bytes"));
        let (size, limit, round_id) = (u64_at(2), u64_at(10), u64_at(18));
        let mlen = bytes[26] as usize;
        let body_len = 27 + mlen + 16 + 32;
        if mlen == 0 || mlen > MARKET_MAX || bytes.len() != body_len + 64 {
            return Err(bad("wrong length"));
        }
        let market = std::str::from_utf8(&bytes[27..27 + mlen])
            .map_err(|_| bad("market is not utf-8"))?
            .to_string();
        let salt: [u8; 16] = bytes[27 + mlen..27 + mlen + 16]
            .try_into()
            .expect("16 bytes");
        let investor: [u8; 32] = bytes[27 + mlen + 16..body_len]
            .try_into()
            .expect("32 bytes");
        let sig = Signature::from_bytes(bytes[body_len..].try_into().expect("64 bytes"));
        let vk = VerifyingKey::from_bytes(&investor).map_err(|_| bad("investor key"))?;
        vk.verify_strict(&Self::signing_message(&bytes[..body_len]), &sig)
            .map_err(|_| bad("signature does not verify"))?;
        let order = Order {
            side,
            size,
            limit,
            round_id,
            market,
            salt,
            investor,
        };
        // re-encode to reject any non-canonical input
        if order.body()? != bytes[..body_len] {
            return Err(bad("non-canonical encoding"));
        }
        Ok(order)
    }

    /// A stable identifier for logs; not secret.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"kalyptra/v1/order-id");
        h.update(self.body().unwrap_or_default());
        h.finalize().into()
    }
}

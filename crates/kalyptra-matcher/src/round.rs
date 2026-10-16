//! Round configuration and the signed statements matcher nodes exchange.
//!
//! Every statement is signed by a node's Ed25519 key (separate from its decryption share) over
//! a domain-separated encoding, so a signature for one kind of statement cannot be passed off
//! as another.

use std::collections::BTreeMap;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::auction::Fill;
use crate::order::Side;
use crate::MatcherError;

pub type Hash = [u8; 32];

/// Short hex for messages.
pub fn short(h: &Hash) -> String {
    h[..6].iter().map(|b| format!("{b:02x}")).collect()
}

fn sha(domain: &[u8], parts: &[&[u8]]) -> Hash {
    let mut h = Sha256::new();
    h.update(domain);
    for p in parts {
        h.update((p.len() as u64).to_le_bytes());
        h.update(p);
    }
    h.finalize().into()
}

/// Identifier of an envelope on the board: a hash of its exact bytes.
pub fn envelope_hash(envelope_bytes: &[u8]) -> Hash {
    sha(b"kalyptra/v1/envelope", &[envelope_bytes])
}

/// Hash of a committed order set: the round id and the sorted envelope hashes.
pub fn set_hash(round_id: u64, envelopes: &[Hash]) -> Hash {
    let mut sorted = envelopes.to_vec();
    sorted.sort();
    let joined: Vec<u8> = sorted.concat();
    sha(b"kalyptra/v1/set", &[&round_id.to_le_bytes(), &joined])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundConfig {
    pub round_id: u64,
    /// Such as `ACME/vUSD`.
    pub market: String,
    /// A round with fewer committed orders is voided: nothing in it is ever decrypted.
    pub min_orders: usize,
}

/// The roster: every node's signing key, plus the threshold `t` that statements need.
#[derive(Clone, Debug)]
pub struct Roster {
    pub threshold: usize,
    pub nodes: BTreeMap<u16, VerifyingKey>,
}

impl Roster {
    pub fn key(&self, node: u16) -> Result<&VerifyingKey, MatcherError> {
        self.nodes.get(&node).ok_or(MatcherError::UnknownNode(node))
    }

    /// Check `sig` is `node`'s signature on `msg`.
    pub fn verify(&self, node: u16, msg: &[u8], sig: &Signature) -> Result<(), MatcherError> {
        self.key(node)?
            .verify_strict(msg, sig)
            .map_err(|_| MatcherError::BadSignature(node))
    }
}

/// "Node `node` accepted envelope `envelope` into round `round_id`." An envelope with receipts
/// from `t` nodes must be in the committed set; if it is not, the receipts prove censorship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub node: u16,
    pub round_id: u64,
    pub envelope: Hash,
    pub sig: Signature,
}

fn receipt_msg(round_id: u64, envelope: &Hash) -> Vec<u8> {
    [
        b"kalyptra/v1/receipt".as_slice(),
        &round_id.to_le_bytes(),
        envelope,
    ]
    .concat()
}

impl Receipt {
    pub fn new(key: &SigningKey, node: u16, round_id: u64, envelope: Hash) -> Self {
        let sig = key.sign(&receipt_msg(round_id, &envelope));
        Receipt {
            node,
            round_id,
            envelope,
            sig,
        }
    }

    pub fn verify(&self, roster: &Roster) -> Result<(), MatcherError> {
        roster.verify(
            self.node,
            &receipt_msg(self.round_id, &self.envelope),
            &self.sig,
        )
    }
}

/// The highest epoch a commit vote may carry. Each epoch needs the one before it to be
/// provably unable to finish, so a handful is plenty; the cap stops a faulty node from making
/// others store votes without end.
pub const MAX_EPOCH: u32 = 16;

/// "At close, node `node` commits round `round_id` to exactly this set of envelopes", or votes
/// to void it. `t` matching votes make the commit final.
///
/// The vote also signs the round's `min_orders`, so a published round cannot be given a
/// different minimum afterwards: whether a round was rightly voided is checkable from the
/// signed votes alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitVote {
    pub node: u16,
    pub round_id: u64,
    /// Which attempt at agreeing this vote belongs to; 0 is the first. A node votes again, in
    /// the next epoch, only when the previous one can no longer reach `t` matching votes (for
    /// example because the nodes saw different receipts at close). See `MatcherNode::revote`.
    pub epoch: u32,
    /// Strictly increasing: sorted, and every envelope at most once.
    pub set: Vec<Hash>,
    pub min_orders: u32,
    pub voided: bool,
    pub sig: Signature,
}

fn commit_msg(round_id: u64, epoch: u32, set: &[Hash], min_orders: u32, voided: bool) -> Vec<u8> {
    [
        b"kalyptra/v1/commit".as_slice(),
        &round_id.to_le_bytes(),
        &epoch.to_le_bytes(),
        &set_hash(round_id, set),
        &(set.len() as u32).to_le_bytes(),
        &min_orders.to_le_bytes(),
        &[voided as u8],
    ]
    .concat()
}

impl CommitVote {
    /// A vote in the first epoch.
    pub fn new(
        key: &SigningKey,
        node: u16,
        round_id: u64,
        set: Vec<Hash>,
        min_orders: usize,
        voided: bool,
    ) -> Self {
        Self::in_epoch(key, node, round_id, 0, set, min_orders, voided)
    }

    pub fn in_epoch(
        key: &SigningKey,
        node: u16,
        round_id: u64,
        epoch: u32,
        mut set: Vec<Hash>,
        min_orders: usize,
        voided: bool,
    ) -> Self {
        // Sorted, but deliberately NOT de-duplicated: `well_formed` rejects a set with a
        // repeated envelope, and tests need to be able to build one.
        set.sort();
        let min_orders = u32::try_from(min_orders).unwrap_or(u32::MAX);
        let sig = key.sign(&commit_msg(round_id, epoch, &set, min_orders, voided));
        CommitVote {
            node,
            round_id,
            epoch,
            set,
            min_orders,
            voided,
            sig,
        }
    }

    /// The roster signature only.
    pub fn verify(&self, roster: &Roster) -> Result<(), MatcherError> {
        roster.verify(
            self.node,
            &commit_msg(
                self.round_id,
                self.epoch,
                &self.set,
                self.min_orders,
                self.voided,
            ),
            &self.sig,
        )
    }

    /// The set is strictly increasing. A repeated envelope would be counted twice by the
    /// auction, giving that order double weight.
    pub fn well_formed(&self) -> Result<(), MatcherError> {
        if self.epoch > MAX_EPOCH {
            return Err(MatcherError::BadCommit(
                "the vote's epoch is beyond the limit",
            ));
        }
        if self.set.windows(2).all(|w| w[0] < w[1]) {
            Ok(())
        } else {
            Err(MatcherError::BadCommit(
                "the committed set must be sorted and hold every envelope at most once",
            ))
        }
    }

    /// Whether `voided` is the right decision for this set and minimum.
    pub fn void_is_consistent(&self) -> bool {
        self.voided == ((self.set.len() as u64) < u64::from(self.min_orders))
    }

    /// What two votes must agree on to count together.
    pub fn key(&self) -> (Hash, bool, u32) {
        (
            set_hash(self.round_id, &self.set),
            self.voided,
            self.min_orders,
        )
    }
}

/// An order as published after the round: no salt, and no lasting identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicOrder {
    pub side: Side,
    pub size: u64,
    pub limit: u64,
    /// The order's one-time signing key. It names no one (investors use a fresh key per
    /// order) but only its holder can authorise this order's settlement.
    pub owner: [u8; 32],
}

/// The public result of a round. `orders` follow the committed set's order (sorted envelope
/// hash); an envelope whose content is not a valid order for this round is listed in
/// `invalid` and takes no part in the auction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundRecord {
    pub round_id: u64,
    pub market: String,
    pub set: Vec<Hash>,
    /// `(position in set, order)` for every valid order.
    pub orders: Vec<(u32, PublicOrder)>,
    /// Positions in `set` whose envelope did not hold a valid order.
    pub invalid: Vec<u32>,
    pub price: Option<u64>,
    pub volume: u64,
    pub fills: Vec<Fill>,
}

impl RoundRecord {
    /// Canonical encoding; this is what nodes sign.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"kalyptra/v1/record");
        b.extend_from_slice(&self.round_id.to_le_bytes());
        b.push(self.market.len() as u8);
        b.extend_from_slice(self.market.as_bytes());
        b.extend_from_slice(&set_hash(self.round_id, &self.set));
        b.extend_from_slice(&(self.orders.len() as u32).to_le_bytes());
        for (i, o) in &self.orders {
            b.extend_from_slice(&i.to_le_bytes());
            b.push(o.side as u8);
            b.extend_from_slice(&o.size.to_le_bytes());
            b.extend_from_slice(&o.limit.to_le_bytes());
            b.extend_from_slice(&o.owner);
        }
        b.extend_from_slice(&(self.invalid.len() as u32).to_le_bytes());
        for i in &self.invalid {
            b.extend_from_slice(&i.to_le_bytes());
        }
        match self.price {
            None => b.push(0),
            Some(p) => {
                b.push(1);
                b.extend_from_slice(&p.to_le_bytes());
            }
        }
        b.extend_from_slice(&self.volume.to_le_bytes());
        b.extend_from_slice(&(self.fills.len() as u32).to_le_bytes());
        for f in &self.fills {
            b.extend_from_slice(&f.id.to_le_bytes());
            b.push(f.side as u8);
            b.extend_from_slice(&f.quantity.to_le_bytes());
        }
        b
    }

    pub fn hash(&self) -> Hash {
        sha(b"kalyptra/v1/record-hash", &[&self.encode()])
    }
}

/// "Node `node` signs this round record." `t` matching votes make the record final.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordVote {
    pub node: u16,
    pub round_id: u64,
    pub record: Hash,
    pub sig: Signature,
}

fn record_vote_msg(round_id: u64, record: &Hash) -> Vec<u8> {
    [
        b"kalyptra/v1/record-vote".as_slice(),
        &round_id.to_le_bytes(),
        record,
    ]
    .concat()
}

impl RecordVote {
    pub fn new(key: &SigningKey, node: u16, record: &RoundRecord) -> Self {
        let h = record.hash();
        RecordVote {
            node,
            round_id: record.round_id,
            record: h,
            sig: key.sign(&record_vote_msg(record.round_id, &h)),
        }
    }

    pub fn verify(&self, roster: &Roster) -> Result<(), MatcherError> {
        roster.verify(
            self.node,
            &record_vote_msg(self.round_id, &self.record),
            &self.sig,
        )
    }
}

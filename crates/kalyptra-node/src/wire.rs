//! JSON forms of everything that crosses the network or sits in a file. Bytes are hex.
//! Converting back always re-validates: signatures are checked by the receiver, points and
//! scalars are parsed strictly.

use std::collections::BTreeMap;

use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use ed25519_dalek::{Signature, VerifyingKey};
use kalyptra_matcher::auction::Fill;
use kalyptra_matcher::round::PublicOrder;
use kalyptra_matcher::{
    CommitVote, PublicRound, Receipt, RecordVote, Roster, RoundConfig, RoundRecord, Side,
};
use kalyptra_tse::{GroupKey, PartialDecryption};
use serde::{Deserialize, Serialize};

pub type WireResult<T> = Result<T, String>;

pub fn hx(b: &[u8]) -> String {
    hex::encode(b)
}

pub fn unhex(s: &str) -> WireResult<Vec<u8>> {
    hex::decode(s).map_err(|e| format!("bad hex: {e}"))
}

pub fn unhex_n<const N: usize>(s: &str) -> WireResult<[u8; N]> {
    unhex(s)?
        .try_into()
        .map_err(|_| format!("expected {N} bytes"))
}

pub fn point(s: &str) -> WireResult<RistrettoPoint> {
    CompressedRistretto(unhex_n::<32>(s)?)
        .decompress()
        .ok_or_else(|| "not a valid ristretto point".to_string())
}

fn sig(s: &str) -> WireResult<Signature> {
    Ok(Signature::from_bytes(&unhex_n::<64>(s)?))
}

fn side_str(s: Side) -> String {
    match s {
        Side::Buy => "buy".into(),
        Side::Sell => "sell".into(),
    }
}

pub fn parse_side(s: &str) -> WireResult<Side> {
    match s {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        other => Err(format!("side must be buy or sell, not {other}")),
    }
}

// ---- group key and roster --------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupJ {
    pub threshold: u16,
    pub pk: String,
    pub shares: BTreeMap<u16, String>,
}

impl GroupJ {
    pub fn from_group(g: &GroupKey) -> Self {
        GroupJ {
            threshold: g.threshold(),
            pk: hx(g.public_key().compress().as_bytes()),
            shares: g
                .node_ids()
                .into_iter()
                .map(|id| {
                    (
                        id,
                        hx(g.verifying_share(id).expect("listed").compress().as_bytes()),
                    )
                })
                .collect(),
        }
    }

    /// Rebuilds and re-checks the group key (consistency and exact threshold).
    pub fn to_group(&self) -> WireResult<GroupKey> {
        let shares = self
            .shares
            .iter()
            .map(|(id, s)| Ok((*id, point(s)?)))
            .collect::<WireResult<BTreeMap<_, _>>>()?;
        GroupKey::new(self.threshold, point(&self.pk)?, shares).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RosterNodeJ {
    pub id: u16,
    /// Base URL, such as `http://127.0.0.1:7001`.
    pub url: String,
    /// Ed25519 key that signs this node's statements.
    pub sign_pk: String,
    /// Ristretto key that receives this node's private key-generation messages.
    pub dkg_pk: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RosterJ {
    pub threshold: u16,
    pub nodes: Vec<RosterNodeJ>,
}

impl RosterJ {
    pub fn roster(&self) -> WireResult<Roster> {
        let mut nodes = BTreeMap::new();
        for n in &self.nodes {
            let vk = VerifyingKey::from_bytes(&unhex_n::<32>(&n.sign_pk)?)
                .map_err(|_| format!("node {}: bad signing key", n.id))?;
            if nodes.insert(n.id, vk).is_some() {
                return Err(format!("node {} listed twice", n.id));
            }
        }
        if self.threshold == 0 || self.threshold as usize > nodes.len() {
            return Err("threshold must be between 1 and the number of nodes".into());
        }
        Ok(Roster {
            threshold: self.threshold as usize,
            nodes,
        })
    }

    pub fn node(&self, id: u16) -> WireResult<&RosterNodeJ> {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("node {id} is not in the roster"))
    }

    pub fn urls(&self) -> Vec<String> {
        self.nodes.iter().map(|n| n.url.clone()).collect()
    }
}

// ---- rounds and statements -------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigJ {
    pub round_id: u64,
    pub market: String,
    pub min_orders: usize,
}

impl From<&RoundConfig> for ConfigJ {
    fn from(c: &RoundConfig) -> Self {
        ConfigJ {
            round_id: c.round_id,
            market: c.market.clone(),
            min_orders: c.min_orders,
        }
    }
}

impl ConfigJ {
    pub fn config(&self) -> RoundConfig {
        RoundConfig {
            round_id: self.round_id,
            market: self.market.clone(),
            min_orders: self.min_orders,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptJ {
    pub node: u16,
    pub round_id: u64,
    pub envelope: String,
    pub sig: String,
}

impl From<&Receipt> for ReceiptJ {
    fn from(r: &Receipt) -> Self {
        ReceiptJ {
            node: r.node,
            round_id: r.round_id,
            envelope: hx(&r.envelope),
            sig: hx(&r.sig.to_bytes()),
        }
    }
}

impl ReceiptJ {
    pub fn receipt(&self) -> WireResult<Receipt> {
        Ok(Receipt {
            node: self.node,
            round_id: self.round_id,
            envelope: unhex_n(&self.envelope)?,
            sig: sig(&self.sig)?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitVoteJ {
    pub node: u16,
    pub round_id: u64,
    #[serde(default)]
    pub epoch: u32,
    pub set: Vec<String>,
    pub min_orders: u32,
    pub voided: bool,
    pub sig: String,
}

impl From<&CommitVote> for CommitVoteJ {
    fn from(v: &CommitVote) -> Self {
        CommitVoteJ {
            node: v.node,
            round_id: v.round_id,
            epoch: v.epoch,
            set: v.set.iter().map(|h| hx(h)).collect(),
            min_orders: v.min_orders,
            voided: v.voided,
            sig: hx(&v.sig.to_bytes()),
        }
    }
}

impl CommitVoteJ {
    pub fn vote(&self) -> WireResult<CommitVote> {
        Ok(CommitVote {
            node: self.node,
            round_id: self.round_id,
            epoch: self.epoch,
            set: self
                .set
                .iter()
                .map(|h| unhex_n(h))
                .collect::<WireResult<_>>()?,
            min_orders: self.min_orders,
            voided: self.voided,
            sig: sig(&self.sig)?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordVoteJ {
    pub node: u16,
    pub round_id: u64,
    pub record: String,
    pub sig: String,
}

impl From<&RecordVote> for RecordVoteJ {
    fn from(v: &RecordVote) -> Self {
        RecordVoteJ {
            node: v.node,
            round_id: v.round_id,
            record: hx(&v.record),
            sig: hx(&v.sig.to_bytes()),
        }
    }
}

impl RecordVoteJ {
    pub fn vote(&self) -> WireResult<RecordVote> {
        Ok(RecordVote {
            node: self.node,
            round_id: self.round_id,
            record: unhex_n(&self.record)?,
            sig: sig(&self.sig)?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialJ {
    pub envelope: String,
    pub partial: String,
}

impl PartialJ {
    pub fn new(envelope: &[u8; 32], p: &PartialDecryption) -> Self {
        PartialJ {
            envelope: hx(envelope),
            partial: hx(&p.to_bytes()),
        }
    }

    pub fn parts(&self) -> WireResult<([u8; 32], PartialDecryption)> {
        Ok((
            unhex_n(&self.envelope)?,
            PartialDecryption::from_bytes(&unhex(&self.partial)?).map_err(|e| e.to_string())?,
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderJ {
    pub side: String,
    pub size: u64,
    pub limit: u64,
    /// The order's one-time key (hex).
    pub owner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FillJ {
    pub id: u32,
    pub side: String,
    pub quantity: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordJ {
    pub round_id: u64,
    pub market: String,
    pub set: Vec<String>,
    pub orders: Vec<(u32, OrderJ)>,
    pub invalid: Vec<u32>,
    pub price: Option<u64>,
    pub volume: u64,
    pub fills: Vec<FillJ>,
}

impl From<&RoundRecord> for RecordJ {
    fn from(r: &RoundRecord) -> Self {
        RecordJ {
            round_id: r.round_id,
            market: r.market.clone(),
            set: r.set.iter().map(|h| hx(h)).collect(),
            orders: r
                .orders
                .iter()
                .map(|(i, o)| {
                    (
                        *i,
                        OrderJ {
                            side: side_str(o.side),
                            size: o.size,
                            limit: o.limit,
                            owner: hx(&o.owner),
                        },
                    )
                })
                .collect(),
            invalid: r.invalid.clone(),
            price: r.price,
            volume: r.volume,
            fills: r
                .fills
                .iter()
                .map(|f| FillJ {
                    id: f.id,
                    side: side_str(f.side),
                    quantity: f.quantity,
                })
                .collect(),
        }
    }
}

impl RecordJ {
    pub fn record(&self) -> WireResult<RoundRecord> {
        Ok(RoundRecord {
            round_id: self.round_id,
            market: self.market.clone(),
            set: self
                .set
                .iter()
                .map(|h| unhex_n(h))
                .collect::<WireResult<_>>()?,
            orders: self
                .orders
                .iter()
                .map(|(i, o)| {
                    Ok((
                        *i,
                        PublicOrder {
                            side: parse_side(&o.side)?,
                            size: o.size,
                            limit: o.limit,
                            owner: unhex_n(&o.owner)?,
                        },
                    ))
                })
                .collect::<WireResult<_>>()?,
            invalid: self.invalid.clone(),
            price: self.price,
            volume: self.volume,
            fills: self
                .fills
                .iter()
                .map(|f| {
                    Ok(Fill {
                        id: f.id,
                        side: parse_side(&f.side)?,
                        quantity: f.quantity,
                    })
                })
                .collect::<WireResult<_>>()?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicRoundJ {
    pub config: ConfigJ,
    pub envelopes: Vec<String>,
    pub receipts: Vec<ReceiptJ>,
    pub commit_votes: Vec<CommitVoteJ>,
    pub partials: Vec<PartialJ>,
    pub record: Option<RecordJ>,
    pub record_votes: Vec<RecordVoteJ>,
}

impl From<&PublicRound> for PublicRoundJ {
    fn from(p: &PublicRound) -> Self {
        PublicRoundJ {
            config: (&p.config).into(),
            envelopes: p.envelopes.iter().map(|e| hx(e)).collect(),
            receipts: p.receipts.iter().map(Into::into).collect(),
            commit_votes: p.commit_votes.iter().map(Into::into).collect(),
            partials: p
                .partials
                .iter()
                .map(|(h, x)| PartialJ::new(h, x))
                .collect(),
            record: p.record.as_ref().map(Into::into),
            record_votes: p.record_votes.iter().map(Into::into).collect(),
        }
    }
}

impl PublicRoundJ {
    pub fn public(&self) -> WireResult<PublicRound> {
        Ok(PublicRound {
            config: self.config.config(),
            envelopes: self
                .envelopes
                .iter()
                .map(|e| unhex(e))
                .collect::<WireResult<_>>()?,
            receipts: self
                .receipts
                .iter()
                .map(|r| r.receipt())
                .collect::<WireResult<_>>()?,
            commit_votes: self
                .commit_votes
                .iter()
                .map(|v| v.vote())
                .collect::<WireResult<_>>()?,
            partials: self
                .partials
                .iter()
                .map(|p| p.parts())
                .collect::<WireResult<_>>()?,
            record: self.record.as_ref().map(|r| r.record()).transpose()?,
            record_votes: self
                .record_votes
                .iter()
                .map(|v| v.vote())
                .collect::<WireResult<_>>()?,
        })
    }
}

// ---- messages between nodes ------------------------------------------------------------------

/// Gossip between matcher nodes. Every kind is self-authenticating: envelopes carry the
/// sealer's proof, statements carry roster signatures, partials carry proofs. The transport
/// needs no authentication for these.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Gossip {
    Envelope { round_id: u64, envelope: String },
    Receipt(ReceiptJ),
    Commit(CommitVoteJ),
    Partial { round_id: u64, partial: PartialJ },
    RecordVote(RecordVoteJ),
}

/// Key generation, round 1: a public broadcast, signed by the sender's roster key.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dkg1J {
    pub from: u16,
    pub msg: String,
    pub sig: String,
}

/// Key generation, round 2: a private share, sealed to the recipient's key and signed by the
/// sender's roster key.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dkg2J {
    pub from: u16,
    pub to: u16,
    pub envelope: String,
    pub sig: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusJ {
    pub id: u16,
    pub dkg_done: bool,
    pub group: Option<GroupJ>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundStatusJ {
    pub round_id: u64,
    pub phase: String,
    /// Present once `t` nodes signed the record and it matches this node's own computation.
    pub record: Option<RecordJ>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmitJ {
    pub envelope: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorJ {
    pub error: String,
}

// ---- settlement ------------------------------------------------------------------------------

/// What an investor keeps after submitting an order: everything needed to settle it later.
/// `key` is the order's one-time signing key, so this file is secret.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TicketJ {
    pub round_id: u64,
    pub market: String,
    pub side: String,
    pub size: u64,
    pub limit: u64,
    /// The sealed envelope's hash: how the investor finds its order in the record.
    pub envelope: String,
    pub key: String,
}

/// An owner hands over its part (a finished action group) for one order.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettleJ {
    pub order: u32,
    pub part: String,
    pub sig: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegJ {
    pub order: u32,
    pub side: String,
    pub base: u64,
    pub quote: u64,
    /// This node holds a verified part for the order.
    pub received: bool,
}

/// A node's view of a round's settlement.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettlementJ {
    pub round_id: u64,
    pub leader: u16,
    pub legs: Vec<LegJ>,
    /// `collecting`, `assembling`, `settled`, `failed` or `nothing` (no fills).
    pub status: String,
    pub txid: Option<String>,
    pub height: Option<u32>,
    /// Orders whose parts did not match their legs (the leader dropped them).
    pub bad: Vec<u32>,
    pub error: Option<String>,
}

impl LegJ {
    pub fn new(l: &kalyptra_matcher::settlement::Leg, received: bool) -> Self {
        LegJ {
            order: l.order,
            side: side_str(l.side),
            base: l.base,
            quote: l.quote,
            received,
        }
    }
}

/// Asset ids by symbol (`ACME`, `vUSD`), hex. Written once, when the assets are issued.
pub type AssetsJ = BTreeMap<String, String>;

/// A company listing: which asset id stands for a symbol.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListingJ {
    pub symbol: String,
    pub asset_id: String,
}

/// What a node signs to accept a listing. The signature says the node accepted this binding of
/// symbol to asset id (and found it well formed and not in conflict with one it already holds).
/// It does not say the supply is real; the nodes cannot see inside a shielded issuance.
pub fn listing_msg(symbol: &str, asset_id: &[u8; 32]) -> Vec<u8> {
    [
        b"kalyptra/v1/listing".as_slice(),
        &(symbol.len() as u32).to_le_bytes(),
        symbol.as_bytes(),
        asset_id,
    ]
    .concat()
}

/// One node's signature on a listing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListingSigJ {
    pub symbol: String,
    pub asset_id: String,
    pub node: u16,
    pub sig: String,
}

impl ListingSigJ {
    /// Check the node's signature against the roster.
    pub fn verify(&self, roster: &Roster) -> WireResult<()> {
        let id = unhex_n::<32>(&self.asset_id)?;
        roster
            .verify(self.node, &listing_msg(&self.symbol, &id), &sig(&self.sig)?)
            .map_err(|e| e.to_string())
    }
}

/// A symbol is 1 to 12 letters or digits.
pub fn valid_symbol(s: &str) -> bool {
    !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// One line of a node's list of rounds.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundSummaryJ {
    pub round_id: u64,
    pub market: String,
    pub min_orders: usize,
    pub phase: String,
    /// Sealed envelopes this node holds.
    pub orders: usize,
}

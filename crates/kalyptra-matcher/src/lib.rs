//! # kalyptra-matcher: the matcher node
//!
//! A Kalyptra round is a sealed-bid, uniform-price auction run by `n` matcher nodes, any `t` of
//! which are needed to open orders. This crate is the node's logic, transport-agnostic:
//!
//! * [`order`]: investors' signed orders, and the round's authenticated data;
//! * [`node`]: the bulletin board and the round state machine (open, close, commit or void,
//!   release partials, clear, sign the record);
//! * [`auction`]: the uniform-price call auction;
//! * [`round`]: the signed statements nodes exchange (receipts, commit votes, record votes);
//! * [`checker`]: re-verifies a whole round from its public data, trusting no node;
//! * [`settlement`]: each filled order's swap terms, and the owner's authorisation of its part.
//!
//! Encryption to the group key and threshold opening come from `kalyptra-tse`.
//!
//! ## Trust, in one paragraph
//!
//! Fewer than `t` nodes can neither read an order before the round closes, nor silently drop
//! an order that `t` nodes receipted (the receipts prove it), nor publish a price that the
//! checker does not reproduce. `t` colluding nodes can open orders early; that is the security
//! threshold. Investors must use a **fresh signing key for every order**, because committed
//! orders are opened publicly after the round so the price can be checked.

pub mod auction;
pub mod checker;
mod error;
pub mod node;
pub mod order;
pub mod round;
pub mod settlement;

pub use checker::{check, CheckReport, Finding, PublicRound};
pub use error::MatcherError;
pub use node::{MatcherNode, Phase};
pub use order::{round_aad, Order, Side};
pub use round::{
    CommitVote, Hash, Receipt, RecordVote, Roster, RoundConfig, RoundRecord, MAX_EPOCH,
};

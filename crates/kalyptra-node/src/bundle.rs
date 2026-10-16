//! A round saved to one file, and verified offline.
//!
//! `export` fetches every node's view of a round, merges it, and writes it with the group key.
//! `verify` needs no network and trusts no node: it re-runs the public checker on the file.
//!
//! What a verifier must supply from outside the file:
//!
//! * **the roster** (each node's signing key and the threshold), which is what makes votes and
//!   receipts count;
//! * ideally **the group key** the investors sealed to, as announced when the nodes started.
//!   Without it the file's own group key is used, and `verify` says so. A made-up group key
//!   cannot pass anyway: sealer proofs in the real envelopes are bound to the real key, so
//!   every envelope would fail. Pinning rules out a key that is real but not the announced one.

use kalyptra_matcher::{check as check_round, CheckReport};
use serde::{Deserialize, Serialize};

use crate::client;
use crate::wire::{hx, GroupJ, PublicRoundJ, RosterJ, WireResult};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleJ {
    /// The group key the round was sealed to.
    pub group: GroupJ,
    pub round: PublicRoundJ,
}

/// The outcome of an offline verification.
pub struct Verification {
    pub report: CheckReport,
    /// The group key the check used, hex.
    pub group_key: String,
    /// True if the caller pinned the group key and it matched.
    pub pinned: bool,
}

/// Fetch and merge every node's view of a round, with the group key.
pub fn export(roster: &RosterJ, round_id: u64) -> WireResult<BundleJ> {
    let group = client::group_key(roster)?;
    let round = client::fetch_public(roster, round_id)?;
    Ok(BundleJ {
        group: GroupJ::from_group(&group),
        round: PublicRoundJ::from(&round),
    })
}

/// Re-check a saved round. `pin` is the announced group key (hex), if the verifier has it.
pub fn verify(bundle: &BundleJ, roster: &RosterJ, pin: Option<&str>) -> WireResult<Verification> {
    let group = bundle.group.to_group()?;
    let checker = roster.roster()?;
    let key = hx(group.public_key().compress().as_bytes());
    if let Some(pin) = pin {
        if !pin.trim().eq_ignore_ascii_case(&key) {
            return Err(format!(
                "the file's group key {key} is not the pinned key {}",
                pin.trim()
            ));
        }
    }
    let mut ids = group.node_ids();
    ids.sort_unstable();
    let mut roster_ids: Vec<u16> = roster.nodes.iter().map(|n| n.id).collect();
    roster_ids.sort_unstable();
    if ids != roster_ids || group.threshold() != roster.threshold {
        return Err("the file's group key and the roster describe different node sets".into());
    }
    let public = bundle.round.public()?;
    Ok(Verification {
        report: check_round(&public, &group, &checker),
        group_key: key,
        pinned: pin.is_some(),
    })
}

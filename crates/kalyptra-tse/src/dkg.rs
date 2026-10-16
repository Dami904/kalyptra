//! Distributed key generation, one node at a time.
//!
//! Each node runs its own [`Participant`] and exchanges only bytes with the others. No node,
//! and no dealer, ever sees the group secret. The protocol is the Zcash Foundation's FROST
//! distributed key generation (a Pedersen DKG with proofs of knowledge), in three rounds:
//!
//! 1. **Round 1 (broadcast):** each node publishes a commitment to its secret polynomial and a
//!    proof that it knows the constant term. Everyone checks everyone else's proof.
//! 2. **Round 2 (private, one message per peer):** each node sends every peer that peer's share
//!    of its polynomial.
//! 3. **Round 3 (local):** each node checks the shares it received against the round 1
//!    commitments, adds them up into its own secret share, and computes the group key.
//!
//! A node that cheats in round 1 or round 2 is named in the error ([`Error::DkgBlame`]). That
//! is a local judgement by the node that saw the bad message: there is no complaint protocol,
//! so the blame cannot be proven to the other nodes, and because sender ids are not
//! authenticated here, a network attacker could frame an honest node. Treat a blame as a
//! reason to abort and investigate, not as proof.
//!
//! ## What the caller must provide
//!
//! * **Authentication.** Messages carry a sender id, but nothing here proves who sent them.
//!   Run the exchange over authenticated channels.
//! * **Confidentiality for round 2.** Each round 2 message holds a secret share. Send it
//!   encrypted, to its recipient only. Round 1 messages are public.
//! * **Agreement on round 1.** Every node must see the same round 1 messages (a reliable
//!   broadcast). If a node sends different round 1 messages to different peers, the group
//!   key can differ per node; compare the final [`GroupKey`] values across nodes.

use std::collections::BTreeMap;

use frost_ristretto255 as frost;
use rand_core::{CryptoRng, RngCore};

use crate::keys::{dkg_err, id_to_u16, GroupKey, NodeKey};
use crate::Error;

/// A private round 2 message, addressed to one peer. `bytes` start with the sender's id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round2Msg {
    pub to: u16,
    pub bytes: Vec<u8>,
}

enum State {
    New,
    AfterRound1(frost::keys::dkg::round1::SecretPackage),
    AfterRound2(frost::keys::dkg::round2::SecretPackage),
    Done,
}

/// One node's side of the key generation. Node ids are `1..=n`.
pub struct Participant {
    id: u16,
    n: u16,
    t: u16,
    state: State,
    /// This node's own round 1 broadcast, to recognise it if it comes back in the list.
    my_round1: Option<Vec<u8>>,
}

fn fid(i: u16) -> Result<frost::Identifier, Error> {
    frost::Identifier::try_from(i).map_err(dkg_err)
}

fn blame(e: frost::Error) -> Error {
    // FROST names the culprit(s) for a bad proof of knowledge or a bad secret share
    match e.culprits().first().map(id_to_u16) {
        Some(Ok(node)) => Error::DkgBlame {
            node,
            reason: e.to_string(),
        },
        _ => Error::Dkg(e.to_string()),
    }
}

/// Split `[sender id (2 bytes)][payload]` and check the sender is a valid peer.
fn split_sender(msg: &[u8], me: u16, n: u16) -> Result<(u16, &[u8]), Error> {
    if msg.len() < 2 {
        return Err(Error::BadLength);
    }
    let sender = u16::from_le_bytes([msg[0], msg[1]]);
    if sender == 0 || sender > n || sender == me {
        return Err(Error::Dkg(format!("message from invalid sender {sender}")));
    }
    Ok((sender, &msg[2..]))
}

fn prefixed(sender: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + payload.len());
    out.extend_from_slice(&sender.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

impl Participant {
    /// `id` in `1..=n`, threshold `t` with `1 <= t <= n`.
    pub fn new(id: u16, n: u16, t: u16) -> Result<Self, Error> {
        if id == 0 || id > n || t == 0 || t > n {
            return Err(Error::Dkg("need 1 <= id <= n and 1 <= t <= n".into()));
        }
        Ok(Participant {
            id,
            n,
            t,
            state: State::New,
            my_round1: None,
        })
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    /// Round 1. Returns the message to broadcast to every other node (public).
    pub fn round1<R: RngCore + CryptoRng>(&mut self, rng: &mut R) -> Result<Vec<u8>, Error> {
        if !matches!(self.state, State::New) {
            return Err(Error::Dkg("round 1 already done".into()));
        }
        let (secret, public) =
            frost::keys::dkg::part1(fid(self.id)?, self.n, self.t, rng).map_err(dkg_err)?;
        let bytes = public.serialize().map_err(dkg_err)?;
        self.state = State::AfterRound1(secret);
        let msg = prefixed(self.id, &bytes);
        self.my_round1 = Some(msg.clone());
        Ok(msg)
    }

    fn parse_round1(
        &self,
        msgs: &[Vec<u8>],
    ) -> Result<BTreeMap<frost::Identifier, frost::keys::dkg::round1::Package>, Error> {
        let mut out = BTreeMap::new();
        for m in msgs {
            // Our own broadcast may be in the list: skip it, but only if it is byte-for-byte
            // what we sent. Anything else claiming our id is an impersonation.
            if m.len() >= 2 && u16::from_le_bytes([m[0], m[1]]) == self.id {
                if self.my_round1.as_deref() == Some(m.as_slice()) {
                    continue;
                }
                return Err(Error::Dkg(
                    "a round 1 message claims to come from this node but is not ours".into(),
                ));
            }
            let (sender, payload) = split_sender(m, self.id, self.n)?;
            let pkg = frost::keys::dkg::round1::Package::deserialize(payload).map_err(dkg_err)?;
            if out.insert(fid(sender)?, pkg).is_some() {
                return Err(Error::Dkg(format!(
                    "two round 1 messages from node {sender}"
                )));
            }
        }
        if out.len() != self.n as usize - 1 {
            return Err(Error::Dkg(format!(
                "need round 1 messages from all {} peers, got {}",
                self.n - 1,
                out.len()
            )));
        }
        Ok(out)
    }

    /// Round 2. Takes everyone's round 1 broadcasts (your own may be included) and returns
    /// one **private** message for each peer. Fails, naming the culprit, if a peer's proof of
    /// knowledge does not verify.
    ///
    /// Malformed, missing or duplicate input is rejected without consuming this node's state,
    /// so the call can be retried with corrected input. A failure inside the key generation
    /// itself (a bad proof from a peer) ends this participant: start a new run.
    pub fn round2(&mut self, round1_msgs: &[Vec<u8>]) -> Result<Vec<Round2Msg>, Error> {
        let others = self.parse_round1(round1_msgs)?;
        let State::AfterRound1(secret) = std::mem::replace(&mut self.state, State::Done) else {
            return Err(Error::Dkg("round 2 needs round 1 first".into()));
        };
        let (secret2, outgoing) = frost::keys::dkg::part2(secret, &others).map_err(blame)?;
        self.state = State::AfterRound2(secret2);
        outgoing
            .into_iter()
            .map(|(to, pkg)| {
                let to = id_to_u16(&to)?;
                let bytes = pkg.serialize().map_err(dkg_err)?;
                Ok(Round2Msg {
                    to,
                    bytes: prefixed(self.id, &bytes),
                })
            })
            .collect()
    }

    /// Round 3. Takes everyone's round 1 broadcasts and the round 2 messages addressed to this
    /// node. Fails, naming the culprit, if a received share does not match its sender's
    /// commitments. Returns this node's secret share and the public group key.
    pub fn round3(
        &mut self,
        round1_msgs: &[Vec<u8>],
        round2_msgs: &[Vec<u8>],
    ) -> Result<(NodeKey, GroupKey), Error> {
        let r1 = self.parse_round1(round1_msgs)?;
        let mut r2 = BTreeMap::new();
        for m in round2_msgs {
            let (sender, payload) = split_sender(m, self.id, self.n)?;
            let pkg = frost::keys::dkg::round2::Package::deserialize(payload).map_err(dkg_err)?;
            if r2.insert(fid(sender)?, pkg).is_some() {
                return Err(Error::Dkg(format!(
                    "two round 2 messages from node {sender}"
                )));
            }
        }
        if r2.len() != self.n as usize - 1 {
            return Err(Error::Dkg(format!(
                "need round 2 messages from all {} peers, got {}",
                self.n - 1,
                r2.len()
            )));
        }
        let State::AfterRound2(secret2) = std::mem::replace(&mut self.state, State::Done) else {
            return Err(Error::Dkg("round 3 needs round 2 first".into()));
        };
        let (kp, pkg) = frost::keys::dkg::part3(&secret2, &r1, &r2).map_err(blame)?;
        Ok((
            NodeKey::from_frost(&kp)?,
            GroupKey::from_frost(&pkg, self.t)?,
        ))
    }
}

/// Run the whole key generation for `n` nodes inside one process, routing only bytes between
/// the participants. For tests and the local demo. Node ids are `1..=n`.
pub fn run_local_dkg<R: RngCore + CryptoRng>(
    n: u16,
    t: u16,
    rng: &mut R,
) -> Result<(Vec<NodeKey>, GroupKey), Error> {
    let mut nodes: Vec<Participant> = (1..=n)
        .map(|i| Participant::new(i, n, t))
        .collect::<Result<_, _>>()?;
    let round1: Vec<Vec<u8>> = nodes
        .iter_mut()
        .map(|p| p.round1(&mut *rng))
        .collect::<Result<_, _>>()?;

    let mut inbox: BTreeMap<u16, Vec<Vec<u8>>> = BTreeMap::new();
    for p in nodes.iter_mut() {
        for msg in p.round2(&round1)? {
            inbox.entry(msg.to).or_default().push(msg.bytes);
        }
    }

    let mut keys = Vec::new();
    let mut group: Option<GroupKey> = None;
    for p in nodes.iter_mut() {
        let mine = inbox.remove(&p.id()).unwrap_or_default();
        let (key, g) = p.round3(&round1, &mine)?;
        match &group {
            None => group = Some(g),
            Some(first) if *first != g => {
                return Err(Error::Dkg("nodes disagree on the group key".into()))
            }
            Some(_) => {}
        }
        keys.push(key);
    }
    Ok((keys, group.expect("at least one node")))
}

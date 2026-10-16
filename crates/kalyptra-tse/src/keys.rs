//! Group and node keys.
//!
//! The group public key `PK = s * G` is the encryption key. The secret `s` is never held
//! by anyone: each node `i` holds only a Shamir share `s_i`, and any `t` of the `n`
//! shares can jointly act as `s`. Key generation uses the Zcash Foundation's FROST
//! distributed key generation (Pedersen DKG with proofs of knowledge), so no trusted
//! dealer ever sees `s`.
//!
//! Use a DKG run dedicated to encryption. Never reuse signing key shares for decryption.

use std::collections::BTreeMap;
use std::fmt;

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;
use frost_ristretto255 as frost;
use zeroize::Zeroize;

use crate::lagrange;
use crate::Error;

/// Public description of a node group: the threshold, the group key and every node's
/// public verifying share. Safe to publish; everyone who checks a proof needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupKey {
    threshold: u16,
    pk: RistrettoPoint,
    shares: BTreeMap<u16, RistrettoPoint>,
}

impl GroupKey {
    /// Build a group key and check it:
    ///
    /// * no point is the identity;
    /// * the verifying shares lie on one polynomial of degree **below** `t` through
    ///   `(0, pk)` (so any `t` of them open envelopes), and
    /// * that polynomial has degree **exactly** `t - 1` (so `t - 1` of them cannot). Without
    ///   this, a key generated at a lower threshold could be presented as a higher one.
    pub fn new(
        threshold: u16,
        pk: RistrettoPoint,
        shares: BTreeMap<u16, RistrettoPoint>,
    ) -> Result<Self, Error> {
        if threshold == 0 || shares.len() < threshold as usize || shares.contains_key(&0) {
            return Err(Error::Dkg("bad threshold or node ids".into()));
        }
        let identity = RistrettoPoint::identity();
        if pk == identity || shares.values().any(|y| *y == identity) {
            return Err(Error::InvalidPoint);
        }
        let g = GroupKey {
            threshold,
            pk,
            shares,
        };
        if !g.consistent_at(threshold) {
            return Err(Error::Dkg(
                "verifying shares do not interpolate to the group key".into(),
            ));
        }
        if threshold > 1 && g.consistent_at(threshold - 1) {
            return Err(Error::Dkg(format!(
                "the shares were generated with a threshold below {threshold}"
            )));
        }
        Ok(g)
    }

    pub fn threshold(&self) -> u16 {
        self.threshold
    }

    pub fn public_key(&self) -> &RistrettoPoint {
        &self.pk
    }

    pub fn node_ids(&self) -> Vec<u16> {
        self.shares.keys().copied().collect()
    }

    pub fn verifying_share(&self, id: u16) -> Option<&RistrettoPoint> {
        self.shares.get(&id)
    }

    /// True if every `t` verifying shares interpolate to `pk`.
    pub fn is_consistent(&self) -> bool {
        self.consistent_at(self.threshold)
    }

    /// True if the shares lie on one polynomial of degree below `t` through `(0, pk)`.
    ///
    /// Checking the `n` cyclic windows of `t` consecutive ids is enough: two adjacent windows
    /// share `t - 1` points and both pass through `(0, pk)`, which pins one polynomial of
    /// degree below `t`, so by induction every share lies on it.
    fn consistent_at(&self, t: u16) -> bool {
        let ids: Vec<u16> = self.shares.keys().copied().collect();
        let t = t as usize;
        if t == 0 || ids.len() < t {
            return false;
        }
        (0..ids.len()).all(|start| {
            let subset: Vec<u16> = (0..t).map(|k| ids[(start + k) % ids.len()]).collect();
            let lambdas = match lagrange::coefficients_at_zero(&subset) {
                Some(l) => l,
                None => return false,
            };
            let sum: RistrettoPoint = subset
                .iter()
                .zip(lambdas.iter())
                .map(|(id, l)| self.shares[id] * l)
                .sum();
            sum == self.pk
        })
    }

    /// Build from the public package every FROST DKG participant ends with.
    pub fn from_frost(pkg: &frost::keys::PublicKeyPackage, threshold: u16) -> Result<Self, Error> {
        let pk = point_from_bytes(&pkg.verifying_key().serialize().map_err(dkg_err)?)?;
        let mut shares = BTreeMap::new();
        for (id, vs) in pkg.verifying_shares() {
            shares.insert(
                id_to_u16(id)?,
                point_from_bytes(&vs.serialize().map_err(dkg_err)?)?,
            );
        }
        GroupKey::new(threshold, pk, shares)
    }
}

/// One node's secret share. Never printed, wiped from memory on drop.
pub struct NodeKey {
    id: u16,
    share: Scalar,
    verifying_share: RistrettoPoint,
}

impl NodeKey {
    pub fn new(id: u16, share: Scalar) -> Self {
        NodeKey {
            id,
            share,
            verifying_share: G * share,
        }
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn verifying_share(&self) -> &RistrettoPoint {
        &self.verifying_share
    }

    pub(crate) fn share(&self) -> &Scalar {
        &self.share
    }

    /// The secret share as bytes, for storing it in a node's key file. The returned buffer is
    /// wiped when dropped. Anyone holding `t` of these can open every envelope: store them
    /// like private keys.
    pub fn share_bytes(&self) -> zeroize::Zeroizing<[u8; 32]> {
        zeroize::Zeroizing::new(self.share.to_bytes())
    }

    /// Load a share saved with [`NodeKey::share_bytes`]. Rejects non-canonical and zero
    /// scalars.
    pub fn from_share_bytes(id: u16, bytes: &[u8; 32]) -> Result<Self, Error> {
        let share = Option::<Scalar>::from(Scalar::from_canonical_bytes(*bytes))
            .ok_or(Error::InvalidScalar)?;
        if share == Scalar::ZERO || id == 0 {
            return Err(Error::InvalidScalar);
        }
        Ok(NodeKey::new(id, share))
    }

    /// Build from the secret package a FROST DKG participant ends with.
    pub fn from_frost(kp: &frost::keys::KeyPackage) -> Result<Self, Error> {
        let id = id_to_u16(kp.identifier())?;
        let mut raw = kp.signing_share().serialize();
        let mut bytes: [u8; 32] = raw.as_slice().try_into().map_err(|_| Error::BadLength)?;
        raw.zeroize();
        let share = Option::<Scalar>::from(Scalar::from_canonical_bytes(bytes));
        bytes.zeroize();
        let share = share.ok_or(Error::InvalidScalar)?;
        if share == Scalar::ZERO {
            return Err(Error::InvalidScalar);
        }
        Ok(NodeKey::new(id, share))
    }
}

impl Drop for NodeKey {
    fn drop(&mut self) {
        self.share.zeroize();
    }
}

impl fmt::Debug for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeKey")
            .field("id", &self.id)
            .field("share", &"<redacted>")
            .finish()
    }
}

pub(crate) fn dkg_err<E: fmt::Display>(e: E) -> Error {
    Error::Dkg(e.to_string())
}

pub(crate) fn point_from_bytes(bytes: &[u8]) -> Result<RistrettoPoint, Error> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| Error::BadLength)?;
    CompressedRistretto(arr)
        .decompress()
        .ok_or(Error::InvalidPoint)
}

pub(crate) fn id_to_u16(id: &frost::Identifier) -> Result<u16, Error> {
    let b = id.serialize();
    if b.len() != 32 || b[2..].iter().any(|x| *x != 0) {
        return Err(Error::Dkg("identifier is not a small integer".into()));
    }
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

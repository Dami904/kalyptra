//! Sealing, partial decryption and opening.
//!
//! Sealing (anyone, using only the public group key `PK`):
//!   pick a random `r`, set `R = r*G` and `S = r*PK`; derive an AEAD key from `S`, `R`,
//!   `PK` and the caller's `aad`; encrypt. Then prove knowledge of `r` with a Schnorr proof
//!   whose challenge binds `PK`, `R`, the `aad` and the ciphertext. The envelope is `R`, that
//!   proof, and the ciphertext.
//!
//! Partial decryption (node `i`, holding share `s_i`):
//!   check the sealer's proof first, then `P_i = s_i * R`, with a proof that the same `s_i`
//!   sits behind the node's public verifying share.
//!
//! Opening (anyone holding `t` valid partials):
//!   `S = sum l_i * P_i = s * R = r * PK`, then derive the same key and decrypt.
//!   Fewer than `t` partials give a wrong `S`, so decryption fails.
//!
//! ## Why the sealer proves knowledge of `r`
//!
//! A partial decryption depends only on `R`. Without the proof, an attacker could copy the
//! `R` of someone else's envelope (for example an order in a round that was voided and must
//! never be opened) into a new envelope of their own, get the nodes to release partials for
//! it in a later round, and open the original. The proof is bound to the round's `aad` and to
//! the ciphertext, so only whoever chose `r` can produce a valid envelope around that `R`. This
//! is the construction of Shoup and Gennaro's TDH1 (Eurocrypt 1998), with a Schnorr proof in
//! place of their exact one.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;
use hkdf::Hkdf;
use rand_core::{CryptoRng, RngCore};
use sha2::{Digest, Sha256, Sha512};
use std::collections::BTreeSet;
use zeroize::Zeroize;

use crate::dleq::{self, DleqProof};
use crate::keys::{GroupKey, NodeKey};
use crate::lagrange;
use crate::Error;

const KDF_SALT: &[u8] = b"kalyptra-tse/v1/kdf";
const SEAL_DOMAIN: &[u8] = b"kalyptra-tse/v1/seal";
const TAG_LEN: usize = 16;
/// Ephemeral point, then the two scalars of the sealer's proof.
const HEADER_LEN: usize = 32 + 32 + 32;

/// The sealer's Schnorr proof that it knows `r` with `R = r*G`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SealProof {
    pub c: Scalar,
    pub z: Scalar,
}

/// A sealed message: the ephemeral point, the sealer's proof, and the AEAD ciphertext (with
/// its tag).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub ephemeral: CompressedRistretto,
    pub proof: SealProof,
    pub ciphertext: Vec<u8>,
}

fn canonical_scalar(bytes: &[u8]) -> Result<Scalar, Error> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| Error::BadLength)?;
    Option::<Scalar>::from(Scalar::from_canonical_bytes(arr)).ok_or(Error::InvalidScalar)
}

impl Envelope {
    /// `R (32) || c (32) || z (32) || ciphertext with tag (at least 16)`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.ciphertext.len());
        out.extend_from_slice(self.ephemeral.as_bytes());
        out.extend_from_slice(self.proof.c.as_bytes());
        out.extend_from_slice(self.proof.z.as_bytes());
        out.extend_from_slice(&self.ciphertext);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < HEADER_LEN + TAG_LEN {
            return Err(Error::BadLength);
        }
        let mut r = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        Ok(Envelope {
            ephemeral: CompressedRistretto(r),
            proof: SealProof {
                c: canonical_scalar(&bytes[32..64])?,
                z: canonical_scalar(&bytes[64..96])?,
            },
            ciphertext: bytes[HEADER_LEN..].to_vec(),
        })
    }

    /// The ephemeral point, rejecting the identity (an envelope sealed with `r = 0` would be
    /// readable by anyone).
    fn ephemeral_point(&self) -> Result<RistrettoPoint, Error> {
        let r = self.ephemeral.decompress().ok_or(Error::InvalidPoint)?;
        if r == RistrettoPoint::identity() {
            return Err(Error::InvalidPoint);
        }
        Ok(r)
    }
}

/// One node's contribution to opening one envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartialDecryption {
    pub node_id: u16,
    pub point: CompressedRistretto,
    pub proof: DleqProof,
}

impl PartialDecryption {
    /// 2 + 32 + 32 + 32 = 98 bytes: node id, partial point, proof challenge, proof response.
    pub const BYTES: usize = 98;

    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut out = [0u8; Self::BYTES];
        out[..2].copy_from_slice(&self.node_id.to_le_bytes());
        out[2..34].copy_from_slice(self.point.as_bytes());
        out[34..66].copy_from_slice(self.proof.c.as_bytes());
        out[66..98].copy_from_slice(self.proof.z.as_bytes());
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::BYTES {
            return Err(Error::BadLength);
        }
        let mut p = [0u8; 32];
        p.copy_from_slice(&bytes[2..34]);
        Ok(PartialDecryption {
            node_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            point: CompressedRistretto(p),
            proof: DleqProof {
                c: canonical_scalar(&bytes[34..66])?,
                z: canonical_scalar(&bytes[66..98])?,
            },
        })
    }
}

/// What `open` learned besides the plaintext.
#[derive(Debug, PartialEq, Eq)]
pub struct Opened {
    pub plaintext: Vec<u8>,
    /// Nodes whose valid partials were combined.
    pub used: Vec<u16>,
    /// Nodes whose partials were dropped: unknown id, duplicate, or a failed proof.
    pub rejected: Vec<u16>,
}

fn derive(
    shared: &RistrettoPoint,
    ephemeral: &RistrettoPoint,
    group_pk: &RistrettoPoint,
    aad: &[u8],
) -> ([u8; 32], [u8; 12]) {
    let mut ikm = Vec::with_capacity(96);
    ikm.extend_from_slice(shared.compress().as_bytes());
    ikm.extend_from_slice(ephemeral.compress().as_bytes());
    ikm.extend_from_slice(group_pk.compress().as_bytes());
    let hk = Hkdf::<Sha256>::new(Some(KDF_SALT), &ikm);
    let mut okm = [0u8; 44];
    hk.expand(aad, &mut okm)
        .expect("44 bytes is a valid HKDF-SHA256 output length");
    ikm.zeroize();
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 12];
    key.copy_from_slice(&okm[..32]);
    nonce.copy_from_slice(&okm[32..]);
    okm.zeroize();
    (key, nonce)
}

/// Challenge for the sealer's proof. Variable-length fields are length-prefixed.
fn seal_challenge(
    group_pk: &RistrettoPoint,
    ephemeral: &RistrettoPoint,
    commitment: &RistrettoPoint,
    aad: &[u8],
    ciphertext: &[u8],
) -> Scalar {
    let mut h = Sha512::new();
    h.update(SEAL_DOMAIN);
    h.update(group_pk.compress().as_bytes());
    h.update(ephemeral.compress().as_bytes());
    h.update(commitment.compress().as_bytes());
    h.update((aad.len() as u64).to_le_bytes());
    h.update(aad);
    h.update((ciphertext.len() as u64).to_le_bytes());
    h.update(ciphertext);
    Scalar::from_hash(h)
}

fn seal_inner(
    group_pk: &RistrettoPoint,
    aad: &[u8],
    plaintext: &[u8],
    r: &Scalar,
    w: &Scalar,
) -> Envelope {
    let ephemeral = G * r;
    let shared = group_pk * r;
    let (mut key, nonce) = derive(&shared, &ephemeral, group_pk, aad);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .expect("encryption of an in-memory buffer cannot fail");
    key.zeroize();
    let commitment = G * w;
    let c = seal_challenge(group_pk, &ephemeral, &commitment, aad, &ciphertext);
    let z = w + c * r;
    Envelope {
        ephemeral: ephemeral.compress(),
        proof: SealProof { c, z },
        ciphertext,
    }
}

/// Encrypt `plaintext` to the group. `aad` is authenticated but not secret: put the round id
/// and any context the plaintext must stay bound to in it. Nodes refuse to decrypt the
/// envelope under any other `aad`.
pub fn seal<R: RngCore + CryptoRng>(
    group_pk: &RistrettoPoint,
    aad: &[u8],
    plaintext: &[u8],
    rng: &mut R,
) -> Envelope {
    let mut r = Scalar::random(rng);
    let mut w = Scalar::random(rng);
    let env = seal_inner(group_pk, aad, plaintext, &r, &w);
    r.zeroize();
    w.zeroize();
    env
}

/// Check the sealer's proof: that whoever built this envelope knew `r` for its `R`, and built
/// it for this group, this `aad` and this exact ciphertext. Nodes call this before releasing
/// a partial decryption; [`partial_decrypt`] and [`open`] call it for you.
pub fn verify_envelope(
    group_pk: &RistrettoPoint,
    envelope: &Envelope,
    aad: &[u8],
) -> Result<(), Error> {
    let r = envelope.ephemeral_point()?;
    let commitment = G * envelope.proof.z - r * envelope.proof.c;
    if seal_challenge(group_pk, &r, &commitment, aad, &envelope.ciphertext) == envelope.proof.c {
        Ok(())
    } else {
        Err(Error::InvalidProof)
    }
}

/// Node `i`'s partial decryption of `envelope`, with a proof that it used its real share.
///
/// Refuses (with [`Error::InvalidProof`]) unless the envelope's sealer proof verifies for this
/// group and this `aad`. Deciding *when* to release partials (only after the round closes,
/// never for a voided round) is the caller's job.
pub fn partial_decrypt<R: RngCore + CryptoRng>(
    node: &NodeKey,
    group: &GroupKey,
    envelope: &Envelope,
    aad: &[u8],
    rng: &mut R,
) -> Result<PartialDecryption, Error> {
    verify_envelope(group.public_key(), envelope, aad)?;
    let r = envelope.ephemeral_point()?;
    let (p, proof) = dleq::prove(node.id(), node.share(), &r, rng);
    Ok(PartialDecryption {
        node_id: node.id(),
        point: p.compress(),
        proof,
    })
}

/// Open an envelope sealed with [`seal`] to a **single** recipient's public key `secret * G`
/// (no threshold). Used to send one node a private message, such as its key-generation share.
/// The sealer's proof must verify for this key and `aad`.
pub fn open_single(secret: &Scalar, envelope: &Envelope, aad: &[u8]) -> Result<Vec<u8>, Error> {
    let pk = G * secret;
    verify_envelope(&pk, envelope, aad)?;
    let r = envelope.ephemeral_point()?;
    let shared = r * secret;
    decrypt_with(&pk, envelope, aad, &shared)
}

/// Check one partial decryption against the group's public data.
pub fn verify_partial(
    group: &GroupKey,
    envelope: &Envelope,
    partial: &PartialDecryption,
) -> Result<(), Error> {
    let y = group
        .verifying_share(partial.node_id)
        .ok_or(Error::UnknownNode(partial.node_id))?;
    let r = envelope.ephemeral_point()?;
    let p = partial.point.decompress().ok_or(Error::InvalidPoint)?;
    if dleq::verify(partial.node_id, y, &r, &p, &partial.proof) {
        Ok(())
    } else {
        Err(Error::InvalidProof)
    }
}

/// Open an envelope from any `t` or more partials. The sealer's proof must verify for this
/// `aad`. Invalid, unknown and duplicate partials are dropped (and reported); a cheating node
/// cannot break opening as long as `t` honest partials remain.
pub fn open(
    group: &GroupKey,
    envelope: &Envelope,
    aad: &[u8],
    partials: &[PartialDecryption],
) -> Result<Opened, Error> {
    verify_envelope(group.public_key(), envelope, aad)?;
    let t = group.threshold() as usize;
    let mut seen = BTreeSet::new();
    let mut valid: Vec<(u16, RistrettoPoint)> = Vec::new();
    let mut rejected = Vec::new();
    for p in partials {
        if !seen.insert(p.node_id) {
            rejected.push(p.node_id);
            continue;
        }
        match (verify_partial(group, envelope, p), p.point.decompress()) {
            (Ok(()), Some(pt)) => valid.push((p.node_id, pt)),
            _ => rejected.push(p.node_id),
        }
    }
    if valid.len() < t {
        return Err(Error::NotEnoughPartials {
            have: valid.len(),
            need: t,
        });
    }
    valid.sort_by_key(|(id, _)| *id);
    valid.truncate(t);
    let ids: Vec<u16> = valid.iter().map(|(id, _)| *id).collect();
    let shared = interpolate(&ids, valid.iter().map(|(_, p)| *p))?;
    let plaintext = decrypt_with(group.public_key(), envelope, aad, &shared)?;
    Ok(Opened {
        plaintext,
        used: ids,
        rejected,
    })
}

fn interpolate(
    ids: &[u16],
    points: impl Iterator<Item = RistrettoPoint>,
) -> Result<RistrettoPoint, Error> {
    let l = lagrange::coefficients_at_zero(ids).ok_or(Error::BadNodeSet)?;
    Ok(points.zip(l.iter()).map(|(p, l)| p * l).sum())
}

fn decrypt_with(
    group_pk: &RistrettoPoint,
    envelope: &Envelope,
    aad: &[u8],
    shared: &RistrettoPoint,
) -> Result<Vec<u8>, Error> {
    let r = envelope.ephemeral_point()?;
    let (mut key, nonce) = derive(shared, &r, group_pk, aad);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let out = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &envelope.ciphertext,
                aad,
            },
        )
        .map_err(|_| Error::Decrypt);
    key.zeroize();
    out
}

// ---- test-only entry points: compiled only with the `test-vectors` feature ----------------

/// Seal with caller-chosen `r` and proof nonce `w`. Reusing either breaks the encryption or
/// leaks `r`. Test vectors only.
#[cfg(feature = "test-vectors")]
pub fn seal_with_nonces(
    group_pk: &RistrettoPoint,
    aad: &[u8],
    plaintext: &[u8],
    r: &Scalar,
    w: &Scalar,
) -> Envelope {
    seal_inner(group_pk, aad, plaintext, r, w)
}

/// The key and nonce derivation, exposed so vectors can show intermediate values.
#[cfg(feature = "test-vectors")]
pub fn derive_for_vectors(
    shared: &RistrettoPoint,
    ephemeral: &RistrettoPoint,
    group_pk: &RistrettoPoint,
    aad: &[u8],
) -> ([u8; 32], [u8; 12]) {
    derive(shared, ephemeral, group_pk, aad)
}

/// Attack simulation: interpolate and decrypt from partials WITHOUT any proof check and
/// WITHOUT requiring `t` of them. Shows that fewer than `t` shares do not yield the plaintext.
#[cfg(feature = "test-vectors")]
pub fn open_unchecked_for_tests(
    group: &GroupKey,
    envelope: &Envelope,
    aad: &[u8],
    partials: &[PartialDecryption],
) -> Result<Vec<u8>, Error> {
    let ids: Vec<u16> = partials.iter().map(|p| p.node_id).collect();
    let pts: Vec<RistrettoPoint> = partials
        .iter()
        .map(|p| p.point.decompress().ok_or(Error::InvalidPoint))
        .collect::<Result<_, _>>()?;
    let shared = interpolate(&ids, pts.into_iter())?;
    decrypt_with(group.public_key(), envelope, aad, &shared)
}

/// Build a partial decryption WITHOUT checking the sealer's proof, as a misbehaving node
/// would. Tests use it to show that `open` still refuses an envelope with a bad proof.
#[cfg(feature = "test-vectors")]
pub fn partial_decrypt_unchecked_for_tests<R: RngCore + CryptoRng>(
    node: &NodeKey,
    envelope: &Envelope,
    rng: &mut R,
) -> Result<PartialDecryption, Error> {
    let r = envelope.ephemeral_point()?;
    let (p, proof) = dleq::prove(node.id(), node.share(), &r, rng);
    Ok(PartialDecryption {
        node_id: node.id(),
        point: p.compress(),
        proof,
    })
}

//! Non-interactive Chaum-Pedersen proof of discrete-log equality.
//!
//! Proves, without revealing `s`, that the same secret `s` satisfies both
//! `Y = s * G` (the node's public verifying share) and `P = s * R` (its partial
//! decryption of an envelope with ephemeral point `R`). This is what lets anyone
//! catch a node that returns a wrong partial decryption.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand_core::{CryptoRng, RngCore};
use sha2::{Digest, Sha512};
use zeroize::Zeroize;

const DOMAIN: &[u8] = b"kalyptra-tse/v1/dleq";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DleqProof {
    pub c: Scalar,
    pub z: Scalar,
}

fn challenge(
    node_id: u16,
    r: &RistrettoPoint,
    y: &RistrettoPoint,
    p: &RistrettoPoint,
    a1: &RistrettoPoint,
    a2: &RistrettoPoint,
) -> Scalar {
    let mut h = Sha512::new();
    h.update(DOMAIN);
    h.update(node_id.to_le_bytes());
    for pt in [&G, r, y, p, a1, a2] {
        h.update(pt.compress().as_bytes());
    }
    Scalar::from_hash(h)
}

const NONCE_DOMAIN: &[u8] = b"kalyptra-tse/v1/dleq-nonce";

/// Prove that `p = s * r` and `y = s * G` use the same `s`. `node_id` is bound into
/// the challenge so a proof cannot be replayed under another node's identity.
///
/// The nonce is hedged: a hash of the secret, the statement and fresh randomness. A weak or
/// repeating random number generator alone therefore does not leak the share (a repeated
/// nonce across two different statements would).
pub fn prove<R: RngCore + CryptoRng>(
    node_id: u16,
    s: &Scalar,
    r: &RistrettoPoint,
    rng: &mut R,
) -> (RistrettoPoint, DleqProof) {
    let mut seed = [0u8; 32];
    rng.fill_bytes(&mut seed);
    let mut h = Sha512::new();
    h.update(NONCE_DOMAIN);
    h.update(s.as_bytes());
    h.update(node_id.to_le_bytes());
    h.update(r.compress().as_bytes());
    h.update(seed);
    seed.zeroize();
    let mut k = Scalar::from_hash(h);
    let out = prove_inner(node_id, s, r, &k);
    k.zeroize();
    out
}

fn prove_inner(
    node_id: u16,
    s: &Scalar,
    r: &RistrettoPoint,
    k: &Scalar,
) -> (RistrettoPoint, DleqProof) {
    let y = G * s;
    let p = r * s;
    let a1 = G * k;
    let a2 = r * k;
    let c = challenge(node_id, r, &y, &p, &a1, &a2);
    let z = k + c * s;
    (p, DleqProof { c, z })
}

/// Prove with a caller-chosen nonce `k`. Only compiled with the `test-vectors` feature: a
/// nonce that repeats or is predictable reveals the secret `s`.
#[cfg(feature = "test-vectors")]
pub fn prove_with_nonce(
    node_id: u16,
    s: &Scalar,
    r: &RistrettoPoint,
    k: &Scalar,
) -> (RistrettoPoint, DleqProof) {
    prove_inner(node_id, s, r, k)
}

/// Check a proof against the public verifying share `y`.
pub fn verify(
    node_id: u16,
    y: &RistrettoPoint,
    r: &RistrettoPoint,
    p: &RistrettoPoint,
    proof: &DleqProof,
) -> bool {
    let a1 = G * proof.z - y * proof.c;
    let a2 = r * proof.z - p * proof.c;
    challenge(node_id, r, y, p, &a1, &a2) == proof.c
}

//! Known-answer vectors, generated from fixed inputs, and checked independently by
//! `tools/check_vectors.py` (libsodium, a separate implementation of ristretto255 and
//! ChaCha20-Poly1305). Compiled only with the `test-vectors` feature.
//!
//! Format: one `name = hex` line per value. All scalars are 32-byte little-endian canonical
//! encodings; all points are 32-byte compressed ristretto255.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use sha2::{Digest, Sha512};

use crate::dleq::prove_with_nonce;
use crate::envelope::{derive_for_vectors, seal_with_nonces};
use crate::lagrange;
use crate::PartialDecryption;

pub const AAD: &[u8] = b"kalyptra/round-7";
pub const PLAINTEXT: &[u8] = b"BUY 10 ACME @ 5 (limit price 5 vUSD)";

fn scalar(label: &str) -> Scalar {
    let mut h = Sha512::new();
    h.update(b"kalyptra-tse/kat/v3/");
    h.update(label.as_bytes());
    Scalar::from_hash(h)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn pt(p: &RistrettoPoint) -> String {
    hex(p.compress().as_bytes())
}

/// The vector file, as text.
pub fn v3() -> String {
    let mut o = String::new();
    let mut line = |k: &str, v: String| {
        o.push_str(k);
        o.push_str(" = ");
        o.push_str(&v);
        o.push('\n');
    };

    // a 2-of-3 sharing of a fixed secret: f(x) = s + a1 * x, share_i = f(i)
    let s = scalar("secret");
    let a1 = scalar("poly-a1");
    let shares: Vec<Scalar> = (1..=3u64).map(|i| s + a1 * Scalar::from(i)).collect();
    let pk = G * s;

    let r = scalar("ephemeral");
    let w = scalar("seal-proof-nonce");
    let env = seal_with_nonces(&pk, AAD, PLAINTEXT, &r, &w);
    let ephemeral = G * r;
    let shared = pk * r;
    let (key, nonce) = derive_for_vectors(&shared, &ephemeral, &pk, AAD);

    line("version", "kalyptra-tse-kat-v3".into());
    line("threshold", "2".into());
    line("nodes", "3".into());
    line("secret_s", hex(s.as_bytes()));
    line("poly_a1", hex(a1.as_bytes()));
    for (i, sh) in shares.iter().enumerate() {
        line(&format!("share_{}", i + 1), hex(sh.as_bytes()));
    }
    line("group_pk", pt(&pk));
    for (i, sh) in shares.iter().enumerate() {
        line(&format!("verifying_share_{}", i + 1), pt(&(G * sh)));
    }
    line("aad", hex(AAD));
    line("plaintext", hex(PLAINTEXT));
    line("ephemeral_scalar", hex(r.as_bytes()));
    line("envelope_R", pt(&ephemeral));
    line("shared_point_S", pt(&shared));
    line("kdf_key", hex(&key));
    line("kdf_nonce", hex(&nonce));
    line("ciphertext", hex(&env.ciphertext));
    line("seal_proof_nonce", hex(w.as_bytes()));
    line("seal_proof_c", hex(env.proof.c.as_bytes()));
    line("seal_proof_z", hex(env.proof.z.as_bytes()));
    line("envelope_wire", hex(&env.to_bytes()));

    let mut partials = Vec::new();
    for (i, sh) in shares.iter().enumerate() {
        let id = i as u16 + 1;
        let k = scalar(&format!("dleq-nonce-{id}"));
        let (p, proof) = prove_with_nonce(id, sh, &ephemeral, &k);
        line(&format!("dleq_nonce_{id}"), hex(k.as_bytes()));
        line(&format!("partial_point_{id}"), pt(&p));
        line(&format!("proof_c_{id}"), hex(proof.c.as_bytes()));
        line(&format!("proof_z_{id}"), hex(proof.z.as_bytes()));
        let wire = PartialDecryption {
            node_id: id,
            point: p.compress(),
            proof,
        }
        .to_bytes();
        line(&format!("partial_wire_{id}"), hex(&wire));
        partials.push((id, p));
    }

    // any two partials interpolate to the shared point S
    for (a, b) in [(1usize, 2usize), (1, 3), (2, 3)] {
        let ids = [partials[a - 1].0, partials[b - 1].0];
        let l = lagrange::coefficients_at_zero(&ids).expect("distinct nonzero ids");
        let s_rebuilt = partials[a - 1].1 * l[0] + partials[b - 1].1 * l[1];
        assert_eq!(s_rebuilt, shared, "interpolation must rebuild S");
        line(&format!("interpolated_S_{a}{b}"), pt(&s_rebuilt));
    }

    // a partial with one hex digit changed in its point: must be rejected
    let mut bad = hex(&PartialDecryption {
        node_id: 1,
        point: partials[0].1.compress(),
        proof: prove_with_nonce(1, &shares[0], &ephemeral, &scalar("dleq-nonce-1")).1,
    }
    .to_bytes())
    .into_bytes();
    bad[40] = if bad[40] == b'0' { b'1' } else { b'0' };
    line("bad_partial_wire_1", String::from_utf8(bad).expect("ascii"));

    o
}

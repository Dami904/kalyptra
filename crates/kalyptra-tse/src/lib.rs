//! # kalyptra-tse: threshold sealed envelopes
//!
//! Encrypt to a group key that **no single node holds**. Any `t` of `n` nodes can open an
//! envelope, and fewer than `t` learn nothing about it. Each node's contribution carries a
//! proof, so a node that returns a wrong partial decryption is caught and excluded.
//!
//! ```text
//! setup     n nodes run a distributed key generation  ->  group key PK, one share each
//! seal      anyone encrypts to PK, proving it knows r  ->  envelope (R, proof, ciphertext)
//! partial   node i checks the sealer's proof, then     ->  P_i = s_i * R + a proof
//! open      any t valid partials combine               ->  plaintext
//! ```
//!
//! ## What this does and does not give you
//!
//! * Security needs **fewer than `t` colluding nodes**. At `t` colluding nodes, everything
//!   sealed to the group opens.
//! * Nothing here stops nodes from simply *choosing* to open early. Releasing partials only
//!   after a round closes, and never for a voided round, is the application's job; this
//!   library supplies the cryptography that makes early opening impossible for fewer than `t`
//!   nodes.
//! * Encryption is an AEAD (ChaCha20-Poly1305) under a key derived from a ristretto255
//!   Diffie-Hellman with the group key. The `aad` (put the round id in it) is authenticated:
//!   nodes refuse to decrypt an envelope under any other `aad`, and an envelope's ephemeral
//!   point cannot be lifted into a new envelope, because the sealer must prove knowledge of
//!   `r` bound to the `aad` and the ciphertext.
//! * It does **not** authenticate *who* sealed the message. Put the investor's signature
//!   inside the plaintext.
//! * The threshold-decryption protocol (partial Diffie-Hellman with Chaum-Pedersen proofs,
//!   plus a sealer proof of knowledge in the style of Shoup and Gennaro's TDH1) is our own
//!   composition of standard parts. It has not been independently audited. Do not use it to
//!   protect real assets.
//!
//! ## Feature `test-vectors`
//!
//! Enables entry points with caller-chosen randomness and attack simulations, used by the
//! tests, the known-answer vectors and `examples/gen_vectors.rs`. Never enable it in a build
//! that handles real data.

pub mod dkg;
pub mod dleq;
mod envelope;
mod error;
pub mod keys;
mod lagrange;
#[cfg(feature = "test-vectors")]
pub mod test_vectors;

pub use dkg::{run_local_dkg, Participant, Round2Msg};
#[cfg(feature = "test-vectors")]
pub use envelope::{
    derive_for_vectors, open_unchecked_for_tests, partial_decrypt_unchecked_for_tests,
    seal_with_nonces,
};
pub use envelope::{
    open, open_single, partial_decrypt, seal, verify_envelope, verify_partial, Envelope, Opened,
    PartialDecryption, SealProof,
};
pub use error::Error;
pub use keys::{GroupKey, NodeKey};

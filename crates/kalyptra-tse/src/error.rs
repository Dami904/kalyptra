use thiserror::Error;

/// Everything that can go wrong in kalyptra-tse. Variants carry no secret material.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("not enough valid partial decryptions: have {have}, need {need}")]
    NotEnoughPartials { have: usize, need: usize },

    #[error("bytes are not a valid ristretto point")]
    InvalidPoint,

    #[error("bytes are not a canonical scalar")]
    InvalidScalar,

    #[error("input has the wrong length")]
    BadLength,

    #[error("decryption failed: wrong group key, wrong aad, or a tampered ciphertext")]
    Decrypt,

    /// A proof did not verify: the sealer's proof of knowledge on an envelope, or a node's
    /// proof on a partial decryption.
    #[error("proof did not verify")]
    InvalidProof,

    /// The node ids handed to interpolation were zero or repeated.
    #[error("node ids must be distinct and nonzero")]
    BadNodeSet,

    #[error("node id {0} is not part of this group")]
    UnknownNode(u16),

    #[error("distributed key generation failed: {0}")]
    Dkg(String),

    /// Key generation failed and the cause is traced to one participant (a bad proof of
    /// knowledge in round 1, or a share that does not match its commitments in round 2).
    #[error("distributed key generation failed; node {node} is at fault: {reason}")]
    DkgBlame { node: u16, reason: String },
}

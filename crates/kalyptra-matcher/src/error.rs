use thiserror::Error;

/// Everything that can go wrong in a matcher node. Variants carry no secret material.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MatcherError {
    #[error("unknown round {0}")]
    UnknownRound(u64),

    #[error("round {0} already exists")]
    RoundExists(u64),

    #[error("round {round} is {phase}, which does not allow this")]
    WrongPhase { round: u64, phase: &'static str },

    #[error("envelope rejected: {0}")]
    BadEnvelope(kalyptra_tse::Error),

    #[error("signature from node {0} does not verify")]
    BadSignature(u16),

    #[error("node {0} is not in the roster")]
    UnknownNode(u16),

    #[error("bad order: {0}")]
    BadOrder(&'static str),

    #[error("round {0} has no final commit yet")]
    NotCommitted(u64),

    #[error("round voided: {have} orders committed, at least {need} needed")]
    Voided { have: usize, need: usize },

    #[error("missing envelope {0} needed to release partials")]
    MissingEnvelope(String),

    #[error("envelope {envelope} cannot be opened yet: {source}")]
    CannotOpen {
        envelope: String,
        source: kalyptra_tse::Error,
    },

    #[error("bad configuration: {0}")]
    Config(&'static str),

    /// A node signed two different commit votes or record votes for one round.
    #[error("node {0} signed two different statements for one round")]
    Equivocation(u16),

    #[error("partial decryption from node {0} does not verify")]
    BadPartial(u16),

    #[error("bad commit vote: {0}")]
    BadCommit(&'static str),

    #[error("settlement: {0}")]
    Settlement(&'static str),
}

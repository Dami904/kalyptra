//! Print the known-answer vector file to stdout.
//!
//!     cargo run -p kalyptra-tse --example gen_vectors > crates/kalyptra-tse/tests/vectors/v3.txt
//!
//! Regenerate only on a deliberate format change. `tests/vectors.rs` fails if the output
//! drifts, and `tools/check_vectors.py` re-derives every value with a separate implementation.

fn main() {
    print!("{}", kalyptra_tse::test_vectors::v3());
}

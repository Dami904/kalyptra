//! Regression guard on the known-answer vectors: the committed file must equal what the
//! library produces today. An accidental change to the encoding, the key derivation, the
//! proof transcript or the wire format shows up here. The same file is checked against a
//! separate implementation by `tools/check_vectors.py`.

#[test]
fn committed_known_answer_vectors_match_what_the_library_produces() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/vectors/v3.txt");
    let committed = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
        .replace("\r\n", "\n");
    let fresh = kalyptra_tse::test_vectors::v3();
    assert_eq!(
        committed, fresh,
        "the vector file differs from the library output; if the change is deliberate, \
         regenerate it with the gen_vectors example"
    );
}

#[test]
fn the_vectors_are_deterministic() {
    assert_eq!(
        kalyptra_tse::test_vectors::v3(),
        kalyptra_tse::test_vectors::v3()
    );
}

#!/usr/bin/env python3
"""Independent check of kalyptra-tse's known-answer vectors.

Re-derives every value in crates/kalyptra-tse/tests/vectors/v3.txt using libsodium (loaded
with ctypes) for ristretto255 and ChaCha20-Poly1305, and Python's standard library for
SHA-256, SHA-512, HMAC and HKDF. It shares no code with the Rust crate, which uses
curve25519-dalek and the RustCrypto AEAD, so a mistake in the Rust protocol or encoding
is unlikely to be repeated here.

It follows the written description in crates/kalyptra-tse/src/envelope.rs and dleq.rs, not
the Rust code. Run:  python3 tools/check_vectors.py [path/to/v3.txt]
Needs libsodium (apt: libsodium23). Exit code 0 only if every check passes.
"""
import ctypes
import ctypes.util
import hashlib
import hmac
import sys
from pathlib import Path

LIB = ctypes.util.find_library("sodium")
if not LIB:
    sys.exit("libsodium not found (install libsodium23)")
S = ctypes.CDLL(LIB)
if S.sodium_init() < 0:
    sys.exit("sodium_init failed")

ELL = 2**252 + 27742317777372353535851937790883648493  # order of the ristretto255 group


# ---- thin wrappers over libsodium ---------------------------------------------------------
def _buf(n=32):
    return ctypes.create_string_buffer(n)


def base(n):
    out = _buf()
    assert S.crypto_scalarmult_ristretto255_base(out, n) == 0, "scalarmult_base failed"
    return out.raw


def mul(n, p):
    out = _buf()
    assert S.crypto_scalarmult_ristretto255(out, n, p) == 0, "scalarmult failed"
    return out.raw


def padd(p, q):
    out = _buf()
    assert S.crypto_core_ristretto255_add(out, p, q) == 0
    return out.raw


def psub(p, q):
    out = _buf()
    assert S.crypto_core_ristretto255_sub(out, p, q) == 0
    return out.raw


def valid_point(p):
    return S.crypto_core_ristretto255_is_valid_point(p) == 1


def sadd(x, y):
    out = _buf()
    S.crypto_core_ristretto255_scalar_add(out, x, y)
    return out.raw


def smul(x, y):
    out = _buf()
    S.crypto_core_ristretto255_scalar_mul(out, x, y)
    return out.raw


def reduce64(b):
    out = _buf()
    S.crypto_core_ristretto255_scalar_reduce(out, b)
    return out.raw


def sc(i):
    """Small integer (possibly negative) as a 32-byte little-endian scalar mod ell."""
    return (i % ELL).to_bytes(32, "little")


def hkdf_sha256(salt, ikm, info, length):
    prk = hmac.new(salt, ikm, hashlib.sha256).digest()
    okm, t, n = b"", b"", 1
    while len(okm) < length:
        t = hmac.new(prk, t + info + bytes([n]), hashlib.sha256).digest()
        okm += t
        n += 1
    return okm[:length]


def aead_encrypt(key, nonce, aad, msg):
    c = _buf(len(msg) + 16)
    clen = ctypes.c_ulonglong(0)
    r = S.crypto_aead_chacha20poly1305_ietf_encrypt(
        c, ctypes.byref(clen), msg, ctypes.c_ulonglong(len(msg)),
        aad, ctypes.c_ulonglong(len(aad)), None, nonce, key)
    assert r == 0
    return c.raw[: clen.value]


def aead_decrypt(key, nonce, aad, ct):
    m = _buf(max(len(ct) - 16, 1))
    mlen = ctypes.c_ulonglong(0)
    r = S.crypto_aead_chacha20poly1305_ietf_decrypt(
        m, ctypes.byref(mlen), None, ct, ctypes.c_ulonglong(len(ct)),
        aad, ctypes.c_ulonglong(len(aad)), nonce, key)
    return m.raw[: mlen.value] if r == 0 else None


# ---- the protocol, written from the description -------------------------------------------
G = base(sc(1))
DLEQ_DOMAIN = b"kalyptra-tse/v1/dleq"
KDF_SALT = b"kalyptra-tse/v1/kdf"
SEAL_DOMAIN = b"kalyptra-tse/v1/seal"


def challenge(node_id, r, y, p, a1, a2):
    h = hashlib.sha512()
    h.update(DLEQ_DOMAIN)
    h.update(node_id.to_bytes(2, "little"))
    for pt in (G, r, y, p, a1, a2):
        h.update(pt)
    return reduce64(h.digest())


def seal_challenge(pk, r_pt, w_pt, aad, ct):
    h = hashlib.sha512()
    h.update(SEAL_DOMAIN)
    h.update(pk)
    h.update(r_pt)
    h.update(w_pt)
    h.update(len(aad).to_bytes(8, "little"))
    h.update(aad)
    h.update(len(ct).to_bytes(8, "little"))
    h.update(ct)
    return reduce64(h.digest())


def seal_verify(pk, r_pt, c, z, aad, ct):
    w_pt = psub(mul(z, G), mul(c, r_pt))
    return seal_challenge(pk, r_pt, w_pt, aad, ct) == c


def dleq_verify(node_id, y, r, p, c, z):
    a1 = psub(mul(z, G), mul(c, y))
    a2 = psub(mul(z, r), mul(c, p))
    return challenge(node_id, r, y, p, a1, a2) == c


def lagrange_at_zero(ids):
    out = []
    for i in ids:
        num, den = 1, 1
        for j in ids:
            if j != i:
                num = num * j % ELL
                den = den * (j - i) % ELL
        out.append(num * pow(den, -1, ELL) % ELL)
    return out


def combine(points_by_id):
    ids = sorted(points_by_id)
    total = None
    for i, lam in zip(ids, lagrange_at_zero(ids)):
        term = mul(sc(lam), points_by_id[i])
        total = term if total is None else padd(total, term)
    return total


def parse_partial(wire):
    assert len(wire) == 98
    return int.from_bytes(wire[:2], "little"), wire[2:34], wire[34:66], wire[66:98]


def canonical_scalar(b):
    return int.from_bytes(b, "little") < ELL


# ---- run ----------------------------------------------------------------------------------
def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else (
        Path(__file__).resolve().parent.parent / "crates/kalyptra-tse/tests/vectors/v3.txt")
    v = {}
    for raw in path.read_text().replace("\r\n", "\n").splitlines():
        if raw.strip() and not raw.startswith("#"):
            k, _, val = raw.partition(" = ")
            v[k] = val
    h = lambda k: bytes.fromhex(v[k])

    failures = []
    count = [0]

    def check(name, ok):
        count[0] += 1
        print(("PASS  " if ok else "FAIL  ") + name)
        if not ok:
            failures.append(name)

    assert v["version"] == "kalyptra-tse-kat-v3"
    s, a1 = h("secret_s"), h("poly_a1")
    check("scalars are canonical", canonical_scalar(s) and canonical_scalar(a1))

    shares = {i: sadd(s, smul(a1, sc(i))) for i in (1, 2, 3)}
    for i in (1, 2, 3):
        check(f"share_{i} = s + a1*{i}", shares[i] == h(f"share_{i}"))
    pk = base(s)
    check("group_pk = s*G", pk == h("group_pk"))
    ys = {i: base(shares[i]) for i in (1, 2, 3)}
    for i in (1, 2, 3):
        check(f"verifying_share_{i} = share_{i}*G", ys[i] == h(f"verifying_share_{i}"))
    check("every 2-subset of verifying shares interpolates to group_pk",
          all(combine({a: ys[a], b: ys[b]}) == pk for a, b in ((1, 2), (1, 3), (2, 3))))

    aad, pt_msg = h("aad"), h("plaintext")
    r = h("ephemeral_scalar")
    R = base(r)
    check("envelope_R = r*G", R == h("envelope_R"))
    shared = mul(r, pk)
    check("shared_point_S = r*PK", shared == h("shared_point_S"))
    okm = hkdf_sha256(KDF_SALT, shared + R + pk, aad, 44)
    key, nonce = okm[:32], okm[32:]
    check("HKDF key", key == h("kdf_key"))
    check("HKDF nonce", nonce == h("kdf_nonce"))
    ct = aead_encrypt(key, nonce, aad, pt_msg)
    check("ChaCha20-Poly1305 ciphertext", ct == h("ciphertext"))
    w = h("seal_proof_nonce")
    sc_c = seal_challenge(pk, R, base(w), aad, ct)
    sc_z = sadd(w, smul(sc_c, r))
    check("seal proof c (sealer knows r, bound to pk, aad, ciphertext)", sc_c == h("seal_proof_c"))
    check("seal proof z", sc_z == h("seal_proof_z"))
    check("seal proof verifies (independent verifier)", seal_verify(pk, R, sc_c, sc_z, aad, ct))
    check("seal proof fails under another round id", not seal_verify(pk, R, sc_c, sc_z, aad + b"x", ct))
    check("seal proof fails for a shifted R (R + G)", not seal_verify(pk, padd(R, G), sc_c, sc_z, aad, ct))
    check("envelope wire = R || c || z || ciphertext", R + sc_c + sc_z + ct == h("envelope_wire"))

    partial_pts = {}
    for i in (1, 2, 3):
        k = h(f"dleq_nonce_{i}")
        p = mul(shares[i], R)
        check(f"partial_point_{i} = share_{i}*R", p == h(f"partial_point_{i}"))
        a1p, a2p = base(k), mul(k, R)
        c = challenge(i, R, ys[i], p, a1p, a2p)
        z = sadd(k, smul(c, shares[i]))
        check(f"proof_c_{i}", c == h(f"proof_c_{i}"))
        check(f"proof_z_{i}", z == h(f"proof_z_{i}"))
        check(f"partial_wire_{i} = id || P || c || z",
              i.to_bytes(2, "little") + p + c + z == h(f"partial_wire_{i}"))
        check(f"proof {i} verifies (independent verifier)", dleq_verify(i, ys[i], R, p, c, z))
        check(f"proof {i} fails under a different node id", not dleq_verify(i % 3 + 1, ys[i], R, p, c, z))
        partial_pts[i] = p

    for name, ids in (("12", (1, 2)), ("13", (1, 3)), ("23", (2, 3))):
        got = combine({i: partial_pts[i] for i in ids})
        check(f"interpolated_S_{name}", got == h(f"interpolated_S_{name}") and got == shared)
        okm2 = hkdf_sha256(KDF_SALT, got + R + pk, aad, 44)
        check(f"opening with partials {name} recovers the plaintext",
              aead_decrypt(okm2[:32], okm2[32:], aad, ct) == pt_msg)

    check("no single partial equals S (one share is not enough)",
          all(partial_pts[i] != shared for i in (1, 2, 3)))
    check("wrong aad does not decrypt", aead_decrypt(key, nonce, aad + b"x", ct) is None)

    bad = h("bad_partial_wire_1")
    nid, p, c, z = parse_partial(bad)
    accepted = (nid in ys and canonical_scalar(c) and canonical_scalar(z) and valid_point(p)
                and dleq_verify(nid, ys[nid], R, p, c, z))
    check("bad_partial_wire_1 is rejected (parse or proof)", not accepted)

    print(f"\n{count[0] - len(failures)}/{count[0]} checks passed")
    if failures:
        print("FAILED:", ", ".join(failures))
        sys.exit(1)


if __name__ == "__main__":
    main()

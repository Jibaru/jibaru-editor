#!/usr/bin/env python3
"""Verifies a Tauri updater signature (.sig) against the public key.

    scripts/release/verify-sig.py <file> <file.sig> [--pubkey <base64>] [--version <v>]

The public key defaults to plugins.updater.pubkey in app/src-tauri/tauri.conf.json.
Tauri signatures are minisign signatures, base64-encoded once more: the
signature over the file (Ed25519, of its BLAKE2b-512 hash for the prehashed
"ED" variant) and a global signature over that signature plus the trusted
comment, which carries `version:<v>` since Tauri CLI 2.10. With --version
the signed version must match. Pure Python (no dependencies) so CI can run it
on any runner; exits 1 when the signature is invalid.
"""

import argparse
import base64
import hashlib
import json
import pathlib
import sys

# --- Ed25519 verification (RFC 8032, section 6) ------------------------------
P = 2**255 - 19
Q = 2**252 + 27742317777372353535851937790883648493
D = -121665 * pow(121666, P - 2, P) % P
SQRT_M1 = pow(2, (P - 1) // 4, P)


def _add(a, b):
    A, B = (a[1] - a[0]) * (b[1] - b[0]) % P, (a[1] + a[0]) * (b[1] + b[0]) % P
    C, E = 2 * a[3] * b[3] * D % P, 2 * a[2] * b[2] % P
    e, f, g, h = B - A, E - C, E + C, B + A
    return (e * f, g * h, f * g, e * h)


def _mul(s, pt):
    acc = (0, 1, 1, 0)
    while s > 0:
        if s & 1:
            acc = _add(acc, pt)
        pt = _add(pt, pt)
        s >>= 1
    return acc


def _equal(a, b):
    return (a[0] * b[2] - b[0] * a[2]) % P == 0 and (a[1] * b[2] - b[1] * a[2]) % P == 0


def _recover_x(y, sign):
    if y >= P:
        return None
    x2 = (y * y - 1) * pow(D * y * y + 1, P - 2, P)
    if x2 == 0:
        return None if sign else 0
    x = pow(x2, (P + 3) // 8, P)
    if (x * x - x2) % P != 0:
        x = x * SQRT_M1 % P
    if (x * x - x2) % P != 0:
        return None
    if (x & 1) != sign:
        x = P - x
    return x


_GY = 4 * pow(5, P - 2, P) % P
_GX = _recover_x(_GY, 0)
G = (_GX, _GY, 1, _GX * _GY % P)


def _decompress(s):
    y = int.from_bytes(s, "little")
    sign = y >> 255
    y &= (1 << 255) - 1
    x = _recover_x(y, sign)
    return None if x is None else (x, y, 1, x * y % P)


def ed25519_verify(public: bytes, msg: bytes, sig: bytes) -> bool:
    if len(public) != 32 or len(sig) != 64:
        return False
    a, r = _decompress(public), _decompress(sig[:32])
    if a is None or r is None:
        return False
    s = int.from_bytes(sig[32:], "little")
    if s >= Q:
        return False
    h = int.from_bytes(hashlib.sha512(sig[:32] + public + msg).digest(), "little") % Q
    return _equal(_mul(s, G), _add(r, _mul(h, a)))


# --- minisign / Tauri formats --------------------------------------------------
def _lines(b64: str) -> list[str]:
    return base64.b64decode(b64.strip()).decode().strip().splitlines()


def verify(data: bytes, sig_b64: str, pubkey_b64: str, version: str | None) -> str:
    """Returns the trusted comment; raises ValueError when invalid."""
    key = base64.b64decode(_lines(pubkey_b64)[1])
    if len(key) != 42 or key[:2] != b"Ed":
        raise ValueError("not a minisign public key")
    key_id, public = key[2:10], key[10:]
    lines = _lines(sig_b64)
    if len(lines) < 4 or not lines[2].startswith("trusted comment: "):
        raise ValueError("not a minisign signature")
    sig = base64.b64decode(lines[1])
    alg, sig_id, sig = sig[:2], sig[2:10], sig[10:]
    if sig_id != key_id:
        raise ValueError(f"signed with key {sig_id[::-1].hex().upper()}, expected {key_id[::-1].hex().upper()}")
    msg = hashlib.blake2b(data, digest_size=64).digest() if alg == b"ED" else data
    if alg not in (b"ED", b"Ed") or not ed25519_verify(public, msg, sig):
        raise ValueError("signature does not match the file")
    trusted = lines[2][len("trusted comment: "):]
    if not ed25519_verify(public, sig + trusted.encode(), base64.b64decode(lines[3])):
        raise ValueError("trusted comment signature is invalid")
    if version is not None:
        fields = dict(f.split(":", 1) for f in trusted.split("\t") if ":" in f)
        if fields.get("version", "").lstrip("v") != version.lstrip("v"):
            raise ValueError(f"signed for version {fields.get('version')!r}, expected {version!r}")
    return trusted


def default_pubkey() -> str:
    conf = pathlib.Path(__file__).resolve().parents[2] / "app/src-tauri/tauri.conf.json"
    return json.loads(conf.read_text())["plugins"]["updater"]["pubkey"]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("file")
    ap.add_argument("sig", help="the .sig file, or '-' to read the signature from stdin")
    ap.add_argument("--pubkey")
    ap.add_argument("--version")
    a = ap.parse_args()
    sig = sys.stdin.read() if a.sig == "-" else pathlib.Path(a.sig).read_text()
    try:
        trusted = verify(pathlib.Path(a.file).read_bytes(), sig, a.pubkey or default_pubkey(), a.version)
    except ValueError as e:
        print(f"error: {a.file}: {e}", file=sys.stderr)
        return 1
    print(f"ok: {a.file} ({trusted})")
    return 0


if __name__ == "__main__":
    sys.exit(main())

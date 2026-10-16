#!/usr/bin/env python3
"""Writes the updater manifest (latest.json) for the packages in a directory.

    scripts/release/latest-json.py <dist> --version <v> --base-url <url> [--notes-file <f>]

Looks for the updater packages that scripts/package/{macos.sh,windows.ps1}
produce, each with its .sig next to it:

    Jibaru-Editor-<v>-macos-arm64.app.tar.gz   -> darwin-aarch64
    Jibaru-Editor-<v>-macos-x86_64.app.tar.gz  -> darwin-x86_64
    Jibaru-Editor-<v>-windows-x64-setup.exe    -> windows-x86_64

verifies every signature against the public key in tauri.conf.json (and the
signed version) and writes <dist>/latest.json with `url` = <base-url>/<file>.
Fails when a signature is missing or invalid, or no package is found.
"""

import argparse
import datetime
import importlib.util
import json
import pathlib
import sys

PLATFORMS = {
    "macos-arm64.app.tar.gz": "darwin-aarch64",
    "macos-x86_64.app.tar.gz": "darwin-x86_64",
    "windows-x64-setup.exe": "windows-x86_64",
}


def load_verifier():
    path = pathlib.Path(__file__).with_name("verify-sig.py")
    spec = importlib.util.spec_from_file_location("verify_sig", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("dist")
    ap.add_argument("--version", required=True)
    ap.add_argument("--base-url", required=True)
    ap.add_argument("--notes-file")
    ap.add_argument("--pubkey")
    a = ap.parse_args()
    v = load_verifier()
    pubkey = a.pubkey or v.default_pubkey()
    version = a.version.lstrip("v")
    dist = pathlib.Path(a.dist)
    platforms = {}
    for suffix, target in PLATFORMS.items():
        f = dist / f"Jibaru-Editor-{version}-{suffix}"
        if not f.exists():
            continue
        sig_file = f.with_name(f.name + ".sig")
        if not sig_file.exists():
            print(f"error: {sig_file.name} is missing (is TAURI_SIGNING_PRIVATE_KEY set?)", file=sys.stderr)
            return 1
        sig = sig_file.read_text().strip()
        try:
            trusted = v.verify(f.read_bytes(), sig, pubkey, version)
        except ValueError as e:
            print(f"error: {f.name}: {e}", file=sys.stderr)
            return 1
        print(f"{target}: {f.name} ({trusted})")
        platforms[target] = {"signature": sig, "url": f"{a.base_url.rstrip('/')}/{f.name}"}
    if not platforms:
        print(f"error: no updater packages for {version} in {dist}", file=sys.stderr)
        return 1
    notes = pathlib.Path(a.notes_file).read_text().strip() if a.notes_file else ""
    manifest = {
        "version": version,
        "notes": notes,
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    out = dist / "latest.json"
    out.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

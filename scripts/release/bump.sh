#!/usr/bin/env bash
# Sets the app version everywhere it is declared:
#   Cargo.toml                    [workspace.package] version (every crate and the app)
#   app/src-tauri/tauri.conf.json "version" (what the updater compares)
#   app/package.json              "version"
# and refreshes Cargo.lock. Then commit and tag:
#
#   scripts/release/bump.sh 0.2.0
#   git commit -am "Release 0.2.0" && git tag v0.2.0 && git push origin main v0.2.0
set -euo pipefail

version="${1:?usage: bump.sh <version>   (e.g. 0.2.0)}"
version="${version#v}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "error: '$version' is not a semantic version (x.y.z)" >&2
  exit 1
fi
root="$(cd "$(dirname "$0")/../.." && pwd)"

python3 - "$root" "$version" <<'EOF'
import json, pathlib, re, sys
root, version = pathlib.Path(sys.argv[1]), sys.argv[2]

cargo = root / "Cargo.toml"
text = cargo.read_text()
section = re.search(r"(?ms)^\[workspace\.package\]\n(.*?)(?=^\[|\Z)", text)
body, n = re.subn(r'(?m)^version = "[^"]*"', f'version = "{version}"', section.group(1), count=1)
if n != 1:
    sys.exit("error: no version in [workspace.package]")
cargo.write_text(text[: section.start(1)] + body + text[section.end(1) :])

# Rewrite only the "version" value so the files keep their formatting.
for rel in ("app/src-tauri/tauri.conf.json", "app/package.json"):
    f = root / rel
    text = f.read_text()
    new, n = re.subn(r'(?m)^(  "version": )"[^"]*"', rf'\g<1>"{version}"', text, count=1)
    if n != 1:
        sys.exit(f"error: no top-level version in {rel}")
    json.loads(new)
    f.write_text(new)
EOF

# Cargo.lock records the workspace crates' versions.
(cd "$root" && cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null)

grep -n '^version' "$root/Cargo.toml" | head -1
grep -n '"version"' "$root/app/src-tauri/tauri.conf.json" "$root/app/package.json"
echo "bumped to $version"

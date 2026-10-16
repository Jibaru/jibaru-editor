#!/usr/bin/env bash
# Packages the macOS app and CLI with the FFmpeg dylibs bundled.
#
#   scripts/package/macos.sh <ffmpeg-prefix> <out-dir>
#
# Expects, relative to the repository root:
#   target/release/jb, target/release/jb-mcp       (cargo build --release -p jb-cli -p jb-mcp)
#   target/release/bundle/macos/Jibaru Editor.app  (pnpm tauri build --bundles app)
#
# The app gets the dylibs in Contents/Frameworks and an LC_RPATH of
# @executable_path/../Frameworks; the CLI package is bin/ + lib/ with
# @executable_path/../lib. Every non-system library reference is rewritten to
# @rpath/<name>, the result is checked with otool and then signed: with
# $APPLE_SIGNING_IDENTITY (hardened runtime) when set, ad-hoc otherwise. When
# $APPLE_ID, $APPLE_PASSWORD and $APPLE_TEAM_ID are set the DMG is notarized
# and stapled.
#
# Outputs in <out-dir>:
#   Jibaru-Editor-<version>-macos-<arch>.dmg
#   Jibaru-Editor-<version>-macos-<arch>.app.zip
#   jb-<version>-macos-<arch>.zip   (jb, jb-mcp, ffprobe, FFmpeg dylibs)
set -euo pipefail

ffmpeg="${1:?usage: macos.sh <ffmpeg-prefix> <out-dir>}"
out="${2:?usage: macos.sh <ffmpeg-prefix> <out-dir>}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
ffmpeg="$(cd "$ffmpeg" && pwd)"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
arch="$(uname -m)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
identity="${APPLE_SIGNING_IDENTITY:--}"
release="${CARGO_TARGET_DIR:-$root/target}/release"
app_src="$release/bundle/macos/Jibaru Editor.app"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

is_system() { [[ "$1" == /usr/lib/* || "$1" == /System/* ]]; }

# Non-system dependencies (install names) of a Mach-O file.
deps() { otool -L "$1" | tail -n +2 | awk '{print $1}' | while read -r d; do is_system "$d" || echo "$d"; done; }

# Resolves an install name to a file in the FFmpeg prefix.
resolve() {
  local name; name="$(basename "$1")"
  if [ -e "$ffmpeg/lib/$name" ]; then echo "$ffmpeg/lib/$name"; else echo "$1"; fi
}

# Copies the transitive closure of FFmpeg dylibs needed by the given binaries.
copy_libs() {
  local dest="$1"; shift
  mkdir -p "$dest"
  local queue=("$@") seen=" "
  while [ ${#queue[@]} -gt 0 ]; do
    local f="${queue[0]}"; queue=("${queue[@]:1}")
    for d in $(deps "$f"); do
      local name; name="$(basename "$d")"
      [[ "$seen" == *" $name "* ]] && continue
      [[ "$d" == @executable_path/* || "$d" == @loader_path/* ]] && continue
      [ "$(basename "$f")" == "$name" ] && continue
      seen+="$name "
      local src; src="$(resolve "$d")"
      [ -e "$src" ] || { echo "error: cannot find $d needed by $f" >&2; exit 1; }
      cp -L "$src" "$dest/$name"
      chmod u+w "$dest/$name"
      queue+=("$dest/$name")
    done
  done
}

# Rewrites references to @rpath/<name> and sets the rpath of one Mach-O.
fix_refs() {
  local f="$1" rpath="$2"
  for d in $(deps "$f"); do
    [[ "$d" == @rpath/* || "$d" == @executable_path/* || "$d" == @loader_path/* ]] && continue
    install_name_tool -change "$d" "@rpath/$(basename "$d")" "$f"
  done
  if [[ "$f" == *.dylib ]]; then
    install_name_tool -id "@rpath/$(basename "$f")" "$f"
  fi
  otool -l "$f" | grep -q "path $rpath " || install_name_tool -add_rpath "$rpath" "$f"
}

# Fails if anything still points outside the system or the bundle.
check() {
  local bad=0
  for f in "$@"; do
    for d in $(deps "$f"); do
      if [[ "$d" != @rpath/* && "$d" != @executable_path/* && "$d" != @loader_path/* ]]; then
        echo "error: $f references $d" >&2; bad=1
      fi
    done
    if otool -l "$f" | grep -A2 LC_RPATH | grep -E "path (/opt|/usr/local|/Users|/private|/tmp)"; then
      echo "error: $f has an absolute rpath" >&2; bad=1
    fi
  done
  [ $bad -eq 0 ]
}

sign() {
  if [ "$identity" = "-" ]; then
    codesign --force --sign - "$@"
  else
    codesign --force --timestamp --options runtime --sign "$identity" "$@"
  fi
}

license_files() {
  local dest="$1"
  mkdir -p "$dest"
  cp "$root/LICENSE" "$dest/LICENSE"
  cp "$ffmpeg/share/ffmpeg-license/"* "$dest/" 2>/dev/null || true
  cat > "$dest/FFMPEG-NOTICE.txt" <<EOF
This software uses libraries from the FFmpeg project (https://ffmpeg.org)
under the LGPL version 2.1 or later. They are dynamically linked and can be
replaced. See COPYING.LGPLv2.1 and BUILD.txt for the license and the exact
source and configuration used.
EOF
}

# --- CLI ------------------------------------------------------------------
cli="$stage/jb-$version-macos-$arch"
mkdir -p "$cli/bin" "$cli/lib"
# ffprobe (LGPL, same build) is used by `jb import-media`.
cp "$release/jb" "$release/jb-mcp" "$ffmpeg/bin/ffprobe" "$cli/bin/"
chmod u+w "$cli/bin/"*
copy_libs "$cli/lib" "$cli/bin/"*
for f in "$cli/bin/"*; do fix_refs "$f" "@executable_path/../lib"; done
for f in "$cli/lib/"*.dylib; do fix_refs "$f" "@loader_path"; done
check "$cli/bin/"* "$cli/lib/"*.dylib
license_files "$cli"
sign "$cli/lib/"*.dylib
sign "$cli/bin/"*
"$cli/bin/jb" --version
(cd "$stage" && ditto -c -k --keepParent "$(basename "$cli")" "$out/jb-$version-macos-$arch.zip")
echo "wrote $out/jb-$version-macos-$arch.zip"

# --- App ------------------------------------------------------------------
if [ -d "$app_src" ]; then
  app="$stage/Jibaru Editor.app"
  cp -R "$app_src" "$app"
  exe_name="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"
  exe="$app/Contents/MacOS/$exe_name"
  fw="$app/Contents/Frameworks"
  copy_libs "$fw" "$exe"
  fix_refs "$exe" "@executable_path/../Frameworks"
  for f in "$fw/"*.dylib; do fix_refs "$f" "@loader_path"; done
  check "$exe" "$fw/"*.dylib
  license_files "$app/Contents/Resources/licenses"
  sign "$fw/"*.dylib
  sign "$app"
  codesign --verify --deep --strict "$app"

  base="Jibaru-Editor-$version-macos-$arch"
  (cd "$stage" && ditto -c -k --sequesterRsrc --keepParent "Jibaru Editor.app" "$out/$base.app.zip")
  dmg_root="$stage/dmg"
  mkdir -p "$dmg_root"
  cp -R "$app" "$dmg_root/"
  ln -s /Applications "$dmg_root/Applications"
  hdiutil create -volname "Jibaru Editor" -srcfolder "$dmg_root" -ov -format UDZO "$out/$base.dmg" >/dev/null
  if [ "$identity" != "-" ]; then
    codesign --force --timestamp --sign "$identity" "$out/$base.dmg"
  fi
  if [ "$identity" != "-" ] && [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
    xcrun notarytool submit "$out/$base.dmg" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" --wait
    xcrun stapler staple "$out/$base.dmg"
  fi
  echo "wrote $out/$base.dmg and $out/$base.app.zip"
else
  echo "note: $app_src not found; skipped the app (run pnpm tauri build --bundles app)"
fi

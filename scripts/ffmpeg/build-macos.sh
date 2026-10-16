#!/usr/bin/env bash
# Builds an LGPL, shared-library FFmpeg for macOS from source.
#
#   scripts/ffmpeg/build-macos.sh <prefix> [arm64|x86_64]
#
# The result in <prefix> has lib/, include/ and bin/ffmpeg, bin/ffprobe (the
# tools are used to generate test media and are not shipped). Library install
# names are @rpath/<lib>, so binaries linking against them need an LC_RPATH
# to the directory holding the dylibs (scripts/package/macos.sh adds it).
#
# Autodetection is disabled so libraries from Homebrew can never leak in: the
# only external dependencies are system frameworks (VideoToolbox,
# AudioToolbox, CoreVideo, ...) and the system zlib and bzip2.
# --enable-gpl, --enable-nonfree and --enable-version3 are never passed, so
# the result is LGPL 2.1 or later.
set -euo pipefail

FFMPEG_VERSION="${FFMPEG_VERSION:-9.0.2}"
prefix="${1:?usage: build-macos.sh <prefix> [arm64|x86_64]}"
arch="${2:-arm64}"
mkdir -p "$prefix"
prefix="$(cd "$prefix" && pwd)"
work="${FFMPEG_BUILD_DIR:-${TMPDIR:-/tmp}/ffmpeg-build-$arch}"
min_macos="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

mkdir -p "$work"
cd "$work"
tarball="ffmpeg-$FFMPEG_VERSION.tar.xz"
[ -f "$tarball" ] || curl -fsSL -o "$tarball" "https://ffmpeg.org/releases/$tarball"
rm -rf "ffmpeg-$FFMPEG_VERSION"
tar xf "$tarball"
cd "ffmpeg-$FFMPEG_VERSION"

cross=()
if [ "$arch" != "$(uname -m)" ]; then
  cross=(--enable-cross-compile --target-os=darwin)
fi

./configure \
  --prefix="$prefix" \
  --arch="$arch" ${cross[@]+"${cross[@]}"} \
  --cc="clang -arch $arch" \
  --extra-cflags="-mmacosx-version-min=$min_macos" \
  --extra-ldflags="-mmacosx-version-min=$min_macos" \
  --install-name-dir='@rpath' \
  --enable-shared --disable-static --enable-pic \
  --disable-autodetect \
  --enable-videotoolbox --enable-audiotoolbox \
  --enable-zlib --enable-bzlib \
  --disable-doc --disable-debug \
  --disable-ffplay

make -j"$(sysctl -n hw.ncpu)"
make install

# The ffmpeg/ffprobe tools need an rpath to find the @rpath dylibs.
for tool in ffmpeg ffprobe; do
  install_name_tool -add_rpath "@executable_path/../lib" "$prefix/bin/$tool" 2>/dev/null || true
done

# Refuse to hand out anything that is not LGPL.
buildconf="$("$prefix/bin/ffprobe" -hide_banner -buildconf 2>&1)"
if echo "$buildconf" | grep -Eq -- "--enable-(gpl|nonfree|version3)"; then
  echo "error: FFmpeg was configured with GPL/nonfree components" >&2
  exit 1
fi

# License text and build configuration, shipped next to the libraries.
mkdir -p "$prefix/share/ffmpeg-license"
cp COPYING.LGPLv2.1 LICENSE.md "$prefix/share/ffmpeg-license/"
{
  echo "FFmpeg $FFMPEG_VERSION, LGPL 2.1 or later, built as shared libraries."
  echo "Source: https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
  echo "Build script: scripts/ffmpeg/build-macos.sh in https://github.com/Jibaru/jibaru-editor"
  echo
  echo "$buildconf"
} > "$prefix/share/ffmpeg-license/BUILD.txt"
"$prefix/bin/ffprobe" -hide_banner -L 2>&1 | head -3
echo "FFmpeg $FFMPEG_VERSION ($arch, LGPL) installed to $prefix"

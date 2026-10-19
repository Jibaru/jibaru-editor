#!/usr/bin/env bash
# Regenerates the engine footage used by the promo (public/clips, public/cards,
# public/frames, public/sfx). public/screens holds a screenshot of the app with
# sample content only and is committed as is. Everything is rendered by Jibaru Editor's own engine on its
# procedural sample scenes: no third-party or personal footage.
#
# Usage: bash scripts/generate-clips.sh [path/to/jb]
#   JB         path to the jb binary (default: build it with cargo)
#   HEIGHT     player-frames height (default 2160, so vertical crops stay sharp)
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
PUBLIC="$HERE/public"
HEIGHT="${HEIGHT:-2160}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

JB="${1:-${JB:-}}"
if [[ -z "$JB" ]]; then
  # shellcheck disable=SC1090
  [[ -f ~/.cargo/env ]] && source ~/.cargo/env
  (cd "$REPO" && cargo build --release -p jb-cli)
  JB="${CARGO_TARGET_DIR:-$REPO/target}/release/jb"
fi

# Transitions shown full screen in the promo (registry ids, see
# crates/jb-render/src/library.rs). Each renders a 2 s sample at 30 fps.
CLIPS=(
  jb.candy_swirl
  jb.cube_rotate
  jb.glitch_digital
  jb.cine_film_burn
  jb.light_leak
  jb.fireball_swirl
  jb.comic_page
  jb.carousel_spin_left
  jb.cross_zoom
  jb.flip_pages_right
  jb.film_burn
  jb.dry_brush_sweeps
  jb.dissolve
)

mkdir -p "$PUBLIC/clips" "$PUBLIC/cards" "$PUBLIC/sfx"

for id in "${CLIPS[@]}"; do
  name="${id#jb.}"
  echo "render $id"
  "$JB" fx player-frames "$id" --height "$HEIGHT" -o "$TMP/$name" --json >/dev/null
  ffmpeg -loglevel error -y -framerate 30 -i "$TMP/$name/%03d.jpg" \
    -c:v libx264 -preset slow -crf 17 -pix_fmt yuv420p -movflags +faststart \
    "$PUBLIC/clips/$name.mp4"
done

# Stills of the two sample scenes (first/last frame of the dissolve sample),
# used as clip thumbnails on the recreated timeline.
mkdir -p "$PUBLIC/frames"
ffmpeg -loglevel error -y -i "$PUBLIC/clips/dissolve.mp4" -vf "select=eq(n\\,0),scale=640:-2" -frames:v 1 -q:v 3 "$PUBLIC/frames/scene-a.jpg"
ffmpeg -loglevel error -y -sseof -0.05 -i "$PUBLIC/clips/dissolve.mp4" -vf "scale=640:-2" -frames:v 1 -q:v 3 "$PUBLIC/frames/scene-b.jpg"

# Library card thumbnails (256x144 stills + 6-column hover sprites).
"$JB" fx previews -o "$TMP/previews" --json >/dev/null
CARDS=(
  jb.dissolve jb.blur_dissolve jb.cross_zoom jb.fast_zoom jb.cube_rotate jb.glitch_digital
  jb.film_burn jb.light_leak jb.candy_swirl jb.fireball_swirl jb.comic_page jb.flip_pages_right
  jb.carousel_spin_left jb.cine_film_burn jb.spin_zoom jb.dry_brush_sweeps jb.glitch_bands jb.fast_light_leaks
  jb.vhs jb.glitch jb.mosaic jb.chromatic_aberration jb.camera_shake jb.gaussian_blur
)
for id in "${CARDS[@]}"; do
  src="$(ls "$TMP"/previews/*/"$id.jpg" 2>/dev/null | head -1 || true)"
  [[ -n "$src" ]] && cp "$src" "$PUBLIC/cards/${id#jb.}.jpg"
done

# Subtle synthesized whoosh / hit (filtered noise and a sine thump).
ffmpeg -loglevel error -y -f lavfi -i "anoisesrc=d=0.45:c=pink:a=0.5" \
  -af "highpass=f=400,lowpass=f=6000,afade=t=in:d=0.25,afade=t=out:st=0.25:d=0.2,volume=0.5" \
  "$PUBLIC/sfx/whoosh.wav"
ffmpeg -loglevel error -y -f lavfi -i "sine=f=70:d=0.35" \
  -af "afade=t=out:st=0.02:d=0.33,volume=0.9" "$PUBLIC/sfx/hit.wav"

echo "done -> $PUBLIC"

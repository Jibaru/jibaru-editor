# Render fidelity against Filmora

How closely `jb render` reproduces Filmora 15.7's own exports of the four
user projects (`docs/filmora-coverage.md`), how it is measured, and what is
still different.

## Method

**References.** Filmora exports of the same `.wfp` files, 3840×2160,
H.264 ~40 Mbit/s, AAC 192 kbit/s: the user's own exports of *Project A*, *Project B* and *Project C*, and a fresh export
of a copy of *Project D* made with Filmora 15.7.3 (a fresh export of
*Project C* was decoded frame-identical to the user's export, so the older
exports are valid references). Reference videos are not in the repository.

**Harness.** `crates/jb-cli/examples/fidelity.rs`:

```sh
cargo run --release -p jb-cli --example fidelity -- PROJECT.wfp REF.mp4 \
    --range 1255s-1263s --size 960x540 --out DIR [--strip T N] [--at T,...]
```

It renders the range with the production path (`jb_export::export`, no
camera proxies), decodes both videos with the same FFmpeg conversion
(area resample in YUV, then YUV→RGB at the compare size), aligns
them (±4 frames search; use `--search 0` for whole-project exports, whose
frame count matches ours: a window whose content is off by one frame can
otherwise align at ±1 and hide it), and prints
per-frame mean absolute difference (MAD, 0–255 over RGB) and PSNR, signed
channel bias, a tone table, per-second MAD, and audio RMS envelopes in 50 ms
windows (mean |ΔdB| over windows above −50 dBFS, and lag). `DIR/sheet.png`
stacks reference / ours / |diff|×4 for the worst frames (or `--strip`
consecutive frames); `DIR/frames.csv` has every frame.

Effects were fitted by rendering the same window without the effect (project
converted to `.jbp` with the effect or transition removed) and fitting a
model per frame (scale, offset, blur, gain or tone curve) to the reference;
the scripts are throwaway and not committed, the resulting constants are
documented next to the code.

**Noise floor.** Two encodes of the same frames differ by MAD ≈ 1.1–1.6 at
960×540 (both sides are lossy 4:2:0 H.264); more in detailed or noisy footage
and wherever an effect amplifies noise (exposure).

## Results

MAD mean / max over the window at 960×540 (alignment fixed at 0 with
`--search 0`: every whole-project export has our frame count) unless noted;
audio is mean |ΔdB|. "First pass" is the state before the first round of
fixes, "Before" the state after it (the previous version of this report),
"After" the current one.

| Item | Window | First pass | Before | After | Fix |
|---|---|---|---|---|---|
| Duration / frame count | whole projects | 51340, 30343, 22053, 44158 frames | identical | identical | — |
| Whole project, 320×180 | Project B / Project A / Project C / Project D | — | 2.03 / 2.38 / 2.72 / 4.16 (max 56 / 150 / 49 / 84) | 2.02 / 2.38 / 2.67 / 3.58 (max 4.5 / 74 / 7.1 / 47) | see below |
| Video levels (plain footage) | 4 projects | 2.4–2.8 | 1.1–1.6 | 1.1–1.6 | output level offset |
| Slide (store pack) | Project C 94–99 s | 8.8 / 50 | 8.8 / 50 | 2.31 / 7.8 | per-frame fit |
| Film Burn Transition 02 | Project D 311–316 s | 8.0 / 48 | 8.0 / 48 | 2.57 / 8.9 | per-frame fit |
| Fast Wipe Left | Project B 1256–1261 s | 2.32 / 29.8 | 1.82 / 14.8 | 1.50 / 3.3 | refit |
| Fast Wipe Left | Project B 1286–1291 s | 1.98 / 24.2 | 1.88 / 12.8 | 1.60 / 3.0 | refit |
| Fast Zoom, cut 0.67 tick after a frame | Project A 533–536 s | — | 2.51 / 52.7 | 1.93 / 4.3 | boundary snap |
| Fast Zoom, 0.73 s long | Project A 793.5–797 s | 2.89 / 75 | 2.89 / 75 | 2.88 / 75 | gap (one frame) |
| Fast Zoom | Project A 38–43 s / Project B 1213–1218 s | 1.98 / 3.03 | 1.33 / 1.44 | 1.33 / 1.44 | — |
| Fade (photos + end of timeline) | Project A 995–1012.4 s | 13.4 / 115 | 1.36 / 4.6 | 1.33 / 4.9 | — |
| Last fade to black | Project C 731–735.8 s | — | 1.77 / 2.96 | 0.79 / 1.44 | split level offset |
| Portrait JPEGs | Project A 998.6–1001.2 s | — | 0.82 | 0.73 | no input offset on stills |
| Photo 1600×897 | Project A 1008.6–1010 s | 3.56 | 1.64 | 1.37 | no input offset on stills |
| AdjustColor exposure 45 / 46 | Project C 300–303 / 306–309 s | 21.8 / 18.2 | 1.76 / 1.84 | 1.69 / 1.76 | — |
| AdjustColor exposure 60 | Project C 390–393 s | 23.2 | 2.75 | 2.21 | exposure gain above 50 |
| AdjustColor exposure 92 | Project C 600–603 s | 22.3 | 4.02 | 2.24 | exposure gain above 50 |
| AdjustColor exposure 100 | Project C 312–315 / 365–368 s | 33.5 / 33.9 | 5.60 / 6.37 | 3.19 / 3.71 | exposure gain above 50 |
| Clip starting 1 tick after a frame | Project A 298–300.5 s | — | max 150 (black frame, 320 px) | 1.33 / 2.8 | boundary snap |
| Cut 2.3 ticks after a frame | Project B 1053–1056 s | — | max 56 (320 px) | 1.76 / 2.1 | boundary slack |
| 3× clip ending 0.26 frame into a frame | Project C 274–276 s | — | max 49 (320 px) | 1.42 / 2.0 | nearest-frame bounds |
| 10× clip ending 0.2 frame into a frame | Project D 633.5–636 s | — | max 65 (320 px) | 7.50 / 11.3 | nearest-frame bounds |
| 1× clips after retimed ones (in-points between source frames) | Project D 541–581 s | — | 10.8 (535–560 s, 320 px) | 4.43 (545–548 s) | nearest source frame |
| Speed 3× / 5× / 15× | Project C 270–274.5 s, Project D 254–266 s | — / 3.6 / 2.0 | 2.60 / 2.30 / 1.73 | 2.23 / 2.30 / 1.73 | — |
| Speed 10× | Project D 492.5–499 / 532–541 s | 25.7 (320 px) | 7.55 / 9.57 | 7.55 / 9.57 | — |
| Basic titles | Project A 0–2.5 s / Project B 134–137 s | PSNR 26.4 / 4% narrow | 0.73 / 0.49 | 0.73 / 0.49 | — |
| Audio, plain clips | all projects | 0.1–0.2 dB | 0.1–0.2 dB | 0.1–0.2 dB | — |
| Audio, AI Denoise clips | Project A 12–32 / 41–49 / 141–160 s | 8.7 / 9.1 / 8.3 dB | 4.58 / 3.37 / 7.64 dB | 3.69 / 2.95 / 8.08 dB | makeup gain |
| Audio, AI Denoise, aligned | 20 windows of Project A | — | 3.39 dB (ours +1.95) | 3.23 dB (ours −0.02) | makeup gain |

## What was fixed

First round (previous report): output level offset, frame-grid timing,
transitions out of the last clip, Fast Zoom, Fast Wipe, Fade, Basic title
glyph ratio, fill for near-16:9 photos, the AdjustColor exposure curve,
retimed in-points, audio transitions and AI Denoise import/rendering, and
the harness' audio padding. This round:

- **Slide** (`jb.pan_slide`, store `Slide_771515`): not a pan. Every frame
  of each side is the clip mapped through a uv-space zoom + rotation +
  shift `src = c + (a0, b0) + k R(φ) (uv − c)` with mirrored repeat and a
  box motion blur along `(bx, by)`; the outgoing clip zooms in to ~1.07 and
  tilts to 3.6° before whipping right, the incoming one lands at 1.10 /
  5.0° shifted left and settles with an ease-out. The parameters are fitted
  per frame of the export (per-frame residual 2–6 MAD at 480 px, against
  ~18 for the best scale + offset) and tabulated in `SLIDE_FIT`.
- **Film Burn Transition 02**: procedural, fitted per frame: a radial zoom
  blur about a moving centre followed by a per-channel display-space affine
  `g·c + o0 + o1·(u − 0.5)` whose horizontal term carries the warm leak and
  the brown burn, plus drawn sprocket holes while it washes out
  (`BURN_FIT`). No Filmora asset is used.
- **Fast Wipe**: offsets 0.147 / 0.52 / 0.94 frame at 2 / 1 / 0 frames from
  the cut, mirrored repeat (not wrap) and a gaussian smear with sigma 0.32 ×
  the offset (the fitted kernel; box and one-sided kernels fit worse).
- **Level offset split** (`RenderOptions::input_bias`): part of Filmora's
  offset happens at video decode (−0.8, +0.51, −0.9), the rest after
  compositing (−2.1, −1.56, −1.79); the decode part is amplified by an
  exposure boost and faded with the clip, and still images do not get it.
  Fitted on the exposure 60–100 clips; plain footage is unchanged.
  *Superseded:* the post-composite part was an artifact of the harness'
  reference decode and is gone; the decode part now applies to full-range
  video only (see *Synthetic test media*).
- **Exposure above 50**: the curve stops steepening at 50 (`k = 2.55`) and
  the slider scales the input instead, `g = 1 + 0.0145 (s − 50)`, so
  highlights clip (at x ≈ 0.58 for 100).
- **Clip bounds** (`jb-core` evaluation): a clip shows on the frames whose
  time plus half a frame it contains (bounds between frames round to the
  nearest frame), boundaries up to 100 ticks after a frame time count as at
  it (Filmora writes bounds a few ticks off the grid), and source time
  counts from the frame nearest to the clip's start.
- **Source frame** (`jb-media`): the decoder shows the source frame nearest
  to the requested time, not the one containing it (in-points between
  source frames after speed edits were a frame late from 0.5 frame on).
  This replaces the 1 ms snap, which the pool applied twice.
- **AI Denoise**: denoised voices get a −2.5 dB makeup gain (Filmora's
  denoised speech is quieter than the input even where speech dominates).
- **Export**: an unchanged retimed in-point keeps the project's
  `speed.offset` (the importer snaps it to the frame grid).

## Full-project scan

Every project was compared over its whole length at 320×180
(`--range 0s-END --size 320x180 --search 0`, all 148 k frames); 1 s windows
above MAD 4–6 or with a frame above 3× that were inspected. Found and fixed
this way: the black frame at Project A 299.27 s, the stale frames at Project B
1054.69 s, Project C 274.78 s and Project D 634.71 s (clip bounds), and
the one-frame lag of every clip after a retimed one in Project D (541–581,
635–655, 897–909 s). What remains above the floor:

- Project D's first frame of each 10× clip that follows a 1× clip (492.19,
  631.80, 874.24, 891.42 s; MAD 32–47 on that frame): Filmora shows the
  source frame at `speed.offset` there (3–4 source frames later than the
  rounded retimed in-point it uses from the next frame on). Our clip model
  has one linear source mapping per clip, so this is not reproduced.
- The 0.73 s Fast Zoom frame at Project A 795.23 s (below).
- Project C 510–560 s and Project D high-detail foliage (775, 800 s):
  1 s means of 6–8 at 320 px with no bias; exposure-100 footage and 4K
  foliage amplify encoder noise (see Noise floor).

## Synthetic test media

The reference batches (`docs/filmora-references.md`) use synthetic cards
(gridA/gridB, sceneA/sceneB). Plain windows with no transition measured
MAD 4.1–4.3 (gridA) and 5.0–5.8 (gridB) at 960×540, with a tone curve
that bent up to ±10–18 in the highlights and R/B about −1.4 (default
levels) or +1.3 (`JB_FIDELITY_LEVELS=none`). The cause was not the media's
color tags; it was our level offset plus the harness and the decoder's
downscale filter.

**Color tags.** gridA/gridB: H.264 High, `yuv420p`, `tv` range, BT.709
matrix and primaries, transfer `iec61966-2-1` (sRGB: FFmpeg kept the PNG's
tag although `-color_trc bt709` was passed). The RGB→YUV matrix really is
BT.709 (PNG vs the clip decoded as BT.709: MAD 2.0, bias −0.25; decoded as
BT.601: MAD 6.0). sceneA/sceneB: transfer and primaries unspecified. Every
Filmora export is H.264 Baseline, `yuv420p`, `tv`, BT.709 everywhere. The
user projects' GoPro footage is HEVC `yuvj420p`, **full range** (`pc`).

**Filmora on limited-range video is a pass-through.** Its 1080p export of
a plain gridA window against the source clip, compared plane by plane in
YUV (no RGB conversion involved): Y/Cb/Cr mean difference +0.15 / −0.09 /
−0.09, MAD 0.92 / 1.08 / 1.14 (6 Mbit/s compression). The sRGB and
unspecified transfer tags are ignored (no transfer conversion, BT.709
matrix); re-tagging gridA's transfer to BT.709 changed nothing in our
render either. Our render at 1920×1080 with no offsets matches with bias
(−0.19, +0.05, −0.02). So Filmora applies **no level offset to
limited-range video**.

**Full-range video does get one.** Project C at native 3840×2160 (no
resampling anywhere) with no offsets: plain footage ours−ref
(+0.32, −0.34, +0.93), exposure 45 (+1.84, −0.37, +1.91): the decode-side
offset (−0.8, +0.51, −0.9) and its amplification by exposure, nothing
more. The decode-side offset now applies only to full-range YUV video
(`FILMORA_INPUT_BIAS`); stills and RGB sources never get it.

**The post-composite offset was the harness.** The harness decoded the
reference with `scale=W:H:flags=area` straight to `rgb24`. In FFmpeg 9
that single swscale pass (downscale and YUV→RGB at once, chroma not fully
interpolated) is biased and blurred; our render, already at the compare
size, got an unscaled conversion. Against a box average of a
full-resolution conversion of the same frame:

| Decode of the reference | gridA 1080p→960 | Project C 4K→960 |
|---|---|---|
| one pass, `scale=960:540:flags=area` → rgb24 (old harness) | MAD 2.52, bias (−1.68, −1.37, −1.45) | MAD 1.73, bias (−1.98, −1.23, −1.75) |
| area in YUV, then YUV→RGB (new harness) | MAD 2.49, bias (+0.19, −0.01, +0.24) | MAD 0.54, bias (+0.04, +0.04, +0.06) |

That bias is the old post-composite "output offset" (−2.1, −1.56, −1.79)
to within the fit's accuracy: it was fitted to cancel the harness. The
harness now resamples in YUV (keeping 4:2:0) and converts to RGB at the
compare size, the same chain for both videos, and
`RenderOptions::output_bias` defaults to zero.

**Downscale filter.** The decoder scaled sources to the render size with
swscale bilinear, whose downscale kernel is a wide tent: test-card edges,
checkers and text came out softer than the area-averaged reference
(gridA 960×540 with the new harness: 3.13 bilinear, 1.81 area). Decoded
video is now scaled with `SWS_AREA` (`Scaler::area`); upscales are
unaffected (swscale uses bilinear for them).

**Floor.** Filmora's export against the pass-through source, both through
the harness chain at 960×540 (Filmora's frame-to-frame noise on the static
card is only 0.12–0.15, so the floor is the encode, not temporal noise):

| Window | Low (tr001, 5.9 Mbit/s) | ~15 Mbit/s (tr013) |
|---|---|---|
| gridA 0.1–1.4 s | 1.42 | 1.11 |
| gridB 3.6–3.95 s (just after a transition) | 5.69 | 2.25 |

gridB at Low is all compression: the encoder spends its bits on the
transition and the plain frames after it are blocky. The newer
~15–20 Mbit/s exports (tr013 onwards) have a much lower floor there.

**Before / after** (MAD mean at 960×540, `--search 0`; before = previous
code and harness):

| Window | Before | After | After, `JB_FIDELITY_RENDER_SIZE=1920x1080 JB_FIDELITY_ENCODER=libx264` |
|---|---|---|---|
| gridA, tr001 0.1–1.4 s | 4.18 (levels none 4.31) | 1.81 | 1.30 |
| gridB, tr001 3.6–3.95 s | 5.83 (levels none) | 5.15 | 5.07 |
| gridA, tr013 0.1–1.4 s | 4.23 (levels none) | 1.67 | 1.03 |
| gridB, tr013 3.6–3.95 s | 5.52 (levels none) | 3.02 | 1.86 |
| sceneA, fx001 6.1–7.9 s | 1.91 (lane 4's floor) | 1.52 | — |
| Project C plain, 100–103 s | 2.32 | 1.70 | — |
| Project C last fade to black, 731–735.8 s | 0.79 | 0.83 | — |
| Project C exposure 45, 300–303 s | 1.69 | 1.58 | — |
| Project C exposure 100, 312–315 s | 3.19 | 3.13 | — |

With full-resolution rendering, plain windows sit at or under the floor
estimate (ours does not carry Filmora's own encode error twice). At
960×540 the rest is our own 4:2:0 chain: chroma resampled twice (decode at
half size, RGB, NV12 again) softens colored edges and leaves R/B about
−0.5 on the grid card (flat areas: bias < 0.1, MAD 0.2).

Not re-measured: the other user projects (their Filmora exports are not
on this machine) and the fits made through the old offsets (Slide and
Film Burn tables, lane fits that used the default levels). Fits made with
`JB_FIDELITY_LEVELS=none` are unaffected on the synthetic media, since the
cards are limited range; fits made with the default levels lose the
(−2.9, −1.05, −2.69) total offset they were fitted on top of, which the
old harness hid.

## Remaining gaps

- **Fast Zoom of 0.73 s** (Project A 795.2 s): Filmora shows the outgoing
  clip at full zoom on the frame starting 1 tick after the cut, where 2 s
  Fast Zooms show the incoming one; the rule behind it (the transition's
  odd tick length, 7340667) was not found. One frame.
- **First frame of 10× clips after 1× clips** (Project D, four frames):
  see the scan above.
- **Speed 10×** windows stay at 7.5–9.6: at 10× consecutive frames differ a
  lot, so one source frame of difference on some frames costs a lot; the
  rounding of Filmora's retimed mapping is only known for the first frame.
- **Exposure 100** still 3.2–3.7 (exposure 92 at 2.2): the shape of the
  shoulder below the clip point differs slightly by clip (per-clip optimum
  k 2.3–2.7, g 1.7–1.9).
- **Slide / Film Burn** are tabulated fits of one export each (2 s, center);
  other durations scale in progress, which is unverified. Their worst frames
  (7.8 / 8.9) are on the cut frames.
- **AI Denoise** is still an approximation of a neural denoiser (aligned
  mean |ΔdB| 3.2 over 20 windows): Filmora attenuates speech and noise
  differently per clip. Its output is delayed by 70 ms on 26 of 39 denoised
  clips of Project A (and most of Project C's), with the same media,
  strength and settings as the undelayed ones; no rule in the project data
  explains which, so it is not reproduced (where it applies, the harness
  reports 3–4 dB more than the aligned difference).
- **Detailed footage**: bright foliage at 4K stays at MAD ~5–7 with no
  measurable bias; likely encoder noise.

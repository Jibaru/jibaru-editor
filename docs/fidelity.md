# Render fidelity against Filmora

How closely `jb render` reproduces Filmora 15.7's own exports of the four
user projects (`docs/filmora-coverage.md`), how it is measured, and what is
still different.

## Method

**References.** Filmora exports of the same `.wfp` files, 3840×2160,
H.264 ~40 Mbit/s, AAC 192 kbit/s: the user's own exports of *Proyecto
2026-08-01*, *Fenix Manga Cafe* and *Viaje a Colombia 1*, and a fresh export
of a copy of *Viaje a Colombia 2* made with Filmora 15.7.3 (a fresh export of
*Colombia 1* was decoded frame-identical to the user's export, so the older
exports are valid references). Reference videos are not in the repository.

**Harness.** `crates/jb-cli/examples/fidelity.rs`:

```sh
cargo run --release -p jb-cli --example fidelity -- PROJECT.wfp REF.mp4 \
    --range 1255s-1263s --size 960x540 --out DIR [--strip T N] [--at T,...]
```

It renders the range with the production path (`jb_export::export`, no
camera proxies), decodes both videos with the same FFmpeg conversion, aligns
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
| Whole project, 320×180 | Fenix / Proyecto / Colombia 1 / Colombia 2 | — | 2.03 / 2.38 / 2.72 / 4.16 (max 56 / 150 / 49 / 84) | 2.02 / 2.38 / 2.67 / 3.58 (max 4.5 / 74 / 7.1 / 47) | see below |
| Video levels (plain footage) | 4 projects | 2.4–2.8 | 1.1–1.6 | 1.1–1.6 | output level offset |
| Slide (store pack) | Colombia 1 94–99 s | 8.8 / 50 | 8.8 / 50 | 2.31 / 7.8 | per-frame fit |
| Film Burn Transition 02 | Colombia 2 311–316 s | 8.0 / 48 | 8.0 / 48 | 2.57 / 8.9 | per-frame fit |
| Fast Wipe Left | Fenix 1256–1261 s | 2.32 / 29.8 | 1.82 / 14.8 | 1.50 / 3.3 | refit |
| Fast Wipe Left | Fenix 1286–1291 s | 1.98 / 24.2 | 1.88 / 12.8 | 1.60 / 3.0 | refit |
| Fast Zoom, cut 0.67 tick after a frame | Proyecto 533–536 s | — | 2.51 / 52.7 | 1.93 / 4.3 | boundary snap |
| Fast Zoom, 0.73 s long | Proyecto 793.5–797 s | 2.89 / 75 | 2.89 / 75 | 2.88 / 75 | gap (one frame) |
| Fast Zoom | Proyecto 38–43 s / Fenix 1213–1218 s | 1.98 / 3.03 | 1.33 / 1.44 | 1.33 / 1.44 | — |
| Fade (photos + end of timeline) | Proyecto 995–1012.4 s | 13.4 / 115 | 1.36 / 4.6 | 1.33 / 4.9 | — |
| Last fade to black | Colombia 1 731–735.8 s | — | 1.77 / 2.96 | 0.79 / 1.44 | split level offset |
| Portrait JPEGs | Proyecto 998.6–1001.2 s | — | 0.82 | 0.73 | no input offset on stills |
| Photo 1600×897 | Proyecto 1008.6–1010 s | 3.56 | 1.64 | 1.37 | no input offset on stills |
| AdjustColor exposure 45 / 46 | Colombia 1 300–303 / 306–309 s | 21.8 / 18.2 | 1.76 / 1.84 | 1.69 / 1.76 | — |
| AdjustColor exposure 60 | Colombia 1 390–393 s | 23.2 | 2.75 | 2.21 | exposure gain above 50 |
| AdjustColor exposure 92 | Colombia 1 600–603 s | 22.3 | 4.02 | 2.24 | exposure gain above 50 |
| AdjustColor exposure 100 | Colombia 1 312–315 / 365–368 s | 33.5 / 33.9 | 5.60 / 6.37 | 3.19 / 3.71 | exposure gain above 50 |
| Clip starting 1 tick after a frame | Proyecto 298–300.5 s | — | max 150 (black frame, 320 px) | 1.33 / 2.8 | boundary snap |
| Cut 2.3 ticks after a frame | Fenix 1053–1056 s | — | max 56 (320 px) | 1.76 / 2.1 | boundary slack |
| 3× clip ending 0.26 frame into a frame | Colombia 1 274–276 s | — | max 49 (320 px) | 1.42 / 2.0 | nearest-frame bounds |
| 10× clip ending 0.2 frame into a frame | Colombia 2 633.5–636 s | — | max 65 (320 px) | 7.50 / 11.3 | nearest-frame bounds |
| 1× clips after retimed ones (in-points between source frames) | Colombia 2 541–581 s | — | 10.8 (535–560 s, 320 px) | 4.43 (545–548 s) | nearest source frame |
| Speed 3× / 5× / 15× | Colombia 1 270–274.5 s, Colombia 2 254–266 s | — / 3.6 / 2.0 | 2.60 / 2.30 / 1.73 | 2.23 / 2.30 / 1.73 | — |
| Speed 10× | Colombia 2 492.5–499 / 532–541 s | 25.7 (320 px) | 7.55 / 9.57 | 7.55 / 9.57 | — |
| Basic titles | Proyecto 0–2.5 s / Fenix 134–137 s | PSNR 26.4 / 4% narrow | 0.73 / 0.49 | 0.73 / 0.49 | — |
| Audio, plain clips | all projects | 0.1–0.2 dB | 0.1–0.2 dB | 0.1–0.2 dB | — |
| Audio, AI Denoise clips | Proyecto 12–32 / 41–49 / 141–160 s | 8.7 / 9.1 / 8.3 dB | 4.58 / 3.37 / 7.64 dB | 3.69 / 2.95 / 8.08 dB | makeup gain |
| Audio, AI Denoise, aligned | 20 windows of Proyecto | — | 3.39 dB (ours +1.95) | 3.23 dB (ours −0.02) | makeup gain |

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
this way: the black frame at Proyecto 299.27 s, the stale frames at Fenix
1054.69 s, Colombia 1 274.78 s and Colombia 2 634.71 s (clip bounds), and
the one-frame lag of every clip after a retimed one in Colombia 2 (541–581,
635–655, 897–909 s). What remains above the floor:

- Colombia 2's first frame of each 10× clip that follows a 1× clip (492.19,
  631.80, 874.24, 891.42 s; MAD 32–47 on that frame): Filmora shows the
  source frame at `speed.offset` there (3–4 source frames later than the
  rounded retimed in-point it uses from the next frame on). Our clip model
  has one linear source mapping per clip, so this is not reproduced.
- The 0.73 s Fast Zoom frame at Proyecto 795.23 s (below).
- Colombia 1 510–560 s and Colombia 2 high-detail foliage (775, 800 s):
  1 s means of 6–8 at 320 px with no bias; exposure-100 footage and 4K
  foliage amplify encoder noise (see Noise floor).

## Remaining gaps

- **Fast Zoom of 0.73 s** (Proyecto 795.2 s): Filmora shows the outgoing
  clip at full zoom on the frame starting 1 tick after the cut, where 2 s
  Fast Zooms show the incoming one; the rule behind it (the transition's
  odd tick length, 7340667) was not found. One frame.
- **First frame of 10× clips after 1× clips** (Colombia 2, four frames):
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
  clips of Proyecto (and most of Colombia 1's), with the same media,
  strength and settings as the undelayed ones; no rule in the project data
  explains which, so it is not reproduced (where it applies, the harness
  reports 3–4 dB more than the aligned difference).
- **Detailed footage**: bright foliage at 4K stays at MAD ~5–7 with no
  measurable bias; likely encoder noise.

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
them (±4 frames search; every window reported here aligned at 0), and prints
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

MAD mean / max over the window at 960×540 unless noted; audio is mean |ΔdB|.

| Item | Window | Before | After | Fix |
|---|---|---|---|---|
| Duration / frame count | whole projects | 51340, 30343, 22053, 44158 frames | identical | — |
| Video levels (plain footage) | 4 projects | 2.4–2.8 (ours +2.9 R, +1.0 G, +2.7 B) | 1.1–1.6 (bias < 0.3) | output level offset |
| Source frame on tick-truncated in-points | Proyecto 32.7–40.1 s | 1 frame late (6.1 at 320 px) | aligned, 1.4 | decoder frame snap |
| Range renders off the frame grid | any `--range 1212s-…` | half-frame phase error | on grid | export |
| Fast Zoom | Proyecto 38–43 s | 1.98 / 31.3 | 1.33 / 3.5 | refit |
| Fast Zoom | Fenix 1213–1218 s | 3.03 / 40.5 | 1.44 / 3.3 | refit |
| Fast Zoom, 0.73 s long | Proyecto 793.5–797 s | 2.89 / 75 | 2.89 / 75 | gap (one frame) |
| Fast Wipe Left | Fenix 1256–1261 s | 2.32 / 29.8 | 1.82 / 14.8 | refit |
| Fast Wipe Left | Fenix 1286–1291 s | 1.98 / 24.2 | 1.88 / 12.8 | refit |
| Fade (between photos + end of timeline) | Proyecto 995–1012.4 s | 13.4 / 115 | 1.36 / 4.6 | refit + end transition |
| Fade at end of timeline | Fenix 1708–1713 s | 16.7 / 55 | 1.38 / 2.5 | end transition |
| Basic title, centered | Proyecto 0–2.5 s | text 4.2% narrow, PSNR 26.4 | bbox exact (1043 vs 1042 px at 1080p), PSNR 31.3 | glyph ratio |
| Basic title, lower third | Fenix 134–137 s | 4% narrow | bbox ±1 px, 0.49 | glyph ratio |
| Sticker clip boundaries | Fenix 980–982.7 s | 1.87 / 7.0 | 1.74 / 2.1 | frame times |
| Photo 1600×897 (near 16:9) | Proyecto 1008.6–1010 s | 3.56 | 1.64 | fit snaps to fill |
| AdjustColor exposure 45 | Colombia 1 300–303 s | 21.8 | 1.76 | exposure curve |
| AdjustColor exposure 46 | Colombia 1 306–309 s | 18.2 | 1.84 | exposure curve |
| AdjustColor exposure 60 | Colombia 1 390–393 s | 23.2 | 2.75 | exposure curve |
| AdjustColor exposure 92 | Colombia 1 600–603 s | 22.3 | 4.02 | exposure curve |
| AdjustColor exposure 100 | Colombia 1 312–315 / 365–368 s | 33.5 / 33.9 | 5.6 / 6.4 | exposure curve (partial) |
| AutoEnhance | Colombia 1 | — | — | disabled in the project (`enable: false`), not rendered by either |
| Speed 10× (five clips) | Colombia 2 492.5–499 s (320 px) | 25.7 | 4.4 | retimed in-point |
| Speed 5× / 15× / 3× | Colombia 2 254–266 s, Colombia 1 270–274.5 s | 3.6 / 2.0 / — | 2.3 / 1.8 / 2.6 | retimed in-point |
| Slide (store pack) | Colombia 1 94–99 s | 8.8 / 50 | 8.8 / 50 | gap |
| Film Burn Transition 02 | Colombia 2 311–316 s | 8.0 / 48 | 8.0 / 48 | gap |
| Audio fade (cross-cut) | Fenix 1256–1261 s | 1.10 dB (5.6 dB at the cut) | 0.08 dB | mixer |
| Audio, plain clips | all projects | 0.1–0.2 dB | 0.1–0.2 dB | — |
| Audio, AI Denoise clips | Proyecto 12–32 / 41–49 / 76–115 / 141–160 s | 8.7 / 9.1 / 9.5 / 8.3 dB | 4.6 / 3.4 / 4.8 / 7.6 dB | denoiser |

## What was fixed

- **Output levels** (`jb-render`, `RenderOptions::output_bias`). Filmora's
  output is darker than an exact BT.709 conversion by about −1.6 Y, −0.6 Cb,
  −0.85 Cr code values at every level, i.e. RGB (−2.9, −1.05, −2.69). It is
  applied after compositing: whites pushed up by exposure land at ~252, and
  still images match too. Tests render with exact levels.
- **Frame timing** (`jb-media`, `jb-export`). Source times within 1 ms of a
  frame start select that frame (Filmora writes frame-grid in-points
  truncated to ticks; the previous frame was shown on some clips). Range
  renders start on the sequence frame grid and frames are sampled at their
  exact start rounded to the nearest tick, the rounding Filmora uses for
  clip bounds.
- **Transitions out of the last clip** (`jb-core`): Filmora projects end with
  a Fade on the last clip; it now fades into nothing instead of being
  ignored.
- **Fast Zoom**: no blur; the scale is `1 + 0.3·exp(−60·|p − 0.5|)` (1.30 on
  the first frame of the incoming clip, 1.11/1.04/1.01 one to three frames
  away), the cut at the midpoint.
- **Fast Wipe Left/Right/Up/Down**: two frames on each side of the midpoint;
  offset 0.14·3.2^(2−f) frame widths and box blur ±0.07·5^(2−f) for `f`
  frames from the cut.
- **Fade**: display-space gain `smoothstep(0, 0.475, |p − 0.5|)`.
- **Basic titles** (`jb-filmora`): glyph size 0.752 of the text box height
  (was 0.72).
- **Fit mode**: sources within 1% of the canvas aspect fill it (Filmora
  stretches a 1600×897 photo to 16:9).
- **AdjustColor exposure** (`jb.filmora.adjust_color` → `jb.filmora.exposure`):
  display-space `y = 1 − (1 − x)^k`, `k = 1 + 0.01256·s^1.244`; the other
  sliders still go through `jb.color.adjust`.
- **Retimed clips** (`jb-filmora` import): Filmora plays a retimed clip from
  its retimed in-point (`offset / rate`) rounded to a timeline frame, counted
  from the clip's first frame; at 10× that is up to 4 source frames earlier
  than `speed.offset`.
- **Audio transitions** (`jb-core` voices): Filmora's audio fade ramps the
  outgoing clip 1 → 0.5 until the cut and the incoming one 0.5 → 1 from the
  cut, without overlap; the mixer ignored transitions before.
- **AI Denoise** (`AudioSettings::denoise_db`, `jb-media` denoiser): imported
  from `CacheAlg`/`AudioDenoiseCache` (attenuation limit 10 or 15 dB) and
  rendered by spectral power subtraction against a per-clip noise spectrum
  (median power, ×3), floored at the limit.
- **Harness audio**: decoded audio is padded to its first timestamp; without
  it AAC priming trimmed by the edit list showed up as a 23 ms lag.

## Remaining gaps

- **Slide** (store `Slide_771515`) and **Film Burn Transition 02**: the
  incoming clip in Filmora's Slide is scaled ~1.1–1.3 and shifted, but no
  affine fit of the clean frames explains it (residual MAD ~18), so the
  motion is probably not a plain scale/offset; Film Burn is an overlay-based
  look we approximate. Both are used once in the corpus.
- **Fast Zoom of 0.73 s** (Proyecto 795.2 s): Filmora shows the outgoing
  clip on the frame where the clip boundary and the transition midpoint
  coincide; with 2 s Fast Zooms it shows the incoming one. One frame.
- **Fast Wipe**: the two smeared frames on each side of the cut remain at
  MAD 9–15 (the smear's exact kernel is not identified).
- **Exposure ≥ 90**: 4–6 MAD; Filmora's curve clips harder near white than
  the fitted family at high values (a soft-clipped gain fits s = 92–100
  better but s = 45–60 worse).
- **AI Denoise** is an approximation of a neural denoiser: levels are within
  3–5 dB on most clips but the envelope differs in detail, and Filmora's
  denoised audio of some clips (Proyecto 141–161 s, Colombia 1 312 s) is
  delayed by 70 ms, which we do not reproduce.
- **Near black**: during the last fade to black our output is ~2 levels
  darker than Filmora (the level offset clips at 0 where Filmora's does not).
- **Portrait JPEGs** (Proyecto 997–1007 s): mean level within 0.8, but
  shadows are ~7 levels darker and midtones ~2 lighter than Filmora's,
  suggesting a different JPEG/ICC decode; not investigated.
- **Detailed footage**: bright foliage at 4K (Colombia 2 319–321 s) stays at
  MAD ~5.7 with no measurable bias; likely encoder noise, not a rendering
  difference.

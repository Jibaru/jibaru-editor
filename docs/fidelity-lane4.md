# Render fidelity, lane 4: glitch / warp / creative transitions, effects and filters

Lane 4 of the fidelity work against Filmora 15.7.3 covers 71 glitch,
distortion, warp, ink and creative transitions and 87 video effects,
filters and overlays from the reference batches
(`docs/filmora-references.md`). **24 of 158** items have an
implementation that imported projects reach through `filmora:<GUID>`
aliases (`crates/jb-render/src/aliases_lane4.rs`). Everything was fitted to
Filmora's *exported output* only (pairs of known input frames and the
exported frames); no Filmora asset (shader, LUT, texture, preset file) was
read.

## Method

* **Inputs.** Effects: the batch plays each effect on a gridA clip and a
  sceneA clip (2.002 s each, source from 1.001 s), so the unprocessed frame
  of every exported frame is known exactly (`docs/filmora-references.md`).
  Transitions: the outgoing and incoming clips are the known test cards.
  Fits use the decode-side and output level offsets of `RenderOptions`
  (`FILMORA_INPUT_BIAS`, `FILMORA_OUTPUT_BIAS`) so that the fitted model is
  what sits between them in our pipeline.
* **Measurement.** `crates/jb-cli/examples/fidelity.rs` on the batch
  project and its export, item window, `--size 960x540 --search 0`; the
  tables give MAD mean / max over the window (0–255).
* **Floor.** The batches were exported at Filmora's *Low* quality
  (≈ 6 Mbit/s at 1080p). Plain, unprocessed windows measure MAD **4.1–4.5
  on the grid card** (fine checker and text are what the encoder smears)
  and **1.9 on the scene** (`fx001` 34–36 s, an effect Filmora did not
  apply), with a residual level bias of R −1.5, B −1.4 that belongs to the
  global level offsets, not to any item. Grid numbers should therefore be
  read against ≈ 4.3, scene numbers against ≈ 1.9.

### Models (shared machinery)

| Module | Model | Fitting |
|---|---|---|
| `fx/lane4_looks.rs` | 17³ 3D LUT in display space, optionally followed by a 16×9 bilinear gain/offset field (vignettes, light maps): `out = gain(uv)·lut(in) + offset(uv)`. | LUT by regularized least squares over every pixel pair of the grid and scene windows (trilinear weights, second-difference smoothness so unmeasured colors extrapolate linearly); the field by per-node linear regression, alternated with the LUT three times. Data in `fx/lane4/luts/*.lut|.field`. |
| `fx/lane4_camera.rs` | Per-frame camera: similarity transform about the centre, motion-blur vector, zoom blur, separate red/blue sampling offsets (RGB split), per-channel gain/offset. Rows tabulated by clip time. | Levenberg–Marquardt per frame on the grid and scene frames jointly (variable projection for gain/offset), initialized by phase correlation and the previous frame, blur seeded from a small candidate set. |
| `fx/lane4_reveal.rs` | Transition mask from a procedural reveal map `m(p) = (1−k)·dot(p, dir) + k·|p−c| + amp·(fbm(p·f + seed) − ½)`; the incoming clip shows where `m < thr(progress)`. | Alpha of every exported frame recovered from the known A and B frames; `dir, k, c, amp, f, seed` searched to minimize the mask error with `thr` set per frame to the measured coverage, tabulated. Shapes are our own noise fields, so the timing and coverage match exactly but edges differ in detail. |

## Results

### Effects and filters

| Filmora item | Batch | Ours | Grid (mean / max) | Scene (mean / max) | Notes |
|---|---|---|---|---|---|
| Exposure | fx001 | — | — | — | not implemented |
| Glitch Effect 4 | fx001 | — | — | — | not implemented |
| 1977 | fx001 | `jb.l4.look_rose_fade` | 5.43 / 5.8 | 2.31 / 6.2 |  |
| Amaro | fx001 | `jb.l4.look_glow_matte` | 6.55 / 7.0 | 4.33 / 7.2 |  |
| Baltan | fx001 | — | — | — | not implemented |
| BlueSkies | fx001 | `jb.l4.look_blue_sky` | 5.00 / 5.2 | 2.63 / 2.9 |  |
| Brannan | fx001 | `jb.l4.look_warm_contrast_fade` | 5.32 / 5.5 | 2.96 / 3.4 |  |
| Bright Lights | fx001 | `jb.l4.look_airy_blue_lift` | 5.66 / 5.7 | 3.17 / 3.5 |  |
| Classic | fx001 | — | — | — | not implemented |
| Earlybird | fx001 | `jb.l4.look_golden_morning` | 5.76 / 6.0 | 4.04 / 4.4 |  |
| Hefe | fx001 | `jb.l4.look_warm_punch` | 6.82 / 6.9 | 3.56 / 7.3 |  |
| Hudson | fx001 | `jb.l4.look_icy_vignette` | 6.51 / 6.7 | 4.47 / 7.1 |  |
| InkWell | fx001 | `jb.l4.look_mono_classic` | 4.58 / 5.1 | 2.71 / 4.9 |  |
| Lomo | fx001 | `jb.l4.look_contrast_vignette` | 6.26 / 6.4 | 3.34 / 6.9 |  |
| LordKelvin | fx001 | `jb.l4.look_hot_amber` | 7.04 / 7.2 | 3.18 / 7.9 |  |
| Mild | fx001 | — | — | — | not implemented |
| Money | fx001 | — | — | — | not implemented |
| Nashville | fx001 | `jb.l4.look_pink_pastel` | 6.56 / 7.4 | 2.92 / 3.3 |  |
| Sierra | fx002 | `jb.l4.look_faded_vignette` | 5.02 / 5.4 | 3.45 / 5.7 |  |
| SunSet | fx002 | `jb.l4.look_dusk_fade` | 5.11 / 5.3 | 3.02 / 3.3 |  |
| Sunrise | fx002 | `jb.l4.look_warm_haze` | 7.19 / 7.4 | 3.87 / 4.2 |  |
| Sutro | fx002 | `jb.l4.look_dark_moody` | 5.13 / 5.3 | 3.56 / 4.0 |  |
| Toaster | fx002 | `jb.l4.look_toasted` | 6.56 / 6.7 | 4.60 / 5.0 |  |
| Valencia | fx002 | `jb.l4.look_soft_warm_fade` | 4.94 / 5.2 | 2.44 / 2.8 |  |
| Vivid | fx002 | `jb.l4.look_vivid_warm` | 9.65 / 9.8 | 5.21 / 5.7 |  |
| Walden | fx002 | `jb.l4.look_pale_cyan` | 5.91 / 6.0 | 3.19 / 3.6 |  |
| Wash | fx002 | — | — | — | not implemented |
| XproII | fx002 | `jb.l4.look_cross_process` | 6.83 / 6.9 | 3.62 / 4.0 |  |
| Radial Blur | fx002 | — | — | — | not implemented |
| B&W Style | fx002 | `jb.l4.look_mono_bright` | 3.83 / 4.3 | 2.55 / 4.3 |  |
| Sunny Days | fx002 | `jb.l4.look_clear_day` | 4.76 / 5.0 | 2.43 / 5.4 |  |
| Chromatic Aberration | fx002 | — | — | — | not implemented |
| Mosaic | fx002 | — | — | — | not implemented |
| Tilt-shift Circle | fx002 | — | — | — | not implemented |
| Canvas | fx003 | — | — | — | not implemented |
| Fast Light Leaks 2 | fx003 | — | — | — | not implemented |
| Lut Process | fx003 | `jb.l4.look_dark_muted` | 3.16 / 3.7 | 1.90 / 2.3 |  |
| Flashlight | fx003 | — | — | — | not implemented |
| Camera Focus Bokeh | fx003 | — | — | — | not implemented |
| Drop Spread | fx003 | — | — | — | not implemented |
| Movie Light Effects 1 | fx003 | — | — | — | not implemented |
| Movie Light Effects 2 | fx003 | — | — | — | not implemented |
| Movie Light Effects 4 | fx003 | — | — | — | not implemented |
| Movie Light Effects 5 | fx003 | — | — | — | not implemented |
| Movie Light Effects 6 | fx003 | — | — | — | not implemented |
| Movie Light Effects 7 | fx003 | — | — | — | not implemented |
| Movie Light Effects 8 | fx003 | — | — | — | not implemented |
| Color Flash | fx003 | — | — | — | not implemented |
| Elasticity Jitter | fx003 | — | — | — | not implemented |
| Flicker Jitter 03 | fx003 | — | — | — | not implemented |
| Glow Vibrate | fx003 | — | — | — | not implemented |
| Pulse Jitter | fx003 | — | — | — | not implemented |
| Shake the Camera | fx003 | — | — | — | not implemented |
| Glitch Effect 2 | fx003 | — | — | — | not implemented |
| Local Color | fx003 | — | — | — | not implemented |
| No Signal Glitch 01 | fx003 | — | — | — | not implemented |
| Psychedelic Glitch | fx003 | — | — | — | not implemented |
| Stripe Glitch 01 | fx003 | — | — | — | not implemented |
| Strong Sharpening | fx003 | — | — | — | not implemented |
| Highlight Glow | fx003 | — | — | — | not implemented |
| Old Movie 2 | fx003 | — | — | — | not implemented |
| Old TV Border | fx003 | — | — | — | not implemented |
| Edge Glow 01 | fx003 | — | — | — | not implemented |
| Scanline Effect 1 | fx003 | — | — | — | not implemented |
| Chaos Shake | fx003 | — | — | — | not implemented |
| Collision | fx003 | — | — | — | not implemented |
| Crash Into | fx003 | — | — | — | not implemented |
| Earthquake Shake | fx003 | — | — | — | not implemented |
| Extreme | fx003 | — | — | — | not implemented |
| Flash Exposure | fx003 | — | — | — | not implemented |
| Ripple Shake 02 | fx003 | — | — | — | not implemented |
| Up-Down 1 | fx003 | — | — | — | not implemented |
| Shattered 02 | fx003 | — | — | — | not implemented |
| Shattered 03 | fx003 | — | — | — | not implemented |
| Exposure Zoom | fx003 | — | — | — | not implemented |
| Fast Zoom | fx003 | — | — | — | not implemented |
| Heart Zoom | fx003 | — | — | — | not implemented |
| Parallax Smooth Zoom In | fx003 | — | — | — | not implemented |
| Rabid Camera 1 | fx003 | — | — | — | not implemented |
| Zoom Lens | fx003 | — | — | — | not implemented |
| Aesthetic Light Leak 04 16-9 | fxfix1 | — | — | — | not implemented |
| CALIFORNIA VINTAGE FILM LOOK | fxfix1 | — | — | — | not implemented |
| Vintage Film Pack GRUNGE FILM VINTAGE | fxfix1 | — | — | — | not implemented |
| Four Seasons | fxfix1 | — | — | — | not implemented |
| Glow | fxfix1 | — | — | — | not implemented |
| Radio Explike FX | fxfix1 | — | — | — | not implemented |
| TV Wall | fxfix1 | — | — | — | not implemented |

### Transitions

| Filmora item | Batch | Ours | Window (mean / max) | Notes |
|---|---|---|---|---|
| Glitch line | tr001 | — | — | not implemented |
| Kaleidoscope | tr002 | — | — | not implemented |
| Pixel Sorting Up | tr003 | — | — | not implemented |
| Pixel Game Transition Transition 09 | tr003 | — | — | not implemented |
| Butterfly Wave Scrawler | tr004 | — | — | not implemented |
| Ripple | tr005 | — | — | not implemented |
| Swirl | tr005 | — | — | not implemented |
| Wave | tr005 | — | — | not implemented |
| Orb Twist 1 | tr005 | — | — | not implemented |
| Orb Twist 3 | tr005 | — | — | not implemented |
| Orb Twist 4 | tr005 | — | — | not implemented |
| Liquid Bodies Pack Transition 03 | tr006 | — | — | not implemented |
| Neon Mosaic 03 | tr006 | — | — | not implemented |
| Neon Mosaic 04 | tr006 | — | — | not implemented |
| Neon Mosaic Down | tr006 | — | — | not implemented |
| Warp 3 | tr006 | — | — | not implemented |
| Warp 6 | tr006 | — | — | not implemented |
| Warp 7 | tr006 | — | — | not implemented |
| 3D Warp Box Distortion Left | tr008 | — | — | not implemented |
| 3D Warp Box Distortion Right | tr008 | — | — | not implemented |
| 3D Warp Box Distortion Up | tr008 | — | — | not implemented |
| 3D Warp Box Left | tr008 | — | — | not implemented |
| 3D Warp Box Open Left | tr008 | — | — | not implemented |
| 3D Warp Box Open Right | tr008 | — | — | not implemented |
| 3D Warp Box Rebound | tr008 | — | — | not implemented |
| 3D Warp Box Right | tr008 | — | — | not implemented |
| Depth Mosaic | tr008 | — | — | not implemented |
| Cartoon Liquid Transition 04 | tr010 | — | — | not implemented |
| Comic Smoke Transition 07 | tr010 | — | — | not implemented |
| Brush And Ink Transition 34 | tr010 | — | — | not implemented |
| Brush And Ink Transition 36 | tr010 | — | — | not implemented |
| Brush And Ink Transition 40 | tr010 | — | — | not implemented |
| Brush Paint Pack Transition 09 | tr010 | — | — | not implemented |
| Colorful Ink Slideshow Vol 01 Transition 04 | tr010 | — | — | not implemented |
| Dynamic Ink 01 | tr010 | — | — | not implemented |
| Dynamic Ink 02 | tr011 | — | — | not implemented |
| Dynamic Ink 03 | tr011 | — | — | not implemented |
| Dynamic Ink 05 | tr011 | — | — | not implemented |
| Dynamic Ink 06 | tr011 | — | — | not implemented |
| Dynamic Ink 07 | tr011 | — | — | not implemented |
| Dynamic Ink 08 | tr011 | — | — | not implemented |
| Dynamic Ink 09 | tr011 | — | — | not implemented |
| Dynamic Ink 10 | tr011 | — | — | not implemented |
| Glow Ink Down | tr011 | — | — | not implemented |
| Glow Ink Left | tr011 | — | — | not implemented |
| Glow Ink Right | tr011 | — | — | not implemented |
| Glow Ink Up | tr011 | — | — | not implemented |
| Ink Brush Mask Transition | tr011 | — | — | not implemented |
| Ink Reveal Transition 02 | tr011 | — | — | not implemented |
| Liquid Spread Transition 02 | tr011 | — | — | not implemented |
| Liquid Spread Transition 05 | tr011 | — | — | not implemented |
| Liquid Spread Transition 06 | tr011 | — | — | not implemented |
| Liquid Spread Transition 09 | tr011 | — | — | not implemented |
| Liquid Spread Transition 10 | tr011 | — | — | not implemented |
| Luminous Ink Wash 01 | tr011 | — | — | not implemented |
| Luminous Ink Wash 02 | tr011 | — | — | not implemented |
| Luminous Ink Wash 03 | tr011 | — | — | not implemented |
| Luminous Ink Wash 04 | tr011 | — | — | not implemented |
| Luminous Ink Wash 05 | tr011 | — | — | not implemented |
| Luminous Ink Wash 06 | tr011 | — | — | not implemented |
| Luminous Ink Wash 07 | tr011 | — | — | not implemented |
| Luminous Ink Wash 08 | tr012 | — | — | not implemented |
| Luminous Ink Wash 09 | tr012 | — | — | not implemented |
| Luminous Ink Wash 10 | tr012 | — | — | not implemented |
| Moonlight Ink Transition 01 | tr012 | — | — | not implemented |
| Moonlight Ink Transition 02 | tr012 | — | — | not implemented |
| Moonlight Ink Transition 03 | tr012 | — | — | not implemented |
| Moonlight Ink Transition 04 | tr012 | — | — | not implemented |
| Moonlight Ink Transition 05 | tr012 | — | — | not implemented |
| Blink Eyes 03 | tr012 | — | — | not implemented |
| Mosaic | trfix1 | — | — | not implemented |

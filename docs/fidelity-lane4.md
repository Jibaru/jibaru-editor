# Render fidelity, lane 4: glitch / warp / creative transitions, effects and filters

Lane 4 of the fidelity work against Filmora 15.7.3 covers 71 glitch,
distortion, warp, ink and creative transitions and 87 video effects,
filters and overlays from the reference batches
(`docs/filmora-references.md`). **107 of 158** items have an
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
  read against ≈ 4.3, scene numbers against ≈ 1.9. The tables give the
  *excess over floor* in parentheses (window mean minus 4.13 grid,
  1.91 scene, 4.47 transition grid); part of the floor is a global color
  offset investigated separately and not touched here.
* **Hard cut.** For transitions, the last column is what an unmapped GUID
  renders (outgoing clip up to the cut, incoming after), so an
  implementation is only kept when it beats it.

### Models (shared machinery)

| Module | Model | Fitting |
|---|---|---|
| `fx/lane4_looks.rs` | 17³ 3D LUT in display space, optionally followed by a 16×9 bilinear gain/offset field (vignettes, light maps): `out = gain(uv)·lut(in) + offset(uv)`. | LUT by regularized least squares over every pixel pair of the grid and scene windows (trilinear weights, second-difference smoothness so unmeasured colors extrapolate linearly); the field by per-node linear regression, alternated with the LUT three times. Data in `fx/lane4/luts/*.lut|.field`. |
| `fx/lane4_camera.rs` | Per-frame camera: similarity transform about the centre, motion-blur vector, zoom blur, separate red/blue sampling offsets (RGB split), per-channel gain/offset. Rows tabulated by clip time. | Levenberg–Marquardt per frame on the grid and scene frames jointly (variable projection for gain/offset), initialized by phase correlation and the previous frame, blur seeded from a small candidate set. |
| `fx/lane4_looks.rs` (options) | Per-look field resolution (16×9 or 48×27 nodes for detailed light maps and borders), `tiles` (video wall: box-downscaled copy repeated n×n), `lines` (periodic horizontal gain, 16 samples per period). | Field: as above at the chosen resolution, the finer one kept when it measures better. Lines: period from the row-ratio spectrum, profile folded per phase, LUT refitted on the de-striped target. |
| `fx/lane4_camtr.rs` | Transition: each clip through its own per-frame camera (similarity, motion/zoom blur, lens k1), mixed by a per-frame weight, then gain/offset. | Same LM as the camera fits with both clips known, forward pass from the outgoing side, backward pass from the incoming side where the forward fit is poor. |
| `fx/lane4_warp.rs` | Transition: one displacement for both clips (ripple, swirl, wobble families) and a per-frame mix. | Per-frame LM over the family's three parameters and the mix, multi-start. |
| `fx/lane4_spot.rs` | Monochrome LUT outside a rectangle; inside, a per-rectangle affine color matrix; rectangles jump on a fixed beat. | Rectangles from the saturation mask of each frame, segments where they change, matrices by least squares per segment. |
| `fx/lane4_region.rs` | Fixed rectangle blurred with a separable gaussian. | Rectangle from the smoothed difference map, sigmas by search. |
| `fx/lane4_reveal.rs` | Transition mask from a procedural reveal map `m(p) = (1−k)·dot(p, dir) + k·|p−c| + amp·(fbm(p·f + seed) − ½)`; the incoming clip shows where `m < thr(progress)`. | Alpha of every exported frame recovered from the known A and B frames; `dir, k, c, amp, f, seed` searched to minimize the mask error with `thr` set per frame to the measured coverage, tabulated. Shapes are our own noise fields, so the timing and coverage match exactly but edges differ in detail. |

## Results

### Effects and filters

| Filmora item | Batch | Ours | Grid mean / max (excess) | Scene mean / max (excess) | Notes |
|---|---|---|---|---|---|
| Exposure | fx001 | `jb.l4.look_overexposed` | 7.42 / 7.6 (+3.3) | 5.16 / 5.5 (+3.2) | approximation: soft bloom not modelled |
| Glitch Effect 4 | fx001 | — | — | — | not implemented |
| 1977 | fx001 | `jb.l4.look_rose_fade` | 5.43 / 5.8 (+1.3) | 2.31 / 6.2 (+0.4) |  |
| Amaro | fx001 | `jb.l4.look_glow_matte` | 6.03 / 6.4 (+1.9) | 3.86 / 6.7 (+1.9) | LUT + field; residual light-map detail finer than 16×9 |
| Baltan | fx001 | — | — | — | not implemented |
| BlueSkies | fx001 | `jb.l4.look_blue_sky` | 5.00 / 5.2 (+0.9) | 2.63 / 2.9 (+0.7) |  |
| Brannan | fx001 | `jb.l4.look_warm_contrast_fade` | 5.20 / 5.4 (+1.1) | 2.82 / 3.3 (+0.9) |  |
| Bright Lights | fx001 | `jb.l4.look_airy_blue_lift` | 5.66 / 5.7 (+1.5) | 3.17 / 3.5 (+1.3) |  |
| Classic | fx001 | `jb.l4.look_pencil_tone` | 14.67 / 14.8 (+10.5) | 12.73 / 13.4 (+10.8) |  |
| Earlybird | fx001 | `jb.l4.look_golden_morning` | 5.24 / 5.5 (+1.1) | 3.53 / 3.9 (+1.6) |  |
| Hefe | fx001 | `jb.l4.look_warm_punch` | 5.79 / 5.9 (+1.7) | 2.78 / 6.3 (+0.9) |  |
| Hudson | fx001 | `jb.l4.look_icy_vignette` | 5.85 / 6.0 (+1.7) | 3.92 / 6.4 (+2.0) |  |
| InkWell | fx001 | `jb.l4.look_mono_classic` | 4.57 / 5.1 (+0.4) | 2.71 / 4.9 (+0.8) |  |
| Lomo | fx001 | `jb.l4.look_contrast_vignette` | 6.04 / 6.2 (+1.9) | 3.08 / 6.7 (+1.2) |  |
| LordKelvin | fx001 | `jb.l4.look_hot_amber` | 6.98 / 7.2 (+2.9) | 3.10 / 7.8 (+1.2) |  |
| Mild | fx001 | `jb.l4.cam_gentle_sway` | 8.95 / 10.8 (+4.8) | 3.66 / 13.4 (+1.8) |  |
| Money | fx001 | — | — | — | not implemented |
| Nashville | fx001 | `jb.l4.look_pink_pastel` | 6.56 / 7.4 (+2.4) | 2.92 / 3.3 (+1.0) |  |
| Sierra | fx002 | `jb.l4.look_faded_vignette` | 4.72 / 5.1 (+0.6) | 3.13 / 5.3 (+1.2) |  |
| SunSet | fx002 | `jb.l4.look_dusk_fade` | 4.69 / 4.9 (+0.6) | 2.65 / 3.0 (+0.7) |  |
| Sunrise | fx002 | `jb.l4.look_warm_haze` | 6.74 / 7.0 (+2.6) | 3.53 / 3.9 (+1.6) |  |
| Sutro | fx002 | `jb.l4.look_dark_moody` | 4.49 / 4.7 (+0.4) | 3.05 / 3.5 (+1.1) |  |
| Toaster | fx002 | `jb.l4.look_toasted` | 6.07 / 6.2 (+1.9) | 3.93 / 4.4 (+2.0) |  |
| Valencia | fx002 | `jb.l4.look_soft_warm_fade` | 4.94 / 5.2 (+0.8) | 2.44 / 2.8 (+0.5) |  |
| Vivid | fx002 | `jb.l4.look_vivid_warm` | 9.04 / 9.2 (+4.9) | 4.64 / 5.2 (+2.7) | LUT + field; residual light-map detail finer than 16×9 |
| Walden | fx002 | `jb.l4.look_pale_cyan` | 5.72 / 5.8 (+1.6) | 2.92 / 3.4 (+1.0) |  |
| Wash | fx002 | `jb.l4.look_bright_wash` | 18.16 / 18.3 (+14.0) | 12.86 / 13.5 (+10.9) |  |
| XproII | fx002 | `jb.l4.look_cross_process` | 6.48 / 6.6 (+2.4) | 3.26 / 3.6 (+1.3) |  |
| Radial Blur | fx002 | — | — | — | not implemented |
| B&W Style | fx002 | `jb.l4.look_mono_bright` | 3.83 / 4.3 (+0.0) | 2.55 / 4.3 (+0.6) |  |
| Sunny Days | fx002 | `jb.l4.look_clear_day` | 4.76 / 5.0 (+0.6) | 2.43 / 5.4 (+0.5) |  |
| Chromatic Aberration | fx002 | — | — | — | not implemented |
| Mosaic | fx002 | `jb.l4.region_smear` | 4.85 / 5.5 (+0.7) | 2.38 / 2.5 (+0.5) |  |
| Tilt-shift Circle | fx002 | — | — | — | not implemented |
| Canvas | fx003 | `jb.l4.look_canvas_tone` | 11.81 / 11.9 (+7.7) | 11.94 / 12.0 (+10.0) |  |
| Fast Light Leaks 2 | fx003 | — | — | — | not implemented |
| Lut Process | fx003 | `jb.l4.look_dark_muted` | 3.16 / 3.7 (+0.0) | 1.90 / 2.3 (+0.0) |  |
| Flashlight | fx003 | `jb.l4.look_spotlight` | 20.53 / 32.7 (+16.4) | 16.99 / 28.8 (+15.1) |  |
| Camera Focus Bokeh | fx003 | — | — | — | not implemented |
| Drop Spread | fx003 | — | — | — | not implemented |
| Movie Light Effects 1 | fx003 | — | — | — | not implemented |
| Movie Light Effects 2 | fx003 | — | — | — | not implemented |
| Movie Light Effects 4 | fx003 | — | — | — | not implemented |
| Movie Light Effects 5 | fx003 | — | — | — | not implemented |
| Movie Light Effects 6 | fx003 | — | — | — | not implemented |
| Movie Light Effects 7 | fx003 | — | — | — | not implemented |
| Movie Light Effects 8 | fx003 | — | — | — | not implemented |
| Color Flash | fx003 | `jb.l4.cam_color_pulse` | 11.30 / 16.4 (+7.2) | 7.93 / 19.1 (+6.0) |  |
| Elasticity Jitter | fx003 | `jb.l4.cam_elastic_jitter` | 11.09 / 15.3 (+7.0) | 4.75 / 6.9 (+2.8) |  |
| Flicker Jitter 03 | fx003 | `jb.l4.cam_flicker_jolt` | 11.69 / 18.9 (+7.6) | 7.06 / 14.0 (+5.1) |  |
| Glow Vibrate | fx003 | — | — | — | not implemented |
| Pulse Jitter | fx003 | `jb.l4.cam_pulse_bounce` | 13.18 / 24.9 (+9.1) | 7.95 / 18.3 (+6.0) |  |
| Shake the Camera | fx003 | `jb.l4.cam_impact_shake` | 19.77 / 32.4 (+15.6) | 11.24 / 26.0 (+9.3) |  |
| Glitch Effect 2 | fx003 | — | — | — | not implemented |
| Local Color | fx003 | `jb.l4.spot_color_jump` | 3.17 / 4.0 (+0.0) | 1.84 / 2.1 (+0.0) | per-segment rectangles and tints measured exactly |
| No Signal Glitch 01 | fx003 | — | — | — | not implemented |
| Psychedelic Glitch | fx003 | — | — | — | not implemented |
| Stripe Glitch 01 | fx003 | — | — | — | not implemented |
| Strong Sharpening | fx003 | — | — | — | not implemented |
| Highlight Glow | fx003 | `jb.l4.look_highlight_lift` | 5.66 / 5.9 (+1.5) | 4.98 / 5.6 (+3.1) | approximation: bloom not modelled |
| Old Movie 2 | fx003 | `jb.l4.look_old_movie_tone` | 13.54 / 16.9 (+9.4) | 4.53 / 5.7 (+2.6) |  |
| Old TV Border | fx003 | `jb.l4.look_old_tv_border` | 4.49 / 5.3 (+0.4) | 3.49 / 4.7 (+1.6) | approximation: border shape finer than 16×9 field |
| Edge Glow 01 | fx003 | — | — | — | not implemented |
| Scanline Effect 1 | fx003 | `jb.l4.look_scanlines` | 6.13 / 6.2 (+2.0) | 3.02 / 6.9 (+1.1) |  |
| Chaos Shake | fx003 | `jb.l4.cam_rgb_jolt` | 9.67 / 16.5 (+5.5) | 3.68 / 6.4 (+1.8) |  |
| Collision | fx003 | `jb.l4.cam_impact_flash` | 8.24 / 23.0 (+4.1) | 4.91 / 23.0 (+3.0) |  |
| Crash Into | fx003 | `jb.l4.cam_crash_zoom` | 8.00 / 24.6 (+3.9) | 5.53 / 38.5 (+3.6) |  |
| Earthquake Shake | fx003 | `jb.l4.cam_rumble` | 6.34 / 12.8 (+2.2) | 2.59 / 4.5 (+0.7) |  |
| Extreme | fx003 | `jb.l4.cam_hard_shake` | 10.14 / 14.6 (+6.0) | 3.43 / 5.4 (+1.5) |  |
| Flash Exposure | fx003 | `jb.l4.cam_flash_pop` | 6.45 / 21.4 (+2.3) | 3.92 / 21.8 (+2.0) |  |
| Ripple Shake 02 | fx003 | — | — | — | not implemented |
| Up-Down 1 | fx003 | `jb.l4.cam_vertical_whip` | 9.73 / 16.8 (+5.6) | 5.15 / 16.0 (+3.2) |  |
| Shattered 02 | fx003 | — | — | — | not implemented |
| Shattered 03 | fx003 | — | — | — | not implemented |
| Exposure Zoom | fx003 | `jb.l4.cam_bright_zoom` | 18.22 / 26.3 (+14.1) | 10.51 / 18.0 (+8.6) |  |
| Fast Zoom | fx003 | `jb.l4.cam_zoom_burst` | 5.26 / 11.5 (+1.1) | 2.75 / 4.4 (+0.8) |  |
| Heart Zoom | fx003 | — | — | — | not implemented |
| Parallax Smooth Zoom In | fx003 | `jb.l4.cam_slow_push` | 5.09 / 5.7 (+1.0) | 2.46 / 2.7 (+0.6) |  |
| Rabid Camera 1 | fx003 | `jb.l4.cam_frantic` | 7.41 / 9.5 (+3.3) | 4.76 / 5.9 (+2.8) |  |
| Zoom Lens | fx003 | `jb.l4.cam_lens_punch` | 5.43 / 6.1 (+1.3) | 2.55 / 2.7 (+0.6) |  |
| Aesthetic Light Leak 04 16-9 | fxfix1 | — | — | — | not implemented |
| CALIFORNIA VINTAGE FILM LOOK | fxfix1 | `jb.l4.look_sunbaked_vintage` | 12.16 / 17.7 (+8.0) | 9.72 / 14.8 (+7.8) |  |
| Vintage Film Pack GRUNGE FILM VINTAGE | fxfix1 | `jb.l4.look_grunge_vintage` | 17.49 / 24.7 (+13.4) | 14.53 / 21.4 (+12.6) |  |
| Four Seasons | fxfix1 | `jb.l4.look_season_tint` | 8.35 / 8.7 (+4.2) | 6.86 / 7.5 (+5.0) | approximation: overlay finer than the field |
| Glow | fxfix1 | `jb.l4.look_bright_glow` | 5.94 / 6.3 (+1.8) | 3.83 / 6.4 (+1.9) | approximation: bloom not modelled |
| Radio Explike FX | fxfix1 | `jb.l4.look_prism_rays` | 8.84 / 9.3 (+4.7) | 6.26 / 9.4 (+4.3) |  |
| TV Wall | fxfix1 | `jb.l4.video_wall_3x3` | 11.72 / 11.9 (+7.6) | 3.83 / 12.3 (+1.9) | grid: tiny downscaled text/checker, encoder-dominated |

### Transitions

| Filmora item | Batch | Ours | Window mean / max (excess) | Hard cut (mean) | Notes |
|---|---|---|---|---|---|
| Glitch line | tr001 | — | — | — | not implemented |
| Kaleidoscope | tr002 | — | — | — | not implemented |
| Pixel Sorting Up | tr003 | — | — | — | not implemented |
| Pixel Game Transition Transition 09 | tr003 | — | — | — | not implemented |
| Butterfly Wave Scrawler | tr004 | — | — | — | not implemented |
| Ripple | tr005 | — | — | 22.9 | not implemented |
| Swirl | tr005 | — | — | — | not implemented |
| Wave | tr005 | — | — | 26.9 | not implemented |
| Orb Twist 1 | tr005 | `jb.l4.warp_twist_roll_1` | 18.57 / 26.9 (+14.1) | 43.4 |  |
| Orb Twist 3 | tr005 | `jb.l4.warp_twist_roll_2` | 18.86 / 35.8 (+14.4) | 46.7 |  |
| Orb Twist 4 | tr005 | — | — | — | not implemented |
| Liquid Bodies Pack Transition 03 | tr006 | `jb.l4.reveal_clean_sweep_down_1` | 8.61 / 29.7 (+4.1) | 14.9 |  |
| Neon Mosaic 03 | tr006 | `jb.l4.reveal_glow_bloom_1` | 17.07 / 27.9 (+12.6) | 21.3 |  |
| Neon Mosaic 04 | tr006 | `jb.l4.reveal_glow_bloom_2` | 18.45 / 32.0 (+14.0) | 23.6 |  |
| Neon Mosaic Down | tr006 | `jb.l4.reveal_glow_sweep_down` | 13.33 / 15.7 (+8.9) | 23.8 |  |
| Warp 3 | tr006 | `jb.l4.warp_zoom_burst_1` | 7.62 / 19.7 (+3.2) | 35.6 |  |
| Warp 6 | tr006 | `jb.l4.warp_zoom_burst_2` | 9.94 / 22.2 (+5.5) | 28.2 |  |
| Warp 7 | tr006 | `jb.l4.warp_zoom_burst_3` | 5.29 / 11.1 (+0.8) | 28.3 |  |
| 3D Warp Box Distortion Left | tr008 | — | — | — | not implemented |
| 3D Warp Box Distortion Right | tr008 | — | — | — | not implemented |
| 3D Warp Box Distortion Up | tr008 | — | — | — | not implemented |
| 3D Warp Box Left | tr008 | — | — | — | not implemented |
| 3D Warp Box Open Left | tr008 | — | — | — | not implemented |
| 3D Warp Box Open Right | tr008 | — | — | — | not implemented |
| 3D Warp Box Rebound | tr008 | — | — | — | not implemented |
| 3D Warp Box Right | tr008 | — | — | — | not implemented |
| Depth Mosaic | tr008 | — | — | — | not implemented |
| Cartoon Liquid Transition 04 | tr010 | `jb.l4.reveal_ink_sweep_down_1` | 18.87 / 49.1 (+14.4) | 69.0 | white matte as a flat cover band; shape procedural |
| Comic Smoke Transition 07 | tr010 | `jb.l4.reveal_ink_spread_down_left_1` | 22.87 / 65.7 (+18.4) | 41.9 | smoke matte as a flat cover band; shape procedural |
| Brush And Ink Transition 34 | tr010 | `jb.l4.reveal_clean_sweep_up_right` | 10.34 / 29.1 (+5.9) | 24.8 |  |
| Brush And Ink Transition 36 | tr010 | `jb.l4.reveal_ink_sweep_right_1` | 6.08 / 11.1 (+1.6) | 17.9 |  |
| Brush And Ink Transition 40 | tr010 | `jb.l4.reveal_clean_spread_down_left_1` | 7.42 / 17.3 (+3.0) | 24.4 |  |
| Brush Paint Pack Transition 09 | tr010 | `jb.l4.reveal_ink_sweep_right_2` | 5.68 / 7.0 (+1.2) | 7.8 |  |
| Colorful Ink Slideshow Vol 01 Transition 04 | tr010 | `jb.l4.reveal_clean_spread_right_1` | 26.75 / 57.5 (+22.3) | 49.3 | multicolor ink approximated by one cover color |
| Dynamic Ink 01 | tr010 | — | — | — | dropped: multicolor splats, procedural reveal worse than a cut |
| Dynamic Ink 02 | tr011 | — | — | — | dropped: multicolor splats, procedural reveal worse than a cut |
| Dynamic Ink 03 | tr011 | `jb.l4.reveal_ink_sweep_down_2` | 8.53 / 14.4 (+4.1) | 18.8 |  |
| Dynamic Ink 05 | tr011 | `jb.l4.reveal_ink_spread_up_left_1` | 9.41 / 18.0 (+4.9) | 21.4 |  |
| Dynamic Ink 06 | tr011 | `jb.l4.reveal_clean_sweep_down_2` | 11.43 / 27.5 (+7.0) | 19.3 |  |
| Dynamic Ink 07 | tr011 | `jb.l4.reveal_ink_spread_down_right_1` | 21.59 / 31.1 (+17.1) | 25.7 |  |
| Dynamic Ink 08 | tr011 | `jb.l4.reveal_clean_spread_right_2` | 11.09 / 24.1 (+6.6) | 25.0 |  |
| Dynamic Ink 09 | tr011 | `jb.l4.reveal_ink_spread_down_left_2` | 18.42 / 32.6 (+14.0) | 30.7 |  |
| Dynamic Ink 10 | tr011 | `jb.l4.reveal_clean_spread_left_1` | 12.87 / 46.1 (+8.4) | 31.7 |  |
| Glow Ink Down | tr011 | `jb.l4.reveal_ink_bloom_1` | 16.52 / 36.7 (+12.1) | 21.8 |  |
| Glow Ink Left | tr011 | `jb.l4.reveal_clean_spread_left_2` | 17.26 / 31.3 (+12.8) | 22.3 |  |
| Glow Ink Right | tr011 | `jb.l4.reveal_ink_spread_right_1` | 10.65 / 19.0 (+6.2) | 17.8 |  |
| Glow Ink Up | tr011 | — | — | 10.7 | not implemented |
| Ink Brush Mask Transition | tr011 | `jb.l4.reveal_ink_spread_down_right_2` | 8.90 / 19.9 (+4.4) | 13.0 |  |
| Ink Reveal Transition 02 | tr011 | `jb.l4.reveal_clean_spread_up_right` | 8.50 / 15.8 (+4.0) | 12.8 |  |
| Liquid Spread Transition 02 | tr011 | `jb.l4.reveal_ink_spread_up_left_2` | 11.96 / 27.6 (+7.5) | 25.3 |  |
| Liquid Spread Transition 05 | tr011 | `jb.l4.reveal_clean_spread_right_3` | 9.78 / 16.2 (+5.3) | 19.2 |  |
| Liquid Spread Transition 06 | tr011 | `jb.l4.reveal_ink_bloom_2` | 9.17 / 16.3 (+4.7) | 21.7 |  |
| Liquid Spread Transition 09 | tr011 | `jb.l4.reveal_ink_spread_up_right_1` | 7.46 / 20.5 (+3.0) | 25.4 |  |
| Liquid Spread Transition 10 | tr011 | `jb.l4.reveal_clean_spread_down_right_1` | 9.37 / 13.2 (+4.9) | 18.8 |  |
| Luminous Ink Wash 01 | tr011 | `jb.l4.reveal_ink_bloom_3` | 13.02 / 30.9 (+8.6) | 16.0 |  |
| Luminous Ink Wash 02 | tr011 | `jb.l4.reveal_ink_bloom_4` | 8.15 / 14.3 (+3.7) | 14.2 |  |
| Luminous Ink Wash 03 | tr011 | `jb.l4.reveal_ink_sweep_up_left_2` | 18.51 / 45.3 (+14.0) | 19.3 |  |
| Luminous Ink Wash 04 | tr011 | — | — | — | dropped: glow wash, procedural reveal worse than a cut |
| Luminous Ink Wash 05 | tr011 | `jb.l4.reveal_ink_spread_up_right_2` | 21.52 / 49.5 (+17.1) | 27.5 |  |
| Luminous Ink Wash 06 | tr011 | `jb.l4.reveal_ink_spread_up_right_3` | 20.58 / 36.3 (+16.1) | 22.6 |  |
| Luminous Ink Wash 07 | tr011 | `jb.l4.reveal_clean_spread_left_3` | 21.59 / 43.2 (+17.1) | 23.5 |  |
| Luminous Ink Wash 08 | tr012 | `jb.l4.reveal_ink_spread_down_1` | 11.52 / 28.9 (+7.0) | 29.1 |  |
| Luminous Ink Wash 09 | tr012 | `jb.l4.reveal_ink_spread_down_2` | 12.28 / 22.1 (+7.8) | 19.3 |  |
| Luminous Ink Wash 10 | tr012 | `jb.l4.reveal_ink_sweep_right_3` | 21.05 / 53.6 (+16.6) | 45.8 |  |
| Moonlight Ink Transition 01 | tr012 | `jb.l4.reveal_ink_spread_down_left_3` | 6.34 / 12.4 (+1.9) | 22.1 |  |
| Moonlight Ink Transition 02 | tr012 | `jb.l4.reveal_ink_spread_up_right_4` | 7.27 / 11.3 (+2.8) | 16.9 |  |
| Moonlight Ink Transition 03 | tr012 | `jb.l4.reveal_clean_spread_down` | 6.68 / 13.1 (+2.2) | 19.0 |  |
| Moonlight Ink Transition 04 | tr012 | `jb.l4.reveal_ink_sweep_left` | 7.90 / 10.6 (+3.4) | 16.6 |  |
| Moonlight Ink Transition 05 | tr012 | `jb.l4.reveal_clean_spread_down_right_2` | 6.17 / 8.2 (+1.7) | 17.0 |  |
| Blink Eyes 03 | tr012 | `jb.l4.reveal_ink_sweep_right_4` | 13.60 / 81.6 (+9.1) | 15.5 | black lids not modelled (reveal only) |
| Mosaic | trfix1 | — | — | — | not implemented: random block glitches, a pixelate + mix fit stays at MAD 20–58 |

## Remaining gaps

* **Grid floor.** Every grid number includes the ≈ 4.3 encoder floor of the
  Low-quality exports; looks that sit at 5–7 on the grid are within 1–3 of
  plain footage. Scene numbers are the better guide (floor ≈ 1.9).
* **Looks with textures.** Film looks with grain, scratches or paper
  textures (Canvas, grunge / vintage film, prism rays) are reproduced as a
  LUT plus a smooth light map only; the texture is not recreated, so they
  stay at MAD 6–18.
* **Rotating shakes on the grid** (Mild, Extreme, RGB jolt): 9–10 on the
  grid against 3.4–3.7 on the scene. A per-frame affine, lens and
  perspective refit at 960×540 did not reduce it, so the remaining error
  is how a rotated 10 px checker resamples, not the motion.
* **Procedural reveals.** The coverage of every frame matches; the shape of
  the edge is our own fbm, so masks differ in detail (MAD 6–12 for clean
  masks). Mattes with several ink colors (Dynamic Ink 01/02, Luminous Ink
  Wash 04) came out worse than a hard cut and are not mapped.
* **Not implemented** (no alias, Filmora GUID renders as before):
  - 3D Warp Box family and Depth Mosaic: curved 3D surfaces, not a
    corner pin.
  - Light-leak, film-burn and particle overlays (Movie Light Effects,
    Fast Light Leaks, Aesthetic Light Leak, Camera Focus Bokeh, Heart Zoom's
    hearts): Filmora composites artwork; recreating it procedurally was
    out of reach in this pass.
  - Random glitches (Glitch line, Stripe / No Signal / Glitch Effect 2,
    Pixel Sorting, Pixel Game, Kaleidoscope, Butterfly Wave, Shattered):
    a seeded procedural version would match timing only.
  - Classic (pencil sketch), Money (engraving), Baltan (animated softness
    plus fade-in), Swirl (a twist + mix fit stays at MAD 34).


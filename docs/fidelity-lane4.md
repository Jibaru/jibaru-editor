# Render fidelity, lane 4: glitch / warp / creative transitions, effects and filters

Lane 4 of the fidelity work against Filmora 15.7.3 covers 71 glitch,
distortion, warp, ink and creative transitions and 87 video effects,
filters and overlays from the reference batches
(`docs/filmora-references.md`). **112 of 158** items have an
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
  The test media are limited-range video, so no level offset sits around
  the fitted models (`RenderOptions` defaults since the harness fix);
  frames are decoded the way the harness does (area resample in YUV, then
  RGB at the compare size).
* **Measurement.** `crates/jb-cli/examples/fidelity.rs` on the batch
  project and its export, item window, `--size 960x540 --search 0`; the
  tables give MAD mean / max over the window (0–255).
* **Floor.** The batches were exported at Filmora's *Low* quality
  (≈ 6 Mbit/s at 1080p). Plain, unprocessed windows measure MAD **1.77 on
  the grid card** (`fx001` 0.1–1.9 s), **1.59 on the scene** (`fx001`
  34–36 s, an effect Filmora did not apply) and **3.56** on a plain
  transition-batch card. The tables give the *excess over floor* in
  parentheses (window mean minus 1.77 grid, 1.59 scene, 3.56 transition).
* **Hard cut.** For transitions, the last column is what an unmapped GUID
  renders (outgoing clip up to the cut, incoming after), so an
  implementation is only kept when it beats it.

### Models (shared machinery)

| Module | Model | Fitting |
|---|---|---|
| `fx/lane4_looks.rs` | 17³ 3D LUT in display space, optionally followed by a bilinear gain/offset field (vignettes, light maps): `out = gain(uv)·lut(in) + offset(uv)`. | LUT by regularized least squares over every pixel pair of the grid and scene windows (trilinear weights, second-difference smoothness so unmeasured colors extrapolate linearly); the field by per-node linear regression, alternated with the LUT three times. Data in `fx/lane4/luts/*.lut|.field`. |
| `fx/lane4_camera.rs` | Per-frame camera: similarity transform about the centre, motion-blur vector, zoom blur, separate red/blue sampling offsets (RGB split), per-channel gain/offset. Rows tabulated by clip time. | Levenberg–Marquardt per frame on the grid and scene frames jointly (variable projection for gain/offset), initialized by phase correlation and the previous frame, blur seeded from a small candidate set. |
| `fx/lane4_looks.rs` (options) | Per-look field resolution (16×9 or 48×27 nodes for detailed light maps and borders), `tiles` (video wall: box-downscaled copy repeated n×n), `lines` (periodic horizontal gain, 16 samples per period). | Field: as above at the chosen resolution, the finer one kept when it measures better. Lines: period from the row-ratio spectrum, profile folded per phase, LUT refitted on the de-striped target. |
| `fx/lane4_camtr.rs` | Transition: each clip through its own per-frame camera (similarity, motion/zoom blur, lens k1), mixed by a per-frame weight, then gain/offset. | Same LM as the camera fits with both clips known, forward pass from the outgoing side, backward pass from the incoming side where the forward fit is poor. |
| `fx/lane4_spot.rs` | Monochrome LUT outside a rectangle; inside, a per-rectangle affine color matrix; rectangles jump on a fixed beat. | Rectangles from the saturation mask of each frame, segments where they change, matrices by least squares per segment. |
| `fx/lane4_region.rs` | Fixed rectangle blurred with a separable gaussian. | Rectangle from the smoothed difference map, sigmas by search. |
| `fx/lane4_reveal.rs` | Transition mask from a procedural reveal map `m(p) = (1−k)·dot(p, dir) + k·|p−c| + amp·(fbm(p·f + seed) − ½)`; the incoming clip shows where `m < thr(progress)`. | Alpha of every exported frame recovered from the known A and B frames; `dir, k, c, amp, f, seed` searched to minimize the mask error with `thr` set per frame to the measured coverage, tabulated. Shapes are our own noise fields, so the timing and coverage match exactly but edges differ in detail. |

## Results

### Effects and filters

| Filmora item | Batch | Ours | Grid mean / max (excess) | Scene mean / max (excess) | Notes |
|---|---|---|---|---|---|
| Exposure | fx001 | `jb.l4.look_overexposed` | 6.66 / 6.8 (+4.9) | 4.95 / 5.4 (+3.4) | approximation: soft bloom not modelled |
| Glitch Effect 4 | fx001 | `jb.l4.cam_washout_glitch` | 3.59 / 6.2 (+1.8) | 2.05 / 2.7 (+0.5) |  |
| 1977 | fx001 | `jb.l4.look_rose_fade` | 3.19 / 4.1 (+1.4) | 1.73 / 3.9 (+0.1) |  |
| Amaro | fx001 | `jb.l4.look_glow_matte` | 4.28 / 5.2 (+2.5) | 3.47 / 5.0 (+1.9) | LUT + field; residual light-map detail finer than 16×9 |
| Baltan | fx001 | — | — | — | not implemented |
| BlueSkies | fx001 | `jb.l4.look_blue_sky` | 2.45 / 3.9 (+0.7) | 2.00 / 2.1 (+0.4) |  |
| Brannan | fx001 | `jb.l4.look_warm_contrast_fade` | 2.91 / 4.2 (+1.1) | 2.39 / 2.6 (+0.8) |  |
| Bright Lights | fx001 | `jb.l4.look_airy_blue_lift` | 3.20 / 4.4 (+1.4) | 2.42 / 2.6 (+0.8) |  |
| Classic | fx001 | `jb.l4.look_pencil_tone` | 14.51 / 14.7 (+12.7) | 12.45 / 13.1 (+10.9) |  |
| Earlybird | fx001 | `jb.l4.look_golden_morning` | 4.05 / 5.6 (+2.3) | 3.35 / 3.5 (+1.8) |  |
| Hefe | fx001 | `jb.l4.look_warm_punch` | 3.28 / 4.3 (+1.5) | 2.31 / 3.9 (+0.7) |  |
| Hudson | fx001 | `jb.l4.look_icy_vignette` | 4.30 / 4.9 (+2.5) | 3.58 / 4.8 (+2.0) |  |
| InkWell | fx001 | `jb.l4.look_mono_classic` | 1.69 / 3.7 (+0.0) | 1.62 / 2.1 (+0.0) |  |
| Lomo | fx001 | `jb.l4.look_contrast_vignette` | 3.46 / 4.4 (+1.7) | 2.57 / 4.0 (+1.0) |  |
| LordKelvin | fx001 | `jb.l4.look_hot_amber` | 3.86 / 4.8 (+2.1) | 2.18 / 4.7 (+0.6) |  |
| Mild | fx001 | `jb.l4.cam_gentle_sway` | 7.75 / 9.6 (+6.0) | 2.72 / 13.2 (+1.1) |  |
| Money | fx001 | — | — | — | not implemented |
| Nashville | fx001 | `jb.l4.look_pink_pastel` | 4.68 / 6.6 (+2.9) | 2.37 / 2.5 (+0.8) |  |
| Sierra | fx002 | `jb.l4.look_faded_vignette` | 3.16 / 4.0 (+1.4) | 2.80 / 3.8 (+1.2) |  |
| SunSet | fx002 | `jb.l4.look_dusk_fade` | 2.94 / 4.0 (+1.2) | 2.50 / 2.6 (+0.9) |  |
| Sunrise | fx002 | `jb.l4.look_warm_haze` | 6.26 / 6.7 (+4.5) | 3.29 / 3.5 (+1.7) |  |
| Sutro | fx002 | `jb.l4.look_dark_moody` | 3.29 / 3.8 (+1.5) | 2.89 / 3.1 (+1.3) |  |
| Toaster | fx002 | `jb.l4.look_toasted` | 4.68 / 5.3 (+2.9) | 3.80 / 4.1 (+2.2) |  |
| Valencia | fx002 | `jb.l4.look_soft_warm_fade` | 2.79 / 4.1 (+1.0) | 1.96 / 2.1 (+0.4) |  |
| Vivid | fx002 | `jb.l4.look_vivid_warm` | 5.93 / 6.6 (+4.2) | 4.15 / 4.3 (+2.6) | LUT + field; residual light-map detail finer than 16×9 |
| Walden | fx002 | `jb.l4.look_pale_cyan` | 3.67 / 4.7 (+1.9) | 2.53 / 2.7 (+0.9) |  |
| Wash | fx002 | `jb.l4.look_bright_wash` | 14.69 / 14.8 (+12.9) | 10.89 / 11.4 (+9.3) |  |
| XproII | fx002 | `jb.l4.look_cross_process` | 4.15 / 5.0 (+2.4) | 2.97 / 3.1 (+1.4) |  |
| Radial Blur | fx002 | `jb.l4.cam_radial_blur` | 9.53 / 54.9 (+7.8) | 4.46 / 9.0 (+2.9) |  |
| B&W Style | fx002 | `jb.l4.look_mono_bright` | 1.66 / 2.9 (+0.0) | 1.51 / 2.1 (+0.0) |  |
| Sunny Days | fx002 | `jb.l4.look_clear_day` | 2.29 / 3.6 (+0.5) | 1.76 / 3.0 (+0.2) |  |
| Chromatic Aberration | fx002 | — | — | — | not implemented |
| Mosaic | fx002 | `jb.l4.region_smear` | 2.84 / 4.9 (+1.1) | 2.12 / 2.2 (+0.5) |  |
| Tilt-shift Circle | fx002 | `jb.l4.cam_focus_ring` | 7.66 / 7.7 (+5.9) | 3.38 / 3.6 (+1.8) |  |
| Canvas | fx003 | `jb.l4.look_canvas_tone` | 11.92 / 12.0 (+10.2) | 11.95 / 12.0 (+10.4) |  |
| Fast Light Leaks 2 | fx003 | — | — | — | not implemented |
| Lut Process | fx003 | `jb.l4.look_dark_muted` | 1.57 / 2.5 (+0.0) | 1.23 / 1.5 (+0.0) |  |
| Flashlight | fx003 | `jb.l4.look_spotlight` | 20.09 / 32.6 (+18.3) | 17.52 / 29.3 (+15.9) |  |
| Camera Focus Bokeh | fx003 | — | — | — | not implemented |
| Drop Spread | fx003 | — | — | — | not implemented |
| Movie Light Effects 1 | fx003 | — | — | — | not implemented |
| Movie Light Effects 2 | fx003 | — | — | — | not implemented |
| Movie Light Effects 4 | fx003 | — | — | — | not implemented |
| Movie Light Effects 5 | fx003 | — | — | — | not implemented |
| Movie Light Effects 6 | fx003 | — | — | — | not implemented |
| Movie Light Effects 7 | fx003 | — | — | — | not implemented |
| Movie Light Effects 8 | fx003 | — | — | — | not implemented |
| Color Flash | fx003 | `jb.l4.cam_color_pulse` | 11.49 / 17.3 (+9.7) | 7.70 / 18.7 (+6.1) |  |
| Elasticity Jitter | fx003 | `jb.l4.cam_elastic_jitter` | 11.11 / 16.0 (+9.3) | 4.49 / 6.8 (+2.9) |  |
| Flicker Jitter 03 | fx003 | `jb.l4.cam_flicker_jolt` | 12.07 / 19.2 (+10.3) | 6.76 / 13.7 (+5.2) |  |
| Glow Vibrate | fx003 | `jb.l4.cam_glow_buzz` | 14.71 / 29.4 (+12.9) | 12.94 / 26.2 (+11.3) |  |
| Pulse Jitter | fx003 | `jb.l4.cam_pulse_bounce` | 13.42 / 24.9 (+11.7) | 7.55 / 18.2 (+6.0) |  |
| Shake the Camera | fx003 | `jb.l4.cam_impact_shake` | 20.19 / 32.7 (+18.4) | 10.39 / 25.2 (+8.8) |  |
| Glitch Effect 2 | fx003 | `jb.l4.look_mono_signal` | 14.71 / 27.2 (+12.9) | 6.15 / 14.0 (+4.6) |  |
| Local Color | fx003 | `jb.l4.spot_color_jump` | 1.81 / 3.8 (+0.0) | 1.24 / 1.5 (+0.0) | per-segment rectangles and tints measured exactly |
| No Signal Glitch 01 | fx003 | — | — | — | not implemented |
| Psychedelic Glitch | fx003 | — | — | — | not implemented |
| Stripe Glitch 01 | fx003 | — | — | — | not implemented |
| Strong Sharpening | fx003 | — | — | — | not implemented |
| Highlight Glow | fx003 | `jb.l4.look_highlight_lift` | 3.75 / 4.8 (+2.0) | 4.57 / 5.3 (+3.0) | approximation: bloom not modelled |
| Old Movie 2 | fx003 | `jb.l4.look_old_movie_tone` | 13.08 / 16.6 (+11.3) | 4.47 / 5.8 (+2.9) |  |
| Old TV Border | fx003 | `jb.l4.look_old_tv_border` | 4.12 / 4.6 (+2.4) | 3.21 / 4.3 (+1.6) | approximation: border shape finer than 16×9 field |
| Edge Glow 01 | fx003 | — | — | — | not implemented |
| Scanline Effect 1 | fx003 | `jb.l4.look_scanlines` | 4.01 / 4.8 (+2.2) | 2.69 / 4.7 (+1.1) |  |
| Chaos Shake | fx003 | `jb.l4.cam_rgb_jolt` | 9.27 / 16.8 (+7.5) | 3.22 / 6.8 (+1.6) |  |
| Collision | fx003 | `jb.l4.cam_impact_flash` | 7.25 / 23.2 (+5.5) | 4.40 / 22.6 (+2.8) |  |
| Crash Into | fx003 | `jb.l4.cam_crash_zoom` | 7.26 / 25.6 (+5.5) | 5.12 / 38.7 (+3.5) |  |
| Earthquake Shake | fx003 | `jb.l4.cam_rumble` | 6.38 / 13.5 (+4.6) | 2.14 / 4.4 (+0.6) |  |
| Extreme | fx003 | `jb.l4.cam_hard_shake` | 10.73 / 15.5 (+9.0) | 3.18 / 5.3 (+1.6) |  |
| Flash Exposure | fx003 | `jb.l4.cam_flash_pop` | 5.19 / 21.0 (+3.4) | 3.41 / 21.8 (+1.8) |  |
| Ripple Shake 02 | fx003 | — | — | — | not implemented |
| Up-Down 1 | fx003 | `jb.l4.cam_vertical_whip` | 9.43 / 17.1 (+7.7) | 4.60 / 16.5 (+3.0) |  |
| Shattered 02 | fx003 | — | — | — | not implemented |
| Shattered 03 | fx003 | — | — | — | not implemented |
| Exposure Zoom | fx003 | `jb.l4.cam_bright_zoom` | 18.69 / 27.0 (+16.9) | 9.62 / 17.3 (+8.0) |  |
| Fast Zoom | fx003 | `jb.l4.cam_zoom_burst` | 5.11 / 12.2 (+3.3) | 2.32 / 4.3 (+0.7) |  |
| Heart Zoom | fx003 | — | — | — | not implemented |
| Parallax Smooth Zoom In | fx003 | `jb.l4.cam_slow_push` | 3.93 / 5.0 (+2.2) | 1.75 / 2.0 (+0.2) |  |
| Rabid Camera 1 | fx003 | `jb.l4.cam_frantic` | 6.67 / 9.3 (+4.9) | 4.69 / 5.8 (+3.1) |  |
| Zoom Lens | fx003 | `jb.l4.cam_lens_punch` | 4.37 / 5.4 (+2.6) | 1.95 / 2.2 (+0.4) |  |
| Aesthetic Light Leak 04 16-9 | fxfix1 | — | — | — | not implemented |
| CALIFORNIA VINTAGE FILM LOOK | fxfix1 | `jb.l4.look_sunbaked_vintage` | 10.28 / 17.5 (+8.5) | 9.21 / 15.9 (+7.6) |  |
| Vintage Film Pack GRUNGE FILM VINTAGE | fxfix1 | `jb.l4.look_grunge_vintage` | 17.03 / 24.2 (+15.3) | 14.80 / 21.2 (+13.2) |  |
| Four Seasons | fxfix1 | `jb.l4.look_season_tint` | 6.84 / 8.0 (+5.1) | 6.62 / 7.1 (+5.0) | approximation: overlay finer than the field |
| Glow | fxfix1 | `jb.l4.look_bright_glow` | 4.81 / 5.2 (+3.0) | 3.36 / 5.2 (+1.8) | approximation: bloom not modelled |
| Radio Explike FX | fxfix1 | `jb.l4.look_prism_rays` | 6.83 / 7.2 (+5.1) | 5.87 / 7.3 (+4.3) |  |
| TV Wall | fxfix1 | `jb.l4.video_wall_3x3` | 10.66 / 11.0 (+8.9) | 3.48 / 11.3 (+1.9) | grid: tiny downscaled text/checker, encoder-dominated |

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
| Orb Twist 1 | tr005 | `jb.l4.warp_twist_roll_1` | 18.66 / 26.8 (+15.1) | 43.4 |  |
| Orb Twist 3 | tr005 | `jb.l4.warp_twist_roll_2` | 18.91 / 36.1 (+15.3) | 46.7 |  |
| Orb Twist 4 | tr005 | — | — | — | not implemented |
| Liquid Bodies Pack Transition 03 | tr006 | `jb.l4.reveal_clean_sweep_down_1` | 6.28 / 28.1 (+2.7) | 14.9 |  |
| Neon Mosaic 03 | tr006 | `jb.l4.reveal_glow_bloom_1` | 16.11 / 27.5 (+12.5) | 21.3 |  |
| Neon Mosaic 04 | tr006 | `jb.l4.reveal_glow_bloom_2` | 17.23 / 31.1 (+13.7) | 23.6 |  |
| Neon Mosaic Down | tr006 | `jb.l4.reveal_glow_sweep_down` | 12.42 / 15.0 (+8.9) | 23.8 |  |
| Warp 3 | tr006 | `jb.l4.warp_zoom_burst_1` | 7.83 / 19.7 (+4.3) | 35.6 |  |
| Warp 6 | tr006 | `jb.l4.warp_zoom_burst_2` | 10.04 / 22.6 (+6.5) | 28.2 |  |
| Warp 7 | tr006 | `jb.l4.warp_zoom_burst_3` | 5.51 / 11.1 (+1.9) | 28.3 |  |
| 3D Warp Box Distortion Left | tr008 | — | — | — | not implemented |
| 3D Warp Box Distortion Right | tr008 | — | — | — | not implemented |
| 3D Warp Box Distortion Up | tr008 | — | — | — | not implemented |
| 3D Warp Box Left | tr008 | — | — | — | not implemented |
| 3D Warp Box Open Left | tr008 | — | — | — | not implemented |
| 3D Warp Box Open Right | tr008 | — | — | — | not implemented |
| 3D Warp Box Rebound | tr008 | — | — | — | not implemented |
| 3D Warp Box Right | tr008 | — | — | — | not implemented |
| Depth Mosaic | tr008 | — | — | — | not implemented |
| Cartoon Liquid Transition 04 | tr010 | `jb.l4.reveal_ink_sweep_down_1` | 18.14 / 49.0 (+14.6) | 69.0 | white matte as a flat cover band; shape procedural |
| Comic Smoke Transition 07 | tr010 | `jb.l4.reveal_ink_spread_down_left_1` | 22.31 / 65.9 (+18.8) | 41.9 | smoke matte as a flat cover band; shape procedural |
| Brush And Ink Transition 34 | tr010 | `jb.l4.reveal_clean_sweep_up_right` | 8.48 / 28.2 (+4.9) | 24.8 |  |
| Brush And Ink Transition 36 | tr010 | `jb.l4.reveal_ink_sweep_right_1` | 3.83 / 9.3 (+0.3) | 17.9 |  |
| Brush And Ink Transition 40 | tr010 | `jb.l4.reveal_clean_spread_down_left_1` | 4.99 / 15.4 (+1.4) | 24.4 |  |
| Brush Paint Pack Transition 09 | tr010 | `jb.l4.reveal_ink_sweep_right_2` | 3.23 / 4.4 (+0.0) | 7.8 |  |
| Colorful Ink Slideshow Vol 01 Transition 04 | tr010 | `jb.l4.reveal_clean_spread_right_1` | 26.17 / 57.7 (+22.6) | 49.3 | multicolor ink approximated by one cover color |
| Dynamic Ink 01 | tr010 | — | — | — | dropped: multicolor splats, procedural reveal worse than a cut |
| Dynamic Ink 02 | tr011 | — | — | — | dropped: multicolor splats, procedural reveal worse than a cut |
| Dynamic Ink 03 | tr011 | `jb.l4.reveal_ink_sweep_down_2` | 6.85 / 13.4 (+3.3) | 18.8 |  |
| Dynamic Ink 05 | tr011 | `jb.l4.reveal_ink_spread_up_left_1` | 7.32 / 16.5 (+3.8) | 21.4 |  |
| Dynamic Ink 06 | tr011 | `jb.l4.reveal_clean_sweep_down_2` | 9.52 / 26.2 (+6.0) | 19.3 |  |
| Dynamic Ink 07 | tr011 | `jb.l4.reveal_ink_spread_down_right_1` | 19.87 / 29.8 (+16.3) | 25.7 |  |
| Dynamic Ink 08 | tr011 | `jb.l4.reveal_clean_spread_right_2` | 9.06 / 22.4 (+5.5) | 25.0 |  |
| Dynamic Ink 09 | tr011 | `jb.l4.reveal_ink_spread_down_left_2` | 16.58 / 31.2 (+13.0) | 30.7 |  |
| Dynamic Ink 10 | tr011 | `jb.l4.reveal_clean_spread_left_1` | 10.97 / 45.2 (+7.4) | 31.7 |  |
| Glow Ink Down | tr011 | `jb.l4.reveal_ink_bloom_1` | 14.90 / 35.9 (+11.3) | 21.8 |  |
| Glow Ink Left | tr011 | `jb.l4.reveal_clean_spread_left_2` | 15.95 / 30.6 (+12.4) | 22.3 |  |
| Glow Ink Right | tr011 | `jb.l4.reveal_ink_spread_right_1` | 8.71 / 17.4 (+5.2) | 17.8 |  |
| Glow Ink Up | tr011 | — | — | 10.7 | not implemented |
| Ink Brush Mask Transition | tr011 | `jb.l4.reveal_ink_spread_down_right_2` | 6.83 / 18.5 (+3.3) | 13.0 |  |
| Ink Reveal Transition 02 | tr011 | `jb.l4.reveal_clean_spread_up_right` | 6.71 / 14.5 (+3.1) | 12.8 |  |
| Liquid Spread Transition 02 | tr011 | `jb.l4.reveal_ink_spread_up_left_2` | 10.07 / 26.3 (+6.5) | 25.3 |  |
| Liquid Spread Transition 05 | tr011 | `jb.l4.reveal_clean_spread_right_3` | 7.83 / 14.1 (+4.3) | 19.2 |  |
| Liquid Spread Transition 06 | tr011 | `jb.l4.reveal_ink_bloom_2` | 6.95 / 14.4 (+3.4) | 21.7 |  |
| Liquid Spread Transition 09 | tr011 | `jb.l4.reveal_ink_spread_up_right_1` | 5.27 / 18.8 (+1.7) | 25.4 |  |
| Liquid Spread Transition 10 | tr011 | `jb.l4.reveal_clean_spread_down_right_1` | 7.17 / 11.1 (+3.6) | 18.8 |  |
| Luminous Ink Wash 01 | tr011 | `jb.l4.reveal_ink_bloom_3` | 11.37 / 29.6 (+7.8) | 16.0 |  |
| Luminous Ink Wash 02 | tr011 | `jb.l4.reveal_ink_bloom_4` | 6.18 / 12.9 (+2.6) | 14.2 |  |
| Luminous Ink Wash 03 | tr011 | `jb.l4.reveal_ink_sweep_up_left_2` | 16.67 / 44.3 (+13.1) | 19.3 |  |
| Luminous Ink Wash 04 | tr011 | — | — | — | dropped: glow wash, procedural reveal worse than a cut |
| Luminous Ink Wash 05 | tr011 | `jb.l4.reveal_ink_spread_up_right_2` | 19.83 / 48.5 (+16.3) | 27.5 |  |
| Luminous Ink Wash 06 | tr011 | `jb.l4.reveal_ink_spread_up_right_3` | 18.68 / 34.8 (+15.1) | 22.6 |  |
| Luminous Ink Wash 07 | tr011 | `jb.l4.reveal_clean_spread_left_3` | 19.86 / 42.2 (+16.3) | 23.5 |  |
| Luminous Ink Wash 08 | tr012 | `jb.l4.reveal_ink_spread_down_1` | 9.57 / 27.9 (+6.0) | 29.1 |  |
| Luminous Ink Wash 09 | tr012 | `jb.l4.reveal_ink_spread_down_2` | 10.57 / 20.4 (+7.0) | 19.3 |  |
| Luminous Ink Wash 10 | tr012 | `jb.l4.reveal_ink_sweep_right_3` | 19.92 / 53.0 (+16.4) | 45.8 |  |
| Moonlight Ink Transition 01 | tr012 | `jb.l4.reveal_ink_spread_down_left_3` | 4.23 / 10.6 (+0.7) | 22.1 |  |
| Moonlight Ink Transition 02 | tr012 | `jb.l4.reveal_ink_spread_up_right_4` | 4.98 / 9.3 (+1.4) | 16.9 |  |
| Moonlight Ink Transition 03 | tr012 | `jb.l4.reveal_clean_spread_down` | 4.44 / 11.2 (+0.9) | 19.0 |  |
| Moonlight Ink Transition 04 | tr012 | `jb.l4.reveal_ink_sweep_left` | 5.51 / 8.3 (+1.9) | 16.6 |  |
| Moonlight Ink Transition 05 | tr012 | `jb.l4.reveal_clean_spread_down_right_2` | 3.83 / 5.9 (+0.3) | 17.0 |  |
| Blink Eyes 03 | tr012 | `jb.l4.reveal_ink_sweep_right_4` | 13.43 / 83.0 (+9.9) | 15.5 | black lids not modelled (reveal only) |
| Mosaic | trfix1 | — | — | — | not implemented: random block glitches, a pixelate + mix fit stays at MAD 20–58 |

## Summary

* **Color looks** (24 filters, LUT ± light map): scene windows 1.2–4.2
  (excess over floor 0–2.6), grid windows 1.6–6.3. At target (< 3 mean)
  on both clips: Lut Process, B&W Style, InkWell, Sunny Days, BlueSkies,
  Valencia, Brannan, SunSet, 1977 (scene), Sierra (scene), Sutro (scene).
  Light-map looks with detail finer than the 48×27 field (Vivid, Sunrise,
  Toaster, Hudson, Amaro, Earlybird) stay at 3.3–4.2 on the scene.
* **Camera effects** (shakes, zoom punches, flashes; 21 items): smooth
  zooms reach the floor (Parallax push-in 1.75, Zoom Lens 1.95, Fast Zoom
  2.32 on the scene); shakes are 2.1–5 on the scene but 6–12 on the grid
  (see below); flash-heavy items keep isolated frames at 20–40 where a
  bloom is not modelled.
* **Transitions**: 43 procedural reveals match the measured coverage
  frame by frame; clean masks reach 3.2–5.5 (Moonlight Ink, Brush Paint,
  Brush And Ink 36/40, Liquid Spread 09), noisy or colored mattes 6–26.
  Five zoom-warp / twist transitions at 5.5–19 against hard cuts of
  28–47.

## Remaining gaps

* **Looks with textures.** Film looks with grain, scratches or paper
  textures (Canvas, grunge / vintage film, old movie, pencil sketch,
  spotlight, wash) are reproduced as a LUT plus a smooth light map only;
  the texture is not recreated, so they stay at MAD 4.5–20.
* **Rotating shakes on the grid** (Mild, Extreme, RGB jolt): 7.7–10.7 on
  the grid against 2.7–3.2 on the scene. A per-frame affine, lens and
  perspective refit at 960×540 did not reduce it; the rows were fitted at
  192–640 px, which is not precise enough for a 10 px checker.
* **Procedural reveals.** The coverage of every frame matches; the shape of
  the edge is our own fbm, so masks differ in detail. Mattes with several
  ink colors (Dynamic Ink 01/02, Luminous Ink Wash 04) and a few others
  (Glow Ink Up, Ripple, Wave, Orb Twist 4, Swirl) came out no better than a
  hard cut and are not mapped.
* **Not implemented** (no alias, the Filmora GUID renders as before):
  - 3D Warp Box family and Depth Mosaic: curved 3D surfaces, not a
    corner pin.
  - Light-leak, film-burn and particle overlays (Movie Light Effects,
    Fast Light Leaks, Aesthetic Light Leak, Camera Focus Bokeh, Heart
    Zoom): Filmora composites artwork; recreating it procedurally was out
    of reach in this pass.
  - Random glitches (Glitch line, Stripe / No Signal glitch, Pixel
    Sorting, Pixel Game, Kaleidoscope, Butterfly Wave, Shattered, Mosaic
    transition, Edge Glow, Ripple Shake, Chromatic Aberration, Strong
    Sharpening): a seeded procedural version would match timing only, and
    the fitted approximations measured worse than leaving the clip as is.
  - Money (engraving), Baltan (animated softness plus fade-in).


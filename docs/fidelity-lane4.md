# Render fidelity, lane 4: glitch / warp / creative transitions, effects and filters

Lane 4 of the fidelity work against Filmora 15.7.3 covers 71 glitch,
distortion, warp, ink and creative transitions and 87 video effects,
filters and overlays from the reference batches
(`docs/filmora-references.md`). **98 of 158** items have an
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
| Exposure | fx001 | `jb.l4.look_overexposed` | 8.47 / 8.7 | 5.16 / 5.6 | approximation: soft bloom not modelled |
| Glitch Effect 4 | fx001 | — | — | — | not implemented |
| 1977 | fx001 | `jb.l4.look_rose_fade` | 5.43 / 5.8 | 2.31 / 6.2 |  |
| Amaro | fx001 | `jb.l4.look_glow_matte` | 6.03 / 6.4 | 3.86 / 6.7 | LUT + field; residual light-map detail finer than 16×9 |
| Baltan | fx001 | — | — | — | not implemented |
| BlueSkies | fx001 | `jb.l4.look_blue_sky` | 5.00 / 5.2 | 2.63 / 2.9 |  |
| Brannan | fx001 | `jb.l4.look_warm_contrast_fade` | 5.32 / 5.5 | 2.96 / 3.4 |  |
| Bright Lights | fx001 | `jb.l4.look_airy_blue_lift` | 5.66 / 5.7 | 3.17 / 3.5 |  |
| Classic | fx001 | — | — | — | not implemented |
| Earlybird | fx001 | `jb.l4.look_golden_morning` | 5.24 / 5.5 | 3.53 / 3.9 |  |
| Hefe | fx001 | `jb.l4.look_warm_punch` | 6.82 / 6.9 | 3.56 / 7.3 |  |
| Hudson | fx001 | `jb.l4.look_icy_vignette` | 5.85 / 6.0 | 3.92 / 6.4 |  |
| InkWell | fx001 | `jb.l4.look_mono_classic` | 4.58 / 5.1 | 2.71 / 4.9 |  |
| Lomo | fx001 | `jb.l4.look_contrast_vignette` | 6.26 / 6.4 | 3.34 / 6.9 |  |
| LordKelvin | fx001 | `jb.l4.look_hot_amber` | 7.04 / 7.2 | 3.18 / 7.9 |  |
| Mild | fx001 | `jb.l4.cam_gentle_sway` | 8.95 / 10.8 | 3.66 / 13.4 |  |
| Money | fx001 | — | — | — | not implemented |
| Nashville | fx001 | `jb.l4.look_pink_pastel` | 6.56 / 7.4 | 2.92 / 3.3 |  |
| Sierra | fx002 | `jb.l4.look_faded_vignette` | 5.02 / 5.4 | 3.45 / 5.7 |  |
| SunSet | fx002 | `jb.l4.look_dusk_fade` | 5.11 / 5.3 | 3.02 / 3.3 |  |
| Sunrise | fx002 | `jb.l4.look_warm_haze` | 6.74 / 7.0 | 3.53 / 3.9 |  |
| Sutro | fx002 | `jb.l4.look_dark_moody` | 5.13 / 5.3 | 3.56 / 4.0 |  |
| Toaster | fx002 | `jb.l4.look_toasted` | 6.07 / 6.2 | 3.93 / 4.4 |  |
| Valencia | fx002 | `jb.l4.look_soft_warm_fade` | 4.94 / 5.2 | 2.44 / 2.8 |  |
| Vivid | fx002 | `jb.l4.look_vivid_warm` | 9.04 / 9.2 | 4.64 / 5.2 | LUT + field; residual light-map detail finer than 16×9 |
| Walden | fx002 | `jb.l4.look_pale_cyan` | 5.91 / 6.0 | 3.19 / 3.6 |  |
| Wash | fx002 | — | — | — | not implemented |
| XproII | fx002 | `jb.l4.look_cross_process` | 6.83 / 6.9 | 3.62 / 4.0 |  |
| Radial Blur | fx002 | — | — | — | not implemented |
| B&W Style | fx002 | `jb.l4.look_mono_bright` | 3.83 / 4.3 | 2.55 / 4.3 |  |
| Sunny Days | fx002 | `jb.l4.look_clear_day` | 4.76 / 5.0 | 2.43 / 5.4 |  |
| Chromatic Aberration | fx002 | — | — | — | not implemented |
| Mosaic | fx002 | `jb.l4.region_smear` | 4.85 / 5.5 | 2.38 / 2.5 | not implemented: random block glitches, a pixelate + mix fit stays at MAD 20–58 |
| Tilt-shift Circle | fx002 | — | — | — | not implemented |
| Canvas | fx003 | `jb.l4.look_canvas_tone` | 11.81 / 11.9 | 11.94 / 12.0 |  |
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
| Color Flash | fx003 | `jb.l4.cam_color_pulse` | 11.30 / 16.4 | 7.93 / 19.1 |  |
| Elasticity Jitter | fx003 | `jb.l4.cam_elastic_jitter` | 11.13 / 15.3 | 4.70 / 6.8 |  |
| Flicker Jitter 03 | fx003 | — | — | — | not implemented |
| Glow Vibrate | fx003 | — | — | — | not implemented |
| Pulse Jitter | fx003 | — | — | — | not implemented |
| Shake the Camera | fx003 | `jb.l4.cam_impact_shake` | 19.72 / 32.4 | 11.20 / 26.0 |  |
| Glitch Effect 2 | fx003 | — | — | — | not implemented |
| Local Color | fx003 | `jb.l4.spot_color_jump` | 3.17 / 4.0 | 1.84 / 2.1 | per-segment rectangles and tints measured exactly |
| No Signal Glitch 01 | fx003 | — | — | — | not implemented |
| Psychedelic Glitch | fx003 | — | — | — | not implemented |
| Stripe Glitch 01 | fx003 | — | — | — | not implemented |
| Strong Sharpening | fx003 | — | — | — | not implemented |
| Highlight Glow | fx003 | `jb.l4.look_highlight_lift` | 6.38 / 6.5 | 5.25 / 5.9 | approximation: bloom not modelled |
| Old Movie 2 | fx003 | — | — | — | not implemented |
| Old TV Border | fx003 | `jb.l4.look_old_tv_border` | 4.49 / 5.3 | 3.49 / 4.7 | approximation: border shape finer than 16×9 field |
| Edge Glow 01 | fx003 | — | — | — | not implemented |
| Scanline Effect 1 | fx003 | `jb.l4.look_scanlines` | 6.13 / 6.2 | 3.02 / 6.9 |  |
| Chaos Shake | fx003 | `jb.l4.cam_rgb_jolt` | 9.67 / 16.5 | 3.68 / 6.4 |  |
| Collision | fx003 | `jb.l4.cam_impact_flash` | 8.24 / 23.0 | 4.91 / 23.0 |  |
| Crash Into | fx003 | `jb.l4.cam_crash_zoom` | 8.00 / 24.6 | 5.53 / 38.5 |  |
| Earthquake Shake | fx003 | `jb.l4.cam_rumble` | 6.34 / 12.8 | 2.59 / 4.5 |  |
| Extreme | fx003 | `jb.l4.cam_hard_shake` | 10.14 / 14.6 | 3.43 / 5.4 |  |
| Flash Exposure | fx003 | `jb.l4.cam_flash_pop` | 6.45 / 21.4 | 3.92 / 21.8 |  |
| Ripple Shake 02 | fx003 | — | — | — | not implemented |
| Up-Down 1 | fx003 | `jb.l4.cam_vertical_whip` | 9.90 / 16.8 | 5.12 / 15.7 |  |
| Shattered 02 | fx003 | — | — | — | not implemented |
| Shattered 03 | fx003 | — | — | — | not implemented |
| Exposure Zoom | fx003 | `jb.l4.cam_bright_zoom` | 18.22 / 26.3 | 10.51 / 18.0 |  |
| Fast Zoom | fx003 | `jb.l4.cam_zoom_burst` | 5.26 / 11.5 | 2.75 / 4.4 |  |
| Heart Zoom | fx003 | — | — | — | not implemented |
| Parallax Smooth Zoom In | fx003 | `jb.l4.cam_slow_push` | 5.09 / 5.7 | 2.46 / 2.7 |  |
| Rabid Camera 1 | fx003 | — | — | — | not implemented |
| Zoom Lens | fx003 | `jb.l4.cam_lens_punch` | 5.43 / 6.1 | 2.55 / 2.7 |  |
| Aesthetic Light Leak 04 16-9 | fxfix1 | — | — | — | not implemented |
| CALIFORNIA VINTAGE FILM LOOK | fxfix1 | `jb.l4.look_sunbaked_vintage` | 12.22 / 17.7 | 9.80 / 14.9 |  |
| Vintage Film Pack GRUNGE FILM VINTAGE | fxfix1 | `jb.l4.look_grunge_vintage` | 17.92 / 25.0 | 15.02 / 21.7 |  |
| Four Seasons | fxfix1 | `jb.l4.look_season_tint` | 8.35 / 8.7 | 6.86 / 7.5 | approximation: overlay finer than the field |
| Glow | fxfix1 | `jb.l4.look_bright_glow` | 7.14 / 7.4 | 3.55 / 7.6 | approximation: bloom not modelled |
| Radio Explike FX | fxfix1 | `jb.l4.look_prism_rays` | 8.83 / 9.2 | 6.26 / 9.4 |  |
| TV Wall | fxfix1 | `jb.l4.video_wall_3x3` | 12.24 / 37.7 | 3.83 / 12.3 | grid: tiny downscaled text/checker, encoder-dominated |

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
| Orb Twist 1 | tr005 | `jb.l4.warp_twist_roll_1` | 18.57 / 26.9 |  |
| Orb Twist 3 | tr005 | `jb.l4.warp_twist_roll_2` | 18.86 / 35.8 |  |
| Orb Twist 4 | tr005 | — | — | not implemented |
| Liquid Bodies Pack Transition 03 | tr006 | `jb.l4.reveal_clean_sweep_down_1` | 8.61 / 29.7 |  |
| Neon Mosaic 03 | tr006 | — | — | not implemented |
| Neon Mosaic 04 | tr006 | — | — | not implemented |
| Neon Mosaic Down | tr006 | — | — | not implemented |
| Warp 3 | tr006 | `jb.l4.warp_zoom_burst_1` | 7.62 / 19.7 |  |
| Warp 6 | tr006 | `jb.l4.warp_zoom_burst_2` | 9.94 / 22.2 |  |
| Warp 7 | tr006 | `jb.l4.warp_zoom_burst_3` | 5.29 / 11.1 |  |
| 3D Warp Box Distortion Left | tr008 | — | — | not implemented |
| 3D Warp Box Distortion Right | tr008 | — | — | not implemented |
| 3D Warp Box Distortion Up | tr008 | — | — | not implemented |
| 3D Warp Box Left | tr008 | — | — | not implemented |
| 3D Warp Box Open Left | tr008 | — | — | not implemented |
| 3D Warp Box Open Right | tr008 | — | — | not implemented |
| 3D Warp Box Rebound | tr008 | — | — | not implemented |
| 3D Warp Box Right | tr008 | — | — | not implemented |
| Depth Mosaic | tr008 | — | — | not implemented |
| Cartoon Liquid Transition 04 | tr010 | `jb.l4.reveal_ink_sweep_down_1` | 18.87 / 49.1 | white matte as a flat cover band; shape procedural |
| Comic Smoke Transition 07 | tr010 | `jb.l4.reveal_ink_spread_down_left_1` | 22.87 / 65.7 | smoke matte as a flat cover band; shape procedural |
| Brush And Ink Transition 34 | tr010 | `jb.l4.reveal_clean_sweep_up_right` | 10.34 / 29.1 |  |
| Brush And Ink Transition 36 | tr010 | `jb.l4.reveal_ink_sweep_right_1` | 6.08 / 11.1 |  |
| Brush And Ink Transition 40 | tr010 | `jb.l4.reveal_clean_spread_down_left_1` | 7.42 / 17.3 |  |
| Brush Paint Pack Transition 09 | tr010 | `jb.l4.reveal_ink_sweep_right_2` | 5.68 / 7.0 |  |
| Colorful Ink Slideshow Vol 01 Transition 04 | tr010 | `jb.l4.reveal_clean_spread_right_1` | 26.75 / 57.5 | multicolor ink approximated by one cover color |
| Dynamic Ink 01 | tr010 | — | — | dropped: multicolor splats, procedural reveal worse than a cut |
| Dynamic Ink 02 | tr011 | — | — | dropped: multicolor splats, procedural reveal worse than a cut |
| Dynamic Ink 03 | tr011 | `jb.l4.reveal_ink_sweep_down_2` | 8.53 / 14.4 |  |
| Dynamic Ink 05 | tr011 | `jb.l4.reveal_ink_spread_up_left_1` | 9.41 / 18.0 |  |
| Dynamic Ink 06 | tr011 | `jb.l4.reveal_clean_sweep_down_2` | 11.43 / 27.5 |  |
| Dynamic Ink 07 | tr011 | `jb.l4.reveal_ink_spread_down_right_1` | 21.59 / 31.1 |  |
| Dynamic Ink 08 | tr011 | `jb.l4.reveal_clean_spread_right_2` | 11.09 / 24.1 |  |
| Dynamic Ink 09 | tr011 | `jb.l4.reveal_ink_spread_down_left_2` | 18.42 / 32.6 |  |
| Dynamic Ink 10 | tr011 | `jb.l4.reveal_clean_spread_left_1` | 12.87 / 46.1 |  |
| Glow Ink Down | tr011 | `jb.l4.reveal_ink_bloom_1` | 16.52 / 36.7 |  |
| Glow Ink Left | tr011 | `jb.l4.reveal_clean_spread_left_2` | 17.26 / 31.3 |  |
| Glow Ink Right | tr011 | `jb.l4.reveal_ink_spread_right_1` | 10.65 / 19.0 |  |
| Glow Ink Up | tr011 | `jb.l4.reveal_ink_sweep_up_left_1` | 11.28 / 26.3 |  |
| Ink Brush Mask Transition | tr011 | `jb.l4.reveal_ink_spread_down_right_2` | 8.90 / 19.9 |  |
| Ink Reveal Transition 02 | tr011 | `jb.l4.reveal_clean_spread_up_right` | 8.50 / 15.8 |  |
| Liquid Spread Transition 02 | tr011 | `jb.l4.reveal_ink_spread_up_left_2` | 11.96 / 27.6 |  |
| Liquid Spread Transition 05 | tr011 | `jb.l4.reveal_clean_spread_right_3` | 9.78 / 16.2 |  |
| Liquid Spread Transition 06 | tr011 | `jb.l4.reveal_ink_bloom_2` | 9.17 / 16.3 |  |
| Liquid Spread Transition 09 | tr011 | `jb.l4.reveal_ink_spread_up_right_1` | 7.46 / 20.5 |  |
| Liquid Spread Transition 10 | tr011 | `jb.l4.reveal_clean_spread_down_right_1` | 9.37 / 13.2 |  |
| Luminous Ink Wash 01 | tr011 | `jb.l4.reveal_ink_bloom_3` | 13.02 / 30.9 |  |
| Luminous Ink Wash 02 | tr011 | `jb.l4.reveal_ink_bloom_4` | 8.15 / 14.3 |  |
| Luminous Ink Wash 03 | tr011 | `jb.l4.reveal_ink_sweep_up_left_2` | 18.51 / 45.3 |  |
| Luminous Ink Wash 04 | tr011 | — | — | dropped: glow wash, procedural reveal worse than a cut |
| Luminous Ink Wash 05 | tr011 | `jb.l4.reveal_ink_spread_up_right_2` | 21.52 / 49.5 |  |
| Luminous Ink Wash 06 | tr011 | `jb.l4.reveal_ink_spread_up_right_3` | 20.58 / 36.3 |  |
| Luminous Ink Wash 07 | tr011 | `jb.l4.reveal_clean_spread_left_3` | 21.59 / 43.2 |  |
| Luminous Ink Wash 08 | tr012 | `jb.l4.reveal_ink_spread_down_1` | 11.52 / 28.9 |  |
| Luminous Ink Wash 09 | tr012 | `jb.l4.reveal_ink_spread_down_2` | 12.28 / 22.1 |  |
| Luminous Ink Wash 10 | tr012 | `jb.l4.reveal_ink_sweep_right_3` | 21.05 / 53.6 |  |
| Moonlight Ink Transition 01 | tr012 | `jb.l4.reveal_ink_spread_down_left_3` | 6.34 / 12.4 |  |
| Moonlight Ink Transition 02 | tr012 | `jb.l4.reveal_ink_spread_up_right_4` | 7.27 / 11.3 |  |
| Moonlight Ink Transition 03 | tr012 | `jb.l4.reveal_clean_spread_down` | 6.68 / 13.1 |  |
| Moonlight Ink Transition 04 | tr012 | `jb.l4.reveal_ink_sweep_left` | 7.90 / 10.6 |  |
| Moonlight Ink Transition 05 | tr012 | `jb.l4.reveal_clean_spread_down_right_2` | 6.17 / 8.2 |  |
| Blink Eyes 03 | tr012 | `jb.l4.reveal_ink_sweep_right_4` | 13.60 / 81.6 | black lids not modelled (reveal only) |
| Mosaic | trfix1 | — | — | not implemented: random block glitches, a pixelate + mix fit stays at MAD 20–58 |

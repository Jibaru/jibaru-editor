# Render fidelity: lane 3 transitions

Shape and luma wipes, dissolves and fades, light/flash/leak and themed
"overlay" transitions of Filmora 15.7.3, fitted on Filmora's own exports of
the reference batches (`docs/filmora-references.md`: one transition per cut
between the gridA/gridB test cards, 2.002 s centred on the cut). Code:
`crates/jb-render/src/fx/lane3*.rs`, GUIDs in `crates/jb-render/src/aliases_lane3.rs`.

## Method

Each window was decoded at 192×108–384×216, with the last clean frame
before it as A and the first clean frame after it as B (the cards are
stills, so these are the exact inputs up to the moving marker bar, which
is excluded) and the output level offset removed. A per-frame model was
fitted to every frame and the per-frame parameters reduced to keyframe
tables (Ramer–Douglas–Peucker, tolerance ≈ 1 % of the range):

- **Global mix** (`kit::global_mix`): `out = aA·A + aL·luma(A) + bB·B + bL·luma(B) + o`
  per frame in display space (least squares). Exact for dissolves, dips to a
  colour and grey fades; for the themed items whose artwork is not
  reproduced it is the stand-in: it reproduces when the picture switches,
  the fades and the average colour and brightness of the artwork, not its
  shape (these are marked *stand-in* below).
- **Edge / ramp wipes** (`kit::edge_wipe`, `kit::ramp_wipe`): B where a
  parametric mask (line, centre line, diagonal, ...) is below a threshold,
  with an edge width; threshold and width per frame.
- **Flash** (`kit::FlashFx`, two passes): the current side zoomed by `s`
  and lit as `g·Z + h·blur(Z, σ) + o` (clipped), per frame.

Nothing from Filmora's installed assets was read; light, flares and washes
are fitted from the exported output only and rendered procedurally.

## Measurements

The harness (`crates/jb-cli/examples/fidelity.rs`, `--size 960x540
--search 0`) renders the batch project's window with Jibaru and compares it
with Filmora's export. **Floor** is the same measurement on the clean
frames just before and after the window (mean of both): on these test
cards at the reference's low bitrate (6 Mbit/s) the fine checkerboard and
grid lines are smeared by the encoder, so even untouched frames measure
4–6, well above the 1.1–1.6 of real footage in `docs/fidelity.md`.
**Excess** = MAD mean − floor is what the model leaves; ≲ 1.5 is at the
floor. Stand-ins score far above it wherever the artwork covers the frame.

| Filmora item | Ours | Model | MAD mean | MAD max | Floor | Excess | Notes |
|---|---|---|---:|---:|---:|---:|---|
| Algorithm Combination Set 06 | `jb.warm_flare_flash` | global mix stand-in | 11.84 | 26.56 | 4.81 | +7.03 |  |
| Ancient Civilization Egypt Pack Transition 04 | `jb.golden_dust_sweep` | global mix stand-in | 8.94 | 18.94 | 4.91 | +4.03 |  |
| Animate Effect Transition 03 | `jb.ink_splash_burst` | global mix stand-in | 7.62 | 48.67 | 4.58 | +3.04 |  |
| Anime Attack 03 | `jb.energy_ring_burst` | global mix stand-in | 22.67 | 75.09 | 4.78 | +17.90 |  |
| Anime Flame Transition 09 | `jb.flame_wash` | global mix stand-in | 16.88 | 34.37 | 4.78 | +12.10 |  |
| Anime Style Flow 01 | `jb.starburst_flash` | global mix stand-in | 15.31 | 47.69 | 4.79 | +10.52 |  |
| B&W Dissolve | `jb.mono_dissolve` | global mix stand-in | 9.72 | 26.83 | 5.12 | +4.61 |  |
| Black Friday Sale 02 Transition 06 | `jb.chevron_sweep` | global mix stand-in | 17.80 | 59.61 | 4.83 | +12.96 |  |
| Blind 1 | `jb.blind_bars` | edge front + procedural slats | 10.39 | 19.31 | 4.99 | +5.40 | slat band with hashed per-slat thresholds; slat pattern is not Filmora's |
| Blue Sparkling Awards Transition 04 | `jb.gold_sparkle_burst` | global mix stand-in | 22.56 | 35.85 | 4.97 | +17.59 |  |
| Burning Dash Transition 01 | `jb.fireball_dash_1` | global mix stand-in | 25.98 | 43.46 | 4.82 | +21.16 |  |
| Burning Dash Transition 02 | `jb.fireball_dash_2` | global mix stand-in | 19.98 | 46.00 | 4.78 | +15.21 |  |
| Burning Dash Transition 03 | `jb.fireball_dash_3` | global mix stand-in | 24.08 | 48.58 | 4.71 | +19.38 |  |
| Burning Dash Transition 04 | `jb.fireball_dash_4` | global mix stand-in | 15.51 | 47.17 | 4.71 | +10.80 |  |
| Business Presentation Vol 02 Transition 02 | `jb.stripe_slide_diagonal` | global mix stand-in | 31.97 | 84.82 | 4.79 | +27.18 |  |
| Carousel Down | `jb.carousel_down` | global mix stand-in | 37.12 | 61.32 | 4.71 | +32.41 |  |
| Carousel Down Rebound | `jb.carousel_down_rebound` | global mix stand-in | 25.11 | 48.82 | 4.69 | +20.42 |  |
| Carousel Left | `jb.carousel_left` | panel strip | 17.38 | 46.89 | 4.88 | +12.50 | fitted at 192 px; frames 21–31 (fast scroll) still off |
| Carousel Right | `jb.carousel_right` | global mix stand-in | 34.99 | 65.66 | 4.98 | +30.01 |  |
| Carousel Up | `jb.carousel_up` | global mix stand-in | 26.59 | 49.35 | 4.89 | +21.70 |  |
| Cartoony Vlog Pack Transition 01 | `jb.candy_spiral` | global mix stand-in | 25.03 | 55.57 | 4.77 | +20.27 |  |
| Chalk Swipe Left | `jb.chalk_swipe_left` | edge wipe | 5.25 | 7.01 | 5.24 | +0.01 | slanted edge in ~12 fps steps; the chalk texture of the edge is not drawn |
| Christmas Gift Transition 03 | `jb.bokeh_flash` | global mix stand-in | 10.85 | 21.19 | 5.45 | +5.40 |  |
| Cinematic Transition 03 | `jb.soft_light_leak` | global mix stand-in | 9.55 | 21.38 | 4.64 | +4.91 |  |
| Cinematic Transition 09 | `jb.scratched_film_dip` | global mix stand-in | 7.93 | 19.90 | 4.88 | +3.04 |  |
| Cinematic Travel Vlog Transition 01 | `jb.film_strip_slide` | global mix stand-in | 28.22 | 43.13 | 4.90 | +23.32 |  |
| Classic Movies Transition 06 | `jb.red_flare_flash` | global mix stand-in | 9.74 | 26.30 | 4.83 | +4.91 |  |
| Col Merge | `jb.sides_merge` | edge wipe | 4.70 | 5.46 | 4.80 | -0.10 | two edges closing to the centre line |
| Color Dissolve 3 | `jb.block_glow_dissolve` | global mix stand-in | 18.28 | 51.78 | 5.04 | +13.24 |  |
| Color Distance | `jb.checker_scatter` | global mix stand-in | 9.78 | 28.52 | 5.07 | +4.71 |  |
| Colorful Transition Pack Transition 01 | `jb.liquid_blob_wipe` | global mix stand-in | 11.73 | 50.88 | 4.89 | +6.84 |  |
| Colorful Transitions Vol 06 Transition 16 | `jb.color_rings` | global mix stand-in | 47.83 | 69.98 | 4.80 | +43.03 |  |
| Comic Film Flash | `jb.ink_brush_sweep` | global mix stand-in | 21.79 | 53.22 | 4.71 | +17.09 |  |
| Comic Style 3 | `jb.comic_panels` | global mix stand-in | 40.53 | 51.20 | 5.21 | +35.32 |  |
| Cosmic Gallery | `jb.floating_gallery` | global mix stand-in | 52.52 | 84.30 | 5.28 | +47.24 |  |
| Creepy  Pack Transition 01 | `jb.creature_reveal` | global mix stand-in | 25.89 | 58.29 | 4.83 | +21.06 |  |
| Creepy Pack Transition 02 | `jb.face_slide` | global mix stand-in | 25.77 | 42.82 | 4.86 | +20.91 |  |
| Creepy Pack Transition 03 | `jb.lantern_sweep` | global mix stand-in | 36.01 | 64.97 | 4.85 | +31.16 |  |
| Cross Whisk | `jb.block_whisk` | global mix stand-in | 15.43 | 31.54 | 4.54 | +10.89 |  |
| Crystal Ball 1 | `jb.crystal_ball_iris` | lens ball | 7.37 | 13.62 | 4.78 | +2.59 | ellipse → circle iris with bulge and mid flash colours |
| Crystal Ball 2 | `jb.crystal_ball_lens` | global mix stand-in | 17.76 | 34.50 | 4.64 | +13.12 |  |
| Cut Off | `jb.cut_off` | global mix stand-in | 12.52 | 37.13 | 4.82 | +7.70 |  |
| Depth Slice | `jb.depth_panels` | global mix stand-in | 43.24 | 66.01 | 4.58 | +38.66 |  |
| Diagonal Slice | `jb.split_slide` | global mix stand-in | 12.27 | 36.47 | 5.10 | +7.17 |  |
| Dimensional Drop | `jb.panel_drop` | global mix stand-in | 27.21 | 54.58 | 4.87 | +22.34 |  |
| Dissolve Skew 01 | `jb.diagonal_dissolve` | ramp wipe | 6.94 | 9.69 | 4.81 | +2.13 | diagonal soft front; ~3 above the floor mid-way (front shape) |
| Domino Cascade | `jb.domino_panels` | global mix stand-in | 39.08 | 65.67 | 5.40 | +33.68 |  |
| Energy Sports Transition 02 | `jb.speed_bars` | global mix stand-in | 34.95 | 75.59 | 4.94 | +30.02 |  |
| Erase Line Down | `jb.line_erase_down` | edge wipe | 5.52 | 6.47 | 4.80 | +0.71 | hard edge, fast–slow–fast timing table |
| Erase Line Right | `jb.line_erase_right` | edge wipe | 4.89 | 5.51 | 4.79 | +0.10 | hard edge, fast–slow–fast timing table |
| Fade Dissolve | `jb.sweep_dissolve` | soft edge × eased mix | 7.07 | 7.64 | 4.88 | +2.19 | left-to-right front 0.475 wide at x = 0.047 + 1.4 p, times an eased global weight |
| Fade Grayscale | `jb.gray_fade` | global mix | 6.26 | 7.94 | 5.02 | +1.24 | dissolve through a desaturated mix (luma terms) |
| Fade Single Track | `jb.wash_black` | global mix | 3.65 | 7.03 | 5.17 | -1.52 | dip to black, black on frames 29–32 |
| Fade White | `jb.wash_white` | global mix | 3.98 | 6.92 | 5.31 | -1.33 | linear display-space dip to white, peak on the cut frame |
| Falling Frame Transitions | `jb.falling_frame` | global mix stand-in | 32.74 | 47.77 | 4.76 | +27.98 |  |
| Fast Whirl 1 | `jb.fast_whirl_1` | global mix stand-in | 11.38 | 29.74 | 4.96 | +6.42 |  |
| Fast Whirl 2 | `jb.fast_whirl_2` | global mix stand-in | 11.18 | 29.94 | 4.97 | +6.21 |  |
| Fast Wipe Down | `jb.fast_wipe_down` | existing `jb.fast_wipe_down` | 4.64 | 5.50 | 4.71 | -0.07 | alias only; already at the floor |
| Fast Wipe Up | `jb.fast_wipe_up` | existing `jb.fast_wipe_up` | 4.48 | 5.30 | 4.78 | -0.30 | alias only; already at the floor |
| Fear Halloween 2 | `jb.zoom_through` | global mix stand-in | 31.62 | 45.33 | 5.84 | +25.78 |  |
| Film Grain Style Transition 01 | `jb.warm_glow_wash` | global mix stand-in | 6.62 | 23.63 | 4.83 | +1.79 |  |
| Film Grit 1 | `jb.grit_flash_orange` | global mix stand-in | 8.45 | 40.45 | 4.79 | +3.66 |  |
| Film Grit 2 | `jb.grit_frame` | global mix stand-in | 10.39 | 49.59 | 4.94 | +5.45 |  |
| Flash | `jb.bright_flash` | flash (zoom + bloom) | 7.31 | 10.78 | 4.88 | +2.42 | bloom + overexposure, new clip lands at 1.11× zoom; mid frames ~10 |
| Flashbulbs 02 | `jb.flashbulbs` | global mix stand-in | 9.74 | 39.57 | 4.71 | +5.03 |  |
| Flora Collage Pack Transition 05 | `jb.diagonal_sweep` | global mix stand-in | 14.20 | 29.02 | 4.74 | +9.46 |  |
| Fluorescent Flash | `jb.neon_flicker_flash` | global mix stand-in | 12.35 | 35.42 | 5.83 | +6.52 |  |
| Flyeye | `jb.fly_eye` | global mix stand-in | 20.56 | 25.74 | 5.50 | +15.06 |  |
| Gaming Set Sports Transition 2 | `jb.chevron_wipe` | global mix stand-in | 16.27 | 33.79 | 4.78 | +11.49 |  |
| Give Mom Unique Love Pack Transition 01 | `jb.pink_bokeh` | global mix stand-in | 24.84 | 67.88 | 4.87 | +19.98 |  |
| Gold Blue Transition 02 | `jb.gold_blue_paint_2` | global mix stand-in | 18.98 | 36.70 | 4.83 | +14.14 |  |
| Gold Blue Transition 03 | `jb.gold_blue_paint_3` | global mix stand-in | 18.65 | 40.14 | 4.88 | +13.77 |  |
| Gold Blue Transition 04 | `jb.gold_blue_paint_4` | global mix stand-in | 29.59 | 41.64 | 4.79 | +24.80 |  |
| Gold Blue Transition 05 | `jb.gold_blue_paint_5` | global mix stand-in | 12.67 | 38.73 | 4.76 | +7.91 |  |
| Gold Blue Transition 06 | `jb.gold_blue_paint_6` | global mix stand-in | 12.51 | 41.43 | 4.75 | +7.75 |  |
| Golden Awards Ceremony Transition 05 | `jb.gold_diamond_wipe` | global mix stand-in | 9.36 | 35.86 | 4.79 | +4.57 |  |
| Golden Awards Pack Transition 04 | `jb.gold_ribbon_1` | global mix stand-in | 16.09 | 37.44 | 5.00 | +11.09 |  |
| Golden Awards Transition 01 | `jb.gold_ribbon_2` | global mix stand-in | 19.03 | 36.37 | 4.86 | +14.17 |  |
| Golden Awards Transition 02 | `jb.gold_ribbon_3` | global mix stand-in | 17.24 | 34.96 | 4.83 | +12.41 |  |
| Golden Awards Transition 03 | `jb.gold_ribbon_4` | global mix stand-in | 16.33 | 36.10 | 4.85 | +11.48 |  |
| Golden Awards Transition 04 | `jb.golden_bloom_flash` | global mix stand-in | 12.79 | 21.89 | 4.80 | +7.99 |  |
| Golden Awards Transition 05 | `jb.gold_glitter_flash` | global mix stand-in | 12.58 | 33.05 | 4.99 | +7.59 |  |
| Grammy Award 2024 Transition 02 | `jb.gold_frames_zoom` | global mix stand-in | 15.91 | 41.89 | 4.79 | +11.12 |  |
| Grunge Scribble Vlog Transition 02 | `jb.torn_paper_wipe` | global mix stand-in | 10.30 | 42.70 | 4.81 | +5.49 |  |
| Halloween Colorful Transition 04 | `jb.pumpkin_face_wipe` | global mix stand-in | 38.93 | 64.98 | 4.69 | +34.24 |  |
| Halloween Colorful Transition 06 | `jb.skull_wedge_wipe` | global mix stand-in | 19.82 | 32.39 | 4.69 | +15.13 |  |
| Halloween Colorful Transition 09 | `jb.witch_hat_wipe` | global mix stand-in | 8.07 | 28.01 | 4.69 | +3.38 |  |
| Halloween Colorful Transition 15 | `jb.pumpkin_pop_wipe` | global mix stand-in | 14.89 | 34.98 | 4.70 | +10.19 |  |
| Halloween Colorful Transition 17 | `jb.skull_hat_sweep` | global mix stand-in | 19.58 | 40.38 | 4.68 | +14.90 |  |
| Halloween Colorful Transition 21 | `jb.bat_wing_wipe` | global mix stand-in | 14.18 | 45.53 | 4.67 | +9.51 |  |
| Halloween Colorful Transition 23 | `jb.ghost_curve_wipe` | radial wipe + grade | 12.92 | 18.19 | 4.69 | +8.23 | ghost sprite not drawn |
| Halloween Colorful Transition 25 | `jb.cat_curve_wipe` | global mix stand-in | 23.14 | 33.85 | 4.70 | +18.45 |  |
| Halloween Film Pack Transition 02 | `jb.rust_slash` | global mix stand-in | 10.15 | 38.29 | 4.82 | +5.33 |  |
| Halloween Film Transition 01 | `jb.rust_smear` | global mix stand-in | 9.40 | 46.79 | 4.96 | +4.44 |  |
| Halloween Horrors Pack Transition 02 | `jb.ghost_crowd_rise` | global mix stand-in | 23.87 | 46.77 | 4.87 | +19.00 |  |
| Halloween Party Transition 02 | `jb.lantern_glow` | global mix stand-in | 15.91 | 41.95 | 4.96 | +10.95 |  |
| Halloween Party Transition 03 | `jb.bat_swarm` | global mix stand-in | 12.38 | 52.51 | 4.79 | +7.60 |  |
| Halloween Scary Pack Transition 06 | `jb.blue_fire_smoke` | global mix stand-in | 28.78 | 54.21 | 4.99 | +23.79 |  |
| Horizontal Cutout 1 | `jb.bar_cutout` | procedural bars | 14.96 | 29.89 | 4.71 | +10.26 | bar layout and timing estimated by eye; the bars' content offsets are not reproduced |
| Horizontal Reel | `jb.card_reel` | global mix stand-in | 15.24 | 28.97 | 4.69 | +10.55 |  |
| Horizontal Screen Cutting | `jb.band_cut_zoom` | global mix stand-in | 21.35 | 40.97 | 4.79 | +16.56 |  |
| Horizontal Slice | `jb.slice_slide` | strip slide | 9.41 | 19.11 | 4.97 | +4.44 | 10 strips, fitted edges and easing exponents; mid frames 15–19 (strip motion blur not modelled) |
| Horrible Colorful Transition 02 | `jb.glitch_bands` | global mix stand-in | 15.69 | 39.61 | 4.70 | +10.99 |  |
| Horrible Colorful Transition 13 | `jb.ghost_face_flash` | global mix stand-in | 29.39 | 68.68 | 4.71 | +24.68 |  |
| Horrible Colorful Transition 24 | `jb.witch_silhouette_wipe` | global mix stand-in | 20.30 | 30.50 | 4.67 | +15.64 |  |
| Horrible Film Pack Transition 03 | `jb.bloody_hand` | global mix stand-in | 10.66 | 38.74 | 4.95 | +5.71 |  |
| Horrible Neon Party Transition 01 | `jb.pumpkin_pile` | global mix stand-in | 33.08 | 50.55 | 5.21 | +27.86 |  |
| Horrible Party Transition 01 | `jb.bandage_wrap` | global mix stand-in | 17.07 | 38.39 | 4.66 | +12.41 |  |
| Horror Story Transition 01 | `jb.blood_splash` | global mix stand-in | 9.05 | 35.06 | 4.69 | +4.36 |  |
| Horror Story Transition 02 | `jb.zombie_hands` | global mix stand-in | 17.45 | 38.53 | 4.79 | +12.66 |  |

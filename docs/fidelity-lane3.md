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
- **Global mix + light field** (`kit::field_mix`): the global mix plus,
  per channel, a cubic polynomial in (u, v) added per frame (least
  squares). It is the stand-in for every item without a structural model:
  a smooth gradient that follows the broad colour and brightness layout of
  glows, leaks and artwork (the artwork's shapes are not drawn).
- **Edge / ramp wipes** (`kit::edge_wipe`, `kit::ramp_wipe`): B where a
  parametric mask (line, centre line, diagonal, ...) is below a threshold,
  with an edge width; threshold and width per frame.
- **Flash** (`kit::FlashFx`, two passes): the current side zoomed by `s`
  and lit as `g·Z + h·blur(Z, σ) + o` (clipped), per frame.

**Half-pixel offset.** Some transitions (Fade Dissolve, Dissolve Skew 01,
Blind 1, Color Dissolve 3, Cut Off — Filmora's older engine items) render
every frame shifted half a 1080p pixel right and up: their first frame,
whose content is the clip's, matches the previous clean frame shifted by
(+0.5, −0.5) px with bilinear filtering (1080p MAD 5.3 → 1.8 on the
15 Mbit/s tr013 export), while most store transitions start pixel-exact.
`Item::shifted` reproduces it (found by comparing every window's first and
last frames with the clean neighbours);
other lanes' items from the same engine likely need it too.

Nothing from Filmora's installed assets was read; light, flares and washes
are fitted from the exported output only and rendered procedurally.

## Measurements

The harness (`crates/jb-cli/examples/fidelity.rs`, `--size 960x540
--search 0`) renders the batch project's window with Jibaru and compares it
with Filmora's export. **Floor** is the same measurement on the clean
frames just before and after the window (mean of both), measured with the
corrected harness (unbiased decode, no output level offset): about 2.3 on
gridA and 3–5 on gridB at the reference's low bitrate (6 Mbit/s; the
frames after a transition are coded worse). **Excess** = MAD mean − floor
is what the model leaves; ≲ 1.5 is at the floor. Stand-ins score far above
it wherever the artwork covers the frame; "Before" is the first stand-in
(global mix only, old harness) for comparison.

**Summary.** 110 items mapped: 27 with a structural model, 83 light-field stand-ins. Mean MAD 12.88 (first stand-ins: 17.69); 9 items within 1.5 of the floor, 15 within 3.

| Filmora item | Ours | Model | MAD mean | MAD max | Floor | Excess | Before | Notes |
|---|---|---|---:|---:|---:|---:|---:|---|
| Algorithm Combination Set 06 | `jb.warm_flare_flash` | stand-in: global mix + light field | 10.07 | 20.03 | 2.40 | +7.67 | 11.84 |  |
| Ancient Civilization Egypt Pack Transition 04 | `jb.golden_dust_sweep` | stand-in: global mix + light field | 6.53 | 12.37 | 3.16 | +3.37 | 8.94 |  |
| Animate Effect Transition 03 | `jb.ink_splash_burst` | stand-in: global mix + light field | 5.51 | 44.00 | 2.93 | +2.58 | 7.62 |  |
| Anime Attack 03 | `jb.energy_ring_burst` | stand-in: global mix + light field | 19.15 | 55.11 | 2.62 | +16.52 | 22.67 |  |
| Anime Flame Transition 09 | `jb.flame_wash` | stand-in: global mix + light field | 12.90 | 28.56 | 2.55 | +10.35 | 16.88 |  |
| Anime Style Flow 01 | `jb.starburst_flash` | stand-in: global mix + light field | 11.01 | 28.30 | 2.51 | +8.50 | 15.31 |  |
| B&W Dissolve | `jb.mono_dissolve` | stand-in: global mix + light field | 8.13 | 13.95 | 3.50 | +4.63 | 9.72 |  |
| Black Friday Sale 02 Transition 06 | `jb.chevron_sweep` | stand-in: global mix + light field | 12.26 | 51.62 | 2.44 | +9.82 | 17.80 |  |
| Blind 1 | `jb.blind_bars` | edge front + procedural slats | 8.48 | 17.73 | 3.42 | +5.07 | 10.39 | slat band with hashed per-slat thresholds; slat pattern is not Filmora's; half-pixel offset |
| Blue Sparkling Awards Transition 04 | `jb.gold_sparkle_burst` | stand-in: global mix + light field | 17.75 | 28.65 | 3.33 | +14.43 | 22.56 |  |
| Burning Dash Transition 01 | `jb.fireball_dash_1` | stand-in: global mix + light field | 18.26 | 30.34 | 3.08 | +15.18 | 25.98 |  |
| Burning Dash Transition 02 | `jb.fireball_dash_2` | stand-in: global mix + light field | 14.46 | 28.83 | 3.17 | +11.29 | 19.98 |  |
| Burning Dash Transition 03 | `jb.fireball_dash_3` | stand-in: global mix + light field | 18.46 | 34.76 | 2.86 | +15.60 | 24.08 |  |
| Burning Dash Transition 04 | `jb.fireball_dash_4` | stand-in: global mix + light field | 12.24 | 34.26 | 2.81 | +9.43 | 15.51 |  |
| Business Presentation Vol 02 Transition 02 | `jb.stripe_slide_diagonal` | stand-in: global mix + light field | 25.40 | 58.24 | 2.43 | +22.97 | 31.97 |  |
| Carousel Down | `jb.carousel_down` | stand-in: global mix + light field | 21.09 | 31.87 | 3.60 | +17.49 | 37.12 |  |
| Carousel Down Rebound | `jb.carousel_down_rebound` | stand-in: global mix + light field | 21.39 | 33.03 | 3.61 | +17.78 | 25.11 |  |
| Carousel Left | `jb.carousel_left` | panel strip | 10.27 | 22.75 | 3.83 | +6.44 | 17.38 | flat panels; frames 22-31 (fast scroll) are off: the real panels turn in perspective |
| Carousel Right | `jb.carousel_right` | panel strip | 11.30 | 21.82 | 4.49 | +6.81 | 34.99 | flat panels; the fast-scroll frames (26-35, 47-53) are off: the real panels turn in perspective |
| Carousel Up | `jb.carousel_up` | stand-in: global mix + light field | 21.98 | 33.96 | 4.25 | +17.73 | 26.59 |  |
| Cartoony Vlog Pack Transition 01 | `jb.candy_spiral` | stand-in: global mix + light field | 22.05 | 51.45 | 2.91 | +19.14 | 25.03 |  |
| Chalk Swipe Left | `jb.chalk_swipe_left` | edge wipe | 3.29 | 5.53 | 3.84 | -0.55 | 5.25 | slanted edge in ~12 fps steps; the chalk texture of the edge is not drawn |
| Christmas Gift Transition 03 | `jb.bokeh_flash` | stand-in: global mix + light field | 6.89 | 15.28 | 4.21 | +2.68 | 10.85 |  |
| Cinematic Transition 03 | `jb.soft_light_leak` | stand-in: global mix + light field | 4.65 | 7.15 | 2.43 | +2.22 | 9.55 |  |
| Cinematic Transition 09 | `jb.scratched_film_dip` | stand-in: global mix + light field | 6.04 | 17.44 | 2.56 | +3.48 | 7.93 |  |
| Cinematic Travel Vlog Transition 01 | `jb.film_strip_slide` | stand-in: global mix + light field | 21.36 | 32.50 | 3.05 | +18.31 | 28.22 |  |
| Classic Movies Transition 06 | `jb.red_flare_flash` | stand-in: global mix + light field | 5.87 | 15.87 | 2.59 | +3.28 | 9.74 |  |
| Col Merge | `jb.sides_merge` | edge wipe | 2.26 | 2.86 | 2.33 | -0.07 | 4.70 | two edges closing to the centre line |
| Color Dissolve 3 | `jb.block_glow_dissolve` | stand-in: global mix + light field | 13.36 | 33.11 | 3.58 | +9.78 | 18.28 | half-pixel offset |
| Color Distance | `jb.checker_scatter` | stand-in: global mix + light field | 7.65 | 22.87 | 3.19 | +4.46 | 9.78 |  |
| Colorful Transition Pack Transition 01 | `jb.liquid_blob_wipe` | stand-in: global mix + light field | 7.44 | 26.97 | 2.50 | +4.95 | 11.73 |  |
| Colorful Transitions Vol 06 Transition 16 | `jb.color_rings` | procedural rings | 24.73 | 45.02 | 2.36 | +22.37 | 47.83 | inner ring radii and colours measured, outer bands estimated; coverage from the global fit |
| Comic Film Flash | `jb.ink_brush_sweep` | stand-in: global mix + light field | 13.78 | 27.13 | 2.73 | +11.05 | 21.79 |  |
| Comic Style 3 | `jb.comic_panels` | stand-in: global mix + light field | 35.33 | 42.83 | 3.81 | +31.52 | 40.53 |  |
| Cosmic Gallery | `jb.floating_gallery` | stand-in: global mix + light field | 30.29 | 45.61 | 4.85 | +25.45 | 52.52 |  |
| Creepy  Pack Transition 01 | `jb.creature_reveal` | stand-in: global mix + light field | 18.23 | 32.85 | 2.33 | +15.90 | 25.89 |  |
| Creepy Pack Transition 02 | `jb.face_slide` | stand-in: global mix + light field | 17.42 | 26.60 | 2.37 | +15.06 | 25.77 |  |
| Creepy Pack Transition 03 | `jb.lantern_sweep` | stand-in: global mix + light field | 25.04 | 44.46 | 2.35 | +22.68 | 36.01 |  |
| Cross Whisk | `jb.block_whisk` | stand-in: global mix + light field | 12.48 | 26.77 | 2.82 | +9.66 | 15.43 |  |
| Crystal Ball 1 | `jb.crystal_ball_iris` | lens ball | 7.20 | 13.36 | 3.59 | +3.61 | 7.37 | ellipse → circle iris with bulge and mid flash colours |
| Crystal Ball 2 | `jb.crystal_ball_lens` | lens ball over clip | 15.70 | 35.22 | 3.24 | +12.46 | 17.76 | bulge only; the ball's rim highlight is not drawn |
| Cut Off | `jb.cut_off` | stand-in: global mix + light field | 11.33 | 28.34 | 3.50 | +7.83 | 12.52 | half-pixel offset |
| Depth Slice | `jb.depth_panels` | stand-in: global mix + light field | 31.53 | 42.27 | 2.90 | +28.63 | 43.24 |  |
| Diagonal Slice | `jb.split_slide` | stand-in: global mix + light field | 11.19 | 26.77 | 3.81 | +7.38 | 12.27 |  |
| Dimensional Drop | `jb.panel_drop` | stand-in: global mix + light field | 19.72 | 36.60 | 3.77 | +15.95 | 27.21 |  |
| Dissolve Skew 01 | `jb.diagonal_dissolve` | ramp wipe | 5.23 | 8.45 | 3.29 | +1.95 | 6.94 | diagonal soft front; ~3 above the floor mid-way (front shape); half-pixel offset |
| Domino Cascade | `jb.domino_panels` | stand-in: global mix + light field | 26.14 | 40.74 | 4.26 | +21.88 | 39.08 |  |
| Energy Sports Transition 02 | `jb.speed_bars` | stand-in: global mix + light field | 31.36 | 70.25 | 2.66 | +28.70 | 34.95 |  |
| Erase Line Down | `jb.line_erase_down` | edge wipe | 3.11 | 4.03 | 2.35 | +0.76 | 5.52 | hard edge, fast–slow–fast timing table |
| Erase Line Right | `jb.line_erase_right` | edge wipe | 2.52 | 3.04 | 2.31 | +0.21 | 4.89 | hard edge, fast–slow–fast timing table |
| Fade Dissolve | `jb.sweep_dissolve` | soft edge × eased mix | 5.49 | 6.48 | 3.33 | +2.16 | 7.07 | left-to-right front 0.475 wide at x = 0.047 + 1.4 p, times an eased global weight; half-pixel offset |
| Fade Grayscale | `jb.gray_fade` | global mix | 5.77 | 8.11 | 3.50 | +2.27 | 6.26 | dissolve through a desaturated mix (luma terms) |
| Fade Single Track | `jb.wash_black` | global mix | 3.33 | 6.83 | 3.70 | -0.37 | 3.65 | dip to black, black on frames 29–32 |
| Fade White | `jb.wash_white` | global mix | 3.66 | 6.55 | 4.50 | -0.84 | 3.98 | linear display-space dip to white, peak on the cut frame |
| Falling Frame Transitions | `jb.falling_frame` | framed panel | 23.87 | 42.50 | 3.29 | +20.58 | 32.74 | flat panel with white border and fitted grade; the frame tilts in perspective while it falls (mid frames ~35) |
| Fast Whirl 1 | `jb.fast_whirl_1` | stand-in: global mix + light field | 9.97 | 28.84 | 3.25 | +6.72 | 11.38 |  |
| Fast Whirl 2 | `jb.fast_whirl_2` | stand-in: global mix + light field | 9.49 | 28.47 | 3.22 | +6.27 | 11.18 |  |
| Fast Wipe Down | `jb.fast_wipe_down` | existing `jb.fast_wipe_down` | 2.67 | 4.12 | 3.05 | -0.38 | 4.64 | alias only; already at the floor |
| Fast Wipe Up | `jb.fast_wipe_up` | existing `jb.fast_wipe_up` | 2.35 | 3.57 | 2.39 | -0.04 | 4.48 | alias only; already at the floor |
| Fear Halloween 2 | `jb.zoom_through` | stand-in: global mix + light field | 27.50 | 42.73 | 4.65 | +22.86 | 31.62 |  |
| Film Grain Style Transition 01 | `jb.warm_glow_wash` | stand-in: global mix + light field | 3.61 | 10.87 | 2.57 | +1.04 | 6.62 |  |
| Film Grit 1 | `jb.grit_flash_orange` | stand-in: global mix + light field | 5.63 | 33.64 | 2.60 | +3.02 | 8.45 |  |
| Film Grit 2 | `jb.grit_frame` | stand-in: global mix + light field | 7.40 | 28.02 | 2.93 | +4.47 | 10.39 |  |
| Flash | `jb.bright_flash` | flash (zoom + bloom) | 6.83 | 11.26 | 3.34 | +3.49 | 7.31 | bloom + overexposure, new clip lands at 1.11× zoom; mid frames ~10 |
| Flashbulbs 02 | `jb.flashbulbs` | stand-in: global mix + light field | 6.10 | 22.37 | 2.51 | +3.58 | 9.74 |  |
| Flora Collage Pack Transition 05 | `jb.diagonal_sweep` | stand-in: global mix + light field | 6.59 | 11.63 | 2.52 | +4.07 | 14.20 |  |
| Fluorescent Flash | `jb.neon_flicker_flash` | stand-in: global mix + light field | 10.64 | 27.20 | 5.19 | +5.46 | 12.35 |  |
| Flyeye | `jb.fly_eye` | stand-in: global mix + light field | 20.45 | 25.30 | 4.15 | +16.30 | 20.56 |  |
| Gaming Set Sports Transition 2 | `jb.chevron_wipe` | stand-in: global mix + light field | 10.47 | 20.01 | 2.48 | +7.99 | 16.27 |  |
| Give Mom Unique Love Pack Transition 01 | `jb.pink_bokeh` | stand-in: global mix + light field | 11.12 | 24.54 | 2.47 | +8.64 | 24.84 |  |
| Gold Blue Transition 02 | `jb.gold_blue_paint_2` | stand-in: global mix + light field | 15.90 | 32.69 | 2.43 | +13.47 | 18.98 |  |
| Gold Blue Transition 03 | `jb.gold_blue_paint_3` | stand-in: global mix + light field | 10.98 | 20.31 | 2.53 | +8.45 | 18.65 |  |
| Gold Blue Transition 04 | `jb.gold_blue_paint_4` | stand-in: global mix + light field | 24.00 | 31.24 | 2.86 | +21.14 | 29.59 |  |
| Gold Blue Transition 05 | `jb.gold_blue_paint_5` | stand-in: global mix + light field | 10.19 | 26.32 | 2.96 | +7.23 | 12.67 |  |
| Gold Blue Transition 06 | `jb.gold_blue_paint_6` | stand-in: global mix + light field | 9.21 | 29.48 | 2.83 | +6.39 | 12.51 |  |
| Golden Awards Ceremony Transition 05 | `jb.gold_diamond_wipe` | stand-in: global mix + light field | 6.90 | 25.97 | 2.79 | +4.11 | 9.36 |  |
| Golden Awards Pack Transition 04 | `jb.gold_ribbon_1` | stand-in: global mix + light field | 10.69 | 25.51 | 3.18 | +7.50 | 16.09 |  |
| Golden Awards Transition 01 | `jb.gold_ribbon_2` | stand-in: global mix + light field | 12.21 | 23.17 | 2.45 | +9.76 | 19.03 |  |
| Golden Awards Transition 02 | `jb.gold_ribbon_3` | stand-in: global mix + light field | 11.56 | 21.55 | 2.38 | +9.18 | 17.24 |  |
| Golden Awards Transition 03 | `jb.gold_ribbon_4` | stand-in: global mix + light field | 10.71 | 25.54 | 2.41 | +8.30 | 16.33 |  |
| Golden Awards Transition 04 | `jb.golden_bloom_flash` | stand-in: global mix + light field | 7.45 | 16.02 | 3.01 | +4.44 | 12.79 |  |
| Golden Awards Transition 05 | `jb.gold_glitter_flash` | stand-in: global mix + light field | 8.58 | 26.07 | 3.69 | +4.89 | 12.58 |  |
| Grammy Award 2024 Transition 02 | `jb.gold_frames_zoom` | stand-in: global mix + light field | 13.67 | 31.89 | 2.55 | +11.12 | 15.91 |  |
| Grunge Scribble Vlog Transition 02 | `jb.torn_paper_wipe` | stand-in: global mix + light field | 7.01 | 32.17 | 2.54 | +4.47 | 10.30 |  |
| Halloween Colorful Transition 04 | `jb.pumpkin_face_wipe` | stand-in: global mix + light field | 31.61 | 54.39 | 2.30 | +29.31 | 38.93 |  |
| Halloween Colorful Transition 06 | `jb.skull_wedge_wipe` | radial wipe + light field | 9.54 | 14.49 | 2.31 | +7.23 | 19.82 | skull sprite not drawn |
| Halloween Colorful Transition 09 | `jb.witch_hat_wipe` | radial wipe + light field | 6.22 | 12.52 | 2.29 | +3.92 | 8.44 | hat sprite not drawn; the stand-in scored 8.07 (the sprite's average was in it) |
| Halloween Colorful Transition 15 | `jb.pumpkin_pop_wipe` | radial wipe | 9.47 | 16.08 | 2.31 | +7.16 | 11.42 | pumpkin sprite not drawn |
| Halloween Colorful Transition 17 | `jb.skull_hat_sweep` | stand-in: global mix + light field | 15.71 | 32.29 | 2.33 | +13.39 | 19.58 |  |
| Halloween Colorful Transition 21 | `jb.bat_wing_wipe` | stand-in: global mix + light field | 9.79 | 31.65 | 2.30 | +7.48 | 14.18 |  |
| Halloween Colorful Transition 23 | `jb.ghost_curve_wipe` | radial wipe + light field | 11.52 | 18.57 | 2.31 | +9.21 | 12.92 | ghost sprite not drawn |
| Halloween Colorful Transition 25 | `jb.cat_curve_wipe` | radial wipe + light field | 11.32 | 16.00 | 2.32 | +9.00 | 23.14 | cat sprite not drawn |
| Halloween Film Pack Transition 02 | `jb.rust_slash` | stand-in: global mix + light field | 6.94 | 33.26 | 2.92 | +4.03 | 10.15 |  |
| Halloween Film Transition 01 | `jb.rust_smear` | stand-in: global mix + light field | 7.37 | 28.13 | 3.46 | +3.91 | 9.40 |  |
| Halloween Horrors Pack Transition 02 | `jb.ghost_crowd_rise` | stand-in: global mix + light field | 19.01 | 39.53 | 2.98 | +16.03 | 23.87 |  |
| Halloween Party Transition 02 | `jb.lantern_glow` | stand-in: global mix + light field | 10.57 | 26.58 | 3.04 | +7.53 | 15.91 |  |
| Halloween Party Transition 03 | `jb.bat_swarm` | stand-in: global mix + light field | 9.27 | 39.14 | 2.41 | +6.86 | 12.38 |  |
| Halloween Scary Pack Transition 06 | `jb.blue_fire_smoke` | stand-in: global mix + light field | 20.20 | 36.11 | 3.64 | +16.56 | 28.78 |  |
| Horizontal Cutout 1 | `jb.bar_cutout` | procedural bars | 13.30 | 29.91 | 3.08 | +10.21 | 14.96 | bar layout and timing estimated by eye; the bars' content offsets are not reproduced |
| Horizontal Reel | `jb.card_reel` | stand-in: global mix + light field | 14.42 | 28.25 | 3.06 | +11.36 | 15.24 |  |
| Horizontal Screen Cutting | `jb.band_cut_zoom` | stand-in: global mix + light field | 14.85 | 24.60 | 3.16 | +11.70 | 21.35 |  |
| Horizontal Slice | `jb.slice_slide` | strip slide | 8.25 | 18.11 | 2.78 | +5.47 | 9.41 | 10 strips, fitted edges and easing exponents; mid frames 15–19 (strip motion blur not modelled) |
| Horrible Colorful Transition 02 | `jb.glitch_bands` | stand-in: global mix + light field | 10.73 | 27.87 | 2.33 | +8.41 | 15.69 |  |
| Horrible Colorful Transition 13 | `jb.ghost_face_flash` | stand-in: global mix + light field | 22.67 | 44.49 | 2.40 | +20.27 | 29.39 |  |
| Horrible Colorful Transition 24 | `jb.witch_silhouette_wipe` | radial wipe + light field | 10.61 | 15.46 | 2.43 | +8.18 | 12.36 | witch silhouette not drawn |
| Horrible Film Pack Transition 03 | `jb.bloody_hand` | stand-in: global mix + light field | 8.45 | 32.59 | 3.60 | +4.85 | 10.66 |  |
| Horrible Neon Party Transition 01 | `jb.pumpkin_pile` | stand-in: global mix + light field | 29.65 | 46.24 | 4.18 | +25.46 | 33.08 |  |
| Horrible Party Transition 01 | `jb.bandage_wrap` | stand-in: global mix + light field | 15.52 | 32.15 | 2.81 | +12.71 | 17.07 |  |
| Horror Story Transition 01 | `jb.blood_splash` | stand-in: global mix + light field | 5.88 | 16.54 | 2.85 | +3.03 | 9.05 |  |
| Horror Story Transition 02 | `jb.zombie_hands` | stand-in: global mix + light field | 12.43 | 25.92 | 2.58 | +9.86 | 17.45 |  |

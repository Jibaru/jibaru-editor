# Render fidelity: art lane 3a (graphic and themed transitions)

Graphic, cartoon and themed transitions (camera shutters, comic bursts,
holiday, seasonal, sports and business graphics, motion-graphics packs) that
lanes 1, 3 and 4 rendered with stand-ins (a global mix plus a smooth light
field, which reproduces the timing and average colour of the artwork but not
its shapes), recreated with **original artwork**. Code:
`crates/jb-render/src/fx/art3a*.rs`, `crates/jb-render/src/shaders/art3a/`,
GUIDs in `crates/jb-render/src/aliases_art3a.rs` (one mapping per GUID: the
replaced stand-in's alias was removed from its lane's alias file; the
stand-in effect itself stays registered).

## Originality

Every shape is drawn from scratch as signed-distance artwork in WGSL
(`shaders/art3a/kit.wgsl`: shape primitives, outlines, layering, shading).
Filmora's installed assets were never read. The reference exports were only
watched to learn each item's concept, timing, motion path, screen coverage
over time and palette; no frame or shape was traced, vectorized or copied.
The result is our own drawing in the same style (our pumpkin, our bats, our
camera iris), keyframed on the measured motion. No third-party assets.

## Method

Same harness and windows as lane 3 (`crates/jb-cli/examples/fidelity.rs`,
`--size 960x540 --search 0`, batch projects of `docs/filmora-references.json`).
"Before" is the stand-in re-measured with this harness; "Floor" is the lane 3
noise floor of the window (clean frames around it). An alias was switched only
when the item looks right on side-by-side contact sheets and its MAD is no
worse than the stand-in's. Artwork whose drawing differs from Filmora's (as
it must) keeps a residual above the floor along its edges and details; the
gain over the stand-in is in shape, coverage and motion.

## Upgraded (16)

| Filmora item | Ours | Replaced stand-in | MAD before | MAD now | MAD max | Floor | Notes |
|---|---|---|---:|---:|---:|---:|---|
| Business Presentation Vol 02 Transition 02 | `jb.stripe_close` | `jb.stripe_slide_diagonal` | 25.40 | 5.90 | 17.65 | 2.43 |  |
| Camera Shutter | `jb.iris_shutter` | `jb.shutter_wash_01` | 9.72 | 4.88 | 11.69 | — |  |
| Camera Transition 09 | `jb.lens_iris` | `jb.shutter_wash_02` | 14.04 | 7.60 | 13.15 | — |  |
| Comic Style 3 | `jb.comic_page` | `jb.comic_panels` | 35.33 | 27.50 | 42.94 | 3.81 | camera fitted per frame on the hero panels; the other panels of the reference are halftone-printed and cannot be matched pixel-wise |
| Creepy Pack Transition 03 | `jb.lantern_roll` | `jb.lantern_sweep` | 25.04 | 20.73 | 44.20 | 2.35 |  |
| Gaming Set Sports Transition 2 | `jb.arrow_stack` | `jb.chevron_wipe` | 10.47 | 5.51 | 8.94 | 2.48 |  |
| Golden Awards Ceremony Transition 05 | `jb.gold_diamond` | `jb.gold_diamond_wipe` | 6.90 | 4.63 | 13.90 | 2.79 |  |
| Golden Awards Pack Transition 04 | `jb.gold_plates_sweep` | `jb.gold_ribbon_1` | 10.69 | 8.54 | 19.60 | 3.18 |  |
| Golden Awards Transition 03 | `jb.gold_plates_rise` | `jb.gold_ribbon_4` | 10.71 | 8.55 | 19.85 | 2.41 |  |
| Grammy Award 2024 Transition 02 | `jb.gold_chevrons` | `jb.gold_frames_zoom` | 13.67 | 10.73 | 30.36 | 2.55 |  |
| Halloween Colorful Transition 04 | `jb.pumpkin_grin` | `jb.pumpkin_face_wipe` | 31.61 | 25.65 | 80.12 | 2.30 |  |
| Halloween Colorful Transition 09 | `jb.witch_hat_wave` | `jb.witch_hat_wipe` | 6.22 | 4.10 | 11.27 | 2.29 |  |
| Halloween Colorful Transition 17 | `jb.skull_and_hat` | `jb.skull_hat_sweep` | 15.71 | 10.70 | 20.46 | 2.33 |  |
| Halloween Colorful Transition 21 | `jb.bat_rise` | `jb.bat_wing_wipe` | 9.79 | 7.18 | 24.90 | 2.30 |  |
| Horrible Colorful Transition 02 | `jb.ghost_glide` | `jb.glitch_bands` | 10.73 | 5.52 | 11.99 | 2.33 |  |
| Horrible Colorful Transition 13 | `jb.boo_dive` | `jb.ghost_face_flash` | 22.67 | 21.39 | 86.89 | 2.40 |  |

## Still stand-ins (49)

Mapped by their lane's stand-in; "Tried" is the best original-art attempt when one was measured but not switched (worse or not visually right).

| Filmora item | Stand-in | MAD | Tried | Notes |
|---|---|---:|---:|---|
| Algorithm Combination Set 06 | `jb.warm_flare_flash` | 10.07 | — |  |
| Ancient Civilization Egypt Pack Transition 04 | `jb.golden_dust_sweep` | 6.53 | — |  |
| Animate Effect Transition 03 | `jb.ink_splash_burst` | 5.51 | — |  |
| Anime Attack 03 | `jb.energy_ring_burst` | 19.15 | — |  |
| Anime Flame Transition 09 | `jb.flame_wash` | 12.90 | — |  |
| Anime Style Flow 01 | `jb.starburst_flash` | 11.01 | — |  |
| B&W Dissolve | `jb.mono_dissolve` | 8.13 | — |  |
| Black Friday Sale 02 Transition 06 | `jb.chevron_sweep` | 12.26 | — |  |
| Carousel Down | `jb.carousel_down` | 21.09 | — |  |
| Carousel Down Rebound | `jb.carousel_down_rebound` | 21.39 | — |  |
| Carousel Up | `jb.carousel_up` | 21.98 | — |  |
| Cartoony Vlog Pack Transition 01 | `jb.candy_spiral` | 22.05 | 24.65 | `jb.candy_swirl` drawn (spiral, gem, blob) but the stripe phase of the reference spiral is not matched, so its high-contrast bands measure worse than the smooth stand-in; alias kept on the stand-in |
| Christmas Gift Transition 03 | `jb.bokeh_flash` | 6.89 | — |  |
| Cinematic Transition 03 | `jb.soft_light_leak` | 4.65 | — |  |
| Cinematic Transition 09 | `jb.scratched_film_dip` | 6.04 | — |  |
| Cinematic Travel Vlog Transition 01 | `jb.film_strip_slide` | 21.36 | — |  |
| Classic Movies Transition 06 | `jb.red_flare_flash` | 5.87 | — |  |
| Color Dissolve 3 | `jb.block_glow_dissolve` | 13.36 | — |  |
| Color Distance | `jb.checker_scatter` | 7.65 | — |  |
| Colorful Transition Pack Transition 01 | `jb.liquid_blob_wipe` | 7.44 | — |  |
| Creepy  Pack Transition 01 | `jb.creature_reveal` | 18.23 | — | photoreal creature close-up; not attempted (a stylised drawing cannot match its texture) |
| Creepy Pack Transition 02 | `jb.face_slide` | 17.42 | — | photoreal doll face; an original stylised head measured only 16.97 vs 17.42 and did not look equivalent, so it was not kept |
| Cross Whisk | `jb.block_whisk` | 12.48 | — |  |
| Cut Off | `jb.cut_off` | 11.33 | — |  |
| Diagonal Slice | `jb.split_slide` | 11.19 | — |  |
| Dimensional Drop | `jb.panel_drop` | 19.72 | — |  |
| Domino Cascade | `jb.domino_panels` | 26.14 | — |  |
| Drop Spread | `—` | — | — |  |
| Fast Whirl 1 | `jb.fast_whirl_1` | 9.97 | — |  |
| Fast Whirl 2 | `jb.fast_whirl_2` | 9.49 | — |  |
| Fear Halloween 2 | `jb.zoom_through` | 27.50 | — |  |
| Flora Collage Pack Transition 05 | `jb.diagonal_sweep` | 6.59 | — |  |
| Flyeye | `jb.fly_eye` | 20.45 | — |  |
| Give Mom Unique Love Pack Transition 01 | `jb.pink_bokeh` | 11.12 | — |  |
| Gold Blue Transition 02 | `jb.gold_blue_paint_2` | 15.90 | — |  |
| Gold Blue Transition 03 | `jb.gold_blue_paint_3` | 10.98 | — |  |
| Gold Blue Transition 04 | `jb.gold_blue_paint_4` | 24.00 | — |  |
| Gold Blue Transition 05 | `jb.gold_blue_paint_5` | 10.19 | — |  |
| Gold Blue Transition 06 | `jb.gold_blue_paint_6` | 9.21 | — |  |
| Golden Awards Transition 01 | `jb.gold_ribbon_2` | 12.21 | — |  |
| Golden Awards Transition 02 | `jb.gold_ribbon_3` | 11.56 | — |  |
| Golden Awards Transition 04 | `jb.golden_bloom_flash` | 7.45 | — |  |
| Golden Awards Transition 05 | `jb.gold_glitter_flash` | 8.58 | — |  |
| Halloween Horrors Pack Transition 02 | `jb.ghost_crowd_rise` | 19.01 | 19.44 | `jb.ghost_crowd` drawn; measures 19.44 vs 19.01, alias kept on the stand-in |
| Halloween Party Transition 02 | `jb.lantern_glow` | 10.57 | 13.77 | `jb.lantern_face` drawn; measures 13.77 vs 10.57, alias kept on the stand-in |
| Halloween Party Transition 03 | `jb.bat_swarm` | 9.27 | 10.47 | `jb.bat_swarm_dive` drawn; measures 10.47 vs 9.27, alias kept on the stand-in |
| Halloween Scary Pack Transition 06 | `jb.blue_fire_smoke` | 20.20 | — |  |
| Horizontal Reel | `jb.card_reel` | 14.42 | — |  |
| Horizontal Screen Cutting | `jb.band_cut_zoom` | 14.85 | — |  |

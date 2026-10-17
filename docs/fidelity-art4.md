# Render fidelity: art-4 procedural items

Glitch, warp, 3D-surface and stylized items of Filmora 15.7.3 that lanes 1,
3 and 4 left as stand-ins or unimplemented (`docs/fidelity-lane{1,3,4}.md`),
recreated as our own parametric WGSL (`crates/jb-render/src/fx/art4_*.rs`,
`crates/jb-render/src/shaders/art4/`). Every item is data: a model (pass
structure + shader), a constant vector and a table of keyframe rows over
progress, fitted to Filmora's exported output of the item only (no Filmora
asset file was read). Randomness is seeded integer hashing (PCG) of block
indices and of a frame "beat", so renders are reproducible frame by frame.
Filmora GUIDs map to the items in `crates/jb-render/src/aliases_art4.rs`
(exact case, plus the upper-case form).

## Method

Same harness and windows as the lanes (`crates/jb-cli/examples/fidelity.rs`,
960×540, `--search 0`, windows from `docs/filmora-references.json`). The
keyframe rows were fitted frame by frame with a throwaway pattern-search
fitter at 480×270 against the decoded export, warm-starting each frame from
the previous one. "Before" is the previous mapping (hard cut / effect
dropped, or the lane stand-in); an alias is only switched when "After" is
lower.

## Results

MAD mean / max over the window (0–255 over RGB).

| Item | Batch | id | Before | After | Notes |
|---|---|---|---|---|---|
| Swirl | tr005 | `jb.a4.whirlpool` | 55.63 / 60.6 | 25.54 / 36.7 | elliptical twist + zoom; residual is the fine checker detail of the grid media under a strong warp |
| Ripple | tr005 | `jb.a4.puddle_ripple` | 23.77 / 49.2 | 17.10 / 27.4 | seeded noise bumps; bump positions are statistical, not identical |
| Wave | tr005 | `jb.a4.ring_ripple_reveal` | 27.49 / 48.1 | 21.31 / 34.1 | expanding ring front with concentric ripples |
| Butterfly Wave Scrawler | tr004 | `jb.a4.ribbon_wave_split` | 40.74 / 55.4 | 40.28 / 50.6 | sine-bent lines + RGB split; structure close, phase of the bands differs |
| Mosaic | trfix1 | `jb.a4.block_pixelate` | 43.65 / 102.7 | 41.62 / 64.0 | centre-anchored N×N cells, point-sampled; the fine grid media makes any cell misalignment expensive |
| Strong Sharpening (grid / scene) | fx003 | `jb.a4.crisp_sharpen` | 29.76 / 29.9 · 12.34 / 12.9 | 26.75 / 26.8 · 8.91 / 9.2 | unsharp mask + vertical red/blue fringes + desaturation |
| Chromatic Aberration (grid / scene) | fx002 | `jb.a4.prism_split_bursts` | 16.15 / 59.7 · 7.23 / 33.2 | 11.74 / 45.7 · 4.32 / 11.4 | per-frame red/blue split and smear; burst timing is deterministic in clip time (same on both clips) |
| Heart Zoom (grid / scene) | fx003 | `jb.a4.dreamy_focus_in` | 26.08 / 50.9 · 8.07 / 16.2 | 18.60 / 36.0 · 4.81 / 9.6 | blur that pulls into focus; the heart-shaped bokeh and heart particles are not drawn |
| Baltan (grid / scene) | fx001 | `jb.a4.afterimage_rise` | 28.76 / 86.8 · 27.74 / 78.6 | 3.54 / 5.5 · 6.44 / 10.8 | stepped rise from black; Filmora's temporal echo on moving footage is not modelled (effects have no frame history) |
| Money (grid / scene) | fx001 | `jb.a4.banknote_engrave` | 76.32 / 76.4 · 64.93 / 66.0 | 68.63 / 68.7 · 49.33 / 50.1 | high-contrast grey with a faint engraving screen; the binary line screen cannot be phase-matched, so a full-strength screen measures worse than grey |
| Shattered 02 (grid / scene) | fx003 | `jb.a4.shard_sweep` | 100.45 / 112.3 · 89.81 / 100.8 | 12.21 / 46.3 · 10.49 / 39.7 | Voronoi shards released left to right, shrinking, over black; shard layout is seeded, not Filmora's |
| Shattered 03 (grid / scene) | fx003 | `jb.a4.shard_burst` | 72.76 / 112.3 · 59.22 / 100.2 | 20.51 / 57.2 · 12.43 / 45.2 | same model, radial release from the centre |
| Psychedelic Glitch (grid / scene) | fx003 | `jb.a4.acid_flash` | 25.12 / 127.7 · 21.15 / 120.8 | 16.50 / 70.2 · 9.22 / 49.1 | keyed flashes (grey negative, blue negative, smear, posterize, fringe); the rainbow solarization frames are not matched |
| Stripe Glitch 01 (grid / scene) | fx003 | `jb.a4.stripe_jolt` | 11.33 / 23.1 · 4.32 / 17.6 | 9.77 / 20.0 · 4.12 / 17.3 | per-frame jitter fitted (same timing on both clips); bars are seeded, drawn on the frames where Filmora shows them |
| No Signal Glitch 01 (grid / scene) | fx003 | `jb.a4.weak_signal` | 16.86 / 20.1 · 5.90 / 10.6 | 15.07 / 18.7 · 5.43 / 10.3 | same model; Filmora's blocky smearing of fine detail is not modelled |
| Glitch line | tr001 | `jb.a4.glitch_flash_cut` | 45.84 / 123.5 | 16.03 / 36.2 | keyed colour-glitch rows (washes, negative, smear, swap, row wave) |
| Pixel Game Transition 09 | tr003 | `jb.a4.pixel_fadeout_swap` | 33.38 / 89.2 | 10.33 / 21.1 | seeded square-cell dropout to a dark screen and back, edges first |
| 3D Warp Box Left | tr008 | `jb.a4.curved_box_turn_left` | 28.71 / 78.2 | 13.69 / 35.5 | two curved quad patches (bilinear corners + edge bulges + perspective) fitted frame by frame; residual is the fine grid under curvature |

## Gaps

Still on their previous mapping (hard cut / lane stand-in), not yet fitted:
3D Warp Box Distortion Left / Right / Up, 3D Warp Box Open Left / Right,
3D Warp Box Rebound, 3D Warp Box Right (the `patch2` model used for 3D Warp
Box Left applies; each needs its own per-frame fit), Depth Mosaic and Depth
Slice (floating cards; Depth Slice keeps the lane 3 `jb.depth_panels`
stand-in), Kaleidoscope, Pixel Sorting Up and Ripple Shake 02 (the ripple
model fitted worse than the plain clip, so no alias was added).

Known residuals: Money's binary engraving screen cannot be phase-matched
(a faint screen over grey measures best); Mosaic's cell grid misaligns on
the fine grid media; Heart Zoom's heart bokeh and particles are not drawn;
glitch items match timing (fitted per frame, deterministic in clip time) and
statistics, not Filmora's exact bar positions.

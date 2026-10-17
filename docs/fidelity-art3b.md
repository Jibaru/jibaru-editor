# Render fidelity, art3b: graphic, cartoon and themed motion transitions

Art3b replaces stand-ins of the earlier lanes (a global A/B mix plus a smooth
colour field: right timing and average colour, no shapes) for graphic,
cartoon and themed motion transitions (shapes, comic bursts, holiday,
seasonal, sports and business graphics, motion-graphics packs) with
recreations drawn with **our own original vector art**. **11 of 66**
items are upgraded; imported projects reach them through the
`filmora:<GUID>` aliases in `crates/jb-render/src/aliases_art3b.rs` (the
stand-in alias of each upgraded GUID was removed from the older lane file,
so every GUID has exactly one mapping).

## Originality

No Filmora asset file was read. The reference exports were only *watched*
(contact sheets, per-frame coverage of A / B / art and the mean colour of
the art pixels) to learn the concept, timing, motion path, coverage over
time, palette and how A and B are revealed. Every shape is authored in code
from primitives (rectangles, ellipses, stars, comic bursts, noisy blobs,
capsules, chevrons, rhombi, hearts, arcs, perspective quads); nothing was
traced, vectorized or lifted from frames, and no third-party asset is used.
Ids and display names are our own descriptive names.

## Machinery

* `crates/jb-render/src/fx/art3b_kit.rs`: an item is a function from
  progress to a `Scene`: the two clips through cameras (zoom, shift,
  rotation, blur), a background mix / dip, and an ordered list of
  primitives. Primitives carry colour, gradients (linear, radial), noise
  fills, outlines, softness, additive light, and texture fills (windows onto
  A or B as they are on screen, or pieces of a clip that move with the
  shape). Keyframe helpers (`kf`, `kfs`, eases, `tri`, `rnd`).
* `crates/jb-render/src/shaders/art3b/prims.wgsl`: background pass, then
  the primitives as signed-distance shapes, six per draw, blended over in
  order.
* `crates/jb-render/src/fx/art3b_items.rs`, `art3b_themed.rs`: the scenes.

## Method

`crates/jb-cli/examples/fidelity.rs` on the reference batch project
(`docs/filmora-references.json`; the item's GUID swapped for our id in a
`.jbp` copy for "after"), item window, `--size 960x540 --search 0`. Values
are MAD mean / max over the window (0–255) with the excess over the floor
in parentheses; the floor is the mean of the clean windows 0.27 s before
and after the transition. Each item was also checked on side-by-side
contact sheets. An alias was switched only when the recreation looks right
and its mean is no worse than the stand-in's (exact pixel match is not
expected from original art: shapes differ in detail, so the worst frame can
be higher where a hard-edged shape is a few pixels off).

## Results

| Filmora item | GUID | Batch | Stand-in | Recreation | Floor | Before | After | Status | Notes |
|---|---|---|---|---|---|---|---|---|---|
| Horrible Party Transition 01 | `C30B1825-A2D9-40C5-8DC5-AAA6C17475C9` | tr003 | `jb.bandage_wrap` | `jb.mummy_wrap` (unlisted draft) | 2.88 | 15.52 / 32.1 (+12.64) | 22.57 / 53.6 (+19.69) | kept stand-in |  |
| Horror Story Transition 01 | `B9FD48EE-56C3-427A-B497-1EAF2916664D` | tr003 | `jb.blood_splash` | `jb.blood_splatter` (unlisted draft) | 2.87 | 5.88 / 16.5 (+3.01) | 9.79 / 24.2 (+6.92) | kept stand-in |  |
| Horror Story Transition 02 | `8169CEC2-0E41-4DCA-AF4E-6850BE909274` | tr003 | `jb.zombie_hands` | `jb.zombie_reach` (unlisted draft) | 2.62 | 12.43 / 25.9 (+9.81) | 13.34 / 34.4 (+10.72) | kept stand-in |  |
| Horrors Pack Transition 01 | `A4A64485-7A1E-4CD7-9621-C4EC79433BAA` | tr003 | `jb.dark_smoke_wash_01` | — | 4.02 | 6.75 / 32.1 (+2.73) | — | stand-in |  |
| Influencer Toolkit Vol 1 Transition 01 | `1E059E6B-7E12-4DC1-A5ED-982CF52EDD8F` | tr006 | `jb.shape_burst_wash_03` | `jb.glossy_slab_sweep` | 2.75 | 24.38 / 48.3 (+21.63) | 22.02 / 41.3 (+19.27) | upgraded |  |
| Layered Vision | `0647A4ED-769A-494E-92EF-34984F81DD18` | tr009 | `jb.panel_shuffle_wash_01` | — | 4.0 | 48.60 / 66.0 (+44.60) | — | stand-in |  |
| Linear 14 | `1C10FA86-5D69-4E77-9F5C-C99D2097D86F` | tr004 | `jb.shape_burst_wash_04` | `jb.chevron_trio_right` | 2.43 | 19.11 / 54.5 (+16.68) | 6.50 / 21.0 (+4.07) | upgraded |  |
| Linear 19 | `C2A21964-21C9-48B3-8E17-3A78E97EC290` | tr004 | `jb.shape_burst_wash_05` | `jb.ring_burst_center` | 2.42 | 24.73 / 61.3 (+22.31) | 16.81 / 59.5 (+14.39) | upgraded |  |
| Linear 4 | `2A59A5A5-A492-425C-904E-D7703218AC16` | tr004 | `jb.shape_burst_wash_06` | `jb.spinning_square_frames` | 2.49 | 23.17 / 53.2 (+20.68) | 15.07 / 39.2 (+12.58) | upgraded |  |
| Love Daily Record Transition 03 | `B4DBCD45-650F-4E17-8FDF-7C13C72932EE` | tr006 | `jb.shape_burst_wash_07` | — | 3.51 | 22.59 / 49.4 (+19.08) | — | stand-in |  |
| Memphis Art Transition 06 | `DD751802-4E0F-42F2-AA81-600665DE369C` | tr002 | `jb.shape_burst_wash_08` | — | 2.41 | 25.46 / 47.0 (+23.05) | — | stand-in |  |
| Metropolis Transition 01 | `24f2ecb0-b8cc-f15c-dc60-9b14a61adc78` | tr006 | `jb.shape_burst_wash_09` | — | 2.62 | 18.36 / 40.2 (+15.74) | — | stand-in |  |
| Modern Transparent Transition 08 | `E51B8E25-E17F-4E9A-96C4-67F07830F4A5` | tr006 | `jb.shape_burst_wash_10` | — | 2.91 | 10.70 / 32.4 (+7.79) | — | stand-in |  |
| Modern Transparent Transition 10 | `C9CA2925-ECC7-421C-8E35-60C9984C1EAA` | tr006 | `jb.shape_burst_wash_11` | — | 2.48 | 6.45 / 29.4 (+3.97) | — | stand-in |  |
| Motion Transitions Pack Vol 3 Transition 14 | `6F64A98D-2C50-4D7F-9314-89E859FB9AD4` | tr004 | `jb.shape_burst_wash_02` | — | 2.44 | 14.55 / 52.1 (+12.11) | — | stand-in |  |
| Motion Transitions Pack Vol 4 Transition 14 | `08C887B6-5597-4A52-BE2B-133E6B6EC9BD` | tr002 | `jb.shape_burst_wash_01` | — | 2.42 | 20.63 / 58.3 (+18.21) | — | stand-in |  |
| Movie Atmosphere CN Transition 07 | `FDFA48E8-C537-46AE-9200-3B952A2030FE` | tr002 | `jb.shape_burst_wash_12` | — | 2.37 | 32.50 / 64.5 (+30.13) | — | stand-in |  |
| News Set Channel 7 Transition 2 | `9c12f97c-2022-6562-8868-6beeecfc32d0` | trfix1 | `jb.shape_burst_wash_14` | — | 2.87 | 15.05 / 86.7 (+12.18) | — | stand-in |  |
| Old Camera | `0D696B4E-F6DB-4160-8C64-2E8B68E933EC` | tr012 | `jb.shutter_wash_03` | — | 4.38 | 24.68 / 52.1 (+20.30) | — | stand-in |  |
| Open Envelope | `2D94F069-FB2F-421A-90B2-7A78A1936F44` | tr009 | `jb.film_frame_wash_02` | — | 3.13 | 23.49 / 68.4 (+20.36) | — | stand-in |  |
| Orb Twist 4 | `e9d23961-f4f6-4e02-9ff9-8f6181ec43e6` | tr005 | — | — | 3.79 | 48.04 / 86.1 (+44.25) | — | stand-in |  |
| Perspective Break | `EF9351C0-EA39-4190-93FA-E6633052C8E1` | tr009 | `jb.panel_shuffle_wash_02` | — | 3.7 | 48.73 / 66.6 (+45.03) | — | stand-in |  |
| Photo Spread | `8DCB22B3-BF4F-4523-AF46-E121A5F0E4EB` | tr012 | `jb.film_frame_wash_03` | — | 2.8 | 32.35 / 50.2 (+29.55) | — | stand-in |  |
| Pinwheel | `668D3D30-43B8-41ef-A222-86E7912A497E` | tr005 | `jb.overlay_wash_01` | — | 2.92 | 16.60 / 30.4 (+13.68) | — | stand-in |  |
| Plain Shape 17 | `3FC5FC6D-C669-40FA-B1AF-308BED370634` | tr004 | `jb.shape_burst_wash_15` | `jb.square_frames_zoom` | 2.37 | 23.27 / 54.2 (+20.90) | 19.39 / 57.1 (+17.02) | upgraded |  |
| Plain Shape 20 | `CEC1827B-745E-4EB4-AD1B-A1EBEE58FF0D` | tr004 | `jb.shape_burst_wash_16` | `jb.hex_windows` | 2.39 | 8.69 / 32.5 (+6.30) | 6.56 / 19.3 (+4.17) | upgraded |  |
| Plain Shape 22 | `554FEC3F-E022-46A6-B763-67E2E9D6BA7B` | tr004 | `jb.shape_burst_wash_17` | `jb.white_ring_iris` | 2.43 | 9.41 / 34.2 (+6.98) | 6.32 / 18.2 (+3.89) | upgraded |  |
| Plain Shape 8 | `33745F8B-AE5A-4DC2-8C1B-1C61735B5A85` | tr004 | `jb.shape_burst_wash_18` | `jb.orange_ring_rise` | 2.45 | 12.39 / 39.5 (+9.94) | 11.03 / 35.4 (+8.58) | upgraded |  |
| Plain Shape 9 | `1E111A76-BC1D-4BE8-9315-79CB49CC8C36` | tr004 | `jb.shape_burst_wash_19` | `jb.red_ring_iris` | 2.46 | 13.85 / 34.4 (+11.39) | 8.21 / 21.9 (+5.75) | upgraded |  |
| Powerful Sports Team Transition 02 | `CE1BB859-98C2-4E9F-90D0-DE880C931F12` | tr001 | `jb.shape_burst_wash_20` | — | 2.95 | 11.30 / 34.7 (+8.35) | — | stand-in |  |
| Profile Movie Transition 01 | `CAD04901-7A9A-4172-A68F-82F3E1781E52` | tr002 | `jb.film_frame_wash_04` | — | 2.79 | 46.40 / 61.7 (+43.61) | — | stand-in |  |
| Random Dissolve 01 | `6CAAC772-2684-4C71-94A3-CAF24434F4C2` | tr006 | `jb.cell_dissolve_wash_01` | — | 2.92 | 8.39 / 18.4 (+5.47) | — | stand-in |  |
| Retro blockbusters 1 | `2C7ED650-4815-402C-9A04-B88F4CE70A2E` | tr012 | `jb.film_frame_wash_05` | — | 4.08 | 25.09 / 57.6 (+21.01) | — | stand-in |  |
| Retro blockbusters 2 | `AFE78312-109F-4F35-9A33-F8A59AA71F6F` | tr012 | `jb.film_frame_wash_06` | — | 2.92 | 14.75 / 57.5 (+11.83) | — | stand-in |  |
| Retro blockbusters 4 | `DA41C3BF-3DE7-4FC0-A4E9-92E58D72CF5D` | tr012 | `jb.film_frame_wash_07` | — | 4.5 | 35.49 / 51.5 (+30.99) | — | stand-in |  |
| Row Cos Whisk | `C8C1ED9D-959D-453f-8EFD-93D5BF7F5B23` | trfix1 | `jb.stripe_matte_wash_01` | — | 2.45 | 12.48 / 28.5 (+10.03) | — | stand-in |  |
| Rustic Transition 4 | `69dc5a52-5a08-717e-c9ba-be04df8a1fa4` | tr012 | `jb.paint_stroke_reveal_21` | — | 2.39 | 5.87 / 21.3 (+3.48) | — | stand-in |  |
| Scary Pack Transition 01 | `5E841241-463E-448A-B928-3C9EE0D0E3A0` | tr003 | `jb.dark_smoke_wash_02` | — | 4.05 | 20.63 / 48.4 (+16.58) | — | stand-in |  |
| Scary Pack Transition 02 | `BC27EE2C-8526-45DA-97BF-CF75EEB428C0` | tr003 | `jb.dark_smoke_wash_03` | — | 4.04 | 20.14 / 39.8 (+16.10) | — | stand-in |  |
| Scary Pack Transition 03 | `F2BCEA67-4D6A-4F1F-8729-3ADAD68083A5` | tr003 | `jb.dark_smoke_wash_04` | — | 3.63 | 16.20 / 32.7 (+12.57) | — | stand-in |  |
| Scary Pack Transition 04 | `9C17871B-8EBA-4D88-8E99-98318012A0CA` | tr003 | `jb.dark_smoke_wash_05` | — | 3.88 | 25.51 / 41.6 (+21.63) | — | stand-in |  |
| Scary Pack Transition 08 | `AE6FC8AC-655B-458D-95E0-4D03F7748C23` | tr003 | `jb.dark_smoke_wash_06` | — | 4.11 | 23.35 / 40.0 (+19.24) | — | stand-in |  |
| Scoreboard Transition 01 | `5DC77869-75E6-45F6-9BCA-9B96CBA83DEC` | tr002 | `jb.shape_burst_wash_21` | `jb.red_x_panels` | 2.51 | 12.77 / 43.1 (+10.26) | 9.69 / 28.3 (+7.18) | upgraded |  |
| Shape Gradient Transitions Transition 25 | `108B1920-9128-4983-BAC9-FB4100603A8E` | tr005 | `jb.shape_burst_wash_22` | `jb.diagonal_streak_sweep` | 2.67 | 20.53 / 49.1 (+17.86) | 18.80 / 56.3 (+16.13) | upgraded |  |
| Shape Transition 106 | `D8775488-C02D-4661-937D-4C3C937D4ACB` | tr005 | `jb.cell_dissolve_wash_02` | — | 2.38 | 7.06 / 29.3 (+4.68) | — | stand-in |  |
| Space Rotation Left | `401962DA-6B63-4EB0-95C5-5C43A04601DE` | tr009 | `jb.panel_shuffle_wash_03` | — | 4.12 | 41.18 / 71.4 (+37.06) | — | stand-in |  |
| Special Effects Skill Pack Transition 01 | `E943F100-16E7-4F2D-8821-A26E4C94A201` | tr010 | `jb.cartoon_burst_wash_04` | — | 3.68 | 16.44 / 52.2 (+12.76) | — | stand-in |  |
| Special Effects Skill Transition 03 | `D9AEBB98-B58B-4EC1-9750-F17BC822E7CB` | tr010 | `jb.cartoon_burst_wash_05` | — | 2.77 | 23.95 / 63.4 (+21.18) | — | stand-in |  |
| Special Effects Skill Transition 04 | `AE4878F8-8587-448A-9065-37C631D0AB51` | tr010 | `jb.cartoon_burst_wash_06` | — | 2.48 | 12.57 / 45.4 (+10.09) | — | stand-in |  |
| Special Effects Skill Transition 05 | `58BDBBA2-D53A-4331-83BF-9C0BF0AA1D48` | tr010 | `jb.cartoon_burst_wash_07` | — | 2.86 | 30.37 / 59.4 (+27.51) | — | stand-in |  |
| Special FX Motion Transition 04 | `efbf96b4-211e-ef11-ba94-8045dd702285` | tr010 | `jb.cartoon_burst_wash_01` | — | 3.02 | 15.54 / 62.4 (+12.52) | — | stand-in |  |
| Speed Motivation Transition 01 | `6cdc4237-8c92-ef11-b433-d8bbc19410ef` | tr010 | `jb.cartoon_burst_wash_02` | — | 2.88 | 38.48 / 56.7 (+35.60) | — | stand-in |  |
| Spiral Rise | `34F473F4-EEB9-4606-926D-8FE2540CF7AB` | tr010 | `jb.panel_shuffle_wash_04` | — | 4.14 | 58.75 / 68.0 (+54.61) | — | stand-in |  |
| Sports Broadcast Transition 04 | `657A3B6F-B23F-4804-98EB-3213FF54E84F` | tr001 | `jb.shape_burst_wash_23` | — | 2.58 | 14.08 / 35.7 (+11.50) | — | stand-in |  |
| Sports Pack Transition 05 | `1E9976C8-24BF-4AA2-934D-A533FC015AC0` | tr006 | `jb.shape_burst_wash_24` | — | 3.13 | 29.19 / 55.9 (+26.06) | — | stand-in |  |
| Swap Left | `C3612776-D62E-41a2-8733-0429F50E11F5` | tr005 | `jb.overlay_wash_02` | — | 3.48 | 52.53 / 59.4 (+49.05) | — | stand-in |  |
| Technology HUD Transition 01 | `C9C2E39A-F581-4D9D-B1EF-140FC7F45798` | tr003 | `jb.cell_dissolve_wash_03` | — | 2.49 | 9.80 / 28.7 (+7.31) | — | stand-in |  |
| Top Bottom Gate | `C3043949-5B2B-444E-9079-3A7B7B2F52A1` | tr010 | `jb.panel_shuffle_wash_05` | — | 3.97 | 32.13 / 48.3 (+28.16) | — | stand-in |  |
| Vertical Cutout 1 | `F2F3C70C-0ADF-41BE-B4EB-B2DDC98CDFD9` | tr010 | `jb.panel_shuffle_wash_06` | — | 3.54 | 27.26 / 42.5 (+23.72) | — | stand-in |  |
| Vertical Slice | `3B9CFBA3-EEE5-4C4D-A8AA-457A8B4044C9` | tr001 | `jb.glitch_strobe_wash_01` | — | 3.24 | 17.25 / 34.9 (+14.01) | — | stand-in |  |
| Vintage Glamour Transition 02 | `A6F74AF0-069F-4B0B-985E-9032ED630409` | tr012 | `jb.paint_stroke_reveal_22` | — | 2.4 | 9.72 / 18.9 (+7.32) | — | stand-in |  |
| Vision Blocks | `CA0C9A79-A8F7-4B25-AC48-942275E756A9` | tr010 | `jb.panel_shuffle_wash_07` | — | 3.48 | 34.05 / 65.3 (+30.57) | — | stand-in |  |
| Wedding Invitation Template Transition 01 | `9CC68058-8C13-4EC4-8B59-5BB5752CBEDD` | tr012 | `jb.paint_stroke_reveal_23` | — | 2.44 | 11.59 / 24.4 (+9.15) | — | stand-in |  |
| Wedding Invitation Template Transition 02 | `97426E0D-3458-4316-877B-4FD00C507D82` | tr012 | `jb.paint_stroke_reveal_24` | — | 2.48 | 9.16 / 21.5 (+6.68) | — | stand-in |  |
| Wedding Invitation Template Transition 04 | `546B3563-6DE9-44D1-9386-390BA3D3E73B` | tr012 | `jb.paint_stroke_reveal_25` | — | 2.45 | 8.81 / 23.4 (+6.36) | — | stand-in |  |
| Year of the Tiger Pack Transition 02 | `B3DB8933-F2E5-4094-975D-2595F0121FC2` | tr007 | `jb.sparkle_trail_wash_06` | — | 2.46 | 6.14 / 32.2 (+3.68) | — | stand-in |  |

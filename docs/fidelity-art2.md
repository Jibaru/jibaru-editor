# Render fidelity: art lane 2 (brush, paint, ink and paper transitions)

Brush-stroke, paint, gold-leaf, watercolour, ink and torn-paper transitions that
lanes 1 and 3 rendered with stand-ins (a fitted global colour mix that keeps the
timing and average colour but draws no strokes), recreated with **original
procedural artwork**. Code: `crates/jb-render/src/fx/art2_paint.rs` (stroke kit
and generator), `fx/art2_items.rs` and `fx/art2_gold.rs` (items as stroke data),
`shaders/art2/paint.wgsl`; GUIDs in `crates/jb-render/src/aliases_art2.rs` (one
mapping per GUID: the replaced stand-in alias was removed from
`aliases_lane1.rs` / `aliases_lane3.rs`; the stand-in effects stay registered).

## Originality

Every stroke is generated procedurally: a signed-distance brush along a
quadratic path with a bristle-ragged head (bristle clumps of uneven length),
noisy sides, dry-brush streak gaps that close as the paint soaks in, watercolour
mottling and blooms, torn-paper fibre edges, and paints (gold leaf, paper) with
bristle grooves, sheen, wet-edge darkening and grain. Filmora's installed assets
were never read. The reference exports were only watched to learn each item's
concept, timing, motion direction, coverage over time and palette; no frame or
shape was traced, vectorized or copied. No third-party assets.

## Method

Fidelity harness `crates/jb-cli/examples/fidelity.rs` at `--size 960x540
--search 0` over the item window of its batch (`docs/filmora-references.json`).
Values are MAD mean / worst frame. "Floor" is the mean MAD of the clean frames
just before and after the window. "Before" is the stand-in, "After" our item.
An alias was switched only when the contact sheet (reference over ours) looks
right and the mean MAD is lower than the stand-in's.

24 of 31 items upgraded.

| Item | GUID | Renders as | Floor | Before | After | Notes |
|---|---|---|---|---|---|---|
| Brush Art Design Pack Transition 08 | `3B7844DB-2D95-4136-BE2E-4D59D3FB0CF8` | jb.brush_columns | 2.37 | 20.19 / 29.28 | 7.71 / 10.32 |  |
| Paper Tear Transition 20 | `9ACD289B-0E2C-4A81-8BFF-687BAEB6FC56` | jb.torn_paper_hole | 2.29 | 14.08 / 45.26 | 9.70 / 35.67 |  |
| Beauty Brush Transition 1 | `b20e8acf-35d7-4958-aebc-64d2358c6b97` | jb.dry_brush_sweeps | 2.40 | 9.52 / 29.15 | 7.79 / 21.82 |  |
| Beauty Brush Transition 2 | `2fe80ebb-e89e-4de9-b669-c9de540d15ea` | jb.swelling_brush_stroke | 2.39 | 12.21 / 28.85 | 7.16 / 26.86 |  |
| Grunge Scribble Vlog Transition 02 | `9C246287-A006-4F7D-A161-BAB117D39BAB` | jb.paper_roll_peel | 2.52 | 6.92 / 32.17 | 6.41 / 32.65 | our curl is a flat glossy band; the reference curl is a 3D roll (worst frame slightly higher) |
| Torn Paper Transition 09 | `0EB21BC5-6121-40A3-987A-217661816A98` | jb.torn_paper_rift | 2.88 | 12.11 / 33.28 | 6.86 / 21.69 |  |
| Brush Drift Transition 01 | `7d98be69-834b-f111-9e87-50ebf65600b6` | jb.brush_drift_three | 2.30 | 7.94 / 27.97 | 4.28 / 21.47 |  |
| Brush Drift Transition 02 | `8298be69-834b-f111-9e87-50ebf65600b6` | jb.diagonal_brush_drift | 2.30 | 8.06 / 26.93 | 5.23 / 19.21 |  |
| Brush Drift Transition 03 | `8198be69-834b-f111-9e87-50ebf65600b6` | jb.rising_brush_drift | 2.27 | 7.63 / 25.89 | 5.03 / 19.60 |  |
| Brush Drift Transition 04 | `7f98be69-834b-f111-9e87-50ebf65600b6` | jb.crossing_brush_drift | 2.27 | 7.61 / 27.36 | 6.39 / 24.31 |  |
| Brush Drift Transition 05 | `7e98be69-834b-f111-9e87-50ebf65600b6` | jb.rounded_brush_drift | 2.27 | 7.96 / 28.29 | 5.77 / 27.94 |  |
| Brush Drift Transition 06 | `8398be69-834b-f111-9e87-50ebf65600b6` | jb.paused_diagonal_drift | 2.27 | 11.43 / 27.56 | 5.18 / 17.38 |  |
| Brush Drift Transition 07 | `8098be69-834b-f111-9e87-50ebf65600b6` | jb.closing_paint_edges | 2.27 | 8.06 / 28.63 | 5.01 / 18.58 |  |
| Brush Drift Transition 08 | `8498be69-834b-f111-9e87-50ebf65600b6` | jb.watercolor_strips | 2.27 | 10.91 / 22.89 | 9.96 / 16.93 |  |
| Brush Drift Transition 09 | `8598be69-834b-f111-9e87-50ebf65600b6` | jb.watercolor_bands | 2.31 | 6.32 / 23.02 | 5.72 / 21.07 |  |
| Brush Drift Transition 10 | `8698be69-834b-f111-9e87-50ebf65600b6` | jb.ink_blooms | 2.33 | 7.84 / 22.56 | 6.08 / 15.60 |  |
| Colorful Brush Transition 05 | `d1c03638-dc96-f111-b463-d8bbc19410ee` | stand-in | 2.54 | 8.19 / 20.86 | — |  |
| Colorful Brush Transition 07 | `cfc03638-dc96-f111-b463-d8bbc19410ee` | stand-in | 2.70 | 6.86 / 20.79 | — |  |
| Golden Brush Transition 01 | `c826f80c-db96-f111-b463-d8bbc19410ee` | jb.gold_tide | 2.62 | 19.18 / 38.46 | 15.40 / 28.25 |  |
| Golden Brush Transition 02 | `de3499f9-d196-f111-b463-d8bbc19410ee` | jb.gold_curtain | 2.39 | 20.11 / 38.09 | 16.14 / 33.92 |  |
| Golden Brush Transition 03 | `c726f80c-db96-f111-b463-d8bbc19410ee` | jb.gold_cross_strokes | 2.40 | 18.59 / 41.80 | 17.46 / 40.33 | full-gold frames are near the texture-noise limit (uncorrelated brushing) |
| Golden Brush Transition 04 | `db3499f9-d196-f111-b463-d8bbc19410ee` | jb.gilded_edge_fall | 2.44 | 13.80 / 28.55 | 7.28 / 16.10 |  |
| Golden Brush Transition 05 | `07868c04-ce96-f111-b463-d8bbc19410ee` | jb.gold_led_columns | 2.62 | 15.45 / 31.71 | 12.16 / 25.72 |  |
| Golden Brush Transition 06 | `06868c04-ce96-f111-b463-d8bbc19410ee` | jb.gold_slash_run | 2.58 | 11.48 / 38.70 | 8.90 / 29.34 |  |
| Golden Brush Transition 07 | `05868c04-ce96-f111-b463-d8bbc19410ee` | jb.gilded_torn_edge | 2.32 | 11.12 / 28.52 | 6.87 / 16.44 |  |
| Golden Brush Transition 08 | `df3499f9-d196-f111-b463-d8bbc19410ee` | jb.gold_vortex | 2.59 | 23.17 / 39.47 | 20.71 / 43.67 | worst frame higher than the stand-in (the reference scatters speckles at the vortex edge) |
| Stylish Brush Transition 03 | `CF7E8635-849C-4D99-920C-3BEBDE7E70B2` | stand-in | 2.31 | 13.69 / 29.08 | — |  |
| Stylish Brush Transition 09 | `115818EE-EEF8-4904-AA17-6D3571B9B2BB` | stand-in | 2.29 | 9.98 / 27.50 | — |  |
| Stylish Brush Transition 12 | `309343C0-ED56-491B-9EEB-2C5BF3F553AA` | stand-in | 2.35 | 15.94 / 28.72 | — |  |
| Stylish Brush Transition 17 | `D725AB61-3F6B-44F9-8059-128298182DCC` | stand-in | 2.43 | 9.89 / 26.81 | — |  |
| Stylish Brush Transition 20 | `38E486E2-3A8B-4628-8E29-4D6E17EE9867` | stand-in | 2.37 | 15.72 / 28.15 | — |  |

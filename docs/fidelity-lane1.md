# Lane 1 fidelity: zoom / spin / blur / camera transitions and misc wipes

Filmora 15.7.3 store transitions fitted on its own exports of the reference batches
(`docs/filmora-references.md`: one transition per cut between the gridA / gridB test cards,
2.002 s centred on the cut) and measured with the fidelity harness over the item window:

```sh
cargo run --release -p jb-cli --example fidelity -- batches/trNNN.wfp exports/trNNN.mp4 \
    --range START-END --size 960x540 --search 0
```

## Method

- **Table camera moves** (`fx/lane1_table.rs`, `shaders/lane1/table.wgsl`): each side is the clip seen
  through a camera (shift, zoom, rotation, optional perspective) with spin / zoom / motion blur, and
  the frame is a per-channel display-space mix `al * A' + be * B' + ga` (dissolves, flashes, fades,
  tints) with an optional layer order for transparent edges. The parameters are fitted per frame
  of the export (Nelder-Mead on the geometry and blur, least squares on the mix) against the
  clean frames just outside the window, then reduced to the keyframes linear interpolation needs.
- **Parametric soft wipes** (`fx/lane1_mask.rs`, `shaders/lane1/mask.wgsl`): the B weight of each
  frame is measured (`<ref - A, B - A> / |B - A|^2`) and fitted with a soft mask family (linear,
  circle, clock sweep, pinwheel, star) whose position, edge width and rotation are tabulated.
- Only Filmora's exported frames were used; no Filmora asset was read.

**Floor.** The plain test cards themselves differ from Filmora's export by MAD 4.1 (gridA) and
5.0 (gridB) at 960x540 (a tone / level mismatch on these synthetic clips, R and B about -2.7,
plus the 6 Mbit/s encode of a detailed card), so a perfect transition measures about
4.1–5.0 here, not the 1.5–2.5 of natural footage. "Excess" is the window mean minus the mean
of the two clip floors.

## Results

| Filmora item | GUID | jb id | mean | worst | excess | notes |
|---|---|---|---:|---:|---:|---|
| Slow Bounce | `F9A10F75-ECFC-4DEC-B7B8-BDFB457FBCA0` | `jb.slow_bounce` | 4.88 | 6.66 | +0.33 |  |
| Linear Wipe | `FD4A4891-4E1C-4e05-B290-8195CC55E8C5` | `jb.soft_wipe_diagonal` | 4.56 | 5.30 | +0.01 |  |
| Linear Wipe Down | `C456899A-C306-4f88-ACD9-5B158A5B1F48` | `jb.soft_wipe_down` | 4.73 | 6.32 | +0.18 |  |
| Linear Wipe Right | `6675F015-A329-4ee9-9A89-E5427799D0BD` | `jb.soft_wipe_right` | 4.60 | 5.37 | +0.05 |  |
| Round Zoom In | `ABBD7AED-1384-4fd2-862B-E955E5F9C3AE` | `jb.soft_iris_close` | 6.20 | 7.09 | +1.65 |  |
| Sweep Clockwise | `74876B99-7AD0-49b3-91D4-D3136E9A850A` | `jb.soft_clock_sweep` | 4.68 | 6.06 | +0.13 |  |
| TransBlendBase | `836D2042-F8B9-4E20-96E2-CBD409DB3EDA` | `jb.star_reveal` | 4.89 | 7.53 | +0.34 |  |
| Fast Bounce | `A6E8974E-227F-4F80-8EC0-8F88DC0B8309` | `jb.fast_bounce` | 5.62 | 7.70 | +1.07 |  |
| Fast Spin 2 | `108D84A2-AE20-4A99-A4FD-1EA595812893` | `jb.fast_spin_blur` | 4.04 | 16.18 | -0.51 |  |
| Camera Flare 04 | `71520D52-8F60-4B84-9C43-3C6F34BA37BE` | `jb.pink_flash_cut` | 6.63 | 18.33 | +2.08 | the one-frame pink flash peak is off by up to 18 (worst frames) |
| My_Daily_Vlog_Vol2_Pack_Transition_01 | `B02B7916-3BC6-454F-B737-856E162130C0` | `jb.soft_blur_dissolve` | 5.53 | 6.21 | +0.98 |  |
| Shadow | `BC3CFF30-291A-4995-852D-689CE6A1E1CF` | `jb.scanline_smear` | 5.87 | 7.06 | +1.32 |  |
| The ending black | `fa7ae951-4577-ef11-8851-f44efc291984` | `jb.ending_fade_black` | 2.59 | 5.22 | -1.96 |  |
| Transition 02 | `B9DE5D35-4C24-489A-A656-822DB775B1AF` | `jb.tinted_dissolve` | 5.95 | 7.69 | +1.40 |  |
| fade2 | `B8437D46-123B-425E-B6E6-C091353AE98D` | `jb.warm_wash_dissolve` | 5.01 | 6.08 | +0.46 |  |
| Simple Sliding | `D4A854F2-BCAF-49BA-A160-DDD46CA15655` | `jb.card_slide_up` | 6.93 | 13.40 | +2.38 |  |
| Row Merge | `47EEB6BF-8E99-488f-BCD8-ABD73A495CB4` | `jb.band_close` | 5.04 | 5.68 | +0.49 |  |
| Laser Beam Blue | `2D9EDF1E-BB2B-4BC1-B455-6FAE3B00BD6F` | `jb.laser_band_blue` | 4.96 | 5.79 | +0.41 |  |
| Laser Beam Red | `70D8169D-7B37-4864-9E4B-7593C646ECD3` | `jb.laser_band_red` | 5.13 | 6.11 | +0.58 |  |
| Torn Paper Transition 10 | `44ECCE7F-73FE-4815-8C3C-B1E14B73926F` | `jb.torn_paper_rise` | 5.96 | 8.34 | +1.41 |  |
| Camera Shutter | `582C0031-3A5A-4839-86C2-BD5662120FAD` | `jb.shutter_wash_01` | 11.36 | 46.47 | +6.81 | camera body / aperture overlay |
| Camera Transition 09 | `0A51BCC4-B46C-40E3-AB2E-072317A483D0` | `jb.shutter_wash_02` | 14.77 | 49.29 | +10.22 | camera body / aperture overlay |
| Motion Transitions Pack Vol 4 Transition 14 | `08C887B6-5597-4A52-BE2B-133E6B6EC9BD` | `jb.shape_burst_wash_01` | 22.01 | 58.57 | +17.46 | animated graphic shapes overlay |
| Brush Art Design Pack Transition 08 | `3B7844DB-2D95-4136-BE2E-4D59D3FB0CF8` | `jb.paint_stroke_reveal_01` | 21.47 | 30.15 | +16.92 | brush-stroke matte (hand-painted stroke animation) |
| Motion Transitions Pack Vol 3 Transition 14 | `6F64A98D-2C50-4D7F-9314-89E859FB9AD4` | `jb.shape_burst_wash_02` | 16.38 | 52.57 | +11.83 | animated graphic shapes overlay |
| Drop Bounce | `8EF84F2E-C203-4f98-84D1-959EF228456D` | `jb.drop_bounce` | 7.74 | 12.02 | +3.19 | the dropping clip wraps around with a strip of the outgoing clip at the bottom; the camera model reaches 7.7 / 12, not shipped |
| Beauty Brush Transition 1 | `b20e8acf-35d7-4958-aebc-64d2358c6b97` | `jb.paint_stroke_reveal_02` | 11.36 | 29.86 | +6.81 | brush-stroke matte (hand-painted stroke animation) |
| Beauty Brush Transition 2 | `2fe80ebb-e89e-4de9-b669-c9de540d15ea` | `jb.paint_stroke_reveal_03` | 13.99 | 29.70 | +9.44 | brush-stroke matte (hand-painted stroke animation) |
| Sparkle Motion Transition 07 | `9EE833DD-A002-4583-8ECB-AB2CD8A8FD7C` | `jb.sparkle_trail_wash_01` | 17.00 | 36.73 | +12.45 | particle / sparkle overlay |
| Sparkle Motion Transition 11 | `E8A47581-CE66-4BF5-AFC8-685A3F68487F` | `jb.sparkle_trail_wash_02` | 22.34 | 37.38 | +17.79 | particle / sparkle overlay |
| Sparkle Motion Transition 13 | `B35B100A-817A-499C-BA07-8E6E3C7DC4C8` | `jb.sparkle_trail_wash_03` | 25.00 | 42.85 | +20.45 | particle / sparkle overlay |
| Sparkle Motion Transition 15 | `F8A69A99-8CDD-42D3-9B38-9937F55EEEF5` | `jb.sparkle_trail_wash_04` | 20.04 | 40.04 | +15.49 | particle / sparkle overlay |
| Sparkle Motion Transition 20 | `8B70FA91-0F25-442F-9624-DE656D5E0112` | `jb.sparkle_trail_wash_05` | 22.49 | 40.05 | +17.94 | particle / sparkle overlay |
| Blurry Ejection | `D8287205-6EC2-2DE1-69D6-4D216A398FB3` | `jb.ejected_card` | 29.57 | 65.71 | +25.02 |  |
| Special FX Motion Transition 04 | `efbf96b4-211e-ef11-ba94-8045dd702285` | `jb.cartoon_burst_wash_01` | 16.64 | 62.39 | +12.09 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Speed Motivation Transition 01 | `6cdc4237-8c92-ef11-b433-d8bbc19410ef` | `jb.cartoon_burst_wash_02` | 38.79 | 56.70 | +34.24 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Brush Drift Transition 01 | `7d98be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_04` | 9.97 | 28.84 | +5.42 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 02 | `8298be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_05` | 10.17 | 27.82 | +5.62 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 03 | `8198be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_06` | 9.76 | 26.68 | +5.21 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 04 | `7f98be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_07` | 9.76 | 28.19 | +5.21 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 05 | `7e98be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_08` | 10.09 | 29.16 | +5.54 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 06 | `8398be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_09` | 13.42 | 28.38 | +8.87 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 07 | `8098be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_10` | 10.19 | 29.43 | +5.64 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 08 | `8498be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_11` | 12.67 | 23.74 | +8.12 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 09 | `8598be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_12` | 8.23 | 23.99 | +3.68 | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 10 | `8698be69-834b-f111-9e87-50ebf65600b6` | `jb.paint_stroke_reveal_13` | 9.67 | 23.37 | +5.12 | brush-stroke matte (hand-painted stroke animation) |
| Colorful Brush Transition 05 | `d1c03638-dc96-f111-b463-d8bbc19410ee` | `jb.paint_stroke_reveal_14` | 9.44 | 21.46 | +4.89 | brush-stroke matte (hand-painted stroke animation) |
| Colorful Brush Transition 07 | `cfc03638-dc96-f111-b463-d8bbc19410ee` | `jb.paint_stroke_reveal_15` | 8.07 | 21.57 | +3.52 | brush-stroke matte (hand-painted stroke animation) |
| Golden Brush Transition 01 | `c826f80c-db96-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_01` | 20.00 | 38.77 | +15.45 | painted gold brush overlay |
| Golden Brush Transition 02 | `de3499f9-d196-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_02` | 20.83 | 38.29 | +16.28 | painted gold brush overlay |
| Golden Brush Transition 03 | `c726f80c-db96-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_03` | 19.47 | 42.21 | +14.92 | painted gold brush overlay |
| Golden Brush Transition 04 | `db3499f9-d196-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_04` | 15.44 | 29.25 | +10.89 | painted gold brush overlay |
| Golden Brush Transition 05 | `07868c04-ce96-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_05` | 16.61 | 32.28 | +12.06 | painted gold brush overlay |
| Golden Brush Transition 06 | `06868c04-ce96-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_06` | 13.31 | 39.15 | +8.76 | painted gold brush overlay |
| Golden Brush Transition 07 | `05868c04-ce96-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_07` | 12.98 | 29.21 | +8.43 | painted gold brush overlay |
| Golden Brush Transition 08 | `df3499f9-d196-f111-b463-d8bbc19410ee` | `jb.gold_paint_reveal_08` | 24.15 | 39.70 | +19.60 | painted gold brush overlay |
| Stylish Brush Transition 03 | `CF7E8635-849C-4D99-920C-3BEBDE7E70B2` | `jb.paint_stroke_reveal_16` | 15.33 | 29.96 | +10.78 | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 09 | `115818EE-EEF8-4904-AA17-6D3571B9B2BB` | `jb.paint_stroke_reveal_17` | 11.86 | 28.32 | +7.31 | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 12 | `309343C0-ED56-491B-9EEB-2C5BF3F553AA` | `jb.paint_stroke_reveal_18` | 17.34 | 29.55 | +12.79 | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 17 | `D725AB61-3F6B-44F9-8059-128298182DCC` | `jb.paint_stroke_reveal_19` | 11.59 | 27.71 | +7.04 | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 20 | `38E486E2-3A8B-4628-8E29-4D6E17EE9867` | `jb.paint_stroke_reveal_20` | 17.15 | 29.05 | +12.60 | brush-stroke matte (hand-painted stroke animation) |
| Camera Viewfinder 2 | `72B62645-7A9F-48E6-AEF3-43E57C5118D3` | `jb.viewfinder_blur_cut` | 6.59 | 17.71 | +2.04 |  |
| Old Camera | `0D696B4E-F6DB-4160-8C64-2E8B68E933EC` | `jb.shutter_wash_03` | 24.45 | 51.85 | +19.90 | camera body / aperture overlay |
| Horrors Pack Transition 01 | `A4A64485-7A1E-4CD7-9621-C4EC79433BAA` | `jb.dark_smoke_wash_01` | 8.20 | 32.40 | +3.65 | horror overlay footage |
| Ice Firework Effect Vol 02 Transition 10 | `F1FF9C55-AEDE-4F01-A698-3ABC9C69B682` | `jb.cartoon_burst_wash_03` | 15.95 | 52.51 | +11.40 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Influencer Toolkit Vol 1 Transition 01 | `1E059E6B-7E12-4DC1-A5ED-982CF52EDD8F` | `jb.shape_burst_wash_03` | 25.13 | 48.51 | +20.58 | animated graphic shapes overlay |
| Jitter 01 | `6AE378FF-2240-4716-ACFB-86E591534441` | `jb.flash_jitter` | 14.35 | 33.10 | +9.80 | glitch / strobe flashes with RGB split |
| Layer down | `3E674127-F735-43A4-90F0-261C96499A99` | `jb.layer_drop_down` | 9.07 | 26.71 | +4.52 |  |
| Layer up | `327C8A9F-49E9-4A46-BA26-63E1AF2F2E04` | `jb.layer_rise_up` | 9.10 | 29.24 | +4.55 |  |
| Layered Vision | `0647A4ED-769A-494E-92EF-34984F81DD18` | `jb.panel_shuffle_wash_01` | 47.24 | 64.72 | +42.69 | 3D multi-panel layout |
| Light Leaks Transition 01 | `C90D0AD6-EECC-496A-8254-2BF38D46D6A1` | `jb.leak_wash_01` | 8.13 | 26.93 | +3.58 | light leak / bokeh overlay footage |
| Light Leaks Transition 02 | `B288EAEA-E306-4E5A-8BBB-4AE8F52B65AC` | `jb.leak_wash_02` | 14.64 | 25.09 | +10.09 | light leak / bokeh overlay footage |
| Light Leaks Transition 11 | `5FCF4746-3646-415B-9884-A9ECED01EBE7` | `jb.leak_wash_03` | 24.83 | 37.11 | +20.28 | light leak / bokeh overlay footage |
| Light Leaks Transition Pack Transition 03 | `DE3A553C-34A6-4EA4-B65D-BAC68391C2A1` | `jb.leak_wash_04` | 20.74 | 45.83 | +16.19 | light leak / bokeh overlay footage |
| Light Letters Pack Transition 02 | `250E53BD-AFF4-494C-B7D8-57C0D8E89B95` | `jb.leak_wash_05` | 21.09 | 38.19 | +16.54 | light leak / bokeh overlay footage |
| Linear 14 | `1C10FA86-5D69-4E77-9F5C-C99D2097D86F` | `jb.shape_burst_wash_04` | 20.55 | 54.65 | +16.00 | animated graphic shapes overlay |
| Linear 19 | `C2A21964-21C9-48B3-8E17-3A78E97EC290` | `jb.shape_burst_wash_05` | 25.99 | 61.55 | +21.44 | animated graphic shapes overlay |
| Linear 4 | `2A59A5A5-A492-425C-904E-D7703218AC16` | `jb.shape_burst_wash_06` | 24.39 | 53.50 | +19.84 | animated graphic shapes overlay |
| Love Daily Record Transition 03 | `B4DBCD45-650F-4E17-8FDF-7C13C72932EE` | `jb.shape_burst_wash_07` | 23.00 | 49.53 | +18.45 | animated graphic shapes overlay |
| MaskBlend | `A1B2C3D4-5E6F-7890-ABCD-EF1234567890` | `jb.cyan_mask_blend` | 9.52 | 9.78 | +4.97 |  |
| Memphis Art Transition 06 | `DD751802-4E0F-42F2-AA81-600665DE369C` | `jb.shape_burst_wash_08` | 26.67 | 47.31 | +22.12 | animated graphic shapes overlay |
| Metropolis Transition 01 | `24f2ecb0-b8cc-f15c-dc60-9b14a61adc78` | `jb.shape_burst_wash_09` | 19.32 | 40.43 | +14.77 | animated graphic shapes overlay |
| Modern Transparent Transition 08 | `E51B8E25-E17F-4E9A-96C4-67F07830F4A5` | `jb.shape_burst_wash_10` | 12.33 | 33.02 | +7.78 | animated graphic shapes overlay |
| Modern Transparent Transition 10 | `C9CA2925-ECC7-421C-8E35-60C9984C1EAA` | `jb.shape_burst_wash_11` | 8.34 | 30.17 | +3.79 | animated graphic shapes overlay |
| Movie Atmosphere CN Transition 07 | `FDFA48E8-C537-46AE-9200-3B952A2030FE` | `jb.shape_burst_wash_12` | 32.91 | 63.65 | +28.36 | animated graphic shapes overlay |
| Neon Valentine Promo Transition 07 | `4EECB05F-72B3-434F-B729-824D35B2A29C` | `jb.shape_burst_wash_13` | 26.09 | 40.17 | +21.54 | animated graphic shapes overlay |
| News Set Channel 7 Transition 2 | `9c12f97c-2022-6562-8868-6beeecfc32d0` | `jb.shape_burst_wash_14` | 16.63 | 86.60 | +12.08 | animated graphic shapes overlay |
| Old Film 02 | `5C0F5CDC-0F82-4207-85FE-7AD9B69DBB8D` | `jb.film_frame_wash_01` | 16.36 | 47.56 | +11.81 | film / photo overlay footage |
| Open Envelope | `2D94F069-FB2F-421A-90B2-7A78A1936F44` | `jb.film_frame_wash_02` | 23.57 | 68.02 | +19.02 | film / photo overlay footage |
| Orb 2 | `d8fa3a6c-24bf-4aae-9152-250a6f6e6442` | `jb.orb_warp_1` | 12.58 | 35.88 | +8.03 |  |
| Paper Tear Transition 20 | `9ACD289B-0E2C-4A81-8BFF-687BAEB6FC56` | `jb.paper_fold_wash_01` | 15.80 | 45.65 | +11.25 | paper tear with folds |
| Perspective Break | `EF9351C0-EA39-4190-93FA-E6633052C8E1` | `jb.panel_shuffle_wash_02` | 47.51 | 65.62 | +42.96 | 3D multi-panel layout |
| Photo Freeze | `F4D0756B-F029-413F-A13D-85B87AFDB02E` | `jb.freeze_frame_tilt` | 31.96 | 46.02 | +27.41 |  |
| Photo Snap 1 | `3A028B8F-68ED-4E6C-8BA2-C17FE60FEB89` | `jb.snapshot_flash` | 10.31 | 24.31 | +5.76 | camera HUD overlay with clipped overexposure |
| Photo Spread | `8DCB22B3-BF4F-4523-AF46-E121A5F0E4EB` | `jb.film_frame_wash_03` | 33.26 | 50.57 | +28.71 | film / photo overlay footage |
| Photo Switch Left | `29F21952-1298-4615-A5FA-A4FC3D024A42` | `jb.photo_flip_left` | 18.57 | 51.98 | +14.02 |  |
| Pinwheel | `668D3D30-43B8-41ef-A222-86E7912A497E` | `jb.overlay_wash_01` | 17.02 | 30.74 | +12.47 |  |
| Plain Shape 17 | `3FC5FC6D-C669-40FA-B1AF-308BED370634` | `jb.shape_burst_wash_15` | 24.68 | 54.76 | +20.13 | animated graphic shapes overlay |
| Plain Shape 20 | `CEC1827B-745E-4EB4-AD1B-A1EBEE58FF0D` | `jb.shape_burst_wash_16` | 10.58 | 33.26 | +6.03 | animated graphic shapes overlay |
| Plain Shape 22 | `554FEC3F-E022-46A6-B763-67E2E9D6BA7B` | `jb.shape_burst_wash_17` | 11.23 | 35.04 | +6.68 | animated graphic shapes overlay |
| Plain Shape 8 | `33745F8B-AE5A-4DC2-8C1B-1C61735B5A85` | `jb.shape_burst_wash_18` | 14.00 | 40.08 | +9.45 | animated graphic shapes overlay |
| Plain Shape 9 | `1E111A76-BC1D-4BE8-9315-79CB49CC8C36` | `jb.shape_burst_wash_19` | 15.50 | 34.95 | +10.95 | animated graphic shapes overlay |
| Powerful Sports Team Transition 02 | `CE1BB859-98C2-4E9F-90D0-DE880C931F12` | `jb.shape_burst_wash_20` | 12.74 | 34.82 | +8.19 | animated graphic shapes overlay |
| Profile Movie Transition 01 | `CAD04901-7A9A-4172-A68F-82F3E1781E52` | `jb.film_frame_wash_04` | 47.00 | 61.93 | +42.45 | film / photo overlay footage |
| Random Dissolve 01 | `6CAAC772-2684-4C71-94A3-CAF24434F4C2` | `jb.cell_dissolve_wash_01` | 8.73 | 18.54 | +4.18 | random-order cell / noise dissolve (order not reproducible) |
| Retro blockbusters 1 | `2C7ED650-4815-402C-9A04-B88F4CE70A2E` | `jb.film_frame_wash_05` | 25.47 | 57.31 | +20.92 | film / photo overlay footage |
| Retro blockbusters 2 | `AFE78312-109F-4F35-9A33-F8A59AA71F6F` | `jb.film_frame_wash_06` | 15.59 | 57.37 | +11.04 | film / photo overlay footage |
| Retro blockbusters 4 | `DA41C3BF-3DE7-4FC0-A4E9-92E58D72CF5D` | `jb.film_frame_wash_07` | 35.35 | 51.63 | +30.80 | film / photo overlay footage |
| Rotation Frame 1 | `1A351643-4659-4556-A18C-8ED77A637F3E` | `jb.frame_flip_1` | 11.44 | 47.29 | +6.89 |  |
| Rotation Frame 2 | `24194478-357F-4123-821D-8BD16434B6C4` | `jb.frame_flip_2` | 10.27 | 36.79 | +5.72 |  |
| Rotation Frame 4 | `C19635D5-0650-4B81-80B2-AEF7B1C64A87` | `jb.frame_flip_3` | 10.81 | 40.98 | +6.26 |  |
| Row Cos Whisk | `C8C1ED9D-959D-453f-8EFD-93D5BF7F5B23` | `jb.stripe_matte_wash_01` | 14.23 | 29.68 | +9.68 | stripe halftone matte |
| Rustic Transition 4 | `69dc5a52-5a08-717e-c9ba-be04df8a1fa4` | `jb.paint_stroke_reveal_21` | 7.71 | 22.13 | +3.16 | brush-stroke matte (hand-painted stroke animation) |
| Scary Pack Transition 01 | `5E841241-463E-448A-B928-3C9EE0D0E3A0` | `jb.dark_smoke_wash_02` | 20.80 | 48.46 | +16.25 | horror overlay footage |
| Scary Pack Transition 02 | `BC27EE2C-8526-45DA-97BF-CF75EEB428C0` | `jb.dark_smoke_wash_03` | 20.64 | 39.98 | +16.09 | horror overlay footage |
| Scary Pack Transition 03 | `F2BCEA67-4D6A-4F1F-8729-3ADAD68083A5` | `jb.dark_smoke_wash_04` | 16.51 | 32.90 | +11.96 | horror overlay footage |
| Scary Pack Transition 04 | `9C17871B-8EBA-4D88-8E99-98318012A0CA` | `jb.dark_smoke_wash_05` | 25.85 | 41.63 | +21.30 | horror overlay footage |
| Scary Pack Transition 08 | `AE6FC8AC-655B-458D-95E0-4D03F7748C23` | `jb.dark_smoke_wash_06` | 23.88 | 40.12 | +19.33 | horror overlay footage |
| Scoreboard Transition 01 | `5DC77869-75E6-45F6-9BCA-9B96CBA83DEC` | `jb.shape_burst_wash_21` | 13.98 | 43.33 | +9.43 | animated graphic shapes overlay |
| Shape Gradient Transitions Transition 25 | `108B1920-9128-4983-BAC9-FB4100603A8E` | `jb.shape_burst_wash_22` | 21.73 | 49.33 | +17.18 | animated graphic shapes overlay |
| Shape Transition 106 | `D8775488-C02D-4661-937D-4C3C937D4ACB` | `jb.cell_dissolve_wash_02` | 8.99 | 30.19 | +4.44 | random-order cell / noise dissolve (order not reproducible) |
| Shock | `9750249B-7D24-4AE4-8314-A2C96FE0862A` | `jb.shock_split` | 17.18 | 26.35 | +12.63 | glitch / strobe flashes with RGB split |
| Sliding Down | `8731F468-4D73-4420-9ADE-A7B0168A6B22` | `jb.card_slide_down` | 8.40 | 18.13 | +3.85 | bordered cards with drop shadows; the camera model fits 6.5 mean / 17 worst, not shipped |
| Sliding Left | `A0616F4C-89FE-468A-87B0-BC03F1B1AE98` | `jb.card_slide_left` | 13.99 | 61.73 | +9.44 |  |
| Space Rotation Left | `401962DA-6B63-4EB0-95C5-5C43A04601DE` | `jb.panel_shuffle_wash_03` | 40.50 | 70.89 | +35.95 | 3D multi-panel layout |
| Special Effects Skill Pack Transition 01 | `E943F100-16E7-4F2D-8821-A26E4C94A201` | `jb.cartoon_burst_wash_04` | 17.39 | 52.43 | +12.84 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Special Effects Skill Transition 03 | `D9AEBB98-B58B-4EC1-9750-F17BC822E7CB` | `jb.cartoon_burst_wash_05` | 24.27 | 63.53 | +19.72 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Special Effects Skill Transition 04 | `AE4878F8-8587-448A-9065-37C631D0AB51` | `jb.cartoon_burst_wash_06` | 13.96 | 45.52 | +9.41 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Special Effects Skill Transition 05 | `58BDBBA2-D53A-4331-83BF-9C0BF0AA1D48` | `jb.cartoon_burst_wash_07` | 30.85 | 59.55 | +26.30 | cartoon effect overlay (fire, smoke, energy, clouds) |
| Spiral Rise | `34F473F4-EEB9-4606-926D-8FE2540CF7AB` | `jb.panel_shuffle_wash_04` | 57.84 | 66.96 | +53.29 | 3D multi-panel layout |
| Sports Broadcast Transition 04 | `657A3B6F-B23F-4804-98EB-3213FF54E84F` | `jb.shape_burst_wash_23` | 15.73 | 36.43 | +11.18 | animated graphic shapes overlay |
| Sports Pack Transition 05 | `1E9976C8-24BF-4AA2-934D-A533FC015AC0` | `jb.shape_burst_wash_24` | 29.64 | 56.08 | +25.09 | animated graphic shapes overlay |
| Swap Left | `C3612776-D62E-41a2-8733-0429F50E11F5` | `jb.overlay_wash_02` | 52.12 | 58.80 | +47.57 |  |
| Take A Photo | `35602B81-FAA0-473D-90D1-672D135EEAD8` | `jb.snapshot_punch` | 15.25 | 51.57 | +10.70 |  |
| Technology HUD Transition 01 | `C9C2E39A-F581-4D9D-B1EF-140FC7F45798` | `jb.cell_dissolve_wash_03` | 11.62 | 29.79 | +7.07 | random-order cell / noise dissolve (order not reproducible) |
| Technology Light Transition 05 | `C78AAB26-6D08-404B-8B01-420EB927109A` | `jb.shape_burst_wash_25` | 15.99 | 40.23 | +11.44 | animated graphic shapes overlay |
| Top Bottom Gate | `C3043949-5B2B-444E-9079-3A7B7B2F52A1` | `jb.panel_shuffle_wash_05` | 32.02 | 48.00 | +27.47 | 3D multi-panel layout |
| Torn Paper Transition 09 | `0EB21BC5-6121-40A3-987A-217661816A98` | `jb.torn_band_wash_01` | 13.82 | 34.01 | +9.27 | torn band with moving centre |
| Vertical Cutout 1 | `F2F3C70C-0ADF-41BE-B4EB-B2DDC98CDFD9` | `jb.panel_shuffle_wash_06` | 28.14 | 42.93 | +23.59 | 3D multi-panel layout |
| Vertical Slice | `3B9CFBA3-EEE5-4C4D-A8AA-457A8B4044C9` | `jb.glitch_strobe_wash_01` | 17.91 | 34.98 | +13.36 | glitch / strobe flashes with RGB split |
| Vignette | `FAE8A09D-E8C6-436D-9DD4-773311D875AA` | `jb.vignette_zoom_cut` | 6.42 | 8.07 | +1.87 |  |
| VignetteOverlay | `B27F83E8-3736-4105-8B4B-BAB3CD2FBE32` | `jb.vignette_tint` | 18.54 | 18.79 | +13.99 |  |
| Vintage Glamour Transition 02 | `A6F74AF0-069F-4B0B-985E-9032ED630409` | `jb.paint_stroke_reveal_22` | 11.26 | 19.83 | +6.71 | brush-stroke matte (hand-painted stroke animation) |
| Vision Blocks | `CA0C9A79-A8F7-4B25-AC48-942275E756A9` | `jb.panel_shuffle_wash_07` | 32.74 | 64.41 | +28.19 | 3D multi-panel layout |
| Wedding Invitation Template Transition 01 | `9CC68058-8C13-4EC4-8B59-5BB5752CBEDD` | `jb.paint_stroke_reveal_23` | 12.94 | 25.30 | +8.39 | brush-stroke matte (hand-painted stroke animation) |
| Wedding Invitation Template Transition 02 | `97426E0D-3458-4316-877B-4FD00C507D82` | `jb.paint_stroke_reveal_24` | 10.52 | 22.07 | +5.97 | brush-stroke matte (hand-painted stroke animation) |
| Wedding Invitation Template Transition 04 | `546B3563-6DE9-44D1-9386-390BA3D3E73B` | `jb.paint_stroke_reveal_25` | 10.41 | 24.35 | +5.86 | brush-stroke matte (hand-painted stroke animation) |
| Whirl 1 | `b2f6224e-7040-43ff-be6d-d716d68885b0` | `jb.whirl_warp` | 13.95 | 22.43 | +9.40 |  |
| Wormhole 1 | `ec43f77c-9c92-4c0b-a6ba-2842caf4af82` | `jb.tunnel_warp_1` | 20.52 | 29.87 | +15.97 |  |
| Wormhole 2 | `74ae63b9-ce3e-47e8-966a-0d5c2dada46e` | `jb.tunnel_warp_2` | 17.47 | 24.95 | +12.92 |  |
| Wormhole 3 | `4d71fc4e-d3c5-4e82-b57c-2312572feba1` | `jb.tunnel_warp_3` | 12.89 | 30.71 | +8.34 |  |
| Wormhole 6 | `947d8c0d-576a-430e-8e70-cbffcd33b02e` | `jb.tunnel_warp_4` | 10.94 | 18.82 | +6.39 |  |
| Year of the Tiger Pack Transition 02 | `B3DB8933-F2E5-4094-975D-2595F0121FC2` | `jb.sparkle_trail_wash_06` | 8.11 | 33.07 | +3.56 | particle / sparkle overlay |

## Not implemented

Items of the lane whose look is dominated by overlay artwork (brush strokes, light leaks, sparkles,
drawn shapes, camera bodies, fire/ice/cartoon effects, 3D multi-panel layouts) are not recreated;
they keep rendering as a cut at the middle. Reasons per item:

| Filmora item | GUID | reason |
|---|---|---|
| Rectangle Zoom In | `CACF5228-B4B6-44e2-91C6-4CA8367EECBA` | overlay artwork |
| Rectangle Zoom Out | `F7BD83DF-999C-4d09-8F4C-85DAB842E720` | overlay artwork |
| Orb 3 | `273416f0-3460-4ed6-880b-445f4650cd08` | overlay artwork |
| Orb 4 | `d3797047-d7c1-457b-a606-0f460b0ed516` | overlay artwork |

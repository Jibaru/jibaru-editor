# Fidelity, lane 2: slide, push, roll, cube, flip, door, card, fold, stack and grid transitions

Filmora 15.7.3 transitions of the slide/push/3D families, fitted on Filmora's
own exports of the reference batches (`docs/filmora-references.md`: one
transition per cut between the gridA and gridB test cards). Only the
exported output was measured; no Filmora asset was read.

## Method

**Model.** Every item is a table of *layers* rendered by one shared shader
(`crates/jb-render/src/fx/lane2_layers.rs`, `fx/lane2/layers.wgsl`, tables in
`fx/lane2/tables.txt`). A frame is a background color plus layers drawn back
to front; each layer shows the outgoing clip, the incoming clip or a solid
color through a *corner pin* (where the four corners of a source rect land
on screen), with a per-channel display-space gain field over the face
(`g + gu (u - .5) + gv (v - .5)`, the 3D shading), an offset, an opacity, an
edge mode (clipped and antialiased, mirrored repeat or wrapped repeat) and a
motion blur (the pin averaged over a corner displacement, one tap per ~3 px).
Faces are sampled 2x2 per pixel. Keys are per fitted frame in transition
progress (keys that linear interpolation reproduces within 0.25 px are
dropped), so other durations scale.

**Bilinear vs perspective.** Filmora's 3D transitions do not all project the
same way. Some (Cube Right/Left/Up/Down) map the clip onto each face with a
2D *bilinear* corner pin of the projected corners: the optical-flow residual
of a face is 0.15 px for a bilinear pin and 2.9 px for the best homography.
Others (3D Flip Right, 3D Flip, the 3D Box family) are true perspective
(homography): on 3D Flip Right a bilinear pin leaves MAD 9-18 where the
homography gets 4.2. The fitter tries both on the first clearly non-affine
frame and keeps the better one for the item (`homo` in the table).

**Fitting** (throwaway scripts, not committed): each layer is tracked from
its clean end (outgoing layers forward from the first frame, incoming ones
backward from the last, then forward again with the incoming ones as
context). Per frame the model is rendered, dense optical flow between model
and reference gives correspondences on the pixels the layer owns (masked by
the test cards' hue so A and B tiles are not confused), the corners are
solved by robust least squares (linear for the bilinear pin), and the
photometric terms by linear least squares. When the constant-velocity guess
fails on fast frames, other velocity scales are tried. Motion blur is the
central frame difference of the corners times a shutter fraction chosen per
item. Floor reflections are mirrored layers with one gain field per item.
1-D slides and rolls (repeat edges, long motion blur) are searched
exhaustively per frame instead.

**Measurement.** `crates/jb-cli/examples/fidelity.rs` on the item window.
Three harness options were added for these test cards:
`JB_FIDELITY_RENDER_SIZE=1920x1080` renders at full resolution and compares
at `--size` (at 960x540 our own 4:2:0 encode halves chroma again: Cb error
3.4 vs 1.2 on the cards), `JB_FIDELITY_ENCODER=libx264` (the hardware
encoder smears the 10 px checker), and `JB_FIDELITY_LEVELS=none` (no Filmora
level offsets: the batch exports of the synthetic cards do not carry the
offset measured on the user projects; with it plain frames measure 3.5 with a
bias of -2.5 on R and B, without it 2.7 and no bias).

**Timing.** Fitting decodes the reference exactly like the harness
(`-fps_mode passthrough`): without it FFmpeg can duplicate the first frame
after a seek, which shifts a fit by one frame.

## Results

MAD mean / worst frame over the item window at 960x540 (harness with `JB_FIDELITY_LEVELS=none`, `JB_FIDELITY_ENCODER=libx264`, `JB_FIDELITY_RENDER_SIZE=1920x1080`, `--search 0`, range starting half a frame into the window). Plain frames of the batches measure about 2.7 through the same path.

"Floor" is the first (plain, outgoing clip) frame of the window through the same path; "Excess" is the window mean minus it.

| Filmora item | GUID | Our id | Mean | Worst | Floor | Excess | Notes |
|---|---|---|---:|---:|---:|---:|---|
| Fast Flip | `175D2B82-F671-4D65-AF17-134C941C3A03` | `jb.flip_fast` | 3.55 | 17.0 | 2.35 | +1.20 | worst frame at the switch between faces |
| Fast Push Down | `255239EC-BB3E-4B54-B74B-A51F15AF381F` | `jb.push_down_fast` | 3.71 | 9.2 | 2.21 | +1.50 |  |
| Slow Flip | `F2C5B762-7F0D-4A06-BC2C-FBB8727AADA5` | `jb.flip_slow` | 5.92 | 11.4 | 3.17 | +2.75 | worst frame at the switch between faces |
| 3D Horizontal Closing | `B070D206-0B16-4B67-BCF6-98414767AEA8` | `jb.panels_close_3d` | 20.47 | 33.7 | 4.59 | +15.88 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Horizontal | `EBC160A4-C522-4DEE-8FF2-24596EFF5CDA` | `jb.box_horizontal_3d` | 13.83 | 29.7 | 2.59 | +11.24 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Left | `75E87EE4-EC9E-4398-BA61-0653057F144A` | `jb.box_left_3d` | 3.96 | 5.9 | 2.91 | +1.05 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Vertical | `1C7CC9AD-ACD1-4D08-9A39-82682F5BE775` | `jb.box_vertical_3d` | 12.81 | 25.6 | 2.98 | +9.83 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Flip Right | `B04A402D-5C04-4110-AEAD-2E3DBEC1FEFF` | `jb.flip_3d_right` | 2.46 | 3.8 | 3.07 | -0.61 |  |
| Pivot Flip | `D9864B5B-CAF1-4C24-9124-49B8F7926046` | `jb.pivot_flip` | 7.32 | 36.2 | 2.99 | +4.33 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Fast Push Left | `2DD72324-4654-456D-9EA8-9931F6B6A1FD` | `jb.push_left_fast` | 3.24 | 10.7 | 2.06 | +1.18 | worst frame at the switch between faces |
| Fast Push Right | `95DF8005-6F31-48DD-9713-3FB2EECF102F` | `jb.push_right_fast` | 2.49 | 10.6 | 1.98 | +0.51 | worst frame at the switch between faces |
| Split Slide 1 | `50040234-1ACA-48DB-BB67-935A5014D92B` | — |  |  |  |  | not implemented |
| Roll_clockwise | `1EE81222-189E-4159-A917-D15EE518D2FF` | — |  |  |  |  | not implemented |
| Smooth Slideshow Transition 05 | `DD93DBBA-7FB8-474A-B599-71E4E08B69D5` | — |  |  |  |  | not implemented |
| Study And Education Slideshow Transition 01 | `6BFD47C0-51ED-472B-9CB2-510D21FCA254` | — |  |  |  |  | not implemented |
| Photo Slide 1 | `5561D5AD-7B58-498D-B099-66769FD3B7E3` | `jb.photo_slide_1` | 21.36 | 34.1 | 1.52 | +19.84 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Photo Slide 2 | `3B896470-09FA-4D5D-A23A-1340920ADFCD` | `jb.photo_slide_2` | 16.26 | 35.6 | 2.94 | +13.32 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Photo Slide 3 | `8E4DCEF8-3E13-4D9E-896A-EFD2C7969698` | `jb.photo_slide_3` | 14.91 | 36.1 | 2.86 | +12.05 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Photo Slide 4 | `BD576272-6A8F-41C4-8643-8404A0B16D3E` | `jb.photo_slide_4` | 12.90 | 21.7 | 3.59 | +9.31 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Photo Slide 5 | `B75481DA-EB86-48E6-805C-888F6C051913` | `jb.photo_slide_5` | 22.49 | 39.8 | 2.60 | +19.89 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Photo Slide 6 | `D08AB7BA-2B35-43CA-AF77-2907B1BE31C8` | — |  |  |  |  | not implemented |
| Paper Page Curl | `81527EAA-3AD1-418D-8C56-B7BC4E174D03` | — |  |  |  |  | not implemented |
| Erase Slide Right | `4CA4EF88-5997-41a6-AAE6-0C2226D5A96E` | `jb.erase_slide_right` | 2.23 | 2.8 | 2.59 | -0.36 |  |
| Box Flip Left | `B0FA9DF9-6C71-4B4D-8102-8F106A9E1385` | `jb.box_flip_left` | 8.00 | 32.3 | 5.84 | +2.16 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Box Flip Right | `6C491663-ADFE-49B5-855F-FF2AE53E8F34` | `jb.box_flip_right` | 7.06 | 25.9 | 2.56 | +4.50 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Flip Roll Down | `4057dc68-ff07-4561-90b7-97762af89945` | `jb.flip_roll_down` | 3.76 | 7.9 | 1.95 | +1.81 |  |
| Flip Roll Left | `ae8e58de-6e52-4fca-851f-f8a6dfd41647` | `jb.flip_roll_left` | 4.46 | 11.3 | 3.67 | +0.79 | worst frame at the switch between faces |
| Flip Roll Right | `2787c059-6baa-4819-9298-9ee22071cc95` | `jb.flip_roll_right` | 3.81 | 10.2 | 2.85 | +0.96 | worst frame at the switch between faces |
| Flip Roll Up | `af113622-985e-4ce3-a807-9601607ee43f` | `jb.flip_roll_up` | 4.31 | 10.8 | 3.75 | +0.56 | worst frame at the switch between faces |
| Simple Roll 10 | `691302d8-de75-4f40-82e3-5dd30fc0dc55` | `jb.roll_10` | 4.07 | 8.7 | 2.62 | +1.45 |  |
| Simple Roll 11 | `490e0bdb-f7b7-4fb2-a92a-ed5927ca13b3` | `jb.roll_11` | 4.23 | 8.6 | 3.11 | +1.12 |  |
| Simple Roll 2 | `d8867ccc-4762-49c4-93fd-616208d056cf` | `jb.roll_down_2` | 4.47 | 8.9 | 2.09 | +2.38 |  |
| Simple Roll 3 | `f03e2971-f66f-4481-b308-a640d451d2ca` | `jb.roll_left_3` | 4.32 | 7.7 | 3.03 | +1.29 |  |
| Simple Roll 6 | `3e2c9408-4b1e-4e78-afc4-4d6c837f6c6e` | `jb.roll_6` | 4.99 | 9.2 | 3.15 | +1.84 |  |
| Simple Roll 7 | `0fbaebd7-c224-4ca5-a9f0-d16307f19854` | `jb.roll_7` | 4.85 | 10.2 | 3.27 | +1.58 | worst frame at the switch between faces |
| Simple Roll 8 | `93a969d4-9108-45d6-9c09-f08350b2dca7` | `jb.roll_8` | 4.48 | 7.9 | 2.54 | +1.94 |  |
| Simple Roll 9 | `1568e098-ca48-4861-9667-6d3d6ed19b60` | `jb.roll_9` | 4.36 | 11.3 | 3.20 | +1.16 | worst frame at the switch between faces |
| Retro Box 01 | `7DB28C3E-D012-470E-B872-F71BA20437E2` | `jb.retro_box` | 2.82 | 3.5 | 2.59 | +0.23 |  |
| 3D Box Close | `16BFABD5-AC48-42AF-BAF9-149934D9B990` | — |  |  |  |  | not implemented |
| 3D Box Down | `86437F55-4674-4809-9B7A-0D39DCF9E313` | `jb.box_down_3d` | 8.55 | 49.4 | 1.91 | +6.64 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Drop | `3B310488-6093-4A52-B87E-0367ADF1641A` | `jb.box_drop_3d` | 3.93 | 6.9 | 3.04 | +0.89 |  |
| 3D Box Fall | `0C615FD6-6C3D-469A-955C-5FDF8FA6A23A` | `jb.box_fall_3d` | 3.92 | 8.8 | 1.88 | +2.04 | worst frame at the switch between faces |
| 3D Box Flip 1 | `A84BB82F-36F0-4E6A-8488-FBC2F1AA0BD0` | `jb.box_flip_3d_1` | 5.51 | 23.1 | 2.65 | +2.86 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Flip 2 | `D1334A82-581D-42C6-B3CD-7B0DF3E5F565` | `jb.box_flip_3d_2` | 5.15 | 30.6 | 2.31 | +2.84 | worst frame at the switch between faces |
| 3D Box Flip 3 | `E23C0AF1-1BCB-4083-908D-1952E6FC7494` | `jb.box_flip_3d_3` | 12.34 | 38.8 | 3.18 | +9.16 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Flip 4 | `B88000C0-71DF-4B92-B4DE-A63C7733978D` | `jb.box_flip_3d_4` | 4.93 | 29.5 | 2.03 | +2.90 | worst frame at the switch between faces |
| 3D Box Flip 5 | `64D165BC-C38F-400F-965D-A9E33BA1128C` | `jb.box_flip_3d_5` | 3.73 | 5.5 | 3.90 | -0.17 |  |
| 3D Box Flip 6 | `95EB31D4-DA7A-4642-8E35-CEA17DD93B35` | `jb.box_flip_3d_6` | 4.81 | 22.4 | 1.90 | +2.91 | worst frame at the switch between faces |
| 3D Box Right | `9BF9CABE-864C-43D2-A282-24A578AF699F` | `jb.box_turn_right_3d` | 5.31 | 28.1 | 2.51 | +2.80 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Box Split 1 | `3B28FE15-13ED-4BF8-A844-9DFDAC64816B` | — |  |  |  |  | not implemented |
| 3D Box Split 2 | `93020E77-3F2E-4A15-BE14-A2C09970408D` | — |  |  |  |  | not implemented |
| 3D Box Up | `F9DB5BCD-C24E-43A4-BAE3-7FC4BE93A25F` | `jb.box_up_3d` | 9.57 | 55.8 | 1.99 | +7.58 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Camera Transitions 7 | `F163DDE7-90AF-077C-84B2-1498C379F735` | `jb.camera_cards_7` | 23.49 | 52.9 | 2.59 | +20.90 |  |
| 3D Camera Transitions 8 | `BCD1F0EA-3EEA-EBE7-EAC8-6FE53723769E` | — |  |  |  |  | not implemented |
| 3D Camera Transitions 9 | `39701EAA-6FF3-DDF1-B765-F2F4B76ED063` | — |  |  |  |  | not implemented |
| 3D Card | `1C56ABE1-F948-4426-AC71-6710D6362312` | `jb.card_3d` | 6.19 | 23.5 | 4.50 | +1.69 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Carousel Card Slide Left | `1EFB3402-5599-414F-98BA-ADE56537FD63` | — |  |  |  |  | not implemented |
| 3D Carousel Card Spin Left | `e25b66ac-d8a8-4732-8777-09bef4648e00` | — |  |  |  |  | not implemented |
| 3D Carousel Card Spin Right | `0a336063-7e21-4c2e-9884-22ef56fe59c1` | — |  |  |  |  | not implemented |
| 3D Carousel Cylinder Roll Down | `8A956534-D1A6-4CC8-ACA6-43614CAB526A` | — |  |  |  |  | not implemented |
| 3D Carousel Cylinder Roll Up | `c6347070-6721-4523-8a5e-51f84b36f3e7` | — |  |  |  |  | not implemented |
| 3D Carousel Dual Axis Spin Down | `F7FA07EF-4B62-453A-9A17-6B217F6336D6` | — |  |  |  |  | not implemented |
| 3D Carousel Dual Axis Spin Up | `50AEC96D-D30D-47FE-BE99-B00FEE8C9B15` | — |  |  |  |  | not implemented |
| 3D Carousel Ring Roll Diagonal Left | `8C854521-0CBA-4879-802E-DA284ECEF9E1` | — |  |  |  |  | not implemented |
| 3D Carousel Ring Roll Down | `0A4A76AB-5541-4802-9B35-98A2812DD9A6` | — |  |  |  |  | not implemented |
| 3D Carousel Ring Roll Up | `0EEC266A-D417-461C-B4E0-3F71650478E8` | — |  |  |  |  | not implemented |
| 3D Carousel Spin Left | `8E484E8F-4097-45F5-9857-90E0646776D6` | `jb.carousel_spin_left` | 11.46 | 21.2 | 3.55 | +7.91 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Carousel Spin Left Close | `E5FCA5B4-4D4F-46D3-9C6A-930B3431F675` | `jb.carousel_spin_left_close` | 16.15 | 46.3 | 2.44 | +13.71 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Carousel Spin Right | `1D999F3D-EA93-43EC-8CAB-A9694AF03CC8` | `jb.carousel_spin_right` | 16.88 | 29.4 | 3.39 | +13.49 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Carousel Spin Right Close | `0BC474A4-AFFE-470D-8E52-6981F32C7003` | `jb.carousel_spin_right_close` | 15.24 | 39.1 | 2.45 | +12.79 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Fast Box Right 01 | `93240F33-7900-4971-B90D-442502ED3A48` | `jb.fast_box_right_1` | 11.60 | 35.6 | 3.39 | +8.21 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Fast Box Right 02 | `A9CD0FFC-0A62-42BC-B595-2E85DD1F8F3A` | `jb.fast_box_right_2` | 23.46 | 32.0 | 2.71 | +20.75 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Flip | `EC14B694-D0AD-425A-806C-BA9D288341CC` | `jb.flip_3d_tilt` | 2.88 | 4.3 | 4.32 | -1.44 |  |
| 3D Frame Shift | `8C84620F-C904-4702-B697-F87B224048B9` | — |  |  |  |  | not implemented |
| 3D Page Flip Left | `A985CC65-0A2B-440F-B437-6F802A0305B0` | `jb.page_flip_left_3d` | 13.77 | 39.9 | 7.65 | +6.12 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| 3D Screens | `94860FC5-0D3E-42F4-A8BB-AE99CFBE5DF5` | `jb.screens_3d` | 16.31 | 32.9 | 2.04 | +14.27 |  |
| 3D Slideshow Style 1 | `82F40844-1371-4C82-A822-40DB348B8A8C` | — |  |  |  |  | not implemented |
| 3D Slideshow Style 2 | `DE2B8F85-34CE-43B3-A326-293F597C2B0F` | `jb.slideshow_tilt_2` | 9.29 | 12.8 | 8.63 | +0.66 |  |
| 3D Slideshow Style 3 | `A12CC2AE-1864-44B7-88AE-4F5A3DA50167` | — |  |  |  |  | not implemented |
| 3D Slideshow Style 4 | `6E3365D4-B23E-4787-8D6B-2B25240E3463` | — |  |  |  |  | not implemented |
| 3D Slideshow Style 5 | `5963FA3B-5C16-423F-AE9C-C1F0424BC60D` | — |  |  |  |  | not implemented |
| 3D Slideshow Style 6 | `8E158EBF-E75B-4FD1-BEFC-2D1C742B5245` | — |  |  |  |  | not implemented |
| Arc Slide | `949113DB-25F5-4C3C-81FC-A8AB3E6C93DC` | — |  |  |  |  | not implemented |
| Boxed Reality | `95AE988D-EDE9-4C4F-B813-A24BE7DE7415` | — |  |  |  |  | not implemented |
| Card Spin Left | `50ADFEE2-1269-4B1E-8D44-33CC0620070D` | — |  |  |  |  | not implemented |
| Card Switch | `7F1B4870-2C3C-4E61-9066-2F993F00594E` | `jb.card_switch` | 10.24 | 26.4 | 3.93 | +6.31 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Collage Turn | `17B92348-899F-4BDA-B020-C5CA9081B00F` | — |  |  |  |  | not implemented |
| Cube Down | `59A8A58F-31CB-4740-9F9D-0DCD3CB7AF92` | `jb.cube_turn_down` | 6.44 | 7.8 | 4.84 | +1.60 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Cube Facets Down | `FAC11A67-7C08-497E-B979-1EF1F923DEF6` | `jb.cube_facets_down` | 11.16 | 51.7 | 2.84 | +8.32 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Cube Facets Left | `55C7899F-8326-4E0E-8A14-BAD17A8757E5` | `jb.cube_facets_left` | 7.35 | 22.2 | 2.84 | +4.51 |  |
| Cube Facets Right | `C73CB5B2-6885-48E1-A97D-9DC8C8D1E3BD` | — |  |  |  |  | not implemented |
| Cube Facets Up | `09FB622E-E770-4E86-BC86-A6F3BDFACF75` | — |  |  |  |  | not implemented |
| Cube Left | `A6B6286F-F9FE-4F9F-8C45-538EEC05F620` | `jb.cube_turn_left` | 5.26 | 7.1 | 3.08 | +2.18 |  |
| Cube Up | `5107B490-0EA4-47E1-8343-BD779895685F` | `jb.cube_turn_up` | 5.89 | 6.8 | 4.85 | +1.04 |  |
| Diagonal Slice Reveal | `E5C74593-D113-47B5-B174-252E1716826C` | `jb.diagonal_slice_reveal` | 3.18 | 11.4 | 3.23 | -0.05 | worst frame at the switch between faces |
| Diagonal Slide | `FC0A3B6C-84ED-40EA-B83C-E8043A8BD880` | — |  |  |  |  | not implemented |
| Domino Scroll | `64129C4C-9142-481B-B3AF-933947088D7C` | — |  |  |  |  | not implemented |
| Flip Down | `804982E3-3FC8-425E-B43A-6444F2C53B79` | `jb.flip_down` | 3.62 | 5.1 | 3.44 | +0.18 |  |
| Flip Down | `0A8EF5CE-6993-474F-9609-F0C3B2ADF0C1` | `jb.flip_down_fast` | 5.90 | 30.3 | 1.93 | +3.97 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Flip Left | `F309557C-8B97-499A-9696-29B0FDD6367C` | `jb.flip_left` | 2.60 | 3.6 | 2.22 | +0.38 |  |
| Flip Pages Right | `BE89907B-B1DB-4D6B-A00C-D0F268F3F852` | `jb.flip_pages_right` | 14.30 | 33.7 | 6.82 | +7.48 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Flip Right | `7B680F51-1B11-480D-A4DE-C5065188BBCC` | `jb.flip_right` | 2.91 | 3.7 | 2.84 | +0.07 |  |
| Fold Transition 01 | `6912AFD8-AF5E-4E98-9C99-1D627627B4D6` | `jb.fold_1` | 2.98 | 6.8 | 1.73 | +1.25 |  |
| Fold Transition 02 | `381884E7-ADD2-43EE-9316-D0C7F159C3B5` | `jb.fold_2` | 2.95 | 6.1 | 2.47 | +0.48 |  |
| Fold Transition 03 | `621D9BE3-3FBD-488E-823E-A48E37FE24F7` | `jb.fold_3` | 3.91 | 9.5 | 2.45 | +1.46 |  |
| Fold Transition 04 | `75729E2E-A47F-4E0C-8AC8-4DDA210C158B` | `jb.fold_4` | 3.96 | 9.4 | 3.66 | +0.30 |  |
| Fold Transition 05 | `D1BA4DE3-3FCA-480B-B8EA-4A1048A389DF` | — |  |  |  |  | not implemented |
| Fold Transition 06 | `93816AC5-40CA-4C4F-980A-B12B0E4DD2EF` | — |  |  |  |  | not implemented |
| Fold Transition 07 | `02413DD1-31AC-4BE0-8334-E7067BBB9EE8` | — |  |  |  |  | not implemented |
| Fold Transition 08 | `96262F03-7368-4610-91AB-6B25D29322D2` | `jb.fold_8` | 3.06 | 7.4 | 2.64 | +0.42 |  |
| Glass Panel Flash | `293A55BF-E78C-4F2A-A0C5-2B8E2B023C51` | — |  |  |  |  | not implemented |
| Grid Flip 3D | `D314281B-DD2C-4662-8A52-A955B8A7E54B` | — |  |  |  |  | not implemented |
| Luminous Page Turning | `C8D169E4-A8B6-470C-872E-9CA9A09D7134` | — |  |  |  |  | not implemented |
| Magic Card | `52730ECE-003C-40A3-A880-2A2E23A10295` | — |  |  |  |  | not implemented |
| Panel Collapse | `7C0FB782-E755-4BFE-96B5-CA90BA997B9D` | — |  |  |  |  | not implemented |
| Rolling Screen | `D4E2B8C1-3A5F-4E7B-9C2D-6F8A1B0E7C3F` | — |  |  |  |  | not implemented |
| Rotating Cube | `F23CEC10-EB99-4085-BA42-86CD41883CEE` | `jb.cube_rotate` | 15.98 | 31.3 | 3.56 | +12.42 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Screen Flip 3D | `8D3025A8-8F60-42E7-B0AA-0C06F5608C76` | `jb.screen_flip_3d` | 9.74 | 21.4 | 2.91 | +6.83 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Slice Transform 3D | `563C3469-9210-4B7F-97D9-95C27A1105FA` | — |  |  |  |  | not implemented |
| Spatial Stack | `C46E1588-00C1-428E-A7D4-5835F7522DDE` | — |  |  |  |  | not implemented |
| Split Dimension | `92936B8A-B906-4E4C-B4C3-8664A15D9246` | — |  |  |  |  | not implemented |
| Tear Paper Recovery | `BE9481E4-EE77-4040-B059-42844D024AEC` | — |  |  |  |  | not implemented |
| History Slideshow Transition 01 | `770A5643-34DC-435D-B35B-722B21A5A27D` | — |  |  |  |  | not implemented |
| Film Roll Transition 01 | `E041F777-9B75-4881-B7F6-706101648EF1` | — |  |  |  |  | not implemented |
| Film Roll Transition 03 | `7F1C1BD9-E672-4BD5-A833-0E3FCDAB155B` | — |  |  |  |  | not implemented |
| Photobook Slides | `3E0DC666-ED48-4D40-81F4-D7E25A044A09` | — |  |  |  |  | not implemented |
| Box Flip Down | `bf05541f-7e2b-4ee3-af3c-f2b5608879a8` | `jb.box_flip_down` | 5.53 | 21.5 | 1.80 | +3.73 | worst frame at the switch between faces |
| Box Flip Up | `c76663b6-72d0-4d87-9f35-0e4c57ce37da` | `jb.box_flip_up` | 5.33 | 13.8 | 2.75 | +2.58 | worst frame at the switch between faces |
| Fisheye Roll Down | `0bc06bc0-0c06-4583-a15d-13b01fc71e05` | — |  |  |  |  | not implemented |
| Fisheye Roll Down Right | `5a5e8238-0f9a-4a35-8331-6fd2348fcaf1` | — |  |  |  |  | not implemented |
| Fisheye Roll Right | `05e90c4d-fedc-4102-968a-0d830e04fb68` | — |  |  |  |  | not implemented |
| Fisheye Roll Up | `7c8e17d9-1294-4307-b99f-103fff2cf91f` | — |  |  |  |  | not implemented |
| Fisheye Roll Up Right | `464f09fe-10cd-4701-b2f5-8c5ae4463086` | — |  |  |  |  | not implemented |
| Cross Split | `75A09E8D-21D1-4c4e-AEA9-EB9CEC11A644` | `jb.cross_split` | 1.79 | 2.3 | 1.85 | -0.06 |  |
| DoorWay | `976B2502-3B22-4dd4-AEB4-0AAA2C1E5AC8` | `jb.doorway` | 6.14 | 8.9 | 1.66 | +4.48 | approximation: per-frame layer fit leaves large residuals (multi-face / curved / blurred motion not captured) |
| Skew Left Split | `5003D879-7930-44fa-A333-F21F62564709` | `jb.skew_split_left` | 1.70 | 2.5 | 1.48 | +0.22 |  |
| Skew Right Split | `2D4099D2-9B28-437f-8FEB-2A7E25C0B549` | `jb.skew_split_right` | 1.68 | 2.5 | 1.73 | -0.05 |  |
| Cube Right | `0FCB240F-5328-4e85-869A-2D989F50D387` | `jb.cube_turn_right` | 4.85 | 5.7 | 3.18 | +1.67 |  |

81 of 138 items implemented; 11 within mean < 3 and worst < 10.

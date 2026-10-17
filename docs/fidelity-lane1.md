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

## Not implemented

Items of the lane whose look is dominated by overlay artwork (brush strokes, light leaks, sparkles,
drawn shapes, camera bodies, fire/ice/cartoon effects, 3D multi-panel layouts) are not recreated;
they keep rendering as a cut at the middle. Reasons per item:

| Filmora item | GUID | reason |
|---|---|---|
| Camera Shutter | `582C0031-3A5A-4839-86C2-BD5662120FAD` | camera body / aperture overlay |
| Camera Transition 09 | `0A51BCC4-B46C-40E3-AB2E-072317A483D0` | camera body / aperture overlay |
| Motion Transitions Pack Vol 4 Transition 14 | `08C887B6-5597-4A52-BE2B-133E6B6EC9BD` | animated graphic shapes overlay |
| Brush Art Design Pack Transition 08 | `3B7844DB-2D95-4136-BE2E-4D59D3FB0CF8` | brush-stroke matte (hand-painted stroke animation) |
| Motion Transitions Pack Vol 3 Transition 14 | `6F64A98D-2C50-4D7F-9314-89E859FB9AD4` | animated graphic shapes overlay |
| Drop Bounce | `8EF84F2E-C203-4f98-84D1-959EF228456D` | the dropping clip wraps around with a strip of the outgoing clip at the bottom; the camera model reaches 7.7 / 12, not shipped |
| Beauty Brush Transition 1 | `b20e8acf-35d7-4958-aebc-64d2358c6b97` | brush-stroke matte (hand-painted stroke animation) |
| Beauty Brush Transition 2 | `2fe80ebb-e89e-4de9-b669-c9de540d15ea` | brush-stroke matte (hand-painted stroke animation) |
| Sparkle Motion Transition 07 | `9EE833DD-A002-4583-8ECB-AB2CD8A8FD7C` | particle / sparkle overlay |
| Sparkle Motion Transition 11 | `E8A47581-CE66-4BF5-AFC8-685A3F68487F` | particle / sparkle overlay |
| Sparkle Motion Transition 13 | `B35B100A-817A-499C-BA07-8E6E3C7DC4C8` | particle / sparkle overlay |
| Sparkle Motion Transition 15 | `F8A69A99-8CDD-42D3-9B38-9937F55EEEF5` | particle / sparkle overlay |
| Sparkle Motion Transition 20 | `8B70FA91-0F25-442F-9624-DE656D5E0112` | particle / sparkle overlay |
| Blurry Ejection | `D8287205-6EC2-2DE1-69D6-4D216A398FB3` | overlay artwork |
| Special FX Motion Transition 04 | `efbf96b4-211e-ef11-ba94-8045dd702285` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Speed Motivation Transition 01 | `6cdc4237-8c92-ef11-b433-d8bbc19410ef` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Brush Drift Transition 01 | `7d98be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 02 | `8298be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 03 | `8198be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 04 | `7f98be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 05 | `7e98be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 06 | `8398be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 07 | `8098be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 08 | `8498be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 09 | `8598be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Brush Drift Transition 10 | `8698be69-834b-f111-9e87-50ebf65600b6` | brush-stroke matte (hand-painted stroke animation) |
| Colorful Brush Transition 05 | `d1c03638-dc96-f111-b463-d8bbc19410ee` | brush-stroke matte (hand-painted stroke animation) |
| Colorful Brush Transition 07 | `cfc03638-dc96-f111-b463-d8bbc19410ee` | brush-stroke matte (hand-painted stroke animation) |
| Golden Brush Transition 01 | `c826f80c-db96-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 02 | `de3499f9-d196-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 03 | `c726f80c-db96-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 04 | `db3499f9-d196-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 05 | `07868c04-ce96-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 06 | `06868c04-ce96-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 07 | `05868c04-ce96-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Golden Brush Transition 08 | `df3499f9-d196-f111-b463-d8bbc19410ee` | painted gold brush overlay |
| Stylish Brush Transition 03 | `CF7E8635-849C-4D99-920C-3BEBDE7E70B2` | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 09 | `115818EE-EEF8-4904-AA17-6D3571B9B2BB` | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 12 | `309343C0-ED56-491B-9EEB-2C5BF3F553AA` | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 17 | `D725AB61-3F6B-44F9-8059-128298182DCC` | brush-stroke matte (hand-painted stroke animation) |
| Stylish Brush Transition 20 | `38E486E2-3A8B-4628-8E29-4D6E17EE9867` | brush-stroke matte (hand-painted stroke animation) |
| Camera Viewfinder 2 | `72B62645-7A9F-48E6-AEF3-43E57C5118D3` | overlay artwork |
| Old Camera | `0D696B4E-F6DB-4160-8C64-2E8B68E933EC` | camera body / aperture overlay |
| Rectangle Zoom In | `CACF5228-B4B6-44e2-91C6-4CA8367EECBA` | overlay artwork |
| Rectangle Zoom Out | `F7BD83DF-999C-4d09-8F4C-85DAB842E720` | overlay artwork |
| Horrors Pack Transition 01 | `A4A64485-7A1E-4CD7-9621-C4EC79433BAA` | horror overlay footage |
| Ice Firework Effect Vol 02 Transition 10 | `F1FF9C55-AEDE-4F01-A698-3ABC9C69B682` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Influencer Toolkit Vol 1 Transition 01 | `1E059E6B-7E12-4DC1-A5ED-982CF52EDD8F` | animated graphic shapes overlay |
| Jitter 01 | `6AE378FF-2240-4716-ACFB-86E591534441` | glitch / strobe flashes with RGB split |
| Layer down | `3E674127-F735-43A4-90F0-261C96499A99` | overlay artwork |
| Layer up | `327C8A9F-49E9-4A46-BA26-63E1AF2F2E04` | overlay artwork |
| Layered Vision | `0647A4ED-769A-494E-92EF-34984F81DD18` | 3D multi-panel layout |
| Light Leaks Transition 01 | `C90D0AD6-EECC-496A-8254-2BF38D46D6A1` | light leak / bokeh overlay footage |
| Light Leaks Transition 02 | `B288EAEA-E306-4E5A-8BBB-4AE8F52B65AC` | light leak / bokeh overlay footage |
| Light Leaks Transition 11 | `5FCF4746-3646-415B-9884-A9ECED01EBE7` | light leak / bokeh overlay footage |
| Light Leaks Transition Pack Transition 03 | `DE3A553C-34A6-4EA4-B65D-BAC68391C2A1` | light leak / bokeh overlay footage |
| Light Letters Pack Transition 02 | `250E53BD-AFF4-494C-B7D8-57C0D8E89B95` | light leak / bokeh overlay footage |
| Linear 14 | `1C10FA86-5D69-4E77-9F5C-C99D2097D86F` | animated graphic shapes overlay |
| Linear 19 | `C2A21964-21C9-48B3-8E17-3A78E97EC290` | animated graphic shapes overlay |
| Linear 4 | `2A59A5A5-A492-425C-904E-D7703218AC16` | animated graphic shapes overlay |
| Love Daily Record Transition 03 | `B4DBCD45-650F-4E17-8FDF-7C13C72932EE` | animated graphic shapes overlay |
| MaskBlend | `A1B2C3D4-5E6F-7890-ABCD-EF1234567890` | overlay artwork |
| Memphis Art Transition 06 | `DD751802-4E0F-42F2-AA81-600665DE369C` | animated graphic shapes overlay |
| Metropolis Transition 01 | `24f2ecb0-b8cc-f15c-dc60-9b14a61adc78` | animated graphic shapes overlay |
| Modern Transparent Transition 08 | `E51B8E25-E17F-4E9A-96C4-67F07830F4A5` | animated graphic shapes overlay |
| Modern Transparent Transition 10 | `C9CA2925-ECC7-421C-8E35-60C9984C1EAA` | animated graphic shapes overlay |
| Movie Atmosphere CN Transition 07 | `FDFA48E8-C537-46AE-9200-3B952A2030FE` | animated graphic shapes overlay |
| Neon Valentine Promo Transition 07 | `4EECB05F-72B3-434F-B729-824D35B2A29C` | animated graphic shapes overlay |
| News Set Channel 7 Transition 2 | `9c12f97c-2022-6562-8868-6beeecfc32d0` | animated graphic shapes overlay |
| Old Film 02 | `5C0F5CDC-0F82-4207-85FE-7AD9B69DBB8D` | film / photo overlay footage |
| Open Envelope | `2D94F069-FB2F-421A-90B2-7A78A1936F44` | film / photo overlay footage |
| Orb 2 | `d8fa3a6c-24bf-4aae-9152-250a6f6e6442` | overlay artwork |
| Orb 3 | `273416f0-3460-4ed6-880b-445f4650cd08` | overlay artwork |
| Orb 4 | `d3797047-d7c1-457b-a606-0f460b0ed516` | overlay artwork |
| Paper Tear Transition 20 | `9ACD289B-0E2C-4A81-8BFF-687BAEB6FC56` | paper tear with folds |
| Perspective Break | `EF9351C0-EA39-4190-93FA-E6633052C8E1` | 3D multi-panel layout |
| Photo Freeze | `F4D0756B-F029-413F-A13D-85B87AFDB02E` | overlay artwork |
| Photo Snap 1 | `3A028B8F-68ED-4E6C-8BA2-C17FE60FEB89` | camera HUD overlay with clipped overexposure |
| Photo Spread | `8DCB22B3-BF4F-4523-AF46-E121A5F0E4EB` | film / photo overlay footage |
| Photo Switch Left | `29F21952-1298-4615-A5FA-A4FC3D024A42` | overlay artwork |
| Pinwheel | `668D3D30-43B8-41ef-A222-86E7912A497E` | overlay artwork |
| Plain Shape 17 | `3FC5FC6D-C669-40FA-B1AF-308BED370634` | animated graphic shapes overlay |
| Plain Shape 20 | `CEC1827B-745E-4EB4-AD1B-A1EBEE58FF0D` | animated graphic shapes overlay |
| Plain Shape 22 | `554FEC3F-E022-46A6-B763-67E2E9D6BA7B` | animated graphic shapes overlay |
| Plain Shape 8 | `33745F8B-AE5A-4DC2-8C1B-1C61735B5A85` | animated graphic shapes overlay |
| Plain Shape 9 | `1E111A76-BC1D-4BE8-9315-79CB49CC8C36` | animated graphic shapes overlay |
| Powerful Sports Team Transition 02 | `CE1BB859-98C2-4E9F-90D0-DE880C931F12` | animated graphic shapes overlay |
| Profile Movie Transition 01 | `CAD04901-7A9A-4172-A68F-82F3E1781E52` | film / photo overlay footage |
| Random Dissolve 01 | `6CAAC772-2684-4C71-94A3-CAF24434F4C2` | random-order cell / noise dissolve (order not reproducible) |
| Retro blockbusters 1 | `2C7ED650-4815-402C-9A04-B88F4CE70A2E` | film / photo overlay footage |
| Retro blockbusters 2 | `AFE78312-109F-4F35-9A33-F8A59AA71F6F` | film / photo overlay footage |
| Retro blockbusters 4 | `DA41C3BF-3DE7-4FC0-A4E9-92E58D72CF5D` | film / photo overlay footage |
| Rotation Frame 1 | `1A351643-4659-4556-A18C-8ED77A637F3E` | overlay artwork |
| Rotation Frame 2 | `24194478-357F-4123-821D-8BD16434B6C4` | overlay artwork |
| Rotation Frame 4 | `C19635D5-0650-4B81-80B2-AEF7B1C64A87` | overlay artwork |
| Row Cos Whisk | `C8C1ED9D-959D-453f-8EFD-93D5BF7F5B23` | stripe halftone matte |
| Rustic Transition 4 | `69dc5a52-5a08-717e-c9ba-be04df8a1fa4` | brush-stroke matte (hand-painted stroke animation) |
| Scary Pack Transition 01 | `5E841241-463E-448A-B928-3C9EE0D0E3A0` | horror overlay footage |
| Scary Pack Transition 02 | `BC27EE2C-8526-45DA-97BF-CF75EEB428C0` | horror overlay footage |
| Scary Pack Transition 03 | `F2BCEA67-4D6A-4F1F-8729-3ADAD68083A5` | horror overlay footage |
| Scary Pack Transition 04 | `9C17871B-8EBA-4D88-8E99-98318012A0CA` | horror overlay footage |
| Scary Pack Transition 08 | `AE6FC8AC-655B-458D-95E0-4D03F7748C23` | horror overlay footage |
| Scoreboard Transition 01 | `5DC77869-75E6-45F6-9BCA-9B96CBA83DEC` | animated graphic shapes overlay |
| Shape Gradient Transitions Transition 25 | `108B1920-9128-4983-BAC9-FB4100603A8E` | animated graphic shapes overlay |
| Shape Transition 106 | `D8775488-C02D-4661-937D-4C3C937D4ACB` | random-order cell / noise dissolve (order not reproducible) |
| Shock | `9750249B-7D24-4AE4-8314-A2C96FE0862A` | glitch / strobe flashes with RGB split |
| Sliding Down | `8731F468-4D73-4420-9ADE-A7B0168A6B22` | bordered cards with drop shadows; the camera model fits 6.5 mean / 17 worst, not shipped |
| Sliding Left | `A0616F4C-89FE-468A-87B0-BC03F1B1AE98` | overlay artwork |
| Space Rotation Left | `401962DA-6B63-4EB0-95C5-5C43A04601DE` | 3D multi-panel layout |
| Special Effects Skill Pack Transition 01 | `E943F100-16E7-4F2D-8821-A26E4C94A201` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Special Effects Skill Transition 03 | `D9AEBB98-B58B-4EC1-9750-F17BC822E7CB` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Special Effects Skill Transition 04 | `AE4878F8-8587-448A-9065-37C631D0AB51` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Special Effects Skill Transition 05 | `58BDBBA2-D53A-4331-83BF-9C0BF0AA1D48` | cartoon effect overlay (fire, smoke, energy, clouds) |
| Spiral Rise | `34F473F4-EEB9-4606-926D-8FE2540CF7AB` | 3D multi-panel layout |
| Sports Broadcast Transition 04 | `657A3B6F-B23F-4804-98EB-3213FF54E84F` | animated graphic shapes overlay |
| Sports Pack Transition 05 | `1E9976C8-24BF-4AA2-934D-A533FC015AC0` | animated graphic shapes overlay |
| Swap Left | `C3612776-D62E-41a2-8733-0429F50E11F5` | overlay artwork |
| Take A Photo | `35602B81-FAA0-473D-90D1-672D135EEAD8` | overlay artwork |
| Technology HUD Transition 01 | `C9C2E39A-F581-4D9D-B1EF-140FC7F45798` | random-order cell / noise dissolve (order not reproducible) |
| Technology Light Transition 05 | `C78AAB26-6D08-404B-8B01-420EB927109A` | animated graphic shapes overlay |
| Top Bottom Gate | `C3043949-5B2B-444E-9079-3A7B7B2F52A1` | 3D multi-panel layout |
| Torn Paper Transition 09 | `0EB21BC5-6121-40A3-987A-217661816A98` | torn band with moving centre |
| Vertical Cutout 1 | `F2F3C70C-0ADF-41BE-B4EB-B2DDC98CDFD9` | 3D multi-panel layout |
| Vertical Slice | `3B9CFBA3-EEE5-4C4D-A8AA-457A8B4044C9` | glitch / strobe flashes with RGB split |
| Vignette | `FAE8A09D-E8C6-436D-9DD4-773311D875AA` | overlay artwork |
| VignetteOverlay | `B27F83E8-3736-4105-8B4B-BAB3CD2FBE32` | overlay artwork |
| Vintage Glamour Transition 02 | `A6F74AF0-069F-4B0B-985E-9032ED630409` | brush-stroke matte (hand-painted stroke animation) |
| Vision Blocks | `CA0C9A79-A8F7-4B25-AC48-942275E756A9` | 3D multi-panel layout |
| Wedding Invitation Template Transition 01 | `9CC68058-8C13-4EC4-8B59-5BB5752CBEDD` | brush-stroke matte (hand-painted stroke animation) |
| Wedding Invitation Template Transition 02 | `97426E0D-3458-4316-877B-4FD00C507D82` | brush-stroke matte (hand-painted stroke animation) |
| Wedding Invitation Template Transition 04 | `546B3563-6DE9-44D1-9386-390BA3D3E73B` | brush-stroke matte (hand-painted stroke animation) |
| Whirl 1 | `b2f6224e-7040-43ff-be6d-d716d68885b0` | overlay artwork |
| Wormhole 1 | `ec43f77c-9c92-4c0b-a6ba-2842caf4af82` | overlay artwork |
| Wormhole 2 | `74ae63b9-ce3e-47e8-966a-0d5c2dada46e` | overlay artwork |
| Wormhole 3 | `4d71fc4e-d3c5-4e82-b57c-2312572feba1` | overlay artwork |
| Wormhole 6 | `947d8c0d-576a-430e-8e70-cbffcd33b02e` | overlay artwork |
| Year of the Tiger Pack Transition 02 | `B3DB8933-F2E5-4094-975D-2595F0121FC2` | particle / sparkle overlay |

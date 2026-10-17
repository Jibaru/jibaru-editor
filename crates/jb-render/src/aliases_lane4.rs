//! Filmora store/built-in resources (lane 4: glitch, distortion, warp and
//! creative transitions, video effects and filters) mapped onto the fitted
//! implementations in `fx/lane4_*.rs`. GUIDs keep Filmora's exact case
//! (`register` also adds the upper-case form). Comments give the Filmora
//! item each GUID was measured on.

use crate::registry::Registry;

pub const FILMORA_LANE4: &[(&str, &str)] = &[
    ("BCA8370B-09F8-4781-9366-455C1DEAF546", "jb.l4.cam_bright_zoom"), // Exposure Zoom
    ("E635E891-4C59-4FDC-8886-25E9E491ADD4", "jb.l4.cam_color_pulse"), // Color Flash
    ("880A175D-1188-47CE-849B-2895AFB10B68", "jb.l4.cam_crash_zoom"),  // Crash Into
    ("F84EF9AC-35E0-48E7-AE22-57FCF5F4F58C", "jb.l4.cam_elastic_jitter"), // Elasticity Jitter
    ("F930B799-A2F7-441B-B29F-85E9DD4529A1", "jb.l4.cam_flash_pop"),   // Flash Exposure
    ("B84A6B57-861A-4931-815D-4B52E1C5CA23", "jb.l4.cam_gentle_sway"), // Mild
    ("B8345B57-861A-4931-815D-4B52E1C5CA23", "jb.l4.cam_hard_shake"),  // Extreme
    ("D8F99C1E-F602-4E75-B9E2-3DC77BD43EC3", "jb.l4.cam_impact_flash"), // Collision
    ("FE01364B-0A3A-4017-B36F-2BAC4124ED2B", "jb.l4.cam_impact_shake"), // Shake the Camera
    ("B8AE0CA9-862C-4B50-8388-63562DC9949F", "jb.l4.cam_lens_punch"),  // Zoom Lens
    ("7F32C000-03A3-425C-9E0F-DBEAA4670ED6", "jb.l4.cam_rgb_jolt"),    // Chaos Shake
    ("1DC65F8D-294B-4F68-8FAC-65F1813ADC91", "jb.l4.cam_rumble"),      // Earthquake Shake
    ("A225427F-4665-474A-91C3-8F121CA73962", "jb.l4.cam_slow_push"),   // Parallax Smooth Zoom In
    ("B84A6B57-861A-4931-825D-4B42E1C3CB34", "jb.l4.cam_vertical_whip"), // Up-Down 1
    ("A414A437-227A-462F-9C22-6389016B192B", "jb.l4.cam_zoom_burst"),  // Fast Zoom
    ("740E5BD0-1BAD-4688-AF49-946FAE0F2E7A", "jb.l4.look_airy_blue_lift"), // Bright Lights
    ("23ED6584-3EB2-4451-80CE-89D3529DE5C8", "jb.l4.look_blue_sky"),   // BlueSkies
    ("25E4478A-C617-4c60-B95A-F378D2923E17", "jb.l4.look_bright_glow"), // Glow
    ("B23034C1-24C4-4504-9776-6805CFBD7F03", "jb.l4.look_canvas_tone"), // Canvas
    ("74471F93-01ED-42E7-AA4B-37E1A4E8BB33", "jb.l4.look_clear_day"),  // Sunny Days
    ("BB101941-A076-4332-B70F-22DD4B4236BD", "jb.l4.look_contrast_vignette"), // Lomo
    ("BB101941-A166-4332-B70F-22DD4B4236BD", "jb.l4.look_cross_process"), // XproII
    ("BB101941-A126-4332-B70F-22DD4B4236BD", "jb.l4.look_dark_moody"), // Sutro
    ("905A096A-F23D-4385-A6BC-64D086E7BB31", "jb.l4.look_dark_muted"), // Lut Process
    ("6DE84CB8-C186-4419-8E88-6EE3B2967660", "jb.l4.look_dusk_fade"),  // SunSet
    ("BB101941-A116-4332-B70F-22DD4B4236BD", "jb.l4.look_faded_vignette"), // Sierra
    ("BB101941-A016-4332-B70F-22DD4B4236BD", "jb.l4.look_glow_matte"), // Amaro
    ("BB101941-A036-4332-B70F-22DD4B4236BD", "jb.l4.look_golden_morning"), // Earlybird
    ("b50b58e2-9637-441c-893f-04f93e946256", "jb.l4.look_grunge_vintage"), // Vintage Film Pack GRUNGE FILM VINTAGE
    ("57AA3F05-D0BF-4664-8EA8-DC50AF202C36", "jb.l4.look_highlight_lift"), // Highlight Glow
    ("BB101941-A086-4332-B70F-22DD4B4236BD", "jb.l4.look_hot_amber"),  // LordKelvin
    ("BB101941-A056-4332-B70F-22DD4B4236BD", "jb.l4.look_icy_vignette"), // Hudson
    ("76C761E8-9F21-4654-A764-CBB764F7A3BA", "jb.l4.look_mono_bright"), // B&W Style
    ("BB101941-A066-4332-B70F-22DD4B4236BD", "jb.l4.look_mono_classic"), // InkWell
    ("F2D9D6E1-7E10-4CB2-AD49-58C80669EE39", "jb.l4.look_old_tv_border"), // Old TV Border
    ("B815B718-4DF1-4ACB-BED3-183DA177BB78", "jb.l4.look_overexposed"), // Exposure
    ("BB101941-A156-4332-B70F-22DD4B4236BD", "jb.l4.look_pale_cyan"),  // Walden
    ("BB101941-A096-4332-B70F-22DD4B4236BD", "jb.l4.look_pink_pastel"), // Nashville
    ("2FFCED76-6277-41a5-B1AD-4BB27C8D44D3", "jb.l4.look_prism_rays"), // Radio Explike FX
    ("BB101941-A006-4332-B70F-22DD4B4236BD", "jb.l4.look_rose_fade"),  // 1977
    ("73C53B1A-4233-4509-9009-1B514F465B7A", "jb.l4.look_scanlines"),  // Scanline Effect 1
    ("FB66DCAA-F85E-4be4-9470-9D6771AE26B8", "jb.l4.look_season_tint"), // Four Seasons
    ("BB101941-A146-4332-B70F-22DD4B4236BD", "jb.l4.look_soft_warm_fade"), // Valencia
    ("aa7e00ee-9b40-4160-abde-c18f7f344209", "jb.l4.look_sunbaked_vintage"), // CALIFORNIA VINTAGE FILM LOOK
    ("BB101941-A136-4332-B70F-22DD4B4236BD", "jb.l4.look_toasted"),    // Toaster
    ("9E2DC6CF-BDC2-4368-99DD-DBB8A42A8298", "jb.l4.look_vivid_warm"), // Vivid
    ("BB101941-A026-4332-B70F-22DD4B4236BD", "jb.l4.look_warm_contrast_fade"), // Brannan
    ("BB101941-A106-4332-B70F-22DD4B4236BD", "jb.l4.look_warm_haze"),  // Sunrise
    ("BB101941-A046-4332-B70F-22DD4B4237BD", "jb.l4.look_warm_punch"), // Hefe
    ("43E7D729-DCA4-420D-A6CB-0DDCA64F8C51", "jb.l4.region_smear"),    // Mosaic
    ("7a8c5da4-149c-f011-b98c-a70c5927357a", "jb.l4.reveal_clean_spread_down"), // Moonlight Ink Transition 03
    ("47E7E56D-3AF0-48A1-8787-4456C0A51A1A", "jb.l4.reveal_clean_spread_down_left_1"), // Brush And Ink Transition 40
    ("bb385dac-27c9-f011-ae39-581122bdf438", "jb.l4.reveal_clean_spread_down_right_1"), // Liquid Spread Transition 10
    ("7c8c5da4-149c-f011-b98c-a70c5927357a", "jb.l4.reveal_clean_spread_down_right_2"), // Moonlight Ink Transition 05
    ("5f982dfa-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_clean_spread_left_1"), // Dynamic Ink 10
    ("0334ae95-3f59-ef11-9cc6-50ebf65600b6", "jb.l4.reveal_clean_spread_left_2"), // Glow Ink Left
    ("5684e760-3641-f111-b45f-d8bbc19410ee", "jb.l4.reveal_clean_spread_left_3"), // Luminous Ink Wash 07
    ("C6DC4DC9-230F-4BFF-8AE3-49B8790A00DB", "jb.l4.reveal_clean_spread_right_1"), // Colorful Ink Slideshow Vol 01 Transition 04
    ("5e982dfa-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_clean_spread_right_2"), // Dynamic Ink 08
    ("ba385dac-27c9-f011-ae39-581122bdf438", "jb.l4.reveal_clean_spread_right_3"), // Liquid Spread Transition 05
    ("346BA694-6246-4444-8410-DAF063FD2CEC", "jb.l4.reveal_clean_spread_up_right"), // Ink Reveal Transition 02
    ("A4B409A2-8335-45D9-8C7D-3D7858DBD37C", "jb.l4.reveal_clean_sweep_down_1"),   // Liquid Bodies Pack Transition 03
    ("43c38def-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_clean_sweep_down_2"),   // Dynamic Ink 06
    ("752A5769-FA69-4FEC-ABD4-C24C7A591B80", "jb.l4.reveal_clean_sweep_up_right"), // Brush And Ink Transition 34
    ("06906587-3d59-ef11-9cc6-50ebf65600b6", "jb.l4.reveal_ink_bloom_1"),          // Glow Ink Down
    ("b9385dac-27c9-f011-ae39-581122bdf438", "jb.l4.reveal_ink_bloom_2"),          // Liquid Spread Transition 06
    ("5b84e760-3641-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_bloom_3"),          // Luminous Ink Wash 01
    ("adfdb0d3-3841-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_bloom_4"),          // Luminous Ink Wash 02
    ("aefdb0d3-3841-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_down_1"),    // Luminous Ink Wash 08
    ("affdb0d3-3841-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_down_2"),    // Luminous Ink Wash 09
    ("FD9E8EE9-7A96-4D1F-B4C3-96AFB2BC6E09", "jb.l4.reveal_ink_spread_down_left_1"), // Comic Smoke Transition 07
    ("60982dfa-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_down_left_2"), // Dynamic Ink 09
    ("7b8c5da4-149c-f011-b98c-a70c5927357a", "jb.l4.reveal_ink_spread_down_left_3"), // Moonlight Ink Transition 01
    ("61982dfa-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_down_right_1"), // Dynamic Ink 07
    ("e65db0f1-0118-ef11-9871-d45d64b7b961", "jb.l4.reveal_ink_spread_down_right_2"), // Ink Brush Mask Transition
    ("d6f5d16b-3d59-ef11-9cc6-50ebf65600b6", "jb.l4.reveal_ink_spread_right_1"),   // Glow Ink Right
    ("44c38def-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_up_left_1"), // Dynamic Ink 05
    ("c0385dac-27c9-f011-ae39-581122bdf438", "jb.l4.reveal_ink_spread_up_left_2"), // Liquid Spread Transition 02
    ("b8385dac-27c9-f011-ae39-581122bdf438", "jb.l4.reveal_ink_spread_up_right_1"), // Liquid Spread Transition 09
    ("5e84e760-3641-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_up_right_2"), // Luminous Ink Wash 05
    ("5a84e760-3641-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_spread_up_right_3"), // Luminous Ink Wash 06
    ("788c5da4-149c-f011-b98c-a70c5927357a", "jb.l4.reveal_ink_spread_up_right_4"), // Moonlight Ink Transition 02
    ("B7B30A7F-6111-4073-B3A8-FF5106B35DB4", "jb.l4.reveal_ink_sweep_down_1"),     // Cartoon Liquid Transition 04
    ("45c38def-113f-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_sweep_down_2"),     // Dynamic Ink 03
    ("798c5da4-149c-f011-b98c-a70c5927357a", "jb.l4.reveal_ink_sweep_left"),       // Moonlight Ink Transition 04
    ("3452BC10-7B93-47C3-927A-37F3236B7E55", "jb.l4.reveal_ink_sweep_right_1"),    // Brush And Ink Transition 36
    ("19513BFA-4833-49B1-A9B6-0347E4564330", "jb.l4.reveal_ink_sweep_right_2"),    // Brush Paint Pack Transition 09
    ("5c84e760-3641-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_sweep_right_3"),    // Luminous Ink Wash 10
    ("24B2F0AB-8F3A-484C-98B4-87993078C78F", "jb.l4.reveal_ink_sweep_right_4"),    // Blink Eyes 03
    ("ed8298c1-3a59-ef11-9cc6-50ebf65600b6", "jb.l4.reveal_ink_sweep_up_left_1"),  // Glow Ink Up
    ("5784e760-3641-f111-b45f-d8bbc19410ee", "jb.l4.reveal_ink_sweep_up_left_2"),  // Luminous Ink Wash 03
    ("B8847513-A575-411A-A3D1-B784A3096C7E", "jb.l4.spot_color_jump"),             // Local Color
    ("562330DC-1334-4b02-A5B9-B140FC68B0E2", "jb.l4.video_wall_3x3"),              // TV Wall
    ("9eb554c3-3cc1-4b9b-b53b-3e58974b2562", "jb.l4.warp_twist_roll_1"),           // Orb Twist 1
    ("abd14627-91a7-443f-a802-ef750497cc90", "jb.l4.warp_twist_roll_2"),           // Orb Twist 3
    ("7cd2d726-4e60-4d51-a31b-30b5d23e31a7", "jb.l4.warp_zoom_burst_1"),           // Warp 3
    ("118eb91c-9aec-488b-8052-63592abc37b2", "jb.l4.warp_zoom_burst_2"),           // Warp 6
    ("a544c0be-010a-4b00-9b7b-1f1a9227674b", "jb.l4.warp_zoom_burst_3"),           // Warp 7
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE4 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

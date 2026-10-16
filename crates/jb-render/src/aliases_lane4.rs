//! Filmora store/built-in resources (lane 4: glitch, distortion, warp and
//! creative transitions, video effects and filters) mapped onto the fitted
//! implementations in `fx/lane4_*.rs`. GUIDs keep Filmora's exact case
//! (`register` also adds the upper-case form). Comments give the Filmora
//! item each GUID was measured on.

use crate::registry::Registry;

pub const FILMORA_LANE4: &[(&str, &str)] = &[
    ("740E5BD0-1BAD-4688-AF49-946FAE0F2E7A", "jb.l4.look_airy_blue_lift"), // Bright Lights
    ("23ED6584-3EB2-4451-80CE-89D3529DE5C8", "jb.l4.look_blue_sky"),       // BlueSkies
    ("74471F93-01ED-42E7-AA4B-37E1A4E8BB33", "jb.l4.look_clear_day"),      // Sunny Days
    ("BB101941-A076-4332-B70F-22DD4B4236BD", "jb.l4.look_contrast_vignette"), // Lomo
    ("BB101941-A166-4332-B70F-22DD4B4236BD", "jb.l4.look_cross_process"),  // XproII
    ("BB101941-A126-4332-B70F-22DD4B4236BD", "jb.l4.look_dark_moody"),     // Sutro
    ("905A096A-F23D-4385-A6BC-64D086E7BB31", "jb.l4.look_dark_muted"),     // Lut Process
    ("6DE84CB8-C186-4419-8E88-6EE3B2967660", "jb.l4.look_dusk_fade"),      // SunSet
    ("BB101941-A116-4332-B70F-22DD4B4236BD", "jb.l4.look_faded_vignette"), // Sierra
    ("BB101941-A016-4332-B70F-22DD4B4236BD", "jb.l4.look_glow_matte"),     // Amaro
    ("BB101941-A036-4332-B70F-22DD4B4236BD", "jb.l4.look_golden_morning"), // Earlybird
    ("BB101941-A086-4332-B70F-22DD4B4236BD", "jb.l4.look_hot_amber"),      // LordKelvin
    ("BB101941-A056-4332-B70F-22DD4B4236BD", "jb.l4.look_icy_vignette"),   // Hudson
    ("76C761E8-9F21-4654-A764-CBB764F7A3BA", "jb.l4.look_mono_bright"),    // B&W Style
    ("BB101941-A066-4332-B70F-22DD4B4236BD", "jb.l4.look_mono_classic"),   // InkWell
    ("BB101941-A156-4332-B70F-22DD4B4236BD", "jb.l4.look_pale_cyan"),      // Walden
    ("BB101941-A096-4332-B70F-22DD4B4236BD", "jb.l4.look_pink_pastel"),    // Nashville
    ("BB101941-A006-4332-B70F-22DD4B4236BD", "jb.l4.look_rose_fade"),      // 1977
    ("BB101941-A146-4332-B70F-22DD4B4236BD", "jb.l4.look_soft_warm_fade"), // Valencia
    ("BB101941-A136-4332-B70F-22DD4B4236BD", "jb.l4.look_toasted"),        // Toaster
    ("9E2DC6CF-BDC2-4368-99DD-DBB8A42A8298", "jb.l4.look_vivid_warm"),     // Vivid
    ("BB101941-A026-4332-B70F-22DD4B4236BD", "jb.l4.look_warm_contrast_fade"), // Brannan
    ("BB101941-A106-4332-B70F-22DD4B4236BD", "jb.l4.look_warm_haze"),      // Sunrise
    ("BB101941-A046-4332-B70F-22DD4B4237BD", "jb.l4.look_warm_punch"),     // Hefe
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE4 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

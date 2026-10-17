//! Filmora transition GUIDs recreated with original artwork in art lane 3a
//! (`fx/art3a*.rs`), in the exact case Filmora writes them, mapped onto our
//! implementations. Each GUID is mapped here and nowhere else.

use crate::registry::Registry;

/// (Filmora GUID, our id, Filmora item name for reference).
pub const FILMORA_ART3A: &[(&str, &str, &str)] = &[
    ("582C0031-3A5A-4839-86C2-BD5662120FAD", "jb.iris_shutter", "Camera Shutter"),
    ("0A51BCC4-B46C-40E3-AB2E-072317A483D0", "jb.lens_iris", "Camera Transition 09"),
    ("C4A965E6-3B6F-478A-8379-FE71532866ED", "jb.pumpkin_grin", "Halloween Colorful Transition 04"),
    ("3EBA6851-566C-47A2-B19A-77BF4F1ECC8C", "jb.witch_hat_wave", "Halloween Colorful Transition 09"),
    ("A53E4A22-4646-48A4-9391-A49C68677760", "jb.bat_rise", "Halloween Colorful Transition 21"),
    ("D59BB790-3F65-4AA6-8F59-5485294E671D", "jb.skull_and_hat", "Halloween Colorful Transition 17"),
    ("E85C72C2-7A02-42F6-9B24-BD0456A19847", "jb.ghost_glide", "Horrible Colorful Transition 02"),
    ("37E1E9AB-58F5-459A-8078-327D080B90A7", "jb.boo_dive", "Horrible Colorful Transition 13"),
    ("6C96C244-355D-400C-B335-5442B68ED84C", "jb.stripe_close", "Business Presentation Vol 02 Transition 02"),
    ("af283ec2-2384-f56a-ce90-687075426050", "jb.arrow_stack", "Gaming Set Sports Transition 2"),
    ("801B0E20-1ED6-475B-A580-3807A1367B72", "jb.gold_diamond", "Golden Awards Ceremony Transition 05"),
    ("111DB3F8-9030-48DB-9D22-CF80B3F6CEF1", "jb.gold_chevrons", "Grammy Award 2024 Transition 02"),
    ("87003965-AF55-4819-A386-E7DF971D9456", "jb.gold_plates_sweep", "Golden Awards Pack Transition 04"),
    ("012134A4-92F3-4209-8605-7BCD218DC892", "jb.gold_plates_rise", "Golden Awards Transition 03"),
    ("C10C8F47-92A9-48EA-9D9A-10C1F0B89773", "jb.comic_page", "Comic Style 3"),
];

pub fn register(r: &mut Registry) {
    for (fid, to, _) in FILMORA_ART3A {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

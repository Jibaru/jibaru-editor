//! Filmora store/built-in resources recreated by the art-4 items
//! (`fx/art4_*.rs`, `docs/fidelity-art4.md`): our own procedural versions,
//! mapped from `filmora:<GUID>` so imported projects render with them.
//! GUIDs keep Filmora's exact case (`register` also adds the upper-case
//! form). Comments name the Filmora item each GUID was measured on.

use crate::registry::Registry;

pub const FILMORA_ART4: &[(&str, &str, &str)] = &[
    ("69E652B1-EB6E-460C-9929-DC6F744F02D6", "jb.a4.curved_box_turn_left", "3D Warp Box Left"),
    ("E27040D3-6FEE-4708-B95B-384055CE7FDA", "jb.a4.afterimage_rise", "Baltan"),
    ("3F9270DE-728F-4898-8C4C-7856B4540539", "jb.a4.ribbon_wave_split", "Butterfly Wave Scrawler"),
    ("50D37319-8F37-4593-BBAF-9FF0A6307C01", "jb.a4.prism_split_bursts", "Chromatic Aberration"),
    ("375BA8B0-2C67-4F58-BCDB-5CA6B0E82609", "jb.a4.glitch_flash_cut", "Glitch line"),
    ("3C2E38AD-0F25-4A22-9F99-A7B826B9DDC7", "jb.a4.dreamy_focus_in", "Heart Zoom"),
    ("9C488EA5-79EB-46A6-A4EE-4F16DE09F5F4", "jb.a4.banknote_engrave", "Money"),
    ("be2adb6c-70d7-49c2-a157-ab06541cd309", "jb.a4.block_pixelate", "Mosaic"),
    ("4041B1EF-9E4A-4F75-A58D-0CDC35C788FA", "jb.a4.weak_signal", "No Signal Glitch 01"),
    ("1B4AEE31-7904-45D2-827D-E15D76B271C9", "jb.a4.pixel_fadeout_swap", "Pixel Game Transition Transition 09"),
    ("429EF8B5-CFBB-486D-8130-1CEAE70ACFFC", "jb.a4.acid_flash", "Psychedelic Glitch"),
    ("DB22C576-091B-4160-90C6-9E5359B75595", "jb.a4.puddle_ripple", "Ripple"),
    ("814B656B-D238-4AA9-8639-A3A10E18E5D0", "jb.a4.shard_sweep", "Shattered 02"),
    ("1BF98C38-9A9B-49CF-9E7C-E33EB3AC5D1E", "jb.a4.shard_burst", "Shattered 03"),
    ("6BC77B03-9232-477B-975C-513B18F82705", "jb.a4.stripe_jolt", "Stripe Glitch 01"),
    ("12654A98-AE4F-49D5-ABB6-A3F2EEF977A5", "jb.a4.crisp_sharpen", "Strong Sharpening"),
    ("D429C480-BC4C-4e9d-AC65-A7626B34ECFD", "jb.a4.whirlpool", "Swirl"),
    ("9C91C7DA-1ABE-4304-BD50-CE7780E77EEA", "jb.a4.ring_ripple_reveal", "Wave"),
];

pub fn register(r: &mut Registry) {
    for (fid, to, _) in FILMORA_ART4 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

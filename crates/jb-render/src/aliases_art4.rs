//! Filmora store/built-in resources recreated by the art-4 items
//! (`fx/art4_*.rs`, `docs/fidelity-art4.md`): our own procedural versions,
//! mapped from `filmora:<GUID>` so imported projects render with them.
//! GUIDs keep Filmora's exact case (`register` also adds the upper-case
//! form). Comments name the Filmora item each GUID was measured on.

use crate::registry::Registry;

pub const FILMORA_ART4: &[(&str, &str, &str)] = &[
    ("3F9270DE-728F-4898-8C4C-7856B4540539", "jb.a4.ribbon_wave_split", "Butterfly Wave Scrawler"),
    ("DB22C576-091B-4160-90C6-9E5359B75595", "jb.a4.puddle_ripple", "Ripple"),
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

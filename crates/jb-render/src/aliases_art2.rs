//! Filmora store transitions recreated with our own procedural paint, ink
//! and paper artwork (`fx/art2_*.rs`) mapped to their `jb.*` ids. GUIDs keep
//! their exact case (Filmora resolves them case-sensitively). Each GUID is
//! mapped here only (its stand-in alias was removed from the older lane
//! tables).

use crate::registry::Registry;

/// (Filmora GUID, jb id, Filmora name).
pub const FILMORA_ART2: &[(&str, &str, &str)] = &[
    ("3B7844DB-2D95-4136-BE2E-4D59D3FB0CF8", "jb.brush_columns", "Brush Art Design Pack Transition 08"),
    ("b20e8acf-35d7-4958-aebc-64d2358c6b97", "jb.dry_brush_sweeps", "Beauty Brush Transition 1"),
];

pub fn register(r: &mut Registry) {
    for (fid, to, _) in FILMORA_ART2 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

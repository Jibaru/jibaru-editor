//! Filmora transitions of lane 2 (slides, pushes, rolls, cubes, flips, doors,
//! cards, folds, stacks, grids) mapped to their fitted implementations
//! (`fx/lane2_layers.rs`). GUIDs keep Filmora's exact case; both the exact
//! and the upper-cased form are registered.

use crate::registry::Registry;

/// (Filmora resource GUID, our id).
pub const FILMORA_LANE2: &[(&str, &str)] = &[
    // Cube Right
    ("0FCB240F-5328-4e85-869A-2D989F50D387", "jb.cube_turn_right"),
    // 3D Flip Right
    ("B04A402D-5C04-4110-AEAD-2E3DBEC1FEFF", "jb.flip_3d_right"),
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE2 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

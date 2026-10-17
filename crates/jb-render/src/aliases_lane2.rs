//! Filmora transitions of lane 2 (slides, pushes, rolls, cubes, flips, doors,
//! cards, folds, stacks, grids) mapped to their fitted implementations
//! (`fx/lane2_layers.rs`). GUIDs keep Filmora's exact case; both the exact
//! and the upper-cased form are registered.

use crate::registry::Registry;

/// (Filmora resource GUID, our id).
pub const FILMORA_LANE2: &[(&str, &str)] = &[
    // 3D Box Drop
    ("3B310488-6093-4A52-B87E-0367ADF1641A", "jb.box_drop_3d"),
    // 3D Box Fall
    ("0C615FD6-6C3D-469A-955C-5FDF8FA6A23A", "jb.box_fall_3d"),
    // 3D Box Flip 5
    ("64D165BC-C38F-400F-965D-A9E33BA1128C", "jb.box_flip_3d_5"),
    // Cube Down
    ("59A8A58F-31CB-4740-9F9D-0DCD3CB7AF92", "jb.cube_turn_down"),
    // Cube Left
    ("A6B6286F-F9FE-4F9F-8C45-538EEC05F620", "jb.cube_turn_left"),
    // Cube Right
    ("0FCB240F-5328-4e85-869A-2D989F50D387", "jb.cube_turn_right"),
    // Cube Up
    ("5107B490-0EA4-47E1-8343-BD779895685F", "jb.cube_turn_up"),
    // Diagonal Slice Reveal
    ("E5C74593-D113-47B5-B174-252E1716826C", "jb.diagonal_slice_reveal"),
    // 3D Flip Right
    ("B04A402D-5C04-4110-AEAD-2E3DBEC1FEFF", "jb.flip_3d_right"),
    // 3D Flip
    ("EC14B694-D0AD-425A-806C-BA9D288341CC", "jb.flip_3d_tilt"),
    // Flip Down
    ("804982E3-3FC8-425E-B43A-6444F2C53B79", "jb.flip_down"),
    // Flip Left
    ("F309557C-8B97-499A-9696-29B0FDD6367C", "jb.flip_left"),
    // Flip Right
    ("7B680F51-1B11-480D-A4DE-C5065188BBCC", "jb.flip_right"),
    // Flip Roll Down
    ("4057dc68-ff07-4561-90b7-97762af89945", "jb.flip_roll_down"),
    // Flip Roll Left
    ("ae8e58de-6e52-4fca-851f-f8a6dfd41647", "jb.flip_roll_left"),
    // Flip Roll Up
    ("af113622-985e-4ce3-a807-9601607ee43f", "jb.flip_roll_up"),
    // Fast Push Left
    ("2DD72324-4654-456D-9EA8-9931F6B6A1FD", "jb.push_left_fast"),
    // Simple Roll 11
    ("490e0bdb-f7b7-4fb2-a92a-ed5927ca13b3", "jb.roll_11"),
    // Simple Roll 6
    ("3e2c9408-4b1e-4e78-afc4-4d6c837f6c6e", "jb.roll_6"),
    // Simple Roll 7
    ("0fbaebd7-c224-4ca5-a9f0-d16307f19854", "jb.roll_7"),
    // Simple Roll 2
    ("d8867ccc-4762-49c4-93fd-616208d056cf", "jb.roll_down_2"),
    // Skew Left Split
    ("5003D879-7930-44fa-A333-F21F62564709", "jb.skew_split_left"),
    // Skew Right Split
    ("2D4099D2-9B28-437f-8FEB-2A7E25C0B549", "jb.skew_split_right"),
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE2 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

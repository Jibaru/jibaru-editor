//! Library groups of the art3a transitions.

use crate::library::LibraryGroup;
use crate::registry::FxKind::Transition as T;

pub const LIBRARY: &[LibraryGroup] = &[
    LibraryGroup { kind: T, name: "Camera", items: &["jb.iris_shutter", "jb.lens_iris"] },
    LibraryGroup { kind: T, name: "Graphics", items: &["jb.stripe_close", "jb.arrow_stack"] },
    LibraryGroup {
        kind: T,
        name: "Halloween",
        items: &[
            "jb.pumpkin_grin",
            "jb.witch_hat_wave",
            "jb.bat_rise",
            "jb.skull_and_hat",
            "jb.ghost_glide",
            "jb.boo_dive",
            "jb.ghost_crowd",
            "jb.bat_swarm_dive",
            "jb.lantern_face",
        ],
    },
];

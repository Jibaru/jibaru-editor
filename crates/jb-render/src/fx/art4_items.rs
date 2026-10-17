//! The art-4 items as data (model + constants + keyframe rows, generated
//! into `art4_data.rs`). The rows were fitted on Filmora's exported output
//! of each item, no Filmora asset was read; see `docs/fidelity-art4.md`.

#[path = "art4_data.rs"]
mod data;

pub use data::{LIB_EFFECTS, LIB_FILTERS, LIB_TRANSITIONS, SPECS};

use crate::library::LibraryGroup;
use crate::registry::{FxKind, Registry};

/// Library groups of the art-4 items (empty ones are skipped by
/// [`crate::library::all_groups`]).
pub const LIBRARY: &[LibraryGroup] = &[
    LibraryGroup { kind: FxKind::Transition, name: "Warps & Glitches", items: LIB_TRANSITIONS },
    LibraryGroup { kind: FxKind::Effect, name: "Warps & Glitches", items: LIB_EFFECTS },
    LibraryGroup { kind: FxKind::Filter, name: "Stylized Looks", items: LIB_FILTERS },
];

pub fn register(r: &mut Registry) {
    crate::fx::art4_kit::register(r, SPECS);
}

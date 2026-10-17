//! Art3b: graphic, cartoon and themed motion transitions (shapes, comic
//! bursts, holiday / sports / business graphics, motion-graphics packs)
//! recreated with our own vector art (`art3b_kit.rs`). Timing, motion paths,
//! coverage over time and palettes follow Filmora 15.7.3's exports of the
//! corresponding store items (`docs/fidelity-art3b.md`); every shape is
//! drawn here from primitives, nothing is taken from Filmora's assets or
//! frames. Filmora GUIDs map onto these items in `aliases_art3b.rs`.

#[path = "art3b_items.rs"]
pub mod items;
#[path = "art3b_kit.rs"]
pub mod kit;
#[path = "art3b_themed.rs"]
pub mod themed;

use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    kit::register_items(r, items::ITEMS);
}

//! Lane 4 of the Filmora fidelity work: glitch, distortion, warp and
//! creative transitions, video effects and color filters, each fitted to
//! Filmora 15.7.3's exported output at default settings
//! (`docs/fidelity-lane4.md`). Filmora GUIDs map onto these items in
//! `aliases_lane4.rs`.

#[path = "lane4_camera.rs"]
pub mod camera;
#[path = "lane4_looks.rs"]
pub mod looks;
#[path = "lane4_reveal.rs"]
pub mod reveal;

use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    looks::register(r);
    camera::register(r);
    reveal::register(r);
}

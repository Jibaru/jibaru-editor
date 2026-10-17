//! Lane 4 of the Filmora fidelity work: glitch, distortion, warp and
//! creative transitions, video effects and color filters, each fitted to
//! Filmora 15.7.3's exported output at default settings
//! (`docs/fidelity-lane4.md`). Filmora GUIDs map onto these items in
//! `aliases_lane4.rs`.

#[path = "lane4_camera.rs"]
pub mod camera;
#[path = "lane4_camtr.rs"]
pub mod camtr;
#[path = "lane4_looks.rs"]
pub mod looks;
#[path = "lane4_region.rs"]
pub mod region;
#[path = "lane4_reveal.rs"]
pub mod reveal;
#[path = "lane4_spot.rs"]
pub mod spot;

use crate::registry::Registry;

/// Library listing of the lane-4 stylized effects (one-off models).
pub const LIB_STYLIZED: &[&str] = &["jb.l4.spot_color_jump", "jb.l4.region_smear"];

pub fn register(r: &mut Registry) {
    looks::register(r);
    camera::register(r);
    camtr::register(r);
    reveal::register(r);
    region::register(r);
    spot::register(r);
}

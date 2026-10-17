//! Art lane 3a: graphic, cartoon and themed transitions (camera shutters,
//! comic bursts, holiday, sports and business graphics, motion-graphics
//! packs) recreated with original vector artwork drawn in the shader.
//!
//! Filmora's versions of these items are built around hand-made artwork.
//! Ours follow the timing, motion path, screen coverage and palette measured
//! on Filmora's exports of the reference batches (`docs/fidelity-art3a.md`),
//! with shapes drawn from scratch as signed-distance artwork
//! (`shaders/art3a/kit.wgsl`); nothing is traced or copied from the
//! reference frames, and no Filmora asset is read.

use crate::fx::lane3::kit::{register_items, table, Item};
use crate::registry::Registry;

#[path = "art3a_business.rs"]
mod business;
#[path = "art3a_camera.rs"]
mod camera;
#[path = "art3a_halloween.rs"]
mod halloween;
#[path = "art3a_library.rs"]
mod library;

pub use library::LIBRARY;

/// WGSL of the art kit (shapes, fills, layering), prepended to every item.
pub const ART_KIT: &str = include_str!("../shaders/art3a/kit.wgsl");

/// Duration of the reference transitions in frames (2.002 s at 29.97 fps):
/// keyframe rows written in frames are converted with it.
pub const REF_FRAMES: f32 = 60.0;

/// Keyframe rows written per reference frame (`[frame, values...]`) as
/// progress rows (`frame / REF_FRAMES`) for [`table`].
pub fn at_frames<const K: usize>(rows: &[[f32; K]]) -> Vec<[f32; K]> {
    rows.iter()
        .map(|r| {
            let mut o = *r;
            o[0] = (r[0] / REF_FRAMES).clamp(0.0, 1.0);
            o
        })
        .collect()
}

/// WGSL of an art item: the art kit, `consts` (style constants of a shared
/// body), keyframe tables and the body (which defines `fx`).
pub fn art_wgsl(consts: &str, tables: &[String], body: &str) -> String {
    format!("{ART_KIT}\n{consts}\n{}\n{body}", tables.join("\n"))
}

/// An art transition: lane 3 [`Item`] plumbing with the art kit prepended.
pub fn art_item(
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    wgsl: String,
) -> Item {
    Item::new(id, name, category, description, wgsl)
}

/// Shorthand for one table of an art item.
pub fn tab<const K: usize>(name: &str, rows: &[[f32; K]]) -> String {
    table(name, rows)
}

pub fn register(r: &mut Registry) {
    let mut items = Vec::new();
    items.extend(business::items());
    items.extend(camera::items());
    items.extend(halloween::items());
    register_items(r, items);
}

//! Lane 3 of the Filmora fidelity work: shape/luma wipes, dissolves and
//! fades, light/flash/leak and film-burn transitions, and other transitions
//! fitted on Filmora's exports (results in docs/fidelity-lane3.md).

use crate::registry::Registry;

#[path = "lane3_data.rs"]
pub mod data;
#[path = "lane3_fades.rs"]
pub mod fades;
#[path = "lane3_kit.rs"]
pub mod kit;
#[path = "lane3_light.rs"]
pub mod light;
#[path = "lane3_library.rs"]
mod library;
#[path = "lane3_overlays.rs"]
pub mod overlays;
#[path = "lane3_shapes.rs"]
pub mod shapes;
#[path = "lane3_themed.rs"]
pub mod themed;
#[path = "lane3_wipes.rs"]
pub mod wipes;

pub use library::LIBRARY;

pub fn register(r: &mut Registry) {
    fades::register(r);
    wipes::register(r);
    light::register(r);
    shapes::register(r);
    themed::register(r);
    overlays::register(r);
}

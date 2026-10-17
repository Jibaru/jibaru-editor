//! Themed (holiday, party) transitions whose artwork is not reproduced but
//! whose wipe is: the new clip is revealed by the fitted mask (a circular
//! front around a fitted centre, often far outside the frame so the edge is
//! a gentle curve) with the fitted timing. The cartoon sprites that ride on
//! the wipe in Filmora (ghosts, cats, bats, ...) are left out.

use super::data as D;
use super::kit::{edge_wipe_graded, register_items, Item};
use crate::registry::Registry;

/// Circular front around `(cx, cy)` (uv, aspect-corrected distance); B inside
/// the circle, or outside it when `inv`.
fn radial(cx: f32, cy: f32, inv: bool, rows: &[[f32; 3]], grade: &[[f32; 5]]) -> String {
    let sign = if inv { "-" } else { "" };
    edge_wipe_graded(&format!("{sign}length((uv - vec2f({cx:?}, {cy:?})) * vec2f(A, 1.0))"), rows, grade)
}

fn items() -> Vec<Item> {
    vec![Item::new(
        "jb.ghost_curve_wipe",
        "Ghost Curve Wipe",
        "Creative",
        "A gently curved front rises from the bottom right and sweeps up over the frame to the new clip, in \
         the stepped rhythm of a hand-animated cartoon. Wipe fitted on Filmora's export of Halloween Colorful \
         Transition 23 (its ghost sprite is not drawn).",
        radial(1.129, 3.599, false, D::HC23_EDGE, D::HC23_GRADE),
    )]
}

pub fn register(r: &mut Registry) {
    register_items(r, items());
}

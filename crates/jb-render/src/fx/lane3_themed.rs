//! Themed (holiday, party) transitions whose artwork is not reproduced but
//! whose wipe is: the new clip is revealed by the fitted mask (a circular
//! front around a fitted centre, often far outside the frame so the edge is
//! a gentle curve) with the fitted timing. The cartoon sprites that ride on
//! the wipe in Filmora (ghosts, cats, bats, ...) are left out.

use super::data as D;
use super::kit::{edge_wipe_graded, register_items, table, Item};
use crate::registry::Registry;

/// Circular front around `(cx, cy)` (uv, aspect-corrected distance); B inside
/// the circle, or outside it when `inv`.
fn radial(cx: f32, cy: f32, inv: bool, rows: &[[f32; 3]], grade: &[[f32; 5]]) -> String {
    let sign = if inv { "-" } else { "" };
    edge_wipe_graded(&format!("{sign}length((uv - vec2f({cx:?}, {cy:?})) * vec2f(A, 1.0))"), rows, grade)
}

/// Colour rings: concentric bands around (0.505, 0.522) uv with outer radii
/// 0.135 (cyan), 0.235 (yellow), 0.393 (pink), 0.506 (cyan) frame heights
/// (measured) and estimated outer yellow, pink and cyan bands and a yellow
/// field beyond; each band is drawn on as an arc sweeping clockwise from 12
/// o'clock, inner bands first, and wiped off from the same end, so that the
/// covered fraction follows the fitted `cover` (rows `[p, cover]`); the
/// clips switch at frame 31 under the full cover. Colours measured on the
/// export.
const COLOR_RINGS: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let cover = kf_g(p).x;
    let c = to_c(uv) - to_c(vec2f(0.505, 0.522));
    let r = length(c);
    var band = 7.0;
    var col = vec3f(0.992, 0.996, 0.33);
    if (r < 0.86) { band = 6.0; col = vec3f(0.08, 0.796, 0.843); }
    if (r < 0.75) { band = 5.0; col = vec3f(0.957, 0.38, 0.467); }
    if (r < 0.6) { band = 4.0; col = vec3f(0.992, 0.996, 0.33); }
    if (r < 0.506) { band = 3.0; col = vec3f(0.08, 0.796, 0.843); }
    if (r < 0.393) { band = 2.0; col = vec3f(0.957, 0.38, 0.467); }
    if (r < 0.235) { band = 1.0; col = vec3f(0.992, 0.996, 0.33); }
    if (r < 0.135) { band = 0.0; col = vec3f(0.08, 0.796, 0.843); }
    let ang = fract(atan2(c.x, -c.y) / TAU + 1.0);
    let extent = clamp(cover * 1.7 - 0.1 * band, 0.0, 1.0);
    let second = p >= 30.5 / 60.0;
    let clip = l3_side(second, uv);
    // Drawn on clockwise from 12 o'clock, wiped off from the same end.
    let shown = select((ang < extent), (ang > 1.0 - extent), second);
    if (shown) { return l3_out(col); }
    return clip;
}
"#;

fn items() -> Vec<Item> {
    vec![
    Item::new(
        "jb.color_rings",
        "Color Rings",
        "Creative",
        "Concentric cyan, yellow and pink rings sweep in like a clock hand, inner rings first, until they and a \
         yellow field cover the frame; they sweep away again over the new clip. Ring layout, colours and \
         coverage over time fitted on Filmora's export of Colorful Transitions Vol 06 Transition 16.",
        format!("{}\n{COLOR_RINGS}", table("g", D::RINGS_COVER)),
    ),
    Item::new(
        "jb.ghost_curve_wipe",
        "Ghost Curve Wipe",
        "Creative",
        "A gently curved front rises from the bottom right and sweeps up over the frame to the new clip, in \
         the stepped rhythm of a hand-animated cartoon. Wipe fitted on Filmora's export of Halloween Colorful \
         Transition 23 (its ghost sprite is not drawn).",
        radial(1.129, 3.599, false, D::HC23_EDGE, D::HC23_GRADE),
    ),
    Item::new(
        "jb.skull_wedge_wipe",
        "Skull Wedge Wipe",
        "Creative",
        "A round front opens from a point right of the centre and widens over the frame to the new clip. Wipe \
         fitted on Filmora's export of Halloween Colorful Transition 06 (the skull sprite is not drawn).",
        radial(0.858, 0.506, false, D::HC06_EDGE, D::HC06_GRADE),
    ),
    Item::new(
        "jb.cat_curve_wipe",
        "Cat Curve Wipe",
        "Creative",
        "A curved front sweeps in from the right and across the frame to the new clip. Wipe fitted on \
         Filmora's export of Halloween Colorful Transition 25 (the cat sprite is not drawn).",
        radial(2.034, 3.0, false, D::HC25_EDGE, D::HC25_GRADE),
    ),
    Item::new(
        "jb.witch_hat_wipe",
        "Witch Hat Wipe",
        "Creative",
        "The old clip holds while a small sprite flies over it, then a round front bursts up from the lower \
         left and covers the frame with the new clip in a few frames. Wipe fitted on Filmora's export of \
         Halloween Colorful Transition 09 (the hat sprite is not drawn).",
        radial(0.281, 1.123, false, D::HC09_EDGE, D::HC09_GRADE),
    ),
    Item::new(
        "jb.pumpkin_pop_wipe",
        "Pumpkin Pop Wipe",
        "Creative",
        "A wide hill-shaped front rises from the bottom and covers the frame with the new clip in the second \
         half. Wipe fitted on Filmora's export of Halloween Colorful Transition 15 (the pumpkin sprite is not \
         drawn).",
        radial(0.492, 2.462, false, D::HC15_EDGE, D::GRADE_ID),
    ),
    Item::new(
        "jb.witch_silhouette_wipe",
        "Witch Silhouette Wipe",
        "Creative",
        "A round front grows from the top-left corner and sweeps the new clip across the frame. Wipe fitted on \
         Filmora's export of Horrible Colorful Transition 24 (the witch silhouette is not drawn).",
        radial(0.116, 0.356, false, D::HRC24_EDGE, D::HRC24_GRADE),
    ),
    ]
}

pub fn register(r: &mut Registry) {
    register_items(r, items());
}

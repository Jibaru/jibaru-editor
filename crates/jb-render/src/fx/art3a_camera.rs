//! Camera transitions: a lens iris closes over the outgoing clip and opens
//! on the incoming one, under a viewfinder overlay (original drawing:
//! `shaders/art3a/iris.wgsl`). Opening radius per reference frame measured
//! on Filmora's exports (radius where the clean picture covers half of
//! each ring around the centre).

use super::{art_item, art_wgsl, at_frames, tab};
use crate::fx::lane3::kit::Item;

const IRIS: &str = include_str!("../shaders/art3a/iris.wgsl");

/// Viewfinder of the shutter: a centre ring and eight focus points.
const HUD_SHUTTER: &str = r#"
fn ir_hud(c: vec2f, col0: vec3f, hud: f32, rings: f32, r: f32) -> vec3f {
    if (hud <= 0.0) { return col0; }
    var col = col0;
    let w = vec3f(0.92);
    col = ak_over(col, w, ak_line(sd_circle(c, 0.17), 0.0014) * 0.85 * hud);
    for (var k = 0.0; k < 8.0; k = k + 1.0) {
        let a = k * TAU / 8.0 + PI / 8.0;
        let q = c - vec2f(0.40 * cos(a) * 1.25, 0.36 * sin(a));
        col = ak_over(col, w, ak_line(sd_rbox(q, vec2f(0.016, 0.011), 0.002), 0.001) * 0.7 * hud);
    }
    // Small cross at the centre.
    let cr = min(sd_box(c, vec2f(0.012, 0.0008)), sd_box(c, vec2f(0.0008, 0.012)));
    col = ak_over(col, w, ak_fill(cr) * 0.6 * hud);
    return col;
}
"#;

/// Lens markings of the zoom-lens camera: focus rings, side pointers and a
/// graduated outer ring.
const HUD_LENS: &str = r#"
fn ir_hud(c: vec2f, col0: vec3f, hud: f32, rings: f32, r: f32) -> vec3f {
    if (hud <= 0.0) { return col0; }
    var col = col0;
    let w = vec3f(0.85);
    col = ak_over(col, w, ak_line(sd_circle(c, 0.155), 0.001) * 0.55 * hud);
    col = ak_over(col, w, ak_line(sd_circle(c, 0.425), 0.0012) * 0.6 * hud);
    // Pointers left and right of the outer focus ring.
    let px = vec2f(abs(c.x), c.y);
    let tri = sd_tri(px, vec2f(0.455, 0.0), vec2f(0.475, -0.012), vec2f(0.475, 0.012));
    col = ak_over(col, w, ak_fill(tri) * 0.8 * hud);
    // Focus dots on the inner ring axes.
    let pd = vec2f(abs(c.x), abs(c.y));
    let dot1 = min(sd_circle(pd - vec2f(0.0, 0.25), 0.0035), sd_circle(pd - vec2f(0.25, 0.0), 0.0035));
    col = ak_over(col, w, ak_fill(dot1) * 0.7 * hud);
    col = ak_over(col, w, ak_fill(sd_circle(c, 0.004)) * 0.8 * hud);
    // Graduated scale on the left of the lens barrel.
    let rho = length(c);
    let a = atan2(c.y, c.x);
    let ticks = abs(fract(a * 60.0 / TAU) - 0.5) * TAU / 60.0 * rho;
    let on_arc = step(0.62, abs(a) / PI) * step(abs(a) / PI, 0.86);
    col = ak_over(col, w, ak_fill(max(abs(rho - 0.585) - 0.012, ticks - 0.0012)) * 0.5 * on_arc * hud * rings);
    return col;
}
"#;

fn consts(n: u32, twist: f32, round: f32, dark: f32, light: f32, body: f32, housing: f32) -> String {
    format!(
        "const IR_N: f32 = {n}.0;\nconst IR_TWIST: f32 = {twist:?};\nconst IR_ROUND: f32 = {round:?};\n\
         const IR_DARK: f32 = {dark:?};\nconst IR_LIGHT: f32 = {light:?};\nconst IR_BODY: f32 = {body:?};\n\
         const IR_HOUSING: f32 = {housing:?};\n"
    )
}

/// Camera Shutter (Filmora): seven blades close on A between frames 19 and
/// 30 and reopen on B by frame 43, the focus points showing from frame 14.
/// Rows `[frame, r, rot, side, hud, rings, -, -]`.
const SHUTTER: &[[f32; 8]] = &[
    [0.0, 1.25, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    [13.0, 1.25, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    [15.0, 1.25, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0],
    [18.0, 1.06, 0.1, 0.0, 1.0, 1.0, 0.0, 0.0],
    [19.0, 0.93, 0.16, 0.0, 1.0, 1.0, 0.0, 0.0],
    [21.0, 0.85, 0.2, 0.0, 1.0, 1.0, 0.0, 0.0],
    [22.0, 0.78, 0.23, 0.0, 1.0, 1.0, 0.0, 0.0],
    [23.0, 0.61, 0.3, 0.0, 1.0, 1.0, 0.0, 0.0],
    [24.0, 0.44, 0.38, 0.0, 1.0, 1.0, 0.0, 0.0],
    [25.0, 0.44, 0.38, 0.0, 1.0, 1.0, 0.0, 0.0],
    [26.0, 0.27, 0.46, 0.0, 1.0, 1.0, 0.0, 0.0],
    [27.0, 0.145, 0.52, 0.0, 1.0, 1.0, 0.0, 0.0],
    [28.0, 0.07, 0.55, 0.0, 1.0, 1.0, 0.0, 0.0],
    [29.0, 0.022, 0.58, 0.0, 1.0, 1.0, 0.0, 0.0],
    [30.0, 0.0, 0.59, 0.0, 1.0, 1.0, 0.0, 0.0],
    [30.5, 0.0, 0.59, 0.0, 1.0, 1.0, 0.0, 0.0],
    [30.5, 0.0, 0.59, 1.0, 1.0, 1.0, 0.0, 0.0],
    [32.0, 0.018, 0.58, 1.0, 1.0, 1.0, 0.0, 0.0],
    [33.0, 0.07, 0.55, 1.0, 1.0, 1.0, 0.0, 0.0],
    [34.0, 0.145, 0.52, 1.0, 1.0, 1.0, 0.0, 0.0],
    [35.0, 0.27, 0.46, 1.0, 1.0, 1.0, 0.0, 0.0],
    [36.0, 0.44, 0.38, 1.0, 1.0, 1.0, 0.0, 0.0],
    [37.0, 0.61, 0.3, 1.0, 1.0, 1.0, 0.0, 0.0],
    [38.0, 0.61, 0.3, 1.0, 1.0, 1.0, 0.0, 0.0],
    [39.0, 0.78, 0.23, 1.0, 1.0, 1.0, 0.0, 0.0],
    [40.0, 0.85, 0.2, 1.0, 1.0, 1.0, 0.0, 0.0],
    [41.0, 0.93, 0.16, 1.0, 1.0, 1.0, 0.0, 0.0],
    [42.0, 0.93, 0.16, 1.0, 0.8, 1.0, 0.0, 0.0],
    [43.0, 1.06, 0.1, 1.0, 0.4, 1.0, 0.0, 0.0],
    [44.0, 1.25, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
    [60.0, 1.25, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
];

/// Camera Transition 09 (Filmora): the lens closes slowly on the outgoing
/// clip (frames 1-22), holds shut while the blades turn (to frame 33) and
/// opens on the incoming clip by frame 52.
const LENS: &[[f32; 8]] = &[
    [0.0, 1.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.0, 1.12, 0.05, 0.0, 1.0, 1.0, 0.0, 0.0],
    [2.0, 0.95, 0.1, 0.0, 1.0, 1.0, 0.0, 0.0],
    [3.0, 0.95, 0.1, 0.0, 1.0, 1.0, 0.0, 0.0],
    [4.0, 0.8, 0.17, 0.0, 1.0, 1.0, 0.0, 0.0],
    [5.0, 0.655, 0.24, 0.0, 1.0, 1.0, 0.0, 0.0],
    [6.0, 0.51, 0.31, 0.0, 1.0, 1.0, 0.0, 0.0],
    [7.0, 0.41, 0.36, 0.0, 1.0, 1.0, 0.0, 0.0],
    [8.0, 0.315, 0.41, 0.0, 1.0, 1.0, 0.0, 0.0],
    [9.0, 0.315, 0.41, 0.0, 1.0, 1.0, 0.0, 0.0],
    [10.0, 0.245, 0.45, 0.0, 1.0, 1.0, 0.0, 0.0],
    [11.0, 0.215, 0.47, 0.0, 1.0, 1.0, 0.0, 0.0],
    [12.0, 0.17, 0.5, 0.0, 1.0, 1.0, 0.0, 0.0],
    [13.0, 0.145, 0.52, 0.0, 1.0, 1.0, 0.0, 0.0],
    [14.0, 0.1, 0.55, 0.0, 1.0, 1.0, 0.0, 0.0],
    [15.0, 0.1, 0.55, 0.0, 1.0, 1.0, 0.0, 0.0],
    [16.0, 0.075, 0.57, 0.0, 1.0, 1.0, 0.0, 0.0],
    [17.0, 0.066, 0.58, 0.0, 1.0, 1.0, 0.0, 0.0],
    [18.0, 0.048, 0.6, 0.0, 1.0, 1.0, 0.0, 0.0],
    [19.0, 0.028, 0.62, 0.0, 1.0, 1.0, 0.0, 0.0],
    [21.0, 0.022, 0.64, 0.0, 1.0, 1.0, 0.0, 0.0],
    [22.5, 0.0, 0.66, 0.0, 1.0, 1.0, 0.0, 0.0],
    [33.5, 0.0, 0.9, 0.0, 1.0, 1.0, 0.0, 0.0],
    [33.5, 0.0, 0.9, 1.0, 1.0, 1.0, 0.0, 0.0],
    [34.0, 0.022, 0.91, 1.0, 1.0, 1.0, 0.0, 0.0],
    [35.0, 0.095, 0.95, 1.0, 1.0, 1.0, 0.0, 0.0],
    [36.0, 0.195, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0],
    [37.0, 0.315, 1.06, 1.0, 1.0, 1.0, 0.0, 0.0],
    [38.0, 0.44, 1.12, 1.0, 1.0, 1.0, 0.0, 0.0],
    [39.0, 0.44, 1.12, 1.0, 1.0, 1.0, 0.0, 0.0],
    [40.0, 0.53, 1.16, 1.0, 1.0, 1.0, 0.0, 0.0],
    [41.0, 0.6, 1.19, 1.0, 1.0, 1.0, 0.0, 0.0],
    [42.0, 0.65, 1.21, 1.0, 1.0, 1.0, 0.0, 0.0],
    [43.0, 0.7, 1.23, 1.0, 1.0, 1.0, 0.0, 0.0],
    [44.0, 0.83, 1.28, 1.0, 1.0, 1.0, 0.0, 0.0],
    [45.0, 0.83, 1.28, 1.0, 1.0, 1.0, 0.0, 0.0],
    [46.0, 0.88, 1.3, 1.0, 1.0, 1.0, 0.0, 0.0],
    [47.0, 0.905, 1.31, 1.0, 1.0, 1.0, 0.0, 0.0],
    [48.0, 0.955, 1.33, 1.0, 0.9, 1.0, 0.0, 0.0],
    [49.0, 0.98, 1.34, 1.0, 0.8, 1.0, 0.0, 0.0],
    [50.0, 1.03, 1.36, 1.0, 0.7, 1.0, 0.0, 0.0],
    [52.0, 1.3, 1.4, 1.0, 0.3, 1.0, 0.0, 0.0],
    [53.0, 1.3, 1.4, 1.0, 0.0, 1.0, 0.0, 0.0],
    [60.0, 1.3, 1.4, 1.0, 0.0, 1.0, 0.0, 0.0],
];

pub fn items() -> Vec<Item> {
    vec![
        art_item(
            "jb.iris_shutter",
            "Iris Shutter",
            "Camera",
            "A seven-blade camera iris snaps shut over the outgoing clip under viewfinder focus points and opens on the incoming one.",
            art_wgsl(
                &(consts(7, 0.55, 0.7, 0.1, 0.165, 0.07, 0.92) + HUD_SHUTTER),
                &[tab("ir", &at_frames(SHUTTER))],
                IRIS,
            ),
        ),
        art_item(
            "jb.lens_iris",
            "Lens Iris",
            "Camera",
            "A zoom lens closes its iris slowly on the outgoing clip, turns while shut and opens on the incoming one, with focus-ring markings.",
            art_wgsl(&(consts(9, 0.8, 0.6, 0.11, 0.18, 0.12, 0.98) + HUD_LENS), &[tab("ir", &at_frames(LENS))], IRIS),
        ),
    ]
}

//! Luma-mask wipes: "Wipes" (directional edges) and "Shapes" (shapes that
//! grow to reveal B). Each wipe is an analytic grayscale mask `m(uv)` in 0..1
//! (lower values reveal clip B first), normalized over the frame like the
//! prototype's precomputed masks, and revealed with a soft threshold.

use super::{info, shader};
use crate::registry::{FxKind::Transition, ParamSpec, Registry};

/// Shared helpers for the shape masks.
const SHAPES: &str = r#"
fn heart_inside(px: f32, py: f32) -> bool {
    let a = px * px + py * py - 1.0;
    return a * a * a - px * px * py * py * py <= 0.0;
}
// Smallest scale s for which the point lies inside a heart of size s (sqrt-compressed).
fn heart_raw(uv: vec2f) -> f32 {
    let c = to_c(uv);
    let px = c.x / 0.4;
    let py = -(c.y / 0.4) + 0.12;
    var lo = 0.0;
    var hi = 8.0;
    for (var i = 0; i < 18; i = i + 1) {
        let s = 0.5 * (lo + hi);
        if (heart_inside(px / s, py / s)) { hi = s; } else { lo = s; }
    }
    return sqrt(hi);
}
// Distance from the centre to a 5-point star outline (outer radius 1) along theta (0 = up).
fn star_radius(theta0: f32) -> f32 {
    let points = 5.0;
    let inner = 0.45;
    let step = PI / points;
    let theta = theta0 - floor(theta0 / TAU) * TAU;
    let k = floor(theta / step);
    let a0 = k * step;
    let a1 = (k + 1.0) * step;
    let even = (i32(k) % 2) == 0;
    let r0 = select(inner, 1.0, even);
    let r1 = select(1.0, inner, even);
    let x0 = r0 * sin(a0);
    let y0 = r0 * cos(a0);
    let x1 = r1 * sin(a1);
    let y1 = r1 * cos(a1);
    let dx = sin(theta);
    let dy = cos(theta);
    let ex = x1 - x0;
    let ey = y1 - y0;
    let den = dx * ey - dy * ex;
    if (abs(den) < 1e-6) { return r0; }
    return (x0 * ey - y0 * ex) / den;
}
fn star_raw(uv: vec2f) -> f32 {
    var c = to_c(uv);
    c.y = c.y - 0.04;
    let theta = atan2(c.x, -c.y);
    return sqrt(length(c) / star_radius(theta));
}
fn spiral_raw(uv: vec2f) -> f32 {
    let c = to_c(uv);
    let ang = fract(atan2(c.x, -c.y) / TAU + 1.0);
    let r = length(c) / length(vec2f(aspect() * 0.5, 0.5));
    return r * 4.0 + ang;
}
// Normalize by the largest value at the four corners (minimum ~0 at the centre).
fn by_corners_heart(v: f32) -> f32 {
    let m = max(max(heart_raw(vec2f(0.0, 0.0)), heart_raw(vec2f(1.0, 0.0))), max(heart_raw(vec2f(0.0, 1.0)), heart_raw(vec2f(1.0, 1.0))));
    return v / m;
}
fn by_corners_star(v: f32) -> f32 {
    let m = max(max(star_raw(vec2f(0.0, 0.0)), star_raw(vec2f(1.0, 0.0))), max(star_raw(vec2f(0.0, 1.0)), star_raw(vec2f(1.0, 1.0))));
    return v / m;
}
fn by_corners_spiral(v: f32) -> f32 {
    let e = 0.002;
    let m = max(max(spiral_raw(vec2f(e, e)), spiral_raw(vec2f(1.0 - e, e))), max(spiral_raw(vec2f(e, 1.0 - e)), spiral_raw(vec2f(1.0 - e, 1.0 - e))));
    return v / m;
}
"#;

struct Wipe {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    /// WGSL expression of the mask using `uv`, `c` (= to_c(uv)) and `A` (= aspect()).
    mask: &'static str,
    softness: f32,
}

const WIPES: &[Wipe] = &[
    Wipe { id: "jb.wipe_to_left", name: "Wipe Left", category: "Wipes", description: "The new clip enters with a soft edge from right to left. Filmora: Wipe Left.", mask: "1.0 - uv.x", softness: 0.08 },
    Wipe { id: "jb.wipe_to_right", name: "Wipe Right", category: "Wipes", description: "The new clip enters from left to right. Filmora: Wipe Right.", mask: "uv.x", softness: 0.08 },
    Wipe { id: "jb.wipe_up", name: "Wipe Up", category: "Wipes", description: "The new clip rises from the bottom edge. Filmora: Wipe Up.", mask: "1.0 - uv.y", softness: 0.08 },
    Wipe { id: "jb.wipe_down", name: "Wipe Down", category: "Wipes", description: "The new clip comes down from the top edge. Filmora: Wipe Down.", mask: "uv.y", softness: 0.08 },
    Wipe { id: "jb.wipe_diagonal_down", name: "Diagonal Wipe", category: "Wipes", description: "Diagonal edge from the top-left corner. Filmora: Diagonal Wipe.", mask: "(uv.x * A + uv.y) / (A + 1.0)", softness: 0.08 },
    Wipe { id: "jb.wipe_diagonal_up", name: "Diagonal Wipe Up", category: "Wipes", description: "Diagonal edge from the bottom-left corner. Filmora: Diagonal Wipe.", mask: "(uv.x * A + 1.0 - uv.y) / (A + 1.0)", softness: 0.08 },
    Wipe { id: "jb.wipe_soft", name: "Soft Wipe", category: "Wipes", description: "Left-to-right wipe with a very wide feathered edge. Filmora: Soft Wipe.", mask: "uv.x", softness: 0.6 },
    Wipe { id: "jb.wipe_barn_door_h", name: "Barn Door", category: "Wipes", description: "Opens from a vertical centre line towards the sides. Filmora: Barn Door.", mask: "abs(uv.x - 0.5) * 2.0", softness: 0.08 },
    Wipe { id: "jb.wipe_barn_door_v", name: "Barn Door Vertical", category: "Wipes", description: "Opens from a horizontal centre line up and down. Filmora: Barn Door Vertical.", mask: "abs(uv.y - 0.5) * 2.0", softness: 0.08 },
    Wipe { id: "jb.wipe_blinds_h", name: "Blinds", category: "Wipes", description: "Horizontal slats that open at the same time. Filmora: Blinds.", mask: "fract(uv.y * 8.0)", softness: 0.08 },
    Wipe { id: "jb.wipe_blinds_v", name: "Vertical Blinds", category: "Wipes", description: "Vertical slats that open at the same time. Filmora: Vertical Blinds.", mask: "fract(uv.x * 12.0)", softness: 0.08 },
    Wipe { id: "jb.wipe_clock", name: "Clock Wipe", category: "Wipes", description: "Circular sweep like a clock hand, clockwise from 12. Filmora: Clock Wipe.", mask: "fract(atan2(c.x, -c.y) / TAU + 1.0)", softness: 0.03 },
    Wipe { id: "jb.wipe_wavy", name: "Wave Wipe", category: "Wipes", description: "Left-to-right wipe with a wavy edge. Filmora: Wave Wipe.", mask: "(uv.x + 0.045 * sin(uv.y * TAU * 2.0) + 0.045) / 1.09", softness: 0.06 },
    Wipe { id: "jb.wipe_zigzag", name: "Zigzag Wipe", category: "Wipes", description: "Left-to-right wipe with a saw-tooth edge. Filmora: Zigzag Wipe.", mask: "(uv.x + 0.08 * abs(fract(uv.y * 6.0) - 0.5) * 2.0) / 1.08", softness: 0.04 },
    Wipe { id: "jb.wipe_iris_in", name: "Iris / Circle", category: "Shapes", description: "A circle grows from the centre revealing the new clip. Filmora: Iris / Circle.", mask: "length(c) / length(vec2f(A * 0.5, 0.5))", softness: 0.08 },
    Wipe { id: "jb.wipe_iris_out", name: "Iris Out", category: "Shapes", description: "The new clip enters from the edges while the old one closes into a central circle. Filmora: Iris Out.", mask: "1.0 - length(c) / length(vec2f(A * 0.5, 0.5))", softness: 0.08 },
    Wipe { id: "jb.wipe_box", name: "Box In", category: "Shapes", description: "A rectangle with the frame's proportions grows from the centre. Filmora: Box In.", mask: "max(abs(uv.x - 0.5), abs(uv.y - 0.5)) * 2.0", softness: 0.08 },
    Wipe { id: "jb.wipe_diamond", name: "Diamond", category: "Shapes", description: "A diamond grows from the centre. Filmora: Diamond.", mask: "(abs(c.x) + abs(c.y)) / (A * 0.5 + 0.5)", softness: 0.08 },
    Wipe { id: "jb.wipe_checkerboard", name: "Checkerboard", category: "Shapes", description: "Checkerboard squares fill in two alternating passes. Filmora: Checkerboard.", mask: "f32((i32(floor(uv.x * 8.0)) + i32(floor(uv.y * 5.0))) % 2) * 0.5 + fract(uv.x * 8.0) * 0.5", softness: 0.03 },
    Wipe { id: "jb.wipe_diamonds", name: "Diamond Grid", category: "Shapes", description: "A grid of diamonds grows until it covers the frame. Filmora: Diamond Grid.", mask: "(abs(fract(uv.x * 8.0) - 0.5) + abs(fract(uv.y * 8.0 / A) - 0.5) + 0.25 * uv.x) / 1.25", softness: 0.05 },
    Wipe { id: "jb.wipe_heart", name: "Heart", category: "Shapes", description: "A heart grows from the centre revealing the new clip. Filmora: Heart.", mask: "by_corners_heart(heart_raw(uv))", softness: 0.08 },
    Wipe { id: "jb.wipe_star", name: "Star", category: "Shapes", description: "A five-pointed star grows from the centre. Filmora: Star.", mask: "by_corners_star(star_raw(uv))", softness: 0.08 },
    Wipe { id: "jb.wipe_spiral", name: "Spiral", category: "Shapes", description: "A wipe that sweeps in a spiral from the centre outwards. Filmora: Spiral.", mask: "by_corners_spiral(spiral_raw(uv))", softness: 0.03 },
];

pub fn register(r: &mut Registry) {
    for w in WIPES {
        let soft = ParamSpec::float("softness", "Softness", 0.0, 1.0, w.softness);
        let body = format!(
            r#"{SHAPES}
fn wipe_mask(uv: vec2f) -> f32 {{
    let c = to_c(uv);
    let A = aspect();
    return {};
}}
fn fx(uv: vec2f) -> vec4f {{
    var m = clamp(wipe_mask(uv), 0.0, 1.0);
    if (p_reverse() > 0.5) {{ m = 1.0 - m; }}
    let s = max(p_softness(), 1e-4);
    let t = prog() * (1.0 + s);
    let k = 1.0 - smoothstep(t - s, t, m);
    return mix(sa(uv), sb(uv), k);
}}
"#,
            w.mask
        );
        shader(
            r,
            info(
                w.id,
                w.name,
                w.category,
                Transition,
                w.description,
                &[soft, ParamSpec::boolean("reverse", "Reverse", false)],
            ),
            body,
        );
    }
}

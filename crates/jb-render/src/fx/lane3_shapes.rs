//! Geometric transitions fitted frame by frame on Filmora's exports: lens
//! balls, panel strips and other shapes that move the clips themselves.

use super::data as D;
use super::kit::{register_items, table, Item};
use crate::registry::Registry;

/// Lens ball: inside the ellipse `(x/ax)^2 + (y/ay)^2 <= 1` (centred, height
/// units, `ax = R (1 + m (aspect - 1))`, `ay = R`, so `m = 1` has the frame's
/// proportions and `m = 0` is a circle of radius `R`) the current clip is seen through a bulge: at normalised radius `r`
/// the sample point is divided by `k (1 - b + b sqrt(1 - r^2))`; the colour is
/// `g X + o` (display space) and the outside is black (or the plain clip with
/// `clip_outside`). Rows
/// `[p, R, m, k, b, g, o.rgb]`; `cut` is the progress of the switch to B.
fn lens_ball(rows: &[[f32; 9]], cut: f32, clip_outside: bool) -> String {
    let outside = if clip_outside { format!("l3_side(p >= {cut:?}, uv)") } else { "vec4f(0.0, 0.0, 0.0, 1.0)".to_string() };
    format!(
        r#"{}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let k0 = kf_g(p);
    let k1 = kf_g_1(p);
    let A = aspect();
    let m = clamp(k0.y, 0.0, 1.0);
    let ay = max(k0.x, 1e-3);
    let ax = ay * (1.0 + m * (A - 1.0));
    let c = to_c(uv);
    let rr = length(vec2f(c.x / ax, c.y / ay));
    if (rr > 1.0) {{ return {outside}; }}
    let z = sqrt(max(1.0 - rr * rr, 0.0));
    let mm = max(k0.z * (1.0 - k0.w + k0.w * z), 1e-3);
    let q = clamp(from_c(c / mm), vec2f(0.0), vec2f(1.0));
    var x = to_disp(sa(q)).rgb;
    if (p >= {cut}) {{ x = to_disp(sb(q)).rgb; }}
    return l3_out(k1.x * x + k1.yzw);
}}
"#,
        table("g", rows)
    )
}

/// Panel strip (carousel): the clips are panels in a row on black, panel `k`
/// at `k` pitches along `axis` (A for `k <= 0`, B from 1 on), seen through
/// a zoom `s` and rotation `rot` about the centre and scrolled by `off`
/// panels in direction `dir`, with a box motion blur of `blur` panels along
/// the row. Rows `[p, s, off, rot, pitch, blur]`.
fn panel_strip(rows: &[[f32; 6]], vertical: bool, dir: f32) -> String {
    let (a, o) = if vertical { ("y", "x") } else { ("x", "y") };
    format!(
        r#"{}
fn strip_tap(x: f32, y: f32, off: f32, pitch: f32) -> vec4f {{
    let xy = vec2f(x, y);
    let t = (xy.{a} - 0.5) + {dir:?} * off;
    let k = floor((t + 0.5 * pitch) / pitch);
    let loc = t - k * pitch + 0.5;
    if (abs(loc - 0.5) > 0.5 || xy.{o} < 0.0 || xy.{o} > 1.0) {{ return vec4f(0.0, 0.0, 0.0, 1.0); }}
    var q = xy;
    q.{a} = loc;
    if (k * {dir:?} >= 1.0) {{ return sb(q); }}
    return sa(q);
}}
fn fx(uv: vec2f) -> vec4f {{
    let k0 = kf_g(prog());
    let k1 = kf_g_1(prog());
    let s = max(k0.x, 1e-3);
    let c = rot2(to_c(uv), -k0.z) / s;
    let x = c.x / aspect() + 0.5;
    let y = c.y + 0.5;
    let pitch = max(k0.w, 1.0);
    let bl = k1.x;
    if (bl < 1e-4) {{ return strip_tap(x, y, k0.y, pitch); }}
    var acc = vec4f(0.0);
    let j = hash12(uv * u.size) - 0.5;
    for (var i = 0; i < 12; i = i + 1) {{
        acc = acc + strip_tap(x, y, k0.y + ((f32(i) + 0.5 + j) / 12.0 - 0.5) * bl, pitch);
    }}
    return acc / 12.0;
}}
"#,
        table("g", rows)
    )
}

/// Framed panel over the other clip: the outgoing clip scaled by `s`,
/// rotated by `rot` and moved by `(dx, dy)` (uv), with a white border `bw`
/// frame heights wide, over the incoming clip zoomed by `s2`; the whole
/// picture graded `g c + o`. Rows `[p, s, rot, dx, dy, s2, g, or, og, ob]`.
fn framed_panel(rows: &[[f32; 10]], bw: f32) -> String {
    format!(
        r#"{}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let k0 = kf_g(p);
    let k1 = kf_g_1(p);
    let k2 = kf_g_2(p);
    let s = max(k0.x, 0.05);
    let A = aspect();
    let q = rot2(to_c(uv - vec2f(k0.z, k0.w)), -k0.y) / s;
    let b = {bw:?} / s;
    var c = to_disp(sb(zoom_uv(uv, max(k1.x, 0.05), vec2f(0.5)))).rgb;
    if (abs(q.x) <= 0.5 * A + b && abs(q.y) <= 0.5 + b) {{ c = vec3f(1.0); }}
    if (abs(q.x) <= 0.5 * A && abs(q.y) <= 0.5) {{ c = to_disp(sa(from_c(q))).rgb; }}
    return l3_out(k1.y * c + vec3f(k1.z, k1.w, k2.x));
}}
"#,
        table("g", rows)
    )
}

fn items() -> Vec<Item> {
    vec![
        Item::new(
            "jb.carousel_left",
            "Carousel Left",
            "Motion",
            "The frame shrinks onto a black stage and becomes a panel in a row of panels that scrolls left two \
             panels with a motion blur, dipping in scale as each panel passes, and the new clip settles full \
             frame. Fitted frame by frame on Filmora's export of Carousel Left.",
            panel_strip(D::CAROUSEL_LEFT, false, 1.0),
        ),
        Item::new(
            "jb.falling_frame",
            "Falling Frame",
            "Motion",
            "A white flash, then the clip shrinks slightly inside a thin white frame and falls out of the bottom of \
             the picture, uncovering the new clip. Fitted frame by frame on Filmora's export of Falling Frame \
             Transitions (as a flat panel: the slight perspective tilt of the falling frame is not modelled).",
            framed_panel(D::FALLING_FRAME, 0.012),
        ),
        Item::new(
            "jb.carousel_right",
            "Carousel Right",
            "Motion",
            "Mirror of Carousel Left: the frame shrinks onto a black stage and the row of panels scrolls right \
             two panels with a motion blur, dipping in scale as each panel passes. Fitted frame by frame on \
             Filmora's export of Carousel Right.",
            panel_strip(D::CAROUSEL_RIGHT, false, -1.0),
        ),
        Item::new(
        "jb.crystal_ball_iris",
        "Crystal Ball Iris",
        "Shapes",
        "The clip rounds its corners and shrinks into a glass ball on black, seen magnified through the lens; \
         the ball flares to a bright tinted glow, switches to the new clip and grows back out to full frame. \
         Fitted frame by frame on Filmora's export of Crystal Ball 1.",
        lens_ball(D::CRYSTAL_BALL_IRIS, 29.5 / 60.0, false),
    ),
    Item::new(
        "jb.crystal_ball_lens",
        "Crystal Ball Lens",
        "Shapes",
        "A glass lens swells in the middle of the clip, magnifying it, glows into a bright ball at the cut and \
         opens again over the new clip. Fitted frame by frame on Filmora's export of Crystal Ball 2.",
        lens_ball(D::CRYSTAL_BALL_LENS, 29.5 / 60.0, true),
    ),
    ]
}

pub fn register(r: &mut Registry) {
    register_items(r, items());
}

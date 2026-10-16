//! Movement, zoom and spin transitions (Filmora "Motion", "Zoom" and "Spin"
//! packs): push, slide over, whip pan, zoom in/out, cross zoom, spin,
//! squeeze and bounce.
//!
//! Timing follows the agent-editor prototype: every motion is eased with a
//! smoothstep (`ease_io`) unless noted, and fractions refer to the whole
//! transition (0 = first frame, 1 = last).

use super::{info, shader};
use crate::registry::{FxKind::Transition, ParamSpec, Registry};

/// Gaussian blur of `scene(uv)` along a uv-space vector `d` (one sigma).
const DBLUR: &str = r#"
fn dblur(uv: vec2f, d: vec2f) -> vec4f {
    if (length(d * u.size) < 0.3) { return scene(uv); }
    var acc = vec4f(0.0);
    var ws = 0.0;
    for (var i = -8; i <= 8; i = i + 1) {
        let x = f32(i) / 4.0;
        let w = exp(-0.5 * x * x);
        acc = acc + w * scene(uv + d * x);
        ws = ws + w;
    }
    return acc / ws;
}
"#;

/// Rotational blur of `scene(uv)` around the centre; `a` = angular spread (rad).
const RBLUR: &str = r#"
fn rblur(uv: vec2f, a: f32) -> vec4f {
    if (a < 1e-4) { return scene(uv); }
    var acc = vec4f(0.0);
    var ws = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let x = f32(i) / 15.0 - 0.5;
        let w = 1.0 - abs(x);
        acc = acc + w * scene(spin_uv(uv, a * x, 1.0));
        ws = ws + w;
    }
    return acc / ws;
}
"#;

/// Brightness (MLT `brightness`): multiplies display-referred RGB.
const BRIGHT: &str = r#"
fn bright(c: vec4f, k: f32) -> vec4f {
    let d = to_disp(c);
    return from_disp(vec4f(clamp(d.rgb * k, vec3f(0.0), vec3f(1.0)), d.a));
}
"#;

fn push_body(dx: i32, dy: i32) -> String {
    format!(
        r#"
fn fx(uv: vec2f) -> vec4f {{
    let d = vec2f({dx}.0, {dy}.0);
    let k = ease_io(seg(prog(), 0.1, 0.9));
    return sat(uv - d * k) + sbt(uv + d * (1.0 - k));
}}
"#
    )
}

fn slide_body(sx: i32, sy: i32) -> String {
    format!(
        r#"{BRIGHT}
fn fx(uv: vec2f) -> vec4f {{
    let s = vec2f({sx}.0, {sy}.0);
    let k = ease_io(seg(prog(), 0.05, 0.95));
    let a = bright(sa(uv), 1.0 - 0.35 * k);
    return over(sbt(uv - s * (1.0 - k)), a);
}}
"#
    )
}

fn whip_body(dx: i32, dy: i32) -> String {
    format!(
        r#"
fn scene(uv: vec2f) -> vec4f {{
    let d = vec2f({dx}.0, {dy}.0);
    let k = ease_io(seg(prog(), 0.4, 0.6));
    return sat(uv - d * k) + sbt(uv + d * (1.0 - k));
}}
{DBLUR}
fn fx(uv: vec2f) -> vec4f {{
    // Stacked blurs of the prototype (1%, 2.5%, 4% of the frame along the motion
    // within 0.2 / 0.1 / 0.05 of the cut), combined into one smooth envelope.
    let t = abs(prog() - 0.5);
    let sigma = (0.01 * (1.0 - smoothstep(0.15, 0.2, t))
        + 0.017 * (1.0 - smoothstep(0.07, 0.1, t))
        + 0.021 * (1.0 - smoothstep(0.03, 0.05, t))) * p_blur();
    return dblur(uv, vec2f({ax}.0, {ay}.0) * sigma);
}}
"#,
        ax = dx.abs(),
        ay = dy.abs()
    )
}

fn spin_body(degrees: f32, zoom: f32, k: f32) -> String {
    format!(
        r#"
fn scene(uv: vec2f) -> vec4f {{
    let p = prog();
    let z = {zoom:.4};
    let ka = ease_in(seg(p, 0.5 - {k:.4}, 0.5));
    let kb = ease_out(seg(p, 0.5, 0.5 + {k:.4}));
    let a = sat(spin_uv(uv, radians({degrees:.2}) * ka, 1.0 + (z - 1.0) * ka));
    let b = sbt(spin_uv(uv, -radians({degrees:.2}) * (1.0 - kb), z - (z - 1.0) * kb));
    return mix(a, b, seg(p, 0.5 - 0.034, 0.5 + 0.034));
}}
{RBLUR}
fn fx(uv: vec2f) -> vec4f {{
    let t = abs(prog() - 0.5);
    let spread = radians({degrees:.2}) * 0.12 * (1.0 - smoothstep(0.0, {k:.4} * 0.6, t)) * p_blur();
    return rblur(uv, spread);
}}
"#
    )
}

const BLUR_PARAM: ParamSpec = ParamSpec::float("blur", "Motion blur", 0.0, 2.0, 1.0);

pub fn register(r: &mut Registry) {
    // ------------------------------------------------------------ Push
    for (id, name, desc, dx, dy) in [
        ("jb.push_left", "Push Left", "The new clip enters from the right and pushes the old one out to the left. Filmora: Push Left.", -1, 0),
        ("jb.push_right", "Push Right", "The new clip enters from the left and pushes the old one out to the right. Filmora: Push Right.", 1, 0),
        ("jb.push_up", "Push Up", "The new clip rises from the bottom and pushes the old one up. Filmora: Push Up.", 0, -1),
        ("jb.push_down", "Push Down", "The new clip drops from the top and pushes the old one down. Filmora: Push Down.", 0, 1),
    ] {
        shader(r, info(id, name, "Motion", Transition, desc, &[]), push_body(dx, dy));
    }
    // ------------------------------------------------------------ Slide over
    for (id, name, desc, sx, sy) in [
        ("jb.slide_from_left", "Slide Right", "The new clip slides in from the left over the old one, which darkens. Filmora: Slide Right.", -1, 0),
        ("jb.slide_from_top", "Slide Down", "The new clip slides down from the top and covers the old one. Filmora: Slide Down.", 0, -1),
        ("jb.slide_from_bottom", "Slide Up", "The new clip slides up from the bottom and covers the old one. Filmora: Slide Up.", 0, 1),
    ] {
        shader(r, info(id, name, "Motion", Transition, desc, &[]), slide_body(sx, sy));
    }
    // ------------------------------------------------------------ Whip pan
    for (id, name, desc, dx, dy) in [
        ("jb.whip_right", "Whip Pan Right", "Fast camera pan to the right with motion blur. Filmora: Whip Pan / Fast Wipe Right.", 1, 0),
        ("jb.whip_up", "Whip Pan Up", "Fast vertical camera pan upwards with motion blur. Filmora: Whip Pan Up / Fast Wipe Up.", 0, -1),
        ("jb.whip_down", "Whip Pan Down", "Fast vertical camera pan downwards with motion blur. Filmora: Whip Pan Down / Fast Wipe Down.", 0, 1),
    ] {
        shader(r, info(id, name, "Motion", Transition, desc, &[BLUR_PARAM]), whip_body(dx, dy));
    }
    // ------------------------------------------------------------ Zoom
    shader(
        r,
        info(
            "jb.zoom_in",
            "Zoom In",
            "Zoom",
            Transition,
            "The old clip zooms in while the new one grows from the centre to fill the frame. Filmora: Zoom In.",
            &[],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let a = sat(zoom_uv(uv, 1.0 + 0.8 * ease_in(p), vec2f(0.5)));
    let k = ease_io(seg(p, 0.05, 0.9));
    var b = vec4f(0.0);
    if (k > 1e-3) { b = sbt(zoom_uv(uv, k, vec2f(0.5))) * min(1.0, 3.0 * k); }
    return over(b, a);
}
"#,
    );
    shader(
        r,
        info(
            "jb.zoom_out",
            "Zoom Out",
            "Zoom",
            Transition,
            "The old clip shrinks away while the new one fades in zoomed and pulls back to full frame. Filmora: Zoom Out.",
            &[],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let a = sat(zoom_uv(uv, 1.0 - 0.75 * ease_in(seg(p, 0.0, 0.6)), vec2f(0.5)));
    let b = sbt(zoom_uv(uv, 2.2 - 1.2 * ease_out(seg(p, 0.25, 1.0)), vec2f(0.5))) * seg(p, 0.25, 0.6);
    return over(b, a);
}
"#,
    );
    shader(
        r,
        info(
            "jb.cross_zoom",
            "Cross Zoom",
            "Zoom",
            Transition,
            "Radial zoom blur that builds on the outgoing clip and slowly clears on the incoming one. \
             Timing matched to Filmora: Cross Zoom.",
            &[BLUR_PARAM],
        ),
        format!(
            "{}{}",
            super::transitions::ZOOM_BLUR,
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = p_blur();
    // Outgoing: blur ramps in early, zoom accelerates towards the cut.
    let ua = seg(p, 0.0, 0.5);
    let a = zoom_blur(false, uv, 1.0 + 0.35 * ua * ua, k * (0.3 * smoothstep(0.0, 0.25, p) + 0.25 * ua * ua * ua));
    // Incoming: arrives zoomed and heavily blurred, clears slowly.
    let s = seg(p, 0.47, 1.0);
    let r = 1.0 - s;
    let b = zoom_blur(true, uv, 1.0 + 0.4 * r * r * r, k * 0.5 * r * r);
    return mix(a, b, smoothstep(0.45, 0.5, p));
}
"#
        ),
    );
    // ------------------------------------------------------------ Spin
    shader(
        r,
        info(
            "jb.spin",
            "Spin",
            "Spin",
            Transition,
            "The image spins half a turn and the new clip appears finishing the spin. Filmora: Spin.",
            &[BLUR_PARAM],
        ),
        spin_body(180.0, 1.0, 1.0 / 3.0),
    );
    shader(
        r,
        info(
            "jb.spin_zoom",
            "Spin Zoom",
            "Spin",
            Transition,
            "Fast spin zooming in with blur; the new clip spins out while zooming back. Filmora: Spin Zoom.",
            &[BLUR_PARAM],
        ),
        spin_body(270.0, 2.6, 0.25),
    );
    // ------------------------------------------------------------ Squeeze
    shader(
        r,
        info(
            "jb.squeeze_h",
            "Squeeze Horizontal",
            "Motion",
            Transition,
            "The old clip is squeezed to the left while the new one stretches in from the right. Filmora: Squeeze Horizontal.",
            &[],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let k = ease_io(seg(prog(), 0.1, 0.9));
    let edge = 1.0 - k;
    if (uv.x < edge) { return sa(vec2f(uv.x / max(edge, 1e-4), uv.y)); }
    return sb(vec2f((uv.x - edge) / max(k, 1e-4), uv.y));
}
"#,
    );
    shader(
        r,
        info(
            "jb.squeeze_v",
            "Squeeze Vertical",
            "Motion",
            Transition,
            "The old clip is squeezed upwards while the new one stretches in from the bottom. Filmora: Squeeze Vertical.",
            &[],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let k = ease_io(seg(prog(), 0.1, 0.9));
    let edge = 1.0 - k;
    if (uv.y < edge) { return sa(vec2f(uv.x, uv.y / max(edge, 1e-4))); }
    return sb(vec2f(uv.x, (uv.y - edge) / max(k, 1e-4)));
}
"#,
    );
    // ------------------------------------------------------------ Bounce
    shader(
        r,
        info(
            "jb.bounce_in",
            "Bounce",
            "Motion",
            Transition,
            "The new clip drops from the top and bounces before settling. Filmora: Bounce.",
            &[],
        ),
        format!(
            r#"{BRIGHT}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let a = bright(sa(uv), 1.0 - 0.4 * ease_io(seg(p, 0.05, 0.5)));
    let y = -(1.0 - bounce_out(seg(p, 0.05, 0.95)));
    return over(sbt(uv - vec2f(0.0, y)), a);
}}
"#
        ),
    );
}

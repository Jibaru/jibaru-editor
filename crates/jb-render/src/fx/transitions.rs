//! Basic transitions, dissolves and light transitions (Filmora "Basic" and
//! "Light" packs). Timings follow the agent-editor prototype: curves are
//! piecewise-linear keyframes over the transition's 0..1 progress.

use super::{info, shader, CustomFx};
use crate::fx::FxCx;
use crate::registry::{FxIo, FxKind::Transition, ParamSpec, Registry};

/// WGSL `pwl(...)` call over `pts` (at most 8 points) evaluated at `x`.
pub(crate) fn curve(x: &str, pts: &[(f32, f32)]) -> String {
    assert!(!pts.is_empty() && pts.len() <= 8);
    let mut items: Vec<String> = pts.iter().map(|(a, b)| format!("vec2f({a:.4}, {b:.4})")).collect();
    while items.len() < 8 {
        items.push("vec2f(0.0)".into());
    }
    format!("pwl({x}, array<vec2f, 8>({}), {})", items.join(", "), pts.len())
}

/// Display-space color helpers shared by the bodies below.
const HELPERS: &str = r#"
// Lift towards white per channel (lift_gamma_gain "lift"), display space.
fn lift(c: vec4f, w: vec3f) -> vec4f {
    let d = to_disp(c);
    return from_disp(vec4f(mix(d.rgb, vec3f(1.0), clamp(w, vec3f(0.0), vec3f(1.0))), d.a));
}
// Brightness multiply (8-bit clipped), display space.
fn bright(c: vec4f, k: f32) -> vec4f {
    let d = to_disp(c);
    return from_disp(vec4f(min(d.rgb * k, vec3f(1.0)), d.a));
}
// Mix towards an opaque display-space color.
fn toward(c: vec4f, col: vec3f, k: f32) -> vec4f {
    let d = to_disp(c);
    return from_disp(vec4f(mix(d.rgb, col, k), mix(d.a, 1.0, k)));
}
// Directional gaussian blur of a two-image composite (A at uv-oa, B at uv-ob),
// sigma in uv along `dir`.
fn push_sample(uv: vec2f, oa: vec2f, ob: vec2f) -> vec4f {
    return sat(uv - oa) + sbt(uv - ob);
}
fn push_blur(uv: vec2f, oa: vec2f, ob: vec2f, dir: vec2f, sigma: f32) -> vec4f {
    if (sigma <= 1e-5) { return push_sample(uv, oa, ob); }
    var acc = vec4f(0.0);
    var ws = 0.0;
    for (var i = -12; i <= 12; i = i + 1) {
        let x = f32(i) / 6.0; // -2..2 sigma
        let w = exp(-0.5 * x * x);
        acc = acc + w * push_sample(uv + dir * (x * sigma), oa, ob);
        ws = ws + w;
    }
    return acc / ws;
}
"#;

fn body(code: &str) -> String {
    format!("{HELPERS}\n{code}")
}

/// Dip through an opaque display-space color expression `col`.
fn dip_body(col: &str) -> String {
    body(&format!(
        r#"
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let col = {col};
    if (p < 0.5) {{ return toward(sa(uv), col, p * 2.0); }}
    return toward(sb(uv), col, 2.0 - p * 2.0);
}}
"#
    ))
}

/// Lift-to-color flash with a near-instant cut at the middle.
fn lift_cut_body(a: &[(f32, f32)], b: &[(f32, f32)], rgb: [f32; 3]) -> String {
    body(&format!(
        r#"
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let w = vec3f({}, {}, {});
    if (p < 0.5) {{ return lift(sa(uv), w * {}); }}
    return lift(sb(uv), w * {});
}}
"#,
        rgb[0],
        rgb[1],
        rgb[2],
        curve("p", a),
        curve("p", b)
    ))
}

fn blur_dissolve(fx: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let b = io.b.unwrap_or(io.a);
    let p = io.progress;
    // Prototype: boxblur radius 0.022 * width ~ gaussian sigma 0.6 * radius.
    let r = 0.6 * 0.022 * io.a.width as f32 * io.params.get("amount");
    let ka = (p / 0.5).clamp(0.0, 1.0);
    let kb = ((1.0 - p) / 0.5).clamp(0.0, 1.0);
    let ba = cx.blur(io.a, r * ka, r * ka);
    let bb = cx.blur(b, r * kb, r * kb);
    cx.run(&fx.shaders[0], &ba, Some(&bb), io.out, CustomFx::uniforms(io));
}

pub fn register(r: &mut Registry) {
    shader(
        r,
        info("jb.dissolve", "Dissolve", "Basic", Transition, "Classic cross-fade.", &[]),
        "fn fx(uv: vec2f) -> vec4f { return mix(sa(uv), sb(uv), prog()); }",
    );
    shader(
        r,
        info(
            "jb.fade_black",
            "Fade to Black",
            "Basic",
            Transition,
            "Fades out to black and back in on the next clip. Filmora: Fade / Dip to Black.",
            &[],
        ),
        dip_body("vec3f(0.0)"),
    );
    shader(
        r,
        info(
            "jb.fade_color",
            "Dip to Color",
            "Basic",
            Transition,
            "Fades out to a solid color and back in on the next clip. Filmora: Dip to Color.",
            &[ParamSpec::color("color", "Color", [0.0, 0.0, 0.0, 1.0])],
        ),
        dip_body("lin_to_srgb(p_color().rgb)"),
    );
    shader(
        r,
        info(
            "jb.dip_white",
            "Dip to White",
            "Basic",
            Transition,
            "The clip fades to white and the next one appears out of the white. Filmora: Dip to White.",
            &[],
        ),
        lift_cut_body(&[(0.0, 0.0), (0.25, 0.65), (0.5, 1.0)], &[(0.0, 1.0), (0.5, 1.0), (0.75, 0.65), (1.0, 0.0)], [1.0; 3]),
    );
    shader(
        r,
        info(
            "jb.dip_gray",
            "Dip to Gray",
            "Basic",
            Transition,
            "Dips to a soft dark gray and back, gentler than black. Filmora: Dip to Color.",
            &[],
        ),
        dip_body("vec3f(0.16)"),
    );
    shader(
        r,
        info(
            "jb.fast_zoom",
            "Fast Zoom",
            "Zoom",
            Transition,
            "The clip zooms in quickly right before the cut. Filmora: Fast Zoom.",
            &[ParamSpec::float("zoom", "Zoom", 1.0, 2.0, 1.15)],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let z = p_zoom();
    let k = 1.0 / 15.0;
    let sa_ = 1.0 + (z - 1.0) * seg(p, 0.5 - 2.0 * k, 0.5);
    let sb_ = z + (1.0 - z) * seg(p, 0.5 + k, 0.5 + 4.0 * k);
    let ob = seg(p, 0.5 - k, 0.5 + k);
    let a = sa(zoom_uv(uv, sa_, vec2f(0.5)));
    let b = sb(zoom_uv(uv, sb_, vec2f(0.5)));
    return over(b * ob, a);
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.wipe_left",
            "Fast Wipe Left",
            "Motion",
            Transition,
            "Quick push to the left with motion blur. Filmora: Fast Wipe Left.",
            &[ParamSpec::float("blur", "Blur", 0.0, 2.0, 1.0)],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = 1.0 / 12.0;
    let t = seg(p, 0.5 - k, 0.5 + k);
    let oa = vec2f(-t, 0.0);
    let ob = vec2f(1.0 - t, 0.0);
    // gblur sigma 40 px at 1920 wide, active around the cut.
    let on = select(0.0, 1.0, p >= 0.5 - 2.0 * k && p <= 0.5 + 2.0 * k);
    return push_blur(uv, oa, ob, vec2f(1.0, 0.0), on * p_blur() * 40.0 / 1920.0);
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.slide",
            "Slide",
            "Motion",
            Transition,
            "The new clip slides in from the right over the previous one. Filmora: Slide.",
            &[],
        ),
        "fn fx(uv: vec2f) -> vec4f { return over(sbt(uv - vec2f(1.0 - prog(), 0.0)), sa(uv)); }",
    );
    r.register(CustomFx::new(
        info(
            "jb.blur_dissolve",
            "Blur Dissolve",
            "Basic",
            Transition,
            "Cross-fade while both clips blur and refocus. Filmora: Blur Dissolve.",
            &[ParamSpec::float("amount", "Blur amount", 0.0, 3.0, 1.0)],
        ),
        &["@fragment fn fs_main(in: VsOut) -> @location(0) vec4f { return mix(sa(in.uv), sb(in.uv), seg(prog(), 0.25, 0.75)); }"],
        blur_dissolve,
    ));
    shader(
        r,
        info(
            "jb.smooth_dissolve",
            "Smooth Dissolve",
            "Basic",
            Transition,
            "Slow cross-fade with eased start and end. Filmora: Dissolve (ease).",
            &[],
        ),
        "fn fx(uv: vec2f) -> vec4f { return mix(sa(uv), sb(uv), ease_io(prog())); }",
    );
    shader(
        r,
        info(
            "jb.zoom_dissolve",
            "Zoom Dissolve",
            "Basic",
            Transition,
            "Cross-fade with a gentle zoom on both clips. Filmora: Zoom Dissolve.",
            &[ParamSpec::float("zoom", "Zoom", 1.0, 1.5, 1.1)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let t = prog();
    let z = p_zoom();
    let a = sa(zoom_uv(uv, 1.0 + (z - 1.0) * t, vec2f(0.5)));
    let b = sbt(zoom_uv(uv, z + (1.0 - z) * t, vec2f(0.5)));
    return over(b * ease_io(min(1.0, t / 0.8)), a);
}
"#,
    );
    shader(
        r,
        info(
            "jb.luma_dissolve",
            "Luma Dissolve",
            "Basic",
            Transition,
            "The bright areas of the new clip appear first, then the fade completes. Filmora: Luma Dissolve.",
            &[],
        ),
        body(&format!(
            r#"
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let a = to_disp(bright(sa(uv), {}));
    let b = to_disp(bright(sb(uv), {}));
    return from_disp(vec4f(max(a.rgb, b.rgb), max(a.a, b.a)));
}}
"#,
            curve("p", &[(0.0, 1.0), (0.35, 1.0), (0.85, 0.0)]),
            curve("p", &[(0.0, 0.0), (0.15, 0.3), (0.55, 1.0)])
        )),
    );
    shader(
        r,
        info(
            "jb.white_flash",
            "Flash",
            "Light",
            Transition,
            "A quick white flash right at the cut. Filmora: Flash.",
            &[],
        ),
        lift_cut_body(&[(0.0, 0.0), (0.35, 0.0), (0.5, 1.0)], &[(0.0, 1.0), (0.5, 1.0), (0.7, 0.0)], [1.0; 3]),
    );
    shader(
        r,
        info(
            "jb.additive_dissolve",
            "Additive Dissolve",
            "Light",
            Transition,
            "Both clips are added together so the image brightens at the crossing. Filmora: Additive Dissolve.",
            &[],
        ),
        body(&format!(
            r#"
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let a = to_disp(bright(sa(uv), {}));
    let b = to_disp(bright(sb(uv), {}));
    return from_disp(vec4f(min(a.rgb + b.rgb, vec3f(1.0)), max(a.a, b.a)));
}}
"#,
            curve("p", &[(0.0, 1.0), (0.5, 1.0), (1.0, 0.0)]),
            curve("p", &[(0.0, 0.0), (0.5, 1.0)])
        )),
    );
    shader(
        r,
        info(
            "jb.flicker",
            "Flash Flicker",
            "Light",
            Transition,
            "Two quick white flashes before switching clips. Filmora: Flash Flicker.",
            &[],
        ),
        lift_cut_body(
            &[(0.0, 0.0), (0.3, 0.0), (0.34, 1.0), (0.38, 0.0), (0.44, 0.0), (0.5, 1.0)],
            &[(0.0, 1.0), (0.5, 1.0), (0.56, 0.0), (0.6, 0.0), (0.64, 0.8), (0.72, 0.0)],
            [1.0; 3],
        ),
    );
    shader(
        r,
        info(
            "jb.warm_flash",
            "Light Leak Flash",
            "Light",
            Transition,
            "A warm orange flash burns the image at the cut, like a light leak. Filmora: Light Leak Flash.",
            &[],
        ),
        lift_cut_body(&[(0.0, 0.0), (0.25, 0.0), (0.5, 1.0)], &[(0.0, 1.0), (0.5, 1.0), (0.8, 0.0)], [1.0, 0.62, 0.25]),
    );
    shader(
        r,
        info(
            "jb.overexposure",
            "Burn / Exposure",
            "Light",
            Transition,
            "The image overexposes until it burns out and reveals the next clip. Filmora: Burn / Exposure.",
            &[],
        ),
        body(&format!(
            r#"
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let a = bright(sa(uv), {});
    let b = bright(sb(uv), {});
    return mix(a, b, {});
}}
"#,
            curve("p", &[(0.0, 1.0), (0.15, 1.0), (0.5, 6.0)]),
            curve("p", &[(0.0, 6.0), (0.5, 6.0), (0.9, 1.0), (1.0, 1.0)]),
            curve("p", &[(0.0, 0.0), (0.42, 0.0), (0.58, 1.0), (1.0, 1.0)])
        )),
    );
}

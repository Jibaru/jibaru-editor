//! Blur, distortion, artistic, mirror and motion effects (Filmora "Blur",
//! "Distortion", "Mirror" and "Motion" packs).
//!
//! Sizes from the catalog spec are "1080p pixels": in WGSL they are converted
//! with `px1080()`; blur sigmas handed to `cx.blur` are scaled by the layer
//! height / 1080, so previews and 4K renders look alike.

use super::{info, shader, CustomFx, FxCx};
use crate::registry::{FxIo, FxKind::Effect, ParamSpec as P, Registry};

const INTENSITY: P = P::float("intensity", "Intensity", 0.0, 100.0, 50.0);

/// Pixels at 1080p -> pixels of the layer texture.
fn px(io: &FxIo<'_>, v: f32) -> f32 {
    v * io.a.height as f32 / 1080.0
}

/// Blurs `io.a` by `sigma_1080` and, when the effect has a mix shader, blends
/// original (A) and blurred (B) with it; otherwise copies the blur out.
fn blur_then_mix(me: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>, sigma_1080: f32) {
    let s = px(io, sigma_1080);
    let blurred = cx.blur(io.a, s, s);
    match me.shaders.first() {
        Some(sh) => cx.run(sh, io.a, Some(&blurred), io.out, CustomFx::uniforms(io)),
        None => cx.copy(&blurred, io.out),
    }
}

/// Mix pass: B (blurred) where `mask(uv)` is 1, A elsewhere.
const MASK_MIX: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    return post(mix(sa(in.uv), sb(in.uv), clamp(mask(in.uv), 0.0, 1.0)));
}
"#;

fn smooth_wgsl() -> &'static str {
    "fn smooth_e(e0: f32, e1: f32, x: f32) -> f32 { let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }\n"
}

pub fn register(r: &mut Registry) {
    register_blur(r);
    register_distortion(r);
    register_artistic(r);
    register_mirror(r);
    register_motion(r);
}

fn register_blur(r: &mut Registry) {
    r.register(CustomFx::new(
        info(
            "jb.gaussian_blur",
            "Gaussian Blur",
            "Blur",
            Effect,
            "Blurs the whole clip evenly. Filmora equivalent: Gaussian Blur.",
            &[P::float("intensity", "Intensity", 0.0, 100.0, 40.0)],
        ),
        &[],
        |me, cx, io| blur_then_mix(me, cx, io, io.params.get("intensity") * 0.3),
    ));

    r.register(CustomFx::new(
        info(
            "jb.blur.box",
            "Box Blur",
            "Blur",
            Effect,
            "Uniform blur with a given radius (smooth approximation). Filmora equivalent: Basic Blur.",
            &[P::float("radius", "Radius", 0.0, 100.0, 10.0).unit("px")],
        ),
        &[],
        // A box of radius r has the variance of a gaussian with sigma = r / sqrt(3).
        |me, cx, io| blur_then_mix(me, cx, io, io.params.get("radius") / 3f32.sqrt()),
    ));

    let bg = format!(
        "{}fn post(c: vec4f) -> vec4f {{ return c; }}\n\
         fn mask(uv: vec2f) -> f32 {{ let r = round(p_size()) / 100.0; let d = length((uv - 0.5) / 0.5); return smooth_e(r, r + 0.5, d); }}\n{MASK_MIX}",
        smooth_wgsl()
    );
    r.register(CustomFx::new(
        info(
            "jb.background_blur",
            "Background Blur",
            "Blur",
            Effect,
            "Bokeh: the centre stays sharp while the rest is blurred. Filmora equivalent: Background Blur / Bokeh.",
            &[INTENSITY, P::float("size", "Focus size", 10.0, 90.0, 45.0).unit("%")],
        ),
        &[&bg],
        |me, cx, io| blur_then_mix(me, cx, io, 2.0 + io.params.get("intensity") * 0.25),
    ));

    let tilt = format!(
        "{}fn mask(uv: vec2f) -> f32 {{ let band = round(p_band()) / 100.0; let pos = round(p_position()) / 100.0; return smooth_e(band / 2.0, band / 2.0 + 0.18, abs(uv.y - pos)); }}\n\
         fn post(c: vec4f) -> vec4f {{\n\
             let d = to_disp(c);\n\
             var rgb = saturate3(d.rgb, 1.25);\n\
             rgb = (rgb - 0.5) * 1.05 + 0.5;\n\
             return from_disp(vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), d.a));\n\
         }}\n{MASK_MIX}",
        smooth_wgsl()
    );
    r.register(CustomFx::new(
        info(
            "jb.tilt_shift",
            "Tilt Shift",
            "Blur",
            Effect,
            "A sharp horizontal band with blurred top and bottom, miniature look. Filmora equivalent: Tilt Shift.",
            &[
                INTENSITY,
                P::float("band", "Band width", 5.0, 80.0, 30.0).unit("%"),
                P::float("position", "Position", 0.0, 100.0, 50.0),
            ],
        ),
        &[&tilt],
        |me, cx, io| blur_then_mix(me, cx, io, 3.0 + io.params.get("intensity") * 0.35),
    ));

    shader(
        r,
        info(
            "jb.motion_blur",
            "Motion Blur",
            "Blur",
            Effect,
            "Directional streaks as if the camera moved. Filmora equivalent: Motion Blur / Directional Blur.",
            &[INTENSITY, P::float("angle", "Angle", 0.0, 180.0, 0.0).unit("°")],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let radius = p_intensity() * 0.6;
    if (radius <= 0.0) { return sa(uv); }
    let a = radians(p_angle());
    let dir = vec2f(cos(a), sin(a)) * px1080() * radius;
    var acc = vec4f(0.0);
    var wsum = 0.0;
    for (var i = 0; i < 32; i = i + 1) {
        let t = (f32(i) + 0.5) / 32.0 * 2.0 - 1.0;
        let w = 1.0 - 0.5 * abs(t);
        acc = acc + w * sa(uv + dir * t);
        wsum = wsum + w;
    }
    return acc / wsum;
}
"#,
    );

    shader(
        r,
        info(
            "jb.radial_blur",
            "Radial Blur",
            "Blur",
            Effect,
            "Zoom streaks from the centre towards the edges. Filmora equivalent: Zoom Blur / Radial Blur.",
            &[INTENSITY, P::float("size", "Sharp centre", 0.0, 80.0, 25.0).unit("%")],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let spread = 0.12 * p_intensity() / 100.0;
    let orig = sa(uv);
    if (spread <= 0.0) { return orig; }
    let r = round(p_size()) / 100.0;
    let d = length((uv - 0.5) / 0.5);
    let t = clamp((d - r * 0.6) / 0.6, 0.0, 1.0);
    let m = t * t * (3.0 - 2.0 * t);
    var acc = vec4f(0.0);
    for (var i = 0; i < 24; i = i + 1) {
        let z = 1.0 + spread * f32(i) / 23.0;
        acc = acc + sa(zoom_uv(uv, z, vec2f(0.5)));
    }
    return mix(orig, acc / 24.0, m);
}
"#,
    );
}

fn register_distortion(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.pixel_mosaic",
            "Mosaic",
            "Distortion",
            Effect,
            "Pixelates the clip into square blocks. Filmora equivalent: Mosaic / Pixelate.",
            &[P::float("size", "Size", 2.0, 100.0, 30.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let block = px1080() * max(1.0, p_size() * 1.2);
    let cell = (floor(uv / block) + 0.5) * block;
    return sa(cell);
}
"#,
    );

    shader(
        r,
        info(
            "jb.fisheye",
            "Fisheye",
            "Distortion",
            Effect,
            "Bulging wide-angle lens distortion. Filmora equivalent: Fisheye.",
            &[P::float("intensity", "Intensity", 0.0, 100.0, 60.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let k = p_intensity() / 100.0;
    let k1 = 0.55 * k;
    let k2 = 0.1 * k;
    // Zoom in to crop the corners the barrel distortion leaves empty.
    let z = 1.0 + 0.7 * k;
    let p = to_c(uv) / z;
    let hd2 = 0.25 * aspect() * aspect() + 0.25;
    let r2 = dot(p, p) / hd2;
    let src = p * (1.0 + k1 * r2 + k2 * r2 * r2);
    return sat(from_c(src));
}
"#,
    );

    shader(
        r,
        info(
            "jb.water_ripple",
            "Ripple",
            "Distortion",
            Effect,
            "Animated wavy distortion like a reflection on water. Filmora equivalent: Ripple / Wave.",
            &[
                P::float("amplitude", "Amplitude", 1.0, 100.0, 20.0),
                P::float("speed", "Speed", 0.0, 100.0, 40.0),
            ],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let amp = max(1.0, p_amplitude() * 0.25);
    let ph = u.time * p_speed() / 10.0 * 3.0;
    let freq = TAU * 9.0;
    let d = vec2f(sin(uv.y * freq + ph), sin(uv.x * freq * aspect() * 0.75 + ph * 1.3)) * px1080() * amp;
    // Zoom slightly to hide the scalloped borders.
    let z = 1.0 + 4.0 * amp / 1920.0 + 0.01;
    return sa(zoom_uv(uv, z, vec2f(0.5)) + d);
}
"#,
    );

    shader(
        r,
        info(
            "jb.chromatic_aberration",
            "Chromatic Aberration",
            "Distortion",
            Effect,
            "Splits the red and blue channels apart. Filmora equivalent: Chromatic Aberration.",
            &[P::float("intensity", "Intensity", 0.0, 100.0, 40.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let s = round(p_intensity() * 0.4);
    let o = vec2f(s, round(s / 3.0)) * px1080();
    let c = sa(uv);
    let rr = sa(uv - o);
    let bb = sa(uv + o);
    return vec4f(rr.r, c.g, bb.b, max(c.a, max(rr.a, bb.a)));
}
"#,
    );

    shader(
        r,
        info(
            "jb.glitch",
            "Glitch",
            "Distortion",
            Effect,
            "Digital glitches: shifted RGB channels, noise and horizontal jumps. Filmora equivalent: Glitch / Digital Distortion.",
            &[INTENSITY, P::float("frequency", "Frequency", 0.0, 100.0, 50.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let k = p_intensity() / 100.0;
    let frame = floor(u.time * 30.0);
    // Bursts: one per period (frames), starting at a random offset, 2-5 frames long.
    let period = max(3.0, round(30.0 * (1.6 - 1.4 * p_frequency() / 100.0)));
    let sgi = floor(frame / period);
    let start = floor(hash11(sgi * 1.7 + 3.1) * period * 0.5);
    let len = 2.0 + floor(hash11(sgi * 2.3 + 7.7) * 4.0);
    let f = frame - sgi * period;
    let burst = f >= start && f < start + len;
    var o = uv;
    var shift = vec2f(4.0 + 10.0 * k, 0.0);
    var gshift = vec2f(0.0);
    if (burst) {
        let r = hash11(sgi * 5.3 + frame * 0.37);
        let s = (20.0 + 60.0 * k) * (0.6 + 0.6 * r);
        shift = vec2f(s, s / 4.0);
        gshift = vec2f(-s / 2.0, 0.0);
        let side = select(-1.0, 1.0, hash11(sgi * 9.1) > 0.5);
        o.x = o.x - side * (0.02 + 0.06 * k);
        // A displaced horizontal slice.
        let band = hash11(frame * 1.31 + 0.5);
        if (abs(uv.y - band) < 0.04 + 0.05 * k) {
            o.x = o.x + (hash11(frame * 4.7) - 0.5) * 0.2 * k;
        }
    }
    let ps = px1080();
    let rr = sat(o - vec2f(shift.x, 0.0) * ps);
    let gg = sat(o + gshift * ps);
    let bb = sat(o + vec2f(shift.x, -shift.y) * ps);
    let base = sat(o);
    var c = vec4f(rr.r, select(base.g, gg.g, burst), bb.b, max(base.a, max(rr.a, bb.a)));
    let n = (hash12(uv * u.size + vec2f(frame * 17.0, frame * 31.0)) - 0.5) * (8.0 + 20.0 * k) / 100.0;
    c = vec4f(max(c.rgb + n * c.a, vec3f(0.0)), c.a);
    return c;
}
"#,
    );
}

fn register_artistic(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.sketch",
            "Sketch",
            "Artistic",
            Effect,
            "Turns the image into a pencil drawing with dark outlines on white. Filmora equivalent: Sketch / Edge.",
            &[P::float("intensity", "Intensity", 0.0, 100.0, 60.0)],
        ),
        r#"
fn lum_at(uv: vec2f) -> f32 { return luma(to_disp(sa(uv)).rgb); }
fn edge_at(uv: vec2f, o: vec2f) -> f32 {
    let tl = lum_at(uv + vec2f(-o.x, -o.y));
    let t = lum_at(uv + vec2f(0.0, -o.y));
    let tr = lum_at(uv + vec2f(o.x, -o.y));
    let l = lum_at(uv + vec2f(-o.x, 0.0));
    let r = lum_at(uv + vec2f(o.x, 0.0));
    let bl = lum_at(uv + vec2f(-o.x, o.y));
    let b = lum_at(uv + vec2f(0.0, o.y));
    let br = lum_at(uv + vec2f(o.x, o.y));
    let gx = (tr + 2.0 * r + br) - (tl + 2.0 * l + bl);
    let gy = (bl + 2.0 * b + br) - (tl + 2.0 * t + tr);
    return length(vec2f(gx, gy)) * 0.25;
}
fn fx(uv: vec2f) -> vec4f {
    let k = p_intensity() / 100.0;
    let lo = 0.01 + 0.06 * (1.0 - k);
    let hi = 0.04 + 0.16 * (1.0 - k);
    let o = px1080() * 1.5;
    // Max of edges over a small neighbourhood thickens the pencil strokes.
    var e = edge_at(uv, o);
    e = max(e, edge_at(uv + vec2f(o.x, 0.0), o));
    e = max(e, edge_at(uv + vec2f(0.0, o.y), o));
    let v = 1.0 - smoothstep(lo, hi, e);
    let a = sa(uv).a;
    return from_disp(vec4f(vec3f(v), a));
}
"#,
    );

    shader(
        r,
        info(
            "jb.charcoal",
            "Charcoal",
            "Artistic",
            Effect,
            "Charcoal strokes on paper. Filmora equivalent: Charcoal.",
            &[P::float("intensity", "Intensity", 0.0, 100.0, 50.0)],
        ),
        r#"
fn lum_at(uv: vec2f) -> f32 { return luma(to_disp(sa(uv)).rgb); }
fn fx(uv: vec2f) -> vec4f {
    let k = p_intensity() / 100.0;
    let s = max(1.0, 1.0 + 2.0 * k);
    let scale = 1.0 + p_intensity() / 50.0;
    // Scattered sampling positions give the grainy charcoal texture.
    let j = (hash22(floor(uv * u.size)) - 0.5) * 2.0;
    let o = px1080() * s;
    let c = uv + j * o;
    let gx = lum_at(c + vec2f(o.x, 0.0)) - lum_at(c - vec2f(o.x, 0.0));
    let gy = lum_at(c + vec2f(0.0, o.y)) - lum_at(c - vec2f(0.0, o.y));
    let g = length(vec2f(gx, gy)) * scale * 1.6;
    let v = clamp(1.0 - g, 0.0, 1.0);
    return from_disp(vec4f(vec3f(v), sa(uv).a));
}
"#,
    );

    shader(
        r,
        info(
            "jb.emboss",
            "Emboss",
            "Artistic",
            Effect,
            "Grey relief, as if engraved in metal. Filmora equivalent: Emboss.",
            &[P::float("intensity", "Intensity", 0.0, 100.0, 50.0)],
        ),
        r#"
fn lum_at(uv: vec2f) -> f32 { return luma(to_disp(sa(uv)).rgb); }
fn fx(uv: vec2f) -> vec4f {
    let k = 0.5 + 1.5 * p_intensity() / 100.0;
    let o = 1.0 / u.size;
    let v = 0.5
        - 2.0 * k * lum_at(uv + vec2f(-o.x, -o.y)) - k * lum_at(uv + vec2f(0.0, -o.y))
        - k * lum_at(uv + vec2f(-o.x, 0.0)) + k * lum_at(uv + vec2f(o.x, 0.0))
        + k * lum_at(uv + vec2f(0.0, o.y)) + 2.0 * k * lum_at(uv + vec2f(o.x, o.y));
    return from_disp(vec4f(vec3f(clamp(v, 0.0, 1.0)), sa(uv).a));
}
"#,
    );

    r.register(CustomFx::new(
        info(
            "jb.sharpen",
            "Sharpen",
            "Artistic",
            Effect,
            "Unsharp mask that crisps details and edges. Filmora equivalent: Sharpen.",
            &[
                P::float("amount", "Amount", 0.0, 100.0, 50.0),
                P::float("radius", "Radius", 0.5, 10.0, 1.5).unit("px"),
            ],
        ),
        &[r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let c = sa(in.uv);
    let b = sb(in.uv);
    let k = p_amount() / 100.0 * 2.0;
    let o = c + (c - b) * k;
    return vec4f(clamp(o.rgb, vec3f(0.0), vec3f(o.a)), c.a);
}
"#],
        |me, cx, io| blur_then_mix(me, cx, io, io.params.get("radius")),
    ));
}

fn register_mirror(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.mirror_horizontal",
            "Mirror Horizontal",
            "Mirror",
            Effect,
            "The left half is reflected onto the right. Filmora equivalent: Mirror (horizontal).",
            &[],
        ),
        "fn fx(uv: vec2f) -> vec4f { return sa(vec2f(select(uv.x, 1.0 - uv.x, uv.x > 0.5), uv.y)); }",
    );
    shader(
        r,
        info(
            "jb.mirror_vertical",
            "Mirror Vertical",
            "Mirror",
            Effect,
            "The top half is reflected onto the bottom. Filmora equivalent: Mirror (vertical).",
            &[],
        ),
        "fn fx(uv: vec2f) -> vec4f { return sa(vec2f(uv.x, select(uv.y, 1.0 - uv.y, uv.y > 0.5))); }",
    );
    shader(
        r,
        info(
            "jb.mirror",
            "Mirror",
            "Mirror",
            Effect,
            "Reflects one half of the frame onto the other (0 left→right, 1 right→left, 2 top→bottom, 3 bottom→top). Filmora equivalent: Mirror.",
            &[P::int("mode", "Mode", 0.0, 3.0, 0.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let m = i32(round(p_mode()));
    var q = uv;
    if (m == 0 && uv.x > 0.5) { q.x = 1.0 - uv.x; }
    if (m == 1 && uv.x < 0.5) { q.x = 1.0 - uv.x; }
    if (m == 2 && uv.y > 0.5) { q.y = 1.0 - uv.y; }
    if (m == 3 && uv.y < 0.5) { q.y = 1.0 - uv.y; }
    return sa(q);
}
"#,
    );
    shader(
        r,
        info(
            "jb.flip_horizontal",
            "Flip Horizontal",
            "Mirror",
            Effect,
            "Flips the clip left to right. Filmora equivalent: Flip Horizontal.",
            &[],
        ),
        "fn fx(uv: vec2f) -> vec4f { return sa(vec2f(1.0 - uv.x, uv.y)); }",
    );
    shader(
        r,
        info(
            "jb.flip_vertical",
            "Flip Vertical",
            "Mirror",
            Effect,
            "Flips the clip upside down. Filmora equivalent: Flip Vertical.",
            &[],
        ),
        "fn fx(uv: vec2f) -> vec4f { return sa(vec2f(uv.x, 1.0 - uv.y)); }",
    );
    shader(
        r,
        info(
            "jb.kaleidoscope",
            "Kaleidoscope",
            "Mirror",
            Effect,
            "Symmetric reflections in four quadrants, optionally repeated. Filmora equivalent: Kaleidoscope.",
            &[P::int("tiles", "Tiles", 1.0, 3.0, 1.0)],
        ),
        r#"
fn fold(q: vec2f) -> vec2f { return select(q, 1.0 - q, q > vec2f(0.5)); }
fn fx(uv: vec2f) -> vec4f {
    let n = i32(round(p_tiles()));
    var q = fold(uv);
    for (var i = 1; i < n; i = i + 1) {
        q = fold(q * 2.0);
    }
    return sa(q);
}
"#,
    );
}

fn register_motion(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.handheld_shake",
            "Camera Shake",
            "Motion",
            Effect,
            "Handheld camera shake for the whole clip. Filmora equivalent: Camera Shake.",
            &[INTENSITY, P::float("speed", "Speed", 0.0, 100.0, 60.0)],
        ),
        r#"
fn key(i: f32) -> vec3f {
    return vec3f(hash11(i * 3.17 + 1.0), hash11(i * 5.11 + 2.0), hash11(i * 7.31 + 3.0)) * 2.0 - 1.0;
}
fn fx(uv: vec2f) -> vec4f {
    let amp = 0.005 + 0.035 * p_intensity() / 100.0;
    let step = max(1.0, round(8.0 - 7.0 * p_speed() / 100.0));
    let t = u.time * 30.0 / step;
    let i0 = floor(t);
    let f = t - i0;
    let w = f * f * (3.0 - 2.0 * f);
    let k = mix(key(i0), key(i0 + 1.0), w);
    let rot = radians(k.z * 0.8 * p_intensity() / 100.0);
    let z = 1.0 + 2.2 * amp;
    let shifted = uv - k.xy * amp;
    return sat(spin_uv(shifted, rot, z));
}
"#,
    );

    shader(
        r,
        info(
            "jb.slow_zoom",
            "Slow Zoom",
            "Motion",
            Effect,
            "Smooth, continuous zoom over the clip (Ken Burns). Filmora equivalent: Ken Burns / Slow Zoom.",
            &[
                P::float("zoom", "Zoom", 5.0, 60.0, 20.0).unit("%"),
                P::float("direction", "Direction (0 in, 1 out)", 0.0, 1.0, 0.0),
            ],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let z = 1.0 + p_zoom() / 100.0;
    var t = clamp(prog(), 0.0, 1.0);
    if (p_direction() >= 0.5) { t = 1.0 - t; }
    return sa(zoom_uv(uv, mix(1.0, z, t), vec2f(0.5)));
}
"#,
    );

    shader(
        r,
        info(
            "jb.ken_burns_pan",
            "Slow Pan",
            "Motion",
            Effect,
            "Slow sideways pan over the enlarged image. Filmora equivalent: Ken Burns (pan).",
            &[
                P::float("zoom", "Zoom", 10.0, 60.0, 25.0).unit("%"),
                P::float("direction", "Direction (0 right, 1 left)", 0.0, 1.0, 0.0),
            ],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let z = 1.0 + p_zoom() / 100.0;
    var t = clamp(prog(), 0.0, 1.0);
    if (p_direction() >= 0.5) { t = 1.0 - t; }
    // Top-left of the enlarged image moves from x = 0 to x = 1 - z.
    let origin = vec2f(mix(0.0, 1.0 - z, t), (1.0 - z) / 2.0);
    return sa((uv - origin) / z);
}
"#,
    );

    shader(
        r,
        info(
            "jb.zoom_pulse",
            "Zoom Pulse",
            "Motion",
            Effect,
            "Rhythmic zoom punches following a tempo. Filmora equivalent: Beat Zoom / Pulse.",
            &[INTENSITY, P::float("bpm", "Beats per minute", 30.0, 180.0, 100.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let z = 1.0 + 0.15 * p_intensity() / 100.0;
    let phase = fract(u.time * p_bpm() / 60.0);
    let k = 1.0 - ease_io(seg(phase, 0.0, 0.5));
    return sa(zoom_uv(uv, mix(1.0, z, k), vec2f(0.5)));
}
"#,
    );
}

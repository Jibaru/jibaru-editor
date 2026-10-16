//! Creative transitions (Filmora premium style): glitch, RGB split, pixelate,
//! camera shake, light leak, film burn, ripple, mosaic, strobe, VHS rewind,
//! radial zoom blur and negative flash.
//!
//! Most hide a cut at the middle of the transition behind a burst of
//! distortion that builds up on clip A and fades out on clip B (a triangle
//! envelope peaking at the cut). Randomness is stepped on a nominal 30-frame
//! timeline so it flickers like the prototype regardless of the real length.

use super::{info, shader, CustomFx};
use crate::fx::FxCx;
use crate::registry::{FxIo, FxKind::Transition, ParamSpec, Registry};

/// Helpers shared by these shaders.
const COMMON: &str = r#"
const NF: f32 = 30.0;
// Index of the current nominal frame (0..NF-1).
fn nframe() -> f32 { return floor(prog() * (NF - 1.0) + 0.5); }
fn env(lo: f32, hi: f32) -> f32 { return tri_env(prog(), lo, 0.5, hi); }
fn pick(uv: vec2f, useb: bool) -> vec4f { return select(sa(uv), sb(uv), useb); }
fn rnd(s: f32, k: f32) -> f32 { return hash11(s * 13.17 + k * 7.31 + 0.5); }
fn bright(c: vec4f, k: f32) -> vec4f {
    let d = to_disp(c);
    return from_disp(vec4f(clamp(d.rgb * k, vec3f(0.0), vec3f(1.0)), d.a));
}
// Luma-mask reveal: B shows where m < threshold; `s` = edge softness.
fn luma_reveal(m: f32, q: f32, s: f32) -> f32 {
    let t = q * (1.0 + s);
    return 1.0 - smoothstep(t - s, t, m);
}
"#;

const INTENSITY: ParamSpec = ParamSpec::float("intensity", "Intensity", 0.0, 2.0, 1.0);
const BLUR: ParamSpec = ParamSpec::float("blur", "Blur", 0.0, 2.0, 1.0);

fn body(src: &str) -> String {
    format!("{COMMON}\n{src}")
}

fn light_leak_encode(fx: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let p = io.progress;
    let e = if p < 0.5 { ((p - 0.1) / 0.4).clamp(0.0, 1.0) } else { ((0.9 - p) / 0.4).clamp(0.0, 1.0) };
    let b = io.b.unwrap_or(io.a);
    let (a, b) = if e > 0.45 {
        let s = 0.012 * io.a.height as f32 * e;
        (cx.blur(io.a, s, s), cx.blur(b, s, s))
    } else {
        (io.a.clone(), b.clone())
    };
    cx.run(&fx.shaders[0], &a, Some(&b), io.out, CustomFx::uniforms(io));
}

pub fn register(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.glitch_digital",
            "Glitch",
            "Creative",
            Transition,
            "Signal dropouts with RGB split, jumps and noise around the cut. Filmora: Glitch.",
            &[INTENSITY],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let lo = 0.16;
    let hi = 0.84;
    let k = 1.0 / 24.0;
    // B flickers in a couple of frames before the cut, A flashes back once after it.
    var useb = p >= 0.5;
    if (p >= 0.5 - 3.0 * k && p < 0.5 - 2.0 * k) { useb = true; }
    if (p >= 0.5 + k && p < 0.5 + 2.0 * k) { useb = false; }
    if (p < lo || p >= hi) { return pick(uv, useb); }
    let sg = floor(nframe() / 2.0);
    let e = env(lo, hi);
    let amt = (0.4 + 0.6 * e) * p_intensity();
    let sgn = select(-1.0, 1.0, rnd(sg, 1.0) > 0.5);
    let s = 0.035 * amt;
    let jit = vec2f((rnd(sg, 2.0) * 2.0 - 1.0) * 0.04 * (0.3 + e), (rnd(sg, 3.0) * 2.0 - 1.0) * 0.01) * p_intensity();
    let juv = zoom_uv(uv, 1.12, vec2f(0.5)) - jit;
    let gv = (rnd(sg, 4.0) * 2.0 - 1.0) * 0.3 * s * aspect();
    let cr = pick(fract(juv - vec2f(sgn * s, 0.0)), useb);
    let cg = pick(fract(juv - vec2f(0.0, gv)), useb);
    let cb = pick(fract(juv + vec2f(sgn * s, 0.0)), useb);
    var c = vec4f(cr.r, cg.g, cb.b, max(max(cr.a, cg.a), cb.a));
    let n = (hash12(floor(uv * u.size) + vec2f(nframe() * 17.0, 3.0)) * 2.0 - 1.0) * 0.07;
    c = vec4f(clamp(c.rgb + n * c.a, vec3f(0.0), vec3f(c.a)), c.a);
    return c;
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.rgb_split",
            "RGB Split",
            "Creative",
            Transition,
            "The red, green and blue channels drift apart and come back together on the new clip. Filmora: RGB Split.",
            &[INTENSITY],
        ),
        body(
            r#"
fn split(uv: vec2f, d: f32, useb: bool) -> vec4f {
    let r = pick(uv + vec2f(d, 0.0), useb);
    let g = pick(uv - vec2f(0.0, 0.35 * d * aspect()), useb);
    let b = pick(uv - vec2f(d, 0.0), useb);
    return vec4f(r.r, g.g, b.b, max(max(r.a, g.a), b.a));
}
fn fx(uv: vec2f) -> vec4f {
    let d = 0.06 * pow(env(0.1, 0.9), 1.3) * p_intensity();
    return mix(split(uv, d, false), split(uv, d, true), seg(prog(), 0.45, 0.55));
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.pixelate",
            "Pixelate",
            "Creative",
            Transition,
            "The clip breaks into ever bigger blocks and resolves into the next one. Filmora: Pixelate.",
            &[INTENSITY],
        ),
        body(
            r#"
fn blocky(uv: vec2f, bs: vec2f, useb: bool) -> vec4f {
    let c = (floor(uv / bs) + 0.5) * bs;
    let o = bs * 0.25;
    return 0.25 * (pick(c + vec2f(-o.x, -o.y), useb) + pick(c + vec2f(o.x, -o.y), useb)
        + pick(c + vec2f(-o.x, o.y), useb) + pick(c + vec2f(o.x, o.y), useb));
}
fn fx(uv: vec2f) -> vec4f {
    let size = 0.07 * pow(env(0.08, 0.92), 1.5) * p_intensity();
    let k = seg(prog(), 0.5 - 1.0 / 30.0, 0.5 + 1.0 / 30.0);
    if (size * u.size.x <= 1.0) { return mix(sa(uv), sb(uv), k); }
    let bs = vec2f(size, size * aspect());
    return mix(blocky(uv, bs, false), blocky(uv, bs, true), k);
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.camera_shake",
            "Shake",
            "Creative",
            Transition,
            "The image shakes hard around the cut, with motion blur. Filmora: Shake.",
            &[INTENSITY],
        ),
        body(
            r#"
const LO: f32 = 0.14;
const HI: f32 = 0.86;
// Shake keyframe i (every 2 nominal frames): (dx, dy, scale).
fn key(i: f32) -> vec3f {
    let e = (0.5 + 0.5 * tri_env(2.0 * i / (NF - 1.0), LO, 0.5, HI)) * p_intensity();
    let sx = select(-1.0, 1.0, rnd(i, 1.0) > 0.5);
    let sy = select(-1.0, 1.0, rnd(i, 2.0) > 0.5);
    return vec3f(sx * mix(0.03, 0.06, rnd(i, 3.0)) * e, sy * mix(0.02, 0.045, rnd(i, 4.0)) * e, 1.0 + 0.16 * e);
}
fn scene(uv: vec2f) -> vec4f {
    let ff = prog() * (NF - 1.0) / 2.0;
    let i = floor(ff);
    let kf = mix(key(i), key(i + 1.0), ff - i);
    return select(sat(zoom_uv(uv - kf.xy, kf.z, vec2f(0.5))), sbt(zoom_uv(uv - kf.xy, kf.z, vec2f(0.5))), prog() >= 0.5);
}
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    if (p < LO || p >= HI) { return select(sa(uv), sb(uv), p >= 0.5); }
    let sg = floor(nframe() / 2.0);
    let ang = radians(15.0 + 40.0 * (sg - 3.0 * floor(sg / 3.0)));
    let radius = (0.003 + 0.01 * env(LO, HI)) * p_intensity();
    let d = vec2f(cos(ang), sin(ang) * aspect()) * radius;
    var acc = vec4f(0.0);
    for (var t = -4; t <= 4; t = t + 1) { acc = acc + scene(uv + d * (f32(t) / 4.0)); }
    return acc / 9.0;
}
"#,
        ),
    );
    r.register(CustomFx::new(
        info(
            "jb.light_leak",
            "Light Leak",
            "Light",
            Transition,
            "A warm overexposed flare burns the image and reveals the new clip. Filmora: Light Leak.",
            &[],
        ),
        &[&body(
            r#"
fn warm(c: vec4f, t: f32, level: f32) -> vec4f {
    let d = to_disp(c);
    var rgb = d.rgb * level;
    if (t >= 0.12) {
        rgb = vec3f((1.0 + 0.5 * t) * rgb.r + 0.25 * t * rgb.g, 0.05 * t * rgb.r + (1.0 - 0.3 * t) * rgb.g, (1.0 - 0.6 * t) * rgb.b);
    }
    return from_disp(vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), d.a));
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let e = env(0.1, 0.9);
    let level = 1.0 + 1.4 * e;
    return mix(warm(sa(in.uv), e, level), warm(sb(in.uv), e, level), seg(prog(), 0.4, 0.6));
}
"#,
        )],
        light_leak_encode,
    ));
    shader(
        r,
        info(
            "jb.film_burn",
            "Film Burn",
            "Light",
            Transition,
            "The film burns in overexposed orange blotches that reveal the new clip. Filmora: Film Burn.",
            &[],
        ),
        body(
            r#"
fn burn(c: vec4f, t: f32) -> vec4f {
    if (t < 0.1) { return c; }
    let d = to_disp(c);
    let s = d.rgb;
    var rgb = vec3f((1.0 + 0.5 * t) * s.r + 0.3 * t * s.g, 0.25 * t * s.r + (1.0 - 0.1 * t) * s.g, -0.1 * t * s.g + (1.0 - 0.75 * t) * s.b);
    rgb = rgb * (1.0 + 1.6 * pow(t, 1.5));
    return from_disp(vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), d.a));
}
fn mask(uv: vec2f) -> f32 {
    let q = vec2f(uv.x * 1.78, uv.y);
    let n = 0.6 * vnoise(q * 3.0 + vec2f(11.0, 3.0)) + 0.3 * vnoise(q * 7.0 + vec2f(23.0, 9.0))
        + 0.1 * vnoise(q * 16.0 + vec2f(5.0, 41.0));
    let corner = length(uv - vec2f(0.95, 0.1)) / 1.2;
    return clamp(0.55 * corner + 0.75 * (n - 0.2), 0.0, 1.0);
}
fn fx(uv: vec2f) -> vec4f {
    let t = env(0.08, 0.92);
    let k = luma_reveal(mask(uv), seg(prog(), 0.3, 0.72), 0.25);
    return mix(burn(sa(uv), t), burn(sb(uv), t), k);
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.ripple",
            "Ripple",
            "Creative",
            Transition,
            "The image ripples like water and dissolves into the new clip. Filmora: Ripple.",
            &[INTENSITY],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let amp = mix(1.0 / 1080.0, 0.018, env(0.04, 0.96)) * p_intensity();
    let ph = p * 16.0;
    let c = to_c(uv);
    let d = amp * vec2f(sin(c.y * TAU * 5.0 + ph), sin(c.x * TAU * 5.0 + ph * 1.3));
    var pts = array<vec2f, 8>(vec2f(0.04, 1.0), vec2f(0.25, 1.1), vec2f(0.75, 1.1), vec2f(0.96, 1.0),
        vec2f(0.0), vec2f(0.0), vec2f(0.0), vec2f(0.0));
    let z = pwl(p, pts, 4);
    let suv = zoom_uv(uv + h2uv(d), z, vec2f(0.5));
    return mix(sa(suv), sb(suv), seg(p, 0.3, 0.7));
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.mosaic",
            "Mosaic",
            "Creative",
            Transition,
            "The new clip appears in tiles that flip in random order. Filmora: Mosaic.",
            &[],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let cell = floor(uv * vec2f(16.0, 9.0));
    let m = 0.05 + 0.9 * hash12(cell + vec2f(5.0, 1.0));
    return mix(sa(uv), sb(uv), luma_reveal(m, prog(), 0.02));
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.strobe",
            "Strobe",
            "Light",
            Transition,
            "Intermittent white flashes alternate between both clips until settling on the new one. Filmora: Strobe.",
            &[],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    if (p < 0.16) { return sa(uv); }
    if (p >= 0.84) { return sb(uv); }
    let pf = 2.0 / NF;
    let i = floor((p - 0.25) / pf + 1e-3);
    let im2 = i - 2.0 * floor(i / 2.0);
    let im4 = i - 4.0 * floor(i / 4.0);
    let bvis = (p >= 0.5) != (abs(im4 - 1.0) < 0.5);
    let c = pick(uv, bvis);
    return select(c, bright(c, 2.4), im2 < 0.5);
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.vhs_rewind",
            "VHS Rewind",
            "Creative",
            Transition,
            "VHS tape rewind: tracking lines, noise, smeared color and vertical jumps. Filmora: VHS Rewind.",
            &[INTENSITY],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let lo = 0.12;
    let hi = 0.88;
    let useb = p >= 0.5;
    if (p < lo || p >= hi) { return pick(uv, useb); }
    let f = nframe();
    let sg = floor(f / 2.0);
    let e = env(lo, hi);
    let k = p_intensity();
    let off = vec2f((rnd(sg, 1.0) * 2.0 - 1.0) * 0.01, (rnd(sg, 2.0) * 2.0 - 1.0) * 0.05 * (0.4 + 0.6 * e)) * k;
    let suv = zoom_uv(uv - off, 1.14, vec2f(0.5));
    let ch = 0.012 * k;
    let c0 = to_disp(pick(suv, useb));
    let cl = to_disp(pick(suv - vec2f(ch, 0.0), useb));
    let cr = to_disp(pick(suv + vec2f(ch, 0.0), useb));
    // Smear chroma: luma from the centre tap, Cb from the left, Cr from the right.
    let y = luma(c0.rgb);
    let cb = (cl.b - luma(cl.rgb)) / 1.8556;
    let crr = (cr.r - luma(cr.rgb)) / 1.5748;
    var rgb = vec3f(y + 1.5748 * crr, y - 0.1873 * cb - 0.4681 * crr, y + 1.8556 * cb);
    rgb = mix(vec3f(luma(rgb)), rgb, 0.65);
    rgb = (rgb - 0.5) * 1.15 + 0.5;
    rgb = rgb + (hash12(floor(uv * u.size) + vec2f(f * 31.0, 7.0)) * 2.0 - 1.0) * 0.08 * k;
    // Vertical tape scratches.
    let pxw = u.inv_size.x;
    for (var i = 0; i < 18; i = i + 1) {
        let x = rnd(f, f32(i) + 10.0);
        if (abs(uv.x - x) < 1.5 * pxw * (u.size.y / 1080.0) && rnd(f, f32(i) + 40.0) < 0.5) {
            rgb = rgb + select(-0.2, 0.2, rnd(f, f32(i) + 70.0) > 0.5);
        }
    }
    // Bright tracking band crawling up the frame.
    let bi = floor((f - floor(lo * (NF - 1.0))) / 2.0);
    let y0 = 0.85 - (bi * 0.11 - 0.8 * floor(bi * 0.11 / 0.8));
    if (uv.y >= y0 && uv.y < y0 + 0.06) { rgb = mix(rgb, vec3f(1.0), 0.35); }
    return from_disp(vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), c0.a));
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.radial_zoom_blur",
            "Zoom Blur",
            "Creative",
            Transition,
            "A fast zoom that bulges and blurs towards the centre at the cut. Filmora: Zoom Blur.",
            &[BLUR],
        ),
        body(
            r#"
fn scene(uv: vec2f) -> vec4f {
    let p = prog();
    let e = env(0.1, 0.9);
    let k = select(0.0, -0.45 * e, e > 0.1);
    let c = to_c(uv);
    let luv = from_c(c * (1.0 + k * dot(c, c)));
    if (p < 0.5) { return sat(zoom_uv(luv, mix(1.0, 2.3, seg(p, 0.1, 0.5)), vec2f(0.5))); }
    return sbt(zoom_uv(luv, mix(1.8, 1.0, seg(p, 0.5, 0.9)), vec2f(0.5)));
}
fn fx(uv: vec2f) -> vec4f {
    let e = env(0.1, 0.9);
    let a = select(0.0, 0.3 * pow(e, 1.5) * p_blur(), e > 0.1);
    if (a < 1e-4) { return scene(uv); }
    var acc = vec4f(0.0);
    var ws = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let x = f32(i) / 15.0;
        let w = 1.0 - 0.5 * x;
        acc = acc + w * scene(zoom_uv(uv, 1.0 + a * x, vec2f(0.5)));
        ws = ws + w;
    }
    return acc / ws;
}
"#,
        ),
    );
    shader(
        r,
        info(
            "jb.negative_cut",
            "Negative Flash",
            "Creative",
            Transition,
            "Colors invert with a flash right at the cut and return to normal. Filmora: Negative / Invert Flash.",
            &[],
        ),
        body(
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let useb = p >= 0.5;
    var c = pick(uv, useb);
    if (p < 0.2 || p >= 0.8) { return c; }
    let d = to_disp(c);
    var level = 1.0;
    if (!useb) { level = mix(1.0, 1.5, seg(p, 0.5 - 2.0 / 30.0, 0.5)); }
    else { level = mix(1.5, 1.0, seg(p, 0.5, 0.5 + 2.0 / 30.0)); }
    return from_disp(vec4f(clamp((1.0 - d.rgb) * level, vec3f(0.0), vec3f(1.0)), d.a));
}
"#,
        ),
    );
}

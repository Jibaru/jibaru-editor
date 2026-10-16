// Shared prelude of every jb-render shader.
//
// Conventions:
// - Images are premultiplied RGBA in the renderer working space
//   (display-referred sRGB by default, linear light when `is_linear()`).
//   Color-grading code converts with `to_disp` / `from_disp`.
// - uv has its origin at the top-left corner, y down.
// - Sizes given as "fraction of height" are converted with `h2uv`.

struct U {
    size: vec2f,
    inv_size: vec2f,
    a_size: vec2f,
    b_size: vec2f,
    progress: f32,
    time: f32,
    seed: f32,
    flags: u32,
    e0: vec4f,
    e1: vec4f,
    e2: vec4f,
    e3: vec4f,
    p: array<vec4f, 32>,
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var ta: texture_2d<f32>;
@group(0) @binding(3) var tb: texture_2d<f32>;
@group(0) @binding(4) var tx: texture_2d<f32>;
@group(0) @binding(5) var tlut: texture_3d<f32>;
@group(0) @binding(6) var smp_rep: sampler;

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> VsOut {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var o: VsOut;
    o.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2f(x, y);
    return o;
}

const PI: f32 = 3.14159265358979;
const TAU: f32 = 6.28318530717959;
const LUMA: vec3f = vec3f(0.2126, 0.7152, 0.0722);

fn prog() -> f32 { return u.progress; }
fn aspect() -> f32 { return u.size.x * u.inv_size.y; }
fn is_linear() -> bool { return (u.flags & 1u) != 0u; }
fn in01(uv: vec2f) -> bool { return all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0)); }

fn sa(uv: vec2f) -> vec4f { return textureSampleLevel(ta, smp, uv, 0.0); }
fn sb(uv: vec2f) -> vec4f { return textureSampleLevel(tb, smp, uv, 0.0); }
fn sx(uv: vec2f) -> vec4f { return textureSampleLevel(tx, smp, uv, 0.0); }
// Transparent outside the frame.
fn sat(uv: vec2f) -> vec4f { return select(vec4f(0.0), sa(uv), in01(uv)); }
fn sbt(uv: vec2f) -> vec4f { return select(vec4f(0.0), sb(uv), in01(uv)); }

fn srgb_to_lin(c: vec3f) -> vec3f {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3f(2.4));
    return select(hi, lo, c <= vec3f(0.04045));
}
fn lin_to_srgb(c: vec3f) -> vec3f {
    let x = max(c, vec3f(0.0));
    let lo = x * 12.92;
    let hi = 1.055 * pow(x, vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, x <= vec3f(0.0031308));
}
fn unpremul(c: vec4f) -> vec4f {
    if (c.a <= 1e-5) { return vec4f(0.0); }
    return vec4f(c.rgb / c.a, c.a);
}
// Working (premultiplied) -> display-referred straight color.
fn to_disp(c: vec4f) -> vec4f {
    var s = unpremul(c);
    if (is_linear()) { s = vec4f(lin_to_srgb(s.rgb), s.a); }
    return s;
}
// Display-referred straight color -> working (premultiplied).
fn from_disp(c: vec4f) -> vec4f {
    var rgb = c.rgb;
    if (is_linear()) { rgb = srgb_to_lin(max(rgb, vec3f(0.0))); }
    return vec4f(rgb * c.a, c.a);
}
// Linear RGB color constant (project colors are linear) -> working space.
fn lin_color(c: vec4f) -> vec4f {
    var rgb = c.rgb;
    if (!is_linear()) { rgb = lin_to_srgb(rgb); }
    return vec4f(rgb * c.a, c.a);
}
fn luma(c: vec3f) -> f32 { return dot(c, LUMA); }
fn over(top: vec4f, bottom: vec4f) -> vec4f { return top + bottom * (1.0 - top.a); }

// One "1080p pixel" in uv units.
fn px1080() -> vec2f { return vec2f(1.0 / (1080.0 * aspect()), 1.0 / 1080.0); }
// Vector in fractions of the image height -> uv.
fn h2uv(v: vec2f) -> vec2f { return v * vec2f(1.0 / aspect(), 1.0); }
// Aspect-corrected coordinates centred on the frame (y spans -0.5..0.5).
fn to_c(uv: vec2f) -> vec2f { return (uv - 0.5) * vec2f(aspect(), 1.0); }
fn from_c(p: vec2f) -> vec2f { return p / vec2f(aspect(), 1.0) + 0.5; }
// Where to sample so the image appears scaled by `s` around `c`.
fn zoom_uv(uv: vec2f, s: f32, c: vec2f) -> vec2f { return (uv - c) / s + c; }
fn rot2(v: vec2f, a: f32) -> vec2f {
    let cs = cos(a);
    let sn = sin(a);
    return vec2f(v.x * cs - v.y * sn, v.x * sn + v.y * cs);
}
// Sample uv for an image rotated clockwise by `a` radians and scaled by `s` around the centre.
fn spin_uv(uv: vec2f, a: f32, s: f32) -> vec2f { return from_c(rot2(to_c(uv), -a) / s); }

fn seg(t: f32, a: f32, b: f32) -> f32 { return clamp((t - a) / max(b - a, 1e-6), 0.0, 1.0); }
fn ease_io(t: f32) -> f32 { let x = clamp(t, 0.0, 1.0); return x * x * (3.0 - 2.0 * x); }
fn ease_in(t: f32) -> f32 { let x = clamp(t, 0.0, 1.0); return x * x * x; }
fn ease_out(t: f32) -> f32 { let x = 1.0 - clamp(t, 0.0, 1.0); return 1.0 - x * x * x; }
fn bounce_out(t0: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    var t = clamp(t0, 0.0, 1.0);
    if (t < 1.0 / d1) { return n1 * t * t; }
    if (t < 2.0 / d1) { t = t - 1.5 / d1; return n1 * t * t + 0.75; }
    if (t < 2.5 / d1) { t = t - 2.25 / d1; return n1 * t * t + 0.9375; }
    t = t - 2.625 / d1;
    return n1 * t * t + 0.984375;
}
// Piecewise-linear curve through the first `n` points (x ascending).
fn pwl(x: f32, pts: array<vec2f, 8>, n: i32) -> f32 {
    var p = pts;
    if (x <= p[0].x) { return p[0].y; }
    for (var i = 1; i < n; i = i + 1) {
        if (x < p[i].x) {
            let k = (x - p[i - 1].x) / max(p[i].x - p[i - 1].x, 1e-6);
            return mix(p[i - 1].y, p[i].y, k);
        }
    }
    return p[n - 1].y;
}
// Triangle envelope: 0 at lo and hi, 1 at mid.
fn tri_env(t: f32, lo: f32, mid: f32, hi: f32) -> f32 {
    if (t < mid) { return clamp((t - lo) / max(mid - lo, 1e-6), 0.0, 1.0); }
    return clamp((hi - t) / max(hi - mid, 1e-6), 0.0, 1.0);
}

fn hash11(x: f32) -> f32 { return fract(sin(x * 127.1 + 311.7) * 43758.5453); }
fn hash12(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
fn hash22(p: vec2f) -> vec2f {
    var p3 = fract(vec3f(p.x, p.y, p.x) * vec3f(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}
fn vnoise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let a = hash12(i);
    let b = hash12(i + vec2f(1.0, 0.0));
    let c = hash12(i + vec2f(0.0, 1.0));
    let d = hash12(i + vec2f(1.0, 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}
fn fbm(p0: vec2f) -> f32 {
    var p = p0;
    var a = 0.5;
    var s = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        s = s + a * vnoise(p);
        p = p * 2.03 + vec2f(17.1, 9.2);
        a = a * 0.5;
    }
    return s / 0.9375;
}

fn rgb2hsv(c: vec3f) -> vec3f {
    let k = vec4f(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = select(vec4f(c.gb, k.xy), vec4f(c.bg, k.wz), c.g < c.b);
    let q = select(vec4f(p.xyw, c.r), vec4f(c.r, p.yzx), p.x < c.r);
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3f(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}
fn hsv2rgb(c: vec3f) -> vec3f {
    let k = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, vec3f(0.0), vec3f(1.0)), c.y);
}
fn saturate3(c: vec3f, s: f32) -> vec3f { let l = luma(c); return mix(vec3f(l), c, s); }

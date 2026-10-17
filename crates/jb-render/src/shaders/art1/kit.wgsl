// Shared library of the art1 items (fx/art1_kit.rs): procedural light,
// particles and drawn shapes. Conventions:
// - Colours are display-referred (sRGB) straight RGB; `a1_da`/`a1_db` read
//   the clips that way and `a1_out` writes the result back.
// - Positions are "centred" coordinates (`to_c`): y spans -0.5..0.5 (down),
//   x spans -aspect/2..aspect/2; sizes are fractions of the frame height.

// Key time (progress for transitions, wrapped clip seconds for effects).
fn a1_t() -> f32 { return u.e0.x; }

fn a1_da(uv: vec2f) -> vec3f { return to_disp(sa(clamp(uv, vec2f(0.0), vec2f(1.0)))).rgb; }
fn a1_db(uv: vec2f) -> vec3f { return to_disp(sb(clamp(uv, vec2f(0.0), vec2f(1.0)))).rgb; }
fn a1_dx(uv: vec2f) -> vec4f { return sx(clamp(uv, vec2f(0.0), vec2f(1.0))); }
// The `//@base` effect (display space).
fn a1_base(uv: vec2f) -> vec3f { return to_disp(sx(clamp(uv, vec2f(0.0), vec2f(1.0)))).rgb; }
fn a1_out(c: vec3f) -> vec4f { return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), 1.0)); }
// Effects keep the layer's alpha.
fn a1_out_a(c: vec3f, a: f32) -> vec4f { return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), a)); }
fn a1_alpha(uv: vec2f) -> f32 { return sa(clamp(uv, vec2f(0.0), vec2f(1.0))).a; }

// ---- random numbers (deterministic per index / seed) ----
fn a1_r(i: f32, k: f32) -> f32 { return hash12(vec2f(i * 1.618 + 0.37, k * 7.31 + 1.13)); }
fn a1_r2(i: f32, k: f32) -> vec2f { return vec2f(a1_r(i, k), a1_r(i, k + 17.0)); }
// Signed random in -1..1.
fn a1_s(i: f32, k: f32) -> f32 { return 2.0 * a1_r(i, k) - 1.0; }

// ---- blending ----
fn a1_screen(base: vec3f, light: vec3f) -> vec3f { return 1.0 - (1.0 - base) * (1.0 - clamp(light, vec3f(0.0), vec3f(1.0))); }
// Soft-clipped additive light (no hard clip for bright overlaps).
fn a1_add(base: vec3f, light: vec3f) -> vec3f { return base + light * (1.0 - 0.5 * base); }
fn a1_lerp3(a: vec3f, b: vec3f, w: f32) -> vec3f { return mix(a, b, clamp(w, 0.0, 1.0)); }
// Three-stop colour ramp over 0..1.
fn a1_ramp(x: f32, c0: vec3f, c1: vec3f, c2: vec3f) -> vec3f {
    let t = clamp(x, 0.0, 1.0);
    if (t < 0.5) { return mix(c0, c1, t * 2.0); }
    return mix(c1, c2, t * 2.0 - 1.0);
}
// Fire palette: dark red -> orange -> yellow -> white as heat rises.
fn a1_fire_col(h: f32) -> vec3f {
    let x = clamp(h, 0.0, 1.0);
    return vec3f(min(1.0, 1.6 * x), clamp(1.7 * x - 0.45, 0.0, 1.0), clamp(2.2 * x - 1.3, 0.0, 1.0));
}

// ---- falloffs and simple shapes (d = distance, r = radius, fractions of height) ----
fn a1_gauss(d: f32, r: f32) -> f32 { return exp(-(d * d) / max(r * r, 1e-8)); }
// Long-tailed glow: 1 at the centre, 1/(1+(d/r)^2) falloff.
fn a1_halo(d: f32, r: f32) -> f32 { let x = d / max(r, 1e-6); return 1.0 / (1.0 + x * x); }
fn a1_disc(d: f32, r: f32, soft: f32) -> f32 { return 1.0 - smoothstep(r - soft, r + soft, d); }
// Out-of-focus highlight: a soft disc whose rim is a little brighter than its centre.
fn a1_bokeh(d: f32, r: f32, soft: f32) -> f32 {
    let body = a1_disc(d, r, soft);
    return body * mix(0.72, 1.0, smoothstep(0.35 * r, r, d));
}
fn a1_seg(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let ab = b - a;
    let h = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-9), 0.0, 1.0);
    return length(p - a - ab * h);
}
// Four-point sparkle centred at the origin: bright core plus two thin crossed
// rays of length `len`, rotated by `rot`.
fn a1_sparkle(q0: vec2f, len: f32, rot: f32) -> f32 {
    let q = rot2(q0, -rot);
    let w = 0.022 * len;
    let rx = exp(-abs(q.x) / (0.35 * len)) * exp(-(q.y * q.y) / (w * w));
    let ry = exp(-abs(q.y) / (0.35 * len)) * exp(-(q.x * q.x) / (w * w));
    let core = a1_gauss(length(q), 0.07 * len);
    return max(core, 0.8 * (rx + ry));
}
// Six-ray star flare (camera flash): rays every 60 degrees.
fn a1_starflare(q: vec2f, len: f32, rot: f32) -> f32 {
    let r = length(q);
    let a = atan2(q.y, q.x) + rot;
    let ray = pow(abs(cos(3.0 * a)), 60.0) * exp(-r / (0.3 * len));
    return ray + a1_halo(r, 0.08 * len);
}
// Heart (approximate signed distance, negative inside) of height ~ 2s, point
// down (y down): a square turned 45 degrees with a disc on each upper edge.
fn a1_heart(p0: vec2f, s: f32) -> f32 {
    let q = vec2f(abs(p0.x), -p0.y) / s - vec2f(0.0, -0.2);
    let r = rot2(q, 0.785398);
    let dq = abs(r) - vec2f(0.5);
    let d_sq = length(max(dq, vec2f(0.0))) + min(max(dq.x, dq.y), 0.0);
    let d_c = length(q - vec2f(0.3536, 0.3536)) - 0.5;
    return min(d_sq, d_c) * s;
}
// Rounded box signed distance (half size b, corner radius rr).
fn a1_rbox(p: vec2f, b: vec2f, rr: f32) -> f32 {
    let q = abs(p) - b + rr;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - rr;
}

// ---- noise ----
fn a1_fbm(p0: vec2f, oct: i32) -> f32 {
    var p = p0;
    var a = 0.5;
    var s = 0.0;
    var n = 0.0;
    for (var i = 0; i < oct; i = i + 1) {
        s = s + a * vnoise(p);
        n = n + a;
        p = rot2(p, 0.6) * 2.02 + vec2f(11.3, 5.7);
        a = a * 0.5;
    }
    return s / max(n, 1e-6);
}
// Noise stretched along `dir` (unit): streaks, brush strokes, speed lines.
fn a1_streak(p: vec2f, dir: vec2f, stretch: f32, k: f32, oct: i32) -> f32 {
    let q = vec2f(dot(p, dir) / stretch, dot(p, vec2f(-dir.y, dir.x)));
    return a1_fbm(q * k, oct);
}
// Ramp along `dir` broken up by streaks along `sdir`: the field a stroke
// mask thresholds (covered where field < threshold). Ported in the fitting
// scripts to turn measured coverage into thresholds.
fn a1_stroke_field(p: vec2f, dir: vec2f, sdir: vec2f, stretch: f32, k: f32, amp: f32) -> f32 {
    return dot(p, dir) + amp * (a1_streak(p, sdir, stretch, k, 4) - 0.5);
}
// Turbulent flame density in 0..1 for a flame rising along -y; `t` animates.
fn a1_flame(p: vec2f, t: f32) -> f32 {
    let w = vec2f(a1_fbm(p * 3.0 + vec2f(0.0, 2.5 * t), 3), a1_fbm(p * 3.0 + vec2f(5.2, 2.5 * t + 1.3), 3));
    return a1_fbm(p * 4.0 + 1.6 * (w - 0.5) + vec2f(0.0, 3.0 * t), 4);
}

// ---- paths ----
fn a1_bez(p0: vec2f, p1: vec2f, p2: vec2f, p3: vec2f, s: f32) -> vec2f {
    let t = clamp(s, 0.0, 1.0);
    let it = 1.0 - t;
    return it * it * it * p0 + 3.0 * it * it * t * p1 + 3.0 * it * t * t * p2 + t * t * t * p3;
}

// ---- comets: a head travelling along a Catmull-Rom path from t0 to t1 (key time) ----
// The path's n points are stored two per uniform slot from u.p[base] (xy, zw).
struct Comet {
    base: i32,
    n: i32,
    t0: f32,
    t1: f32,
    // exponent of the head's progress along the path (1 = constant speed)
    ease: f32,
};
fn a1_pt(base: i32, i: i32) -> vec2f {
    let v = u.p[base + i / 2];
    return select(v.xy, v.zw, (i % 2) == 1);
}
fn a1_comet_s(c: Comet, t: f32) -> f32 { return pow(clamp((t - c.t0) / max(c.t1 - c.t0, 1e-5), 0.0, 1.0), c.ease); }
fn a1_comet_at(c: Comet, s: f32) -> vec2f {
    let x = clamp(s, 0.0, 1.0) * f32(c.n - 1);
    let i = min(i32(floor(x)), c.n - 2);
    let f = x - f32(i);
    let p0 = a1_pt(c.base, max(i - 1, 0));
    let p1 = a1_pt(c.base, i);
    let p2 = a1_pt(c.base, i + 1);
    let p3 = a1_pt(c.base, min(i + 2, c.n - 1));
    let f2 = f * f;
    let f3 = f2 * f;
    return 0.5 * (2.0 * p1 + (p2 - p0) * f + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * f3);
}
// Distance from p to the part of the path already travelled (0..smax) and the
// path parameter of the closest point: vec2f(d, s).
fn a1_comet_near(c: Comet, p: vec2f, smax: f32) -> vec2f {
    var best = vec2f(1e9, 0.0);
    var prev = a1_comet_at(c, 0.0);
    let n = 40;
    for (var i = 1; i <= n; i = i + 1) {
        let s1 = smax * f32(i) / f32(n);
        let cur = a1_comet_at(c, s1);
        let ab = cur - prev;
        let h = clamp(dot(p - prev, ab) / max(dot(ab, ab), 1e-9), 0.0, 1.0);
        let d = length(p - prev - ab * h);
        if (d < best.x) { best = vec2f(d, smax * (f32(i - 1) + h) / f32(n)); }
        prev = cur;
    }
    return best;
}
// Glitter shed by a comet: `n` particles born along the travelled path (time
// spread t0..t1), scattered by `spread`, drifting by `drift` per unit key time,
// drifting along `bias`, living `life` (key time) and twinkling. Returns vec2f(sparkle light, dust light).
fn a1_comet_glitter(c: Comet, p: vec2f, t: f32, n: i32, seed: f32, spread: f32, drift: f32, bias: vec2f, life: f32, size: f32, nstar: i32) -> vec2f {
    var star = 0.0;
    var dust = 0.0;
    for (var i = 0; i < n; i = i + 1) {
        let fi = f32(i) + seed * 101.0;
        let tb = mix(c.t0, c.t1, a1_r(fi, 1.0));
        let age = t - tb;
        let li = life * (0.5 + a1_r(fi, 2.0));
        if (age < 0.0 || age > li) { continue; }
        let base = a1_comet_at(c, a1_comet_s(c, tb));
        let off = vec2f(a1_s(fi, 3.0), a1_s(fi, 4.0)) * spread * sqrt(a1_r(fi, 9.0));
        let vel = vec2f(a1_s(fi, 5.0), a1_s(fi, 6.0)) * drift + bias;
        let q = p - (base + off + vel * age);
        let fade = (1.0 - age / li) * (0.55 + 0.45 * sin(40.0 * age + 6.28 * a1_r(fi, 7.0)));
        if (i < nstar) {
            let len = size * (0.6 + 1.2 * a1_r(fi, 8.0));
            if (dot(q, q) < 4.0 * len * len) { star = star + fade * a1_sparkle(q, len, 0.0); }
        } else {
            let r = size * 0.05 * (0.5 + a1_r(fi, 8.0));
            dust = dust + fade * a1_gauss(length(q), r);
        }
    }
    return vec2f(star, dust);
}

// Out-of-focus discs shed by a comet (same scheme as the glitter): returns
// vec2f(light, colour key in 0..1) of the brightest disc covering p.
fn a1_comet_bokeh(c: Comet, p: vec2f, t: f32, n: i32, seed: f32, spread: f32, drift: f32, bias: vec2f, life: f32, rmin: f32, rmax: f32) -> vec2f {
    var light = 0.0;
    var key = 0.0;
    for (var i = 0; i < n; i = i + 1) {
        let fi = f32(i) + seed * 101.0 + 7.0;
        let tb = mix(c.t0, c.t1, a1_r(fi, 1.0));
        let age = t - tb;
        let li = life * (0.5 + a1_r(fi, 2.0));
        if (age < 0.0 || age > li) { continue; }
        let base = a1_comet_at(c, a1_comet_s(c, tb));
        let off = vec2f(a1_s(fi, 3.0), a1_s(fi, 4.0)) * spread * sqrt(a1_r(fi, 9.0));
        let vel = vec2f(a1_s(fi, 5.0), a1_s(fi, 6.0)) * drift + bias;
        let q = p - (base + off + vel * age);
        let r = mix(rmin, rmax, a1_r(fi, 8.0));
        let d = length(q);
        if (d > r * 1.2) { continue; }
        let fade = smoothstep(0.0, 0.08 * li, age) * (1.0 - age / li);
        let b = fade * a1_bokeh(d, r, 0.18 * r) * (0.5 + 0.5 * a1_r(fi, 11.0));
        if (b > light) { key = a1_r(fi, 12.0); }
        light = max(light, b);
    }
    return vec2f(light, key);
}

// ---- lens flare elements along the axis through the light at `l` and the frame centre ----
// Returns additive light (white-ish, tinted per element) at centred position p.
fn a1_flare(p: vec2f, l: vec2f, size: f32, tint: vec3f) -> vec3f {
    let axis = -l;
    var c = vec3f(0.0);
    // ghosts: discs at fractions of the axis, alternating tints
    for (var i = 0; i < 5; i = i + 1) {
        let fi = f32(i);
        let k = 0.4 + 0.35 * fi;
        let gp = l + axis * k;
        let r = size * (0.10 + 0.07 * a1_r(fi, 3.0));
        let g = a1_bokeh(length(p - gp), r, 0.3 * r) * 0.18;
        c = c + g * mix(tint, vec3f(0.6, 0.8, 1.0), a1_r(fi, 5.0));
    }
    // ring halo around the light
    let d = length(p - l);
    c = c + 0.12 * tint * exp(-pow((d - 0.9 * size) / (0.08 * size), 2.0));
    // anamorphic streak
    let q = p - l;
    c = c + 0.5 * vec3f(0.7, 0.85, 1.0) * exp(-abs(q.x) / (2.0 * size)) * exp(-(q.y * q.y) / (0.0004 * size));
    return c;
}

// ---- bokeh fields ----
// `n` out-of-focus discs scattered in a box of half size `ext` around `centre`
// (centred coordinates), each drifting by `drift` per unit key time and
// fading in and out on its own phase. Returns vec2f(light, colour key 0..1)
// of the brightest disc covering p.
fn a1_bokeh_field(p: vec2f, t: f32, n: i32, seed: f32, centre: vec2f, ext: vec2f, drift: vec2f, rmin: f32, rmax: f32, soft: f32) -> vec2f {
    var light = 0.0;
    var key = 0.0;
    for (var i = 0; i < n; i = i + 1) {
        let fi = f32(i) + seed * 131.0 + 3.0;
        let r = mix(rmin, rmax, a1_r(fi, 8.0) * a1_r(fi, 10.0));
        let c = centre + vec2f(a1_s(fi, 3.0), a1_s(fi, 4.0)) * ext + drift * t * (0.5 + a1_r(fi, 5.0));
        let d = length(p - c);
        if (d > r * 1.3) { continue; }
        let tw = 0.55 + 0.45 * sin(6.28 * (a1_r(fi, 6.0) + 1.5 * t));
        let b = tw * a1_bokeh(d, r, soft * r) * (0.5 + 0.5 * a1_r(fi, 11.0));
        if (b > light) { key = a1_r(fi, 12.0); }
        light = max(light, b);
    }
    return vec2f(light, key);
}
// Soft orb: vec4f(colour, coverage) of a large out-of-focus light ball
// o = (x, y, radius, opacity), core colour fading to rim colour.
fn a1_orb(p: vec2f, o: vec4f, core: vec3f, rim: vec3f) -> vec4f {
    let d = length(p - o.xy) / max(o.z, 1e-3);
    let body = 1.0 - smoothstep(0.75, 1.05, d);
    return vec4f(mix(core, rim, smoothstep(0.0, 0.9, d)), o.w * body * (0.85 + 0.15 * (1.0 - min(d, 1.0))));
}

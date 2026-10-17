// Art 2 paint kit: procedural brush strokes, ink blots, sweeps and torn
// paper edges that reveal the incoming clip or lay paint over the frame.
//
// Every item is a list of strokes evaluated in order (later strokes on
// top). The generator (`fx/art2_paint.rs`) appends the stroke and paint
// tables (`ART_S`, `ART_P`, `ART_N`, `ART_DONE`) and calls `art_fx`.
//
// Coordinates: `q` is aspect-corrected, x in 0..aspect, y in 0..1 (y down);
// widths and lengths are fractions of the frame height.
//
// Stroke record (12 vec4):
//   s0 = (ax, ay, cx, cy)    start and control point (uv)
//   s1 = (bx, by, w0, w1)    end point (uv), half widths at start / end
//   s2 = (t0, t1, kind, ease) progress window of the motion
//   s3 = (rag, dry, fill, seed)
//   s4 = (style, rim, op0, opt)
//   s5 = (soft, mottle, jag, tail)
//   s6 = (p1, h1, p2, h2)   optional timing knots of the head (p1 > 0)
//   s7 = (dry_close, g0, g1, gk) progress by which dry-brush gaps have closed;
//        the width grows by gk between progress g0 and g1 (g1 > g0)
//   s8 = (invert, f0, f1, -) invert > 0.5 covers the outside of a blot or the
//        side ahead of a sweep; opacity falls to 0 over f0..f1 (f1 > f0)
//   s9 = (ax, ay, bx, by)  end and start points after the morph
//   s10 = (cx, cy, m0, m1) control point after the morph; a, b and c move to
//        s9 / s10 over progress m0..m1 (m1 > m0)
// kind: 0 brush along a quadratic path (the head travels from a to b),
//       1 blot growing from radius w0 to w1 around a (c = x/y stretch),
//       2 sweep: a ragged edge moving from a to b, covering what it passed,
//       3 path brush whose tail also leaves (w0..w1 window trimmed by s5.w).
// ease: 0 linear, 1 smooth, 2 out (cubic), 3 in (cubic), 4 out (quadratic).
// style: 0 bristle brush, 1 torn paper, 2 clean, 3 watercolour.
// fill: 0 incoming clip, 1 outgoing clip, 2 + k paint k.
// rim > 0: only a band of that width just outside the shape is drawn.
// op0 / opt: opacity starts at op0 and reaches 1 at progress opt (opt > 0).
//
// Paint record (3 vec4):
//   p0 = (r, g, b, streak)   base colour (display space), streak contrast
//   p1 = (r, g, b, sheen)    light colour, sheen amount
//   p2 = (wet, grain, shade, alpha)  wet-edge darkening, paper grain,
//        shade across the stroke (curl), opacity

struct Hit {
    // signed margin to the shape boundary (height units, > 0 inside)
    m: f32,
    // position across the stroke (-1..1) and along it (height units)
    n: f32,
    s: f32,
    // extra alpha factor (dry streaks, mottling)
    k: f32,
};

fn art_ease(x0: f32, e: f32) -> f32 {
    let x = clamp(x0, 0.0, 1.0);
    if (e < 0.5) { return x; }
    if (e < 1.5) { return x * x * (3.0 - 2.0 * x); }
    if (e < 2.5) { let y = 1.0 - x; return 1.0 - y * y * y; }
    if (e < 3.5) { return x * x * x; }
    let y = 1.0 - x;
    return 1.0 - y * y;
}

fn art_q(uv: vec2f) -> vec2f { return vec2f(uv.x * aspect(), uv.y); }

fn art_bez(a: vec2f, c: vec2f, b: vec2f, t: f32) -> vec2f {
    let mt = 1.0 - t;
    return mt * mt * a + 2.0 * mt * t * c + t * t * b;
}

// Bristle profile across a brush: clumps of bristles of uneven length
// (0..1), keyed by the position across the stroke.
fn art_bristle(n: f32, seed: f32) -> f32 {
    let k = n * 14.0;
    let fine = hash12(vec2f(floor(k * 2.3), seed * 7.13));
    let clump = vnoise(vec2f(k * 0.45, seed * 3.7));
    return clamp(0.55 * clump + 0.45 * fine, 0.0, 1.0);
}

// Closest point on the quadratic path a-c-b approximated by 16 segments,
// with the first and last segments extended past the ends. Returns
// (signed distance across, arc length along, total length).
fn art_path(q: vec2f, a: vec2f, c: vec2f, b: vec2f) -> vec3f {
    var best = 1e9;
    var bs = 0.0;
    var bn = 0.0;
    var acc = 0.0;
    var p0 = a;
    let N = 16;
    for (var i = 1; i <= N; i = i + 1) {
        let p1 = art_bez(a, c, b, f32(i) / f32(N));
        let d = p1 - p0;
        let l = max(length(d), 1e-6);
        let t = dot(q - p0, d) / (l * l);
        var tc = clamp(t, 0.0, 1.0);
        if (i == 1) { tc = min(t, 1.0); }
        if (i == N) { tc = max(tc, t); }
        let r = q - (p0 + d * tc);
        let dist = length(r);
        if (dist < best) {
            best = dist;
            bs = acc + tc * l;
            bn = (d.x * r.y - d.y * r.x) / l;
        }
        acc = acc + l;
        p0 = p1;
    }
    return vec3f(bn, bs, acc);
}

// Edge wobble of a stroke side at arc length s: brush bristles (style 0),
// torn paper fibres (1), none (2), soft watercolour blooms (3).
fn art_edge(s: f32, side: f32, w: f32, style: f32, seed: f32) -> f32 {
    let z = vec2f(s / max(w, 0.02), side * 5.3 + seed * 1.7);
    if (style < 0.5) {
        let low = vnoise(z * vec2f(1.1, 1.0)) - 0.5;
        let hi = vnoise(z * vec2f(9.0, 1.0)) - 0.5;
        return 0.7 * low + 0.3 * hi;
    }
    if (style < 1.5) {
        let zz = vec2f(s * 9.0, side * 5.3 + seed * 1.7);
        let low = fbm(zz) - 0.5;
        let fib = vnoise(zz * vec2f(9.0, 1.0)) - 0.5;
        return 0.75 * low + 0.25 * fib;
    }
    if (style < 2.5) { return 0.0; }
    return fbm(z * vec2f(0.8, 1.0)) - 0.5;
}

const ART_STRIDE: i32 = 12;
fn art_rec(i: i32, j: i32) -> vec4f { return ART_S[i * ART_STRIDE + j]; }

// Head position (0..1) of stroke i at progress p: eased over t0..t1, or
// piecewise linear through the knots of s6 when s6.x > 0.
fn art_head(i: i32, p: f32) -> f32 {
    let s2 = art_rec(i, 2);
    let k = art_rec(i, 6);
    if (k.x <= 0.0) { return art_ease((p - s2.x) / max(s2.y - s2.x, 1e-5), s2.w); }
    if (p <= s2.x) { return 0.0; }
    if (p < k.x) { return k.y * art_ease((p - s2.x) / max(k.x - s2.x, 1e-5), s2.w); }
    if (p < k.z) { return mix(k.y, k.w, (p - k.x) / max(k.z - k.x, 1e-5)); }
    return mix(k.w, 1.0, clamp((p - k.z) / max(s2.y - k.z, 1e-5), 0.0, 1.0));
}

// Shape of stroke i at q for progress p.
fn art_hit(i: i32, q: vec2f, p: f32) -> Hit {
    let s0 = art_rec(i, 0);
    let s1 = art_rec(i, 1);
    let s2 = art_rec(i, 2);
    let s3 = art_rec(i, 3);
    let s4 = art_rec(i, 4);
    let s5 = art_rec(i, 5);
    let A = aspect();
    var a = vec2f(s0.x * A, s0.y);
    var c = vec2f(s0.z * A, s0.w);
    var b = vec2f(s1.x * A, s1.y);
    let s8 = art_rec(i, 8);
    let s9 = art_rec(i, 9);
    let s10 = art_rec(i, 10);
    if (s10.w > s10.z) {
        let mk = art_ease((p - s10.z) / (s10.w - s10.z), 1.0);
        a = mix(a, vec2f(s9.x * A, s9.y), mk);
        b = mix(b, vec2f(s9.z * A, s9.w), mk);
        c = mix(c, vec2f(s10.x * A, s10.y), mk);
    }
    let inv = s8.x > 0.5;
    let h = art_head(i, p);
    let kind = s2.z;
    let rag = s3.x;
    let seed = s3.w;
    let style = s4.x;
    var o: Hit;
    o.k = 1.0;
    o.n = 0.0;
    o.s = 0.0;
    if (kind > 0.5 && kind < 1.5) {
        // Blot: radius grows with h; c stretches x / y.
        let st = max(vec2f(s0.z, s0.w), vec2f(0.05));
        let d = (q - a) / st;
        let r = mix(s1.z, s1.w, h);
        let ang = atan2(d.y, d.x);
        let dir = vec2f(cos(ang), sin(ang));
        let e = fbm(dir * 2.2 + vec2f(seed * 3.1, seed * 1.3) + d * 2.0) - 0.5;
        let rr = r * (1.0 + rag * 2.0 * e);
        o.m = (rr - length(d)) * min(st.x, st.y);
        // Swirl coordinates for paint streaks: radius twisted by the angle.
        o.n = length(d) * 3.0 + ang * 0.35;
        o.s = ang * 2.0;
        if (inv) { o.m = -o.m; }
        return o;
    }
    if (kind > 1.5 && kind < 2.5) {
        // Sweep: the edge goes from a to b; covered behind it.
        let dv = b - a;
        let L = max(length(dv), 1e-5);
        let nv = dv / L;
        let pos = a + dv * h;
        let along = dot(q - pos, vec2f(-nv.y, nv.x));
        let e = art_edge(along, 0.0, max(s1.z, 0.01), style, seed);
        let depth = dot(q - pos, nv) - rag * e;
        o.m = -depth;
        // Paint streaks run parallel to the edge.
        o.n = depth * 3.0;
        o.s = along * 0.25;
        if (inv) { o.m = -o.m; }
        return o;
    }
    // Brush along the path.
    let pr = art_path(q, a, c, b);
    let L = pr.z;
    let s = pr.y;
    let tt = clamp(s / max(L, 1e-5), 0.0, 1.0);
    let s7 = art_rec(i, 7);
    var grow = 1.0;
    if (s7.z > s7.y) { grow = mix(1.0, s7.w, art_ease((p - s7.y) / (s7.z - s7.y), 1.0)); }
    let w = mix(s1.z, s1.w, tt) * grow;
    let side = sign(pr.x);
    let e = art_edge(s, side, w, style, seed);
    let wide = w * (1.0 + rag * e);
    let n = pr.x / max(w, 1e-5);
    // Head: bristles of uneven length trail behind the head position.
    let jag = s5.z * w;
    var br = 0.0;
    if (style < 0.5) { br = art_bristle(n, seed); } else { br = fbm(vec2f(n * 3.0, seed)) ; }
    let head = h * (L + jag + w * 0.2) - br * jag;
    var tail = -w * 0.2;
    if (kind > 2.5) {
        // The tail leaves over the second window (s5.w = progress span after t1).
        let ht = art_ease((p - s2.y) / max(s5.w, 1e-5), s2.w);
        tail = ht * (L + jag + w * 0.2) - br * jag;
    }
    o.m = min(min(wide - abs(pr.x), head - s), s - tail);
    if (inv) { o.m = -o.m; }
    o.n = n;
    o.s = s;
    // Dry brush: streak gaps grow toward the edges and along the stroke.
    let dry = s3.y;
    if (dry > 0.0) {
        // Streak bands across the brush (a few wide, many fine), long along it.
        let g = vnoise(vec2f(n * 7.0 + seed * 11.0, s * 0.8 / max(w, 0.02)));
        let g2 = hash12(vec2f(floor(n * 26.0), seed * 5.0 + 3.0));
        var ramp = dry * (0.25 + 0.75 * clamp(s / max(L, 1e-5), 0.0, 1.0)) * (0.5 + 0.5 * n * n);
        // The gaps close (paint soaks in) between t1 and s7.x.
        let close = art_rec(i, 7).x;
        if (close > s2.y) { ramp = ramp * (1.0 - clamp((p - s2.y) / (close - s2.y), 0.0, 1.0)); }
        let x = clamp((0.65 * g + 0.35 * g2 - 0.5) * 2.4 + 0.5, 0.0, 1.0);
        o.k = smoothstep(ramp - 0.04, ramp + 0.04, x);
    }
    return o;
}

// Display colour of paint k for a hit.
fn art_paint(k: i32, q: vec2f, hit: Hit, i: i32) -> vec4f {
    let p0 = ART_P[k * 3];
    let p1 = ART_P[k * 3 + 1];
    let p2 = ART_P[k * 3 + 2];
    let seed = art_rec(i, 3).w;
    // Streaks along the stroke (bristle grooves), broad tone variation.
    let st = vnoise(vec2f(hit.n * 9.0 + seed, hit.s * 1.2)) * 0.6 + hash12(vec2f(floor(hit.n * 40.0), seed)) * 0.4;
    let broad = fbm(q * 2.5 + vec2f(seed, 0.0));
    var t = clamp(broad + p0.w * (st - 0.5), 0.0, 1.0);
    var col = mix(p0.rgb, p1.rgb, t);
    // Sheen: a soft highlight band across the stroke.
    let sh = exp(-pow((hit.n - 0.25) * 2.2, 2.0)) * p1.w;
    col = col + sh * (p1.rgb - col) * 0.8 + vec3f(sh * 0.12);
    // Curl shading across the stroke (paper rolls, folds).
    col = col * (1.0 - p2.z * clamp(0.5 - 0.5 * hit.n, 0.0, 1.0));
    // Wet edge: pigment collects at the boundary.
    col = col * (1.0 - p2.x * exp(-max(hit.m, 0.0) * 120.0));
    // Paper grain.
    col = col * (1.0 - p2.y * (hash12(floor(q * u.size.y)) - 0.5));
    return vec4f(clamp(col, vec3f(0.0), vec3f(1.0)), p2.w);
}

fn art_fx(uv: vec2f) -> vec4f {
    let p = prog();
    let a = to_disp(sa(uv)).rgb;
    let bcol = to_disp(sb(uv)).rgb;
    if (p >= ART_DONE) { return from_disp(vec4f(bcol, 1.0)); }
    // A transition starts exactly on the outgoing clip.
    if (p <= 0.0) { return from_disp(vec4f(a, 1.0)); }
    let q = art_q(uv);
    let px = 1.0 / u.size.y;
    var col = a;
    for (var i = 0; i < ART_N; i = i + 1) {
        let s2 = art_rec(i, 2);
        if (p < s2.x) { continue; }
        let s3 = art_rec(i, 3);
        let s4 = art_rec(i, 4);
        let s5 = art_rec(i, 5);
        let hit = art_hit(i, q, p);
        let aa = px * 0.75 + s5.x;
        var cov = 0.0;
        if (s4.y > 0.0) {
            // Rim just outside the shape.
            let m = -hit.m;
            cov = clamp(0.5 + m / aa, 0.0, 1.0) * clamp(0.5 + (s4.y - m) / aa, 0.0, 1.0);
        } else {
            cov = clamp(0.5 + hit.m / aa, 0.0, 1.0);
        }
        if (cov <= 0.0) { continue; }
        cov = cov * hit.k;
        if (s5.y > 0.0) {
            // Watercolour mottling, clearing as the wash dries in.
            let mo = fbm(q * 7.0 + vec2f(s3.w * 5.0, 0.0));
            cov = cov * clamp(1.0 - s5.y * (1.0 - smoothstep(0.25, 0.75, mo)), 0.0, 1.0);
        }
        if (s4.w > 0.0) {
            cov = cov * mix(s4.z, 1.0, clamp((p - s2.x) / max(s4.w - s2.x, 1e-5), 0.0, 1.0));
        }
        let s8 = art_rec(i, 8);
        if (s8.z > s8.y) {
            cov = cov * (1.0 - clamp((p - s8.y) / (s8.z - s8.y), 0.0, 1.0));
        }
        let f = s3.z;
        var fc = bcol;
        if (f > 0.5 && f < 1.5) { fc = a; }
        if (f > 1.5) {
            var hh = hit;
            if (s4.y > 0.0) {
                hh.n = clamp(1.0 - 2.0 * (-hit.m) / s4.y, -1.0, 1.0);
            }
            let pc = art_paint(i32(f - 1.5), q, hh, i);
            fc = pc.rgb;
            cov = cov * pc.a;
        }
        col = mix(col, fc, cov);
    }
    return from_disp(vec4f(clamp(col, vec3f(0.0), vec3f(1.0)), 1.0));
}

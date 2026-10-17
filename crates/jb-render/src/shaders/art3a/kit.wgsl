// Art kit of the art3a transitions: signed-distance shapes, fills and
// layering for original vector artwork drawn in the shader.
//
// Conventions:
// - Artwork is drawn in "centred" coordinates `c = to_c(uv)`: y spans
//   -0.5..0.5 (down), x spans -aspect/2..aspect/2; lengths are fractions of
//   the frame height, so shapes keep their proportions at any size.
// - Signed distances are negative inside.
// - Colours are display-referred straight RGB (0..1); the item converts its
//   result back with `l3_out`.

// One output pixel in centred units.
fn ak_px() -> f32 { return u.inv_size.y; }
// Anti-aliased coverage of a shape from its signed distance.
fn ak_fill(d: f32) -> f32 { return clamp(0.5 - d / ak_px(), 0.0, 1.0); }
// Coverage with a soft edge of width w (centred units), plus the pixel.
fn ak_soft(d: f32, w: f32) -> f32 { let s = max(w, ak_px()); return clamp(0.5 - d / s, 0.0, 1.0); }
// Coverage of an outline of half-width w around the shape's edge.
fn ak_line(d: f32, w: f32) -> f32 { return ak_fill(abs(d) - w); }
// Paints `col` with opacity `a` over `base`.
fn ak_over(base: vec3f, col: vec3f, a: f32) -> vec3f { return mix(base, col, clamp(a, 0.0, 1.0)); }
// sRGB 0..255 triple to a display colour.
fn ak_rgb(r: f32, g: f32, b: f32) -> vec3f { return vec3f(r, g, b) / 255.0; }

// Point `p` in the frame of a shape placed at `pos`, rotated by `ang`
// (radians, clockwise on screen) and scaled by `s`; distances measured in
// that frame must be multiplied by `s`.
fn ak_local(p: vec2f, pos: vec2f, ang: f32, s: f32) -> vec2f { return rot2(p - pos, -ang) / max(s, 1e-4); }
fn ak_dir(a: f32) -> vec2f { return vec2f(cos(a), sin(a)); }

// --- primitives ---
fn sd_circle(p: vec2f, r: f32) -> f32 { return length(p) - r; }
fn sd_box(p: vec2f, b: vec2f) -> f32 {
    let d = abs(p) - b;
    return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0);
}
fn sd_rbox(p: vec2f, b: vec2f, r: f32) -> f32 { return sd_box(p, b - vec2f(r)) - r; }
fn sd_seg(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-9), 0.0, 1.0);
    return length(pa - ba * h);
}
// Capsule from a to b with radius r.
fn sd_capsule(p: vec2f, a: vec2f, b: vec2f, r: f32) -> f32 { return sd_seg(p, a, b) - r; }
// Tapered capsule: radius ra at a, rb at b.
fn sd_taper(p: vec2f, a: vec2f, b: vec2f, ra: f32, rb: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-9), 0.0, 1.0);
    return length(pa - ba * h) - mix(ra, rb, h);
}
// Ellipse with semi-axes ab (approximate distance, exact sign).
fn sd_ellipse(p: vec2f, ab: vec2f) -> f32 {
    let k0 = length(p / ab);
    let k1 = length(p / (ab * ab));
    return k0 * (k0 - 1.0) / max(k1, 1e-6);
}
fn sd_tri(p: vec2f, p0: vec2f, p1: vec2f, p2: vec2f) -> f32 {
    let e0 = p1 - p0;
    let e1 = p2 - p1;
    let e2 = p0 - p2;
    let v0 = p - p0;
    let v1 = p - p1;
    let v2 = p - p2;
    let pq0 = v0 - e0 * clamp(dot(v0, e0) / dot(e0, e0), 0.0, 1.0);
    let pq1 = v1 - e1 * clamp(dot(v1, e1) / dot(e1, e1), 0.0, 1.0);
    let pq2 = v2 - e2 * clamp(dot(v2, e2) / dot(e2, e2), 0.0, 1.0);
    let s = sign(e0.x * e2.y - e0.y * e2.x);
    let d = min(min(vec2f(dot(pq0, pq0), s * (v0.x * e0.y - v0.y * e0.x)),
                    vec2f(dot(pq1, pq1), s * (v1.x * e1.y - v1.y * e1.x))),
                    vec2f(dot(pq2, pq2), s * (v2.x * e2.y - v2.y * e2.x)));
    return -sqrt(d.x) * sign(d.y);
}
// Regular n-gon of circumradius r, an edge at the top (distance to the
// nearest edge line: exact inside, slightly short outside near corners).
fn sd_ngon(p: vec2f, r: f32, n: f32) -> f32 {
    let an = PI / n;
    let a = atan2(p.x, -p.y);
    let bn = a - 2.0 * an * floor((a + an) / (2.0 * an));
    return length(p) * cos(bn) - r * cos(an);
}
// n-pointed star: outer radius r, inner radius r * m, a point up.
fn sd_star(p0: vec2f, r: f32, n: f32, m: f32) -> f32 {
    let an = PI / n;
    var a = atan2(p0.x, -p0.y);
    a = a - 2.0 * an * floor((a + an) / (2.0 * an));
    let p = length(p0) * vec2f(cos(a), abs(sin(a)));
    let v0 = vec2f(r, 0.0);
    let v1 = vec2f(r * m * cos(an), r * m * sin(an));
    let e = v1 - v0;
    let w = p - v0;
    let h = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    let d = length(w - e * h);
    let s = e.x * w.y - e.y * w.x;
    return select(d, -d, s < 0.0);
}
// Heart of size ~r (point down), centred.
fn sd_heart(p0: vec2f, r: f32) -> f32 {
    var p = vec2f(abs(p0.x), -p0.y) / r + vec2f(0.0, 0.55);
    if (p.y + p.x > 1.0) { return (sqrt(dot(p - vec2f(0.25, 0.75), p - vec2f(0.25, 0.75))) - sqrt(2.0) / 4.0) * r; }
    let q = p - vec2f(0.0, 1.0);
    let t = p - 0.5 * max(p.x + p.y, 0.0);
    let d = min(dot(q, q), dot(t, t));
    return sqrt(d) * sign(p.x - p.y) * r;
}
// Quadratic Bezier curve distance (unsigned).
fn sd_bez(pos: vec2f, A: vec2f, B: vec2f, C: vec2f) -> f32 {
    let a = B - A;
    let b = A - 2.0 * B + C;
    let c = a * 2.0;
    let d = A - pos;
    let bb = max(dot(b, b), 1e-9);
    let kk = 1.0 / bb;
    let kx = kk * dot(a, b);
    let ky = kk * (2.0 * dot(a, a) + dot(d, b)) / 3.0;
    let kz = kk * dot(d, a);
    let p = ky - kx * kx;
    let p3 = p * p * p;
    let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
    var h = q * q + 4.0 * p3;
    var res = 0.0;
    if (h >= 0.0) {
        h = sqrt(h);
        let x = (vec2f(h, -h) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2f(1.0 / 3.0));
        let t = clamp(uv.x + uv.y - kx, 0.0, 1.0);
        let w = d + (c + b * t) * t;
        res = dot(w, w);
    } else {
        let z = sqrt(-p);
        let v = acos(clamp(q / (p * z * 2.0), -1.0, 1.0)) / 3.0;
        let m = cos(v);
        let n = sin(v) * 1.732050808;
        let t = clamp(vec3f(m + m, -n - m, n - m) * z - kx, vec3f(0.0), vec3f(1.0));
        let w1 = d + (c + b * t.x) * t.x;
        let w2 = d + (c + b * t.y) * t.y;
        res = min(dot(w1, w1), dot(w2, w2));
    }
    return sqrt(res);
}
// Signed distance to a closed polygon of n <= 16 points.
fn sd_poly(p: vec2f, v0: array<vec2f, 16>, n: i32) -> f32 {
    var v = v0;
    var d = dot(p - v[0], p - v[0]);
    var s = 1.0;
    var j = n - 1;
    for (var i = 0; i < n; i = i + 1) {
        let e = v[j] - v[i];
        let w = p - v[i];
        let b = w - e * clamp(dot(w, e) / max(dot(e, e), 1e-12), 0.0, 1.0);
        d = min(d, dot(b, b));
        let c0 = p.y >= v[i].y;
        let c1 = p.y < v[j].y;
        let c2 = e.x * w.y > e.y * w.x;
        if ((c0 && c1 && c2) || (!c0 && !c1 && !c2)) { s = -s; }
        j = i;
    }
    return s * sqrt(d);
}

// --- operations ---
fn op_sub(a: f32, b: f32) -> f32 { return max(a, -b); }
fn op_smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}
// Mirror across the vertical axis (symmetric artwork).
fn ak_mx(p: vec2f) -> vec2f { return vec2f(abs(p.x), p.y); }

// --- shading ---
// Parameter of p along the segment a -> b (0 at a, 1 at b), unclamped.
fn ak_along(p: vec2f, a: vec2f, b: vec2f) -> f32 { let ba = b - a; return dot(p - a, ba) / max(dot(ba, ba), 1e-9); }
// Simple light on a rounded shape from its distance: brighter towards the
// edge facing `l` (unit), using the distance gradient by finite difference
// is costly; this approximates it from the position relative to the centre.
fn ak_rim(p: vec2f, centre: vec2f, l: vec2f, r: f32) -> f32 {
    return clamp(dot(normalize(p - centre + vec2f(1e-6)), l) * length(p - centre) / max(r, 1e-4), -1.0, 1.0);
}
// Steps progress to an artwork frame rate (fps) over a transition of `dur` seconds.
fn ak_step(p: f32, fps: f32, dur: f32) -> f32 { let n = max(fps * dur, 1.0); return (floor(p * n) + 0.5) / n; }
// A/B base under the artwork: B where w = 1.
fn ak_base(uv: vec2f, w: f32) -> vec3f {
    if (w <= 0.0) { return l3_da(uv); }
    if (w >= 1.0) { return l3_db(uv); }
    return mix(l3_da(uv), l3_db(uv), w);
}

// Vector-art renderer of the art3b motion-graphics transitions.
//
// `fs_base` draws the background: the outgoing (A) and incoming (B) clips,
// each through a camera (zoom, shift, rotation), mixed in display space and
// optionally dipped to a colour. `vs_prim` / `fs_prim` then draw up to six
// primitives per call as bounding quads blended "over" (premultiplied) on
// top, in order, so later primitives cover earlier ones.
//
// Coordinates are centred and aspect-correct (`to_c`): y spans -0.5..0.5
// (down), x spans -aspect/2..aspect/2. Colours are display-referred sRGB.
//
// Primitive i occupies u.p[5i .. 5i+4]:
//   v0 = (kind + stroke flag, a, b, c)   shape parameters
//   v1 = (cx, cy, rot, soft)              centre, rotation (rad), edge softness
//   v2 = (r, g, b, a)                     colour (or tint of a texture fill)
//   v3 = (fill, f1, f2, f3)               fill mode and its parameters
//   v4 = (stroke, seed, d, e)             outline width, noise seed, extras
// Quads (kind 9) store their inverse homography in v0.yzw, v1.xyz, v3.yzw.

const NPRIM: u32 = 6u;

fn pv(i: u32, k: u32) -> vec4f { return u.p[i * 5u + k]; }

struct PrimOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) @interpolate(flat) idx: u32,
};

// Bounding radius about the centre (fraction of height); < 0: full frame.
fn bound(i: u32) -> f32 {
    let v0 = pv(i, 0u);
    let v1 = pv(i, 1u);
    let v4 = pv(i, 4u);
    let kind = i32(v0.x);
    let pad = 3.0 * v1.w + v4.x + 4.0 * u.inv_size.y;
    switch kind {
        case 1: { return length(v0.yz) + pad; }
        case 2: { return max(v0.y, v0.z) + pad; }
        case 3, 4, 5: { return v0.y * 1.6 + pad; }
        case 6: { return v0.y * (1.0 + v0.z) + pad; }
        case 7: { return v0.y + v0.z * max(1.0, v0.w) + pad; }
        case 10: { return v0.y * 1.5 + pad; }
        case 11: { return v0.y + v0.z + pad; }
        case 13: { return max(v0.y, v0.z) + pad; }
        default: { return -1.0; }
    }
}

@vertex
fn vs_prim(@builtin(vertex_index) vi: u32) -> PrimOut {
    let i = vi / 6u;
    let j = vi % 6u;
    var corner = vec2f(0.0, 0.0);
    switch j {
        case 1u, 3u: { corner = vec2f(1.0, 0.0); }
        case 2u, 5u: { corner = vec2f(0.0, 1.0); }
        case 4u: { corner = vec2f(1.0, 1.0); }
        default: { corner = vec2f(0.0, 0.0); }
    }
    let r = bound(i);
    var uv = corner;
    if (r > 0.0) {
        let c = pv(i, 1u).xy;
        uv = from_c(c + (corner * 2.0 - 1.0) * r);
    }
    var o: PrimOut;
    o.pos = vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    o.idx = i;
    return o;
}

fn sd_box(p: vec2f, b: vec2f, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sd_ellipse(p: vec2f, ab: vec2f) -> f32 {
    // First-order distance estimate (exact for circles).
    let k0 = length(p / ab);
    let k1 = length(p / (ab * ab));
    if (k1 < 1e-6) { return -min(ab.x, ab.y); }
    return k0 * (k0 - 1.0) / k1;
}

// Distance to the segment a-b and the side of it (sign of the cross product).
fn sd_seg_signed(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    let d = length(pa - ba * h);
    let s = ba.x * pa.y - ba.y * pa.x;
    return select(-d, d, s > 0.0);
}

// Star with n points of outer radius r and inner radius r*m (m = cos(pi/n):
// regular polygon). `jit` > 0 shortens each point by a random fraction.
fn sd_star(p: vec2f, r: f32, n: f32, m: f32, jit: f32, seed: f32) -> f32 {
    let an = PI / n;
    let a = atan2(p.y, p.x);
    let k = floor((a + an) / (2.0 * an));
    let fa = abs(a - k * 2.0 * an);
    var rr = r;
    if (jit > 0.0) {
        let kk = k - n * floor(k / n);
        rr = r * (1.0 - jit * hash11(kk * 1.37 + seed));
    }
    let q = length(p) * vec2f(cos(fa), sin(fa));
    let p0 = vec2f(rr, 0.0);
    let p1 = r * m * vec2f(cos(an), sin(an));
    return -sd_seg_signed(q, p0, p1);
}

fn sd_heart(p0: vec2f, s: f32) -> f32 {
    // Unit heart (top at y = -1.1, point at y = +1) scaled by s.
    var p = vec2f(abs(p0.x), -p0.y) / s;
    p.y = p.y + 0.55;
    p = p * 0.62;
    var d = 0.0;
    if (p.y + p.x > 1.0) {
        d = sqrt(dot(p - vec2f(0.25, 0.75), p - vec2f(0.25, 0.75))) - sqrt(2.0) / 4.0;
    } else {
        let a = p - vec2f(0.0, 1.0);
        let b = p - 0.5 * max(p.x + p.y, 0.0);
        d = sqrt(min(dot(a, a), dot(b, b))) * sign(p.x - p.y);
    }
    return d * s / 0.62;
}

// Rhombus with half diagonals b (exact distance).
fn sd_rhombus(p0: vec2f, b: vec2f) -> f32 {
    let p = abs(p0);
    let h = clamp(((b.x - 2.0 * p.x) * b.x - (b.y - 2.0 * p.y) * b.y) / dot(b, b), -1.0, 1.0);
    let d = length(p - 0.5 * b * vec2f(1.0 - h, 1.0 + h));
    return d * sign(p.x * b.y + p.y * b.x - b.x * b.y);
}

// Signed distance of primitive i at local point l (and the screen point p).
fn sdf(i: u32, l: vec2f) -> f32 {
    let v0 = pv(i, 0u);
    let v4 = pv(i, 4u);
    let kind = i32(v0.x);
    switch kind {
        case 1: { return sd_box(l, v0.yz, min(v0.w, min(v0.y, v0.z))); }
        case 2: { return sd_ellipse(l, max(v0.yz, vec2f(1e-5))); }
        case 3: { return sd_star(l, v0.y, max(v0.z, 2.0), v0.w, 0.0, 0.0); }
        case 4: { return sd_star(l, v0.y, max(v0.z, 2.0), v0.w, v4.z, v4.y); }
        case 5: {
            // Burst with spikes of random length and a wobbly core.
            return sd_star(l, v0.y, max(v0.z, 2.0), v0.w, v4.z, v4.y);
        }
        case 6: {
            // Blob: circle of radius a with a noisy outline (amplitude b, frequency c).
            let r = length(l);
            let dir = l / max(r, 1e-6);
            let n = fbm(dir * v0.w + vec2f(v4.y * 7.13, v4.y * 3.71)) - 0.5;
            return r - v0.y * (1.0 + 2.0 * v0.z * n);
        }
        case 7: {
            // Tapered capsule along x: half length a, radius b at -a, b*c at +a.
            let h = clamp((l.x + v0.y) / max(2.0 * v0.y, 1e-6), 0.0, 1.0);
            let rr = v0.z * mix(1.0, v0.w, h);
            return length(vec2f(l.x - clamp(l.x, -v0.y, v0.y), l.y)) - rr;
        }
        case 8: { return l.x; }
        case 10: { return sd_heart(l, v0.y); }
        case 11: {
            // Arc: ring of radius a, thickness b, half-angle c about +x.
            let ang = abs(atan2(l.y, l.x));
            let ring = abs(length(l) - v0.y) - 0.5 * v0.z;
            let ca = vec2f(cos(v0.w), sin(v0.w)) * v0.y;
            let cap = length(vec2f(l.x, abs(l.y)) - ca) - 0.5 * v0.z;
            return select(cap, ring, ang < v0.w);
        }
        case 12: {
            // Chevron pointing to +x: tip at the origin, horizontal thickness a,
            // slope b (x drops by b per unit of |y|), clipped to |y| < c.
            let k = sqrt(1.0 + v0.z * v0.z);
            let e = (l.x + v0.z * abs(l.y)) / k;
            return max(max(e, -e - v0.y / k), abs(l.y) - v0.w);
        }
        case 13: { return sd_rhombus(l, max(v0.yz, vec2f(1e-5))); }
        default: { return 1.0; }
    }
}

fn fill_color(i: u32, l: vec2f, tuv: vec2f, use_uv: bool) -> vec4f {
    let v2 = pv(i, 2u);
    let v3 = pv(i, 3u);
    let v4 = pv(i, 4u);
    let mode = i32(v3.x) % 10;
    // Texture fills (working space, premultiplied) tinted by v2.rgb:
    // 1/2 pieces of A/B moving with the shape, 6/7 windows onto A/B.
    if (mode == 6 || mode == 7) {
        let s = select(sa(tuv), sb(tuv), mode == 7);
        return vec4f(s.rgb * v2.rgb, s.a);
    }
    if (mode == 1 || mode == 2) {
        var q = tuv;
        if (!use_uv) {
            q = from_c(v3.zw + l / max(v3.y, 1e-4));
        }
        var s = select(sa(q), sb(q), mode == 2);
        if (!in01(q)) { s = vec4f(0.0); }
        return vec4f(s.rgb * v2.rgb, s.a);
    }
    var c = v2.rgb;
    if (mode == 3 || mode == 8) {
        // Linear gradient along local y (3) or x (8) from v4.z (colour) to
        // v4.w (colour 2).
        let x = select(l.y, l.x, mode == 8);
        let t = clamp((x - v4.z) / max(v4.w - v4.z, 1e-5), 0.0, 1.0);
        c = mix(v2.rgb, v3.yzw, t);
    } else if (mode == 4) {
        // Radial gradient: colour 2 at the centre to colour at radius v4.z.
        let t = clamp(length(l) / max(v4.z, 1e-5), 0.0, 1.0);
        c = mix(v3.yzw, v2.rgb, t);
    } else if (mode == 5) {
        // Noise texture between colour and colour 2 (frequency v4.z).
        let n = fbm(l * v4.z + vec2f(v4.y * 5.3, v4.w));
        c = mix(v2.rgb, v3.yzw, smoothstep(0.3, 0.7, n));
    }
    return from_disp(vec4f(c, 1.0));
}

@fragment
fn fs_prim(in: PrimOut) -> @location(0) vec4f {
    let i = in.idx;
    let p = to_c(in.uv);
    let v0 = pv(i, 0u);
    let v1 = pv(i, 1u);
    let v2 = pv(i, 2u);
    let v3 = pv(i, 3u);
    let v4 = pv(i, 4u);
    let kind = i32(v0.x);
    let aa = 0.75 * u.inv_size.y;
    var d = 1.0;
    var l = vec2f(0.0);
    var tuv = vec2f(0.0);
    var use_uv = false;
    if (kind == 9) {
        // Quad: inverse homography screen -> unit square; the distance to
        // the square's border is converted to screen units with the
        // analytic Jacobian (no derivatives: the branch is non-uniform).
        let h = vec3f(p, 1.0);
        let x = dot(v0.yzw, h);
        let y = dot(v1.xyz, h);
        let w = dot(v3.yzw, h);
        if (w <= 1e-6) { discard; }
        let s = vec2f(x, y) / w;
        let gx = length((v0.yz * w - x * v3.yz) / (w * w));
        let gy = length((v1.xy * w - y * v3.yz) / (w * w));
        let ex = min(s.x, 1.0 - s.x) / max(gx, 1e-6);
        let ey = min(s.y, 1.0 - s.y) / max(gy, 1e-6);
        d = -min(ex, ey);
        // Texture sub-rectangle: origin v4.yz, size (v4.x, v4.w).
        tuv = v4.yz + s * vec2f(v4.x, v4.w);
        l = (s - 0.5) * vec2f(aspect(), 1.0);
        use_uv = true;
    } else {
        l = rot2(p - v1.xy, -v1.z);
        d = sdf(i, l);
        if (v4.x > 0.0) { d = abs(d) - 0.5 * v4.x; }
    }
    let soft = max(v1.w, 0.0);
    var cov = 0.0;
    if (kind == 9) {
        cov = clamp(0.5 - d / max(aa * 1.33, 1e-6), 0.0, 1.0);
    } else if (soft > aa) {
        cov = 1.0 - smoothstep(-soft, soft, d);
    } else {
        cov = clamp(0.5 - d / (2.0 * aa), 0.0, 1.0);
    }
    if (cov <= 0.0) { discard; }
    let mode = i32(v3.x) % 10;
    if (mode == 6 || mode == 7) { tuv = in.uv; }
    let c = fill_color(i, l, tuv, use_uv);
    let a = cov * v2.a;
    if (v3.x >= 10.0) {
        // Additive light.
        return vec4f(c.rgb * a, 0.0);
    }
    return c * a;
}

// Background: A and B through their cameras, mixed in display space.
fn cam_uv(uv: vec2f, c: vec4f) -> vec2f {
    let q = rot2(to_c(uv) - c.yz, -c.w) / max(c.x, 1e-4);
    return from_c(q);
}

@fragment
fn fs_base(in: VsOut) -> @location(0) vec4f {
    let qa = cam_uv(in.uv, u.e0);
    let qb = cam_uv(in.uv, u.e1);
    var a = to_disp(sa(qa));
    var b = to_disp(sb(qb));
    if (u.e2.w > 0.5) {
        if (!in01(qa)) { a = vec4f(0.0, 0.0, 0.0, 1.0); }
        if (!in01(qb)) { b = vec4f(0.0, 0.0, 0.0, 1.0); }
    }
    var c = mix(a.rgb * u.e2.y, b.rgb * u.e2.z, u.e2.x);
    c = mix(c, u.e3.rgb, u.e3.a);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), 1.0));
}

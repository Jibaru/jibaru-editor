// Table-driven camera transition (lane1_table.rs).
//
// Each side S (A = outgoing, B = incoming) is the clip seen through a camera:
//   P  = (uv - 0.5) * (aspect, 1)                     centred, height units
//   P' = P / (1 + px * P.x + py * P.y)                 optional perspective
//   P' *= exp(dk * (1 - 4 |P'|^2))                     radial bulge (< 0) / tunnel (> 0)
//   P' = R(sw * exp(-|P'|^2 / 0.15)) P'                swirl about the centre
//   Ps = (tx, ty) + k * R(phi) * P'                    source point
// blurred over taps t in [-1, 1] that rotate Ps by rb * t, scale it by
// exp(zb * t) and shift it by t * (bx, by) (spin, zoom and motion blur at once).
// Edge mode: 0 = transparent (black), 1 = clamp, 2 = mirrored repeat.
// The output is a per-channel display-space mix al * A' + be * B' + ga.
//
// Uniforms (u.p): [0] A (tx, ty, k, phi), [1] A (bx, by, zb, rb), [2] A (px, py, dk, sw),
// [3..5] the same for B, [6] al.rgb, [7] be.rgb, [8] ga.rgb, [9] (edge mode, layer order).

const NTAP: i32 = 24;

fn mirror1(x: f32) -> f32 {
    let f = x - 2.0 * floor(x * 0.5);
    return select(f, 2.0 - f, f > 1.0);
}

fn edge_sample(use_b: bool, q: vec2f, mode: i32) -> vec4f {
    var s = q;
    if (mode == 2) { s = vec2f(mirror1(q.x), mirror1(q.y)); }
    let c = select(sa(s), sb(s), use_b);
    if (mode == 0 && !in01(q)) { return vec4f(0.0); }
    return c;
}

// Display-space colour premultiplied by coverage, and the coverage.
fn tap(use_b: bool, q: vec2f, mode: i32) -> vec4f {
    let c = edge_sample(use_b, q, mode);
    return vec4f(to_disp(c).rgb * c.a, c.a);
}

fn side(use_b: bool, uv: vec2f, geo: vec4f, blur: vec4f, warp: vec4f, mode: i32) -> vec4f {
    let asp = aspect();
    var p = (uv - 0.5) * vec2f(asp, 1.0);
    let w = max(1.0 + warp.x * p.x + warp.y * p.y, 1e-3);
    p = p / w;
    let r2 = dot(p, p);
    if (warp.z != 0.0) { p = p * exp(warp.z * (1.0 - 4.0 * r2)); }
    if (warp.w != 0.0) {
        let a = warp.w * exp(-r2 / 0.15);
        p = vec2f(cos(a) * p.x - sin(a) * p.y, sin(a) * p.x + cos(a) * p.y);
    }
    let c = cos(geo.w);
    let s = sin(geo.w);
    let ps = geo.xy + geo.z * vec2f(c * p.x - s * p.y, s * p.x + c * p.y);
    let blurred = abs(blur.x) + abs(blur.y) + abs(blur.z) + abs(blur.w) > 1e-5;
    if (!blurred) {
        return tap(use_b, ps / vec2f(asp, 1.0) + 0.5, mode);
    }
    var acc = vec4f(0.0);
    for (var i = 0; i < NTAP; i = i + 1) {
        let t = (f32(i) + 0.5) / f32(NTAP) * 2.0 - 1.0;
        let cr = cos(blur.w * t);
        let sr = sin(blur.w * t);
        let e = exp(blur.z * t);
        let q = vec2f(cr * ps.x - sr * ps.y, sr * ps.x + cr * ps.y) * e + t * blur.xy;
        acc = acc + tap(use_b, q / vec2f(asp, 1.0) + 0.5, mode);
    }
    return acc / f32(NTAP);
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let mode = i32(u.p[9].x);
    let order = i32(u.p[9].y);
    var a = vec4f(0.0);
    var b = vec4f(0.0);
    if (any(u.p[6].rgb != vec3f(0.0))) { a = side(false, in.uv, u.p[0], u.p[1], u.p[2], mode); }
    if (any(u.p[7].rgb != vec3f(0.0))) { b = side(true, in.uv, u.p[3], u.p[4], u.p[5], mode); }
    // Layer order: 0 sum, 1 A over B, 2 B over A.
    if (order == 1) { b = b * (1.0 - a.a); }
    if (order == 2) { a = a * (1.0 - b.a); }
    let rgb = u.p[8].rgb + u.p[6].rgb * a.rgb + u.p[7].rgb * b.rgb;
    return from_disp(vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), 1.0));
}

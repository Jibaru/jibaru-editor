// Layered corner-pin renderer (lane 2 transitions): see lane2_layers.rs.
//
// One pass draws up to 3 layers over the background (e0.rgb) or over the
// previous pass (`tx`, when e1.x > 0). Each layer occupies 9 slots of u.p:
//   0: corners 0, 1 (screen uv)       1: corners 2, 3
//   2: blur displacement of corners 0, 1 over the shutter
//   3: blur displacement of corners 2, 3
//   4: gain rgb, opacity              5: gain d/du rgb, source (0 A, 1 B, 2 solid)
//   6: gain d/dv rgb, edge (0 clip, 1 mirror, 2 wrap)
//   7: offset rgb (display 0..1), blur taps
//   8: source rect u0 v0 u1 v1
//  10: lens: radial distortion k1, k2 of the screen point before the pin
//      (p -> c + (p - c)(1 + k1 r^2 + k2 r^4), r in frame heights from the centre), saturation
//   9: strip mask in source uv: lo <= dot(n, q) <= hi (n = xy, lo = z, hi = w); |n| is
//      1 / feather, so the edges ramp over one unit of dot(n, q) (or one pixel if wider)
// Corners are where the source rect's corners (TL, TR, BR, BL) land; the
// clip is mapped with a bilinear corner pin (what Filmora's 3D transitions
// measure as), sampled 2x2 per pixel. Compositing is in display space.

// Division-safe denominator (Metal compiles with fast math: no NaN checks).
fn safe(x: f32) -> f32 { return select(-1e-12, 1e-12, x >= 0.0) + x; }

fn cross2(a: vec2f, b: vec2f) -> f32 { return a.x * b.y - a.y * b.x; }

// Bilinear corner pin inverse: (u, v) such that the pin maps it to `p`.
fn inv_bilinear(p: vec2f, a: vec2f, b: vec2f, c: vec2f, d: vec2f) -> vec2f {
    let e = b - a;
    let f = d - a;
    let g = a - b + c - d;
    let h = p - a;
    let k2 = cross2(g, f);
    let k1 = cross2(e, f) + cross2(h, g);
    let k0 = cross2(h, e);
    // Numerically stable roots (k2 is tiny for near-affine pins; the textbook
    // formula cancels catastrophically in f32 there).
    let w = sqrt(max(k1 * k1 - 4.0 * k0 * k2, 0.0));
    let q = -0.5 * (k1 + select(-w, w, k1 >= 0.0));
    let v1 = k0 / safe(q);
    var v = v1;
    if (abs(k2) > 1e-12) {
        let v2 = q / k2;
        v = select(v2, v1, abs(v1 - 0.5) <= abs(v2 - 0.5));
    }
    let dx = e.x + g.x * v;
    let dy = e.y + g.y * v;
    var u: f32;
    if (abs(dx) > abs(dy)) { u = (h.x - f.x * v) / safe(dx); } else { u = (h.y - f.y * v) / safe(dy); }
    return vec2f(u, v);
}

// Perspective pin inverse (Heckbert's square-to-quad, inverted by adjugate).
fn inv_homography(p: vec2f, p0: vec2f, p1: vec2f, p2: vec2f, p3: vec2f) -> vec2f {
    let d1 = p1 - p2;
    let d2 = p3 - p2;
    let d3 = p0 - p1 + p2 - p3;
    let det = cross2(d1, d2);
    let g = cross2(d3, d2) / safe(det);
    let h = cross2(d1, d3) / safe(det);
    let a = p1.x - p0.x + g * p1.x;
    let b = p3.x - p0.x + h * p3.x;
    let d = p1.y - p0.y + g * p1.y;
    let e = p3.y - p0.y + h * p3.y;
    let c = p0.x;
    let f = p0.y;
    // [u v w] = adj(M) [x y 1], M = [[a b c] [d e f] [g h 1]]
    let x = p.x;
    let y = p.y;
    let u = (e - f * h) * x + (c * h - b) * y + (b * f - c * e);
    let v = (f * g - d) * x + (a - c * g) * y + (c * d - a * f);
    let w = (d * h - e * g) * x + (b * g - a * h) * y + (a * e - b * d);
    return vec2f(u, v) / safe(w);
}

fn pin_inv(persp: bool, p: vec2f, a: vec2f, b: vec2f, c: vec2f, d: vec2f) -> vec2f {
    if (persp) { return inv_homography(p, a, b, c, d); }
    return inv_bilinear(p, a, b, c, d);
}

fn fwd_bilinear(st: vec2f, a: vec2f, b: vec2f, c: vec2f, d: vec2f) -> vec2f {
    return mix(mix(a, b, st.x), mix(d, c, st.x), st.y);
}

fn l2_mirror(x: vec2f) -> vec2f {
    let f = x - 2.0 * floor(x * 0.5);
    return select(f, 2.0 - f, f > vec2f(1.0));
}

fn slot(l: i32, k: i32) -> vec4f { return u.p[l * 11 + k]; }

fn lens(l: i32, p: vec2f) -> vec2f {
    let k = slot(l, 10);
    if (k.x == 0.0 && k.y == 0.0) { return p; }
    let d = (p - 0.5) * vec2f(aspect(), 1.0);
    let r2 = dot(d, d);
    return 0.5 + (p - 0.5) * (1.0 + k.x * r2 + k.y * r2 * r2);
}

fn l2_src(s: f32, q: vec2f) -> vec3f {
    if (s < 0.5) { return to_disp(sa(q)).rgb; }
    if (s < 1.5) { return to_disp(sb(q)).rgb; }
    return vec3f(1.0);
}

// Display-space color and coverage of layer l at screen uv `p` (one subsample, one tap).
fn l2_layer(l: i32, p: vec2f, t: f32, fw: vec2f) -> vec4f {
    let c01 = slot(l, 0) + slot(l, 2) * t;
    let c23 = slot(l, 1) + slot(l, 3) * t;
    let persp = slot(l, 6).w > 3.5;
    let st = pin_inv(persp, lens(l, p), c01.xy, c01.zw, c23.xy, c23.zw);
    // Degenerate pins give NaN/inf: draw nothing there (NaN would poison the blend).
    if (!(all(abs(st) < vec2f(1e6)))) { return vec4f(0.0); }
    let rect = slot(l, 8);
    let mode = slot(l, 6).w - select(0.0, 4.0, persp);
    var q = rect.xy + st * (rect.zw - rect.xy);
    var cov = 1.0;
    if (mode > 1.5) {
        q = fract(q);
    } else if (mode > 0.5) {
        q = l2_mirror(q);
    } else {
        let e = min(st, 1.0 - st) / max(fw, vec2f(1e-6));
        let k = clamp(0.5 + e, vec2f(0.0), vec2f(1.0));
        cov = k.x * k.y;
        // Strip clip (pieces cut along parallel lines), antialiased over the footprint.
        let sp = slot(l, 9);
        if (sp.x != 0.0 || sp.y != 0.0) {
            let dn = dot(sp.xy, q);
            let fwn = max(abs(sp.x) * fw.x * (rect.z - rect.x) + abs(sp.y) * fw.y * (rect.w - rect.y), 1.0);
            cov = cov * clamp(0.5 + (dn - sp.z) / fwn, 0.0, 1.0) * clamp(0.5 + (sp.w - dn) / fwn, 0.0, 1.0);
        }
        // Degenerate pins (slivers) can invert to a wrong root: reject points the pin does not map back.
        if (!persp) {
            let back = fwd_bilinear(clamp(st, vec2f(-0.5), vec2f(1.5)), c01.xy, c01.zw, c23.xy, c23.zw);
            if (any(abs(back - lens(l, p)) > 2.0 * u.inv_size)) { cov = 0.0; }
        }
    }
    let g = slot(l, 4);
    let gu = slot(l, 5);
    let gv = slot(l, 6);
    let o = slot(l, 7);
    var s = l2_src(gu.w, q);
    let sat = slot(l, 10).z;
    if (sat != 1.0) { s = mix(vec3f(dot(s, LUMA)), s, sat); }
    let col = s * (g.rgb + gu.rgb * (st.x - 0.5) + gv.rgb * (st.y - 0.5)) + o.rgb;
    return vec4f(col, cov * g.w);
}

fn fx(uv: vec2f) -> vec4f {
    var acc = u.e0.rgb;
    if (u.e1.x > 0.5) { acc = to_disp(sx(uv)).rgb; }
    let n = i32(u.e0.w);
    let px = u.inv_size;
    for (var l = 0; l < n; l = l + 1) {
        // Footprint of half a pixel in source uv (subsample spacing), for edge antialiasing.
        let c01 = slot(l, 0);
        let c23 = slot(l, 1);
        let persp = slot(l, 6).w > 3.5;
        let s0 = pin_inv(persp, lens(l, uv), c01.xy, c01.zw, c23.xy, c23.zw);
        let sx1 = pin_inv(persp, lens(l, uv + vec2f(px.x, 0.0)), c01.xy, c01.zw, c23.xy, c23.zw);
        let sy1 = pin_inv(persp, lens(l, uv + vec2f(0.0, px.y)), c01.xy, c01.zw, c23.xy, c23.zw);
        var fw = 0.5 * (abs(sx1 - s0) + abs(sy1 - s0));
        if (!(all(fw < vec2f(1e6)))) { fw = vec2f(1e6); }
        let taps = max(i32(slot(l, 7).w), 1);
        var lc = vec3f(0.0);
        var la = 0.0;
        for (var t = 0; t < taps; t = t + 1) {
            var tt = 0.0;
            if (taps > 1) { tt = (f32(t) + 0.5) / f32(taps) - 0.5; }
            for (var k = 0; k < 4; k = k + 1) {
                let o = (vec2f(f32(k & 1), f32(k >> 1u)) - 0.5) * 0.5 * px;
                let r = l2_layer(l, uv + o, tt, fw);
                lc = lc + r.rgb * r.a;
                la = la + r.a;
            }
        }
        let m = 1.0 / f32(4 * taps);
        acc = lc * m + acc * (1.0 - la * m);
    }
    return from_disp(vec4f(clamp(acc, vec3f(0.0), vec3f(1.0)), 1.0));
}

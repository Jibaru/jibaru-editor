// Warped prism (see PRISM in fx/art4_models.rs): a box whose faces carry
// the clips is ray-cast in perspective while it turns about one axis.
// Canonical frame: X across the turn, Y along the axis, Z into the screen.

struct Hit { t: f32, face: i32, s: f32 };

// First hit of a 2D ray (o + t d) with the 4 faces of a square prism of
// half-size w whose faces bulge outward by a sagitta b (b >= 0).
fn prism_hit(o: vec2f, d: vec2f, w: f32, b: f32) -> Hit {
    var best = Hit(1e9, -1, 0.0);
    for (var j = 0; j < 4; j++) {
        let a = f32(j) * 0.5 * PI;
        let n2 = vec2f(sin(a), -cos(a));
        let tg = vec2f(cos(a), sin(a));
        var t = 1e9;
        if (b < 1e-4) {
            let dn = dot(d, n2);
            if (dn < -1e-6) {
                let tt = (w - dot(o, n2)) / dn;
                let hp = o + tt * d;
                if (tt > 0.0 && abs(dot(hp, tg)) <= w) { t = tt; }
            }
        } else {
            let r = (w * w + b * b) / (2.0 * b);
            let cc = n2 * (w + b - r);
            let oc = o - cc;
            let qa = dot(d, d);
            let qb = dot(oc, d);
            let qc = dot(oc, oc) - r * r;
            let disc = qb * qb - qa * qc;
            if (disc >= 0.0) {
                let sq = sqrt(disc);
                let t0 = (-qb - sq) / qa;
                let hp = o + t0 * d;
                if (t0 > 0.0 && dot(hp, n2) >= w - 1e-4 && abs(dot(hp, tg)) <= w) { t = t0; }
            }
        }
        if (t < best.t) {
            let hp = o + t * d;
            best = Hit(t, j, dot(hp, tg));
        }
    }
    return best;
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let vert = c(0) < 0.5;
    let asp = aspect();
    // Screen point in half-height units, axis-swapped for horizontal axes.
    var sp = (in.uv - 0.5) * 2.0 * vec2f(asp, 1.0);
    var w = asp;
    var l = 1.0;
    if (!vert) { sp = sp.yx; w = 1.0; l = asp; }
    // Screen-space radial warp k8 (> 0 barrel, < 0 pincushion).
    sp = sp * (1.0 + k(8) * dot(sp, sp) / (w * w + l * l));
    let zoom = max(k(3), 1e-3);
    let p = sp / zoom - vec2f(k(6), k(7));
    let dist = max(c(1), 0.2);
    // Ray from the camera; tilt k4 turns it about the across axis.
    var d = vec3f(p.x, p.y, dist);
    let ct = cos(k(4));
    let st = sin(k(4));
    d = vec3f(d.x, d.y * ct - d.z * st, d.y * st + d.z * ct);
    // Camera dolly k10 (half-heights) moves the eye back along the view axis.
    let o3 = vec3f(0.0, 0.0, -dist - w - k(10));
    // Turn angle at this height: base, twist and bend along the axis.
    let yy = p.y / l;
    let th = k(0) + k(1) * yy + k(5) * yy * yy;
    let o2 = rot2(o3.xz, -th);
    let d2 = rot2(d.xz, -th);
    var h = prism_hit(o2, d2, w, max(k(2), 0.0) * w);
    if (h.face < 0) { return vec4f(0.0); }
    var yh = o3.y + h.t * d.y;
    // Cask: the box is wider in the middle by k9 (refined once from the hit).
    var wc = w;
    if (abs(k(9)) > 1e-5) {
        let q = clamp(yh / l, -1.0, 1.0);
        wc = w * (1.0 + k(9) * (1.0 - q * q));
        h = prism_hit(o2, d2, wc, max(k(2), 0.0) * wc);
        if (h.face < 0) { return vec4f(0.0); }
        yh = o3.y + h.t * d.y;
    }
    if (abs(yh) > l) { return vec4f(0.0); }
    var uvf = vec2f((h.s / wc + 1.0) * 0.5, (yh / l + 1.0) * 0.5);
    if (c(2) > 0.5) { uvf.x = 1.0 - uvf.x; }
    if (!vert) { uvf = uvf.yx; }
    let even = (h.face % 2) == 0;
    var col = select(sb(uvf), sa(uvf), even);
    if (c(3) > 0.5 && h.face == 2) { col = sb(uvf); }
    return col;
}

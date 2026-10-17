// Cyan Comet Glitter: a white-hot comet crosses from the left edge to the upper
// right, laying a broad cyan glow band; the new clip appears along the band in
// the order the comet passed while glitter and sparkles drift and fade.
//@p0 band opacity, reveal, glitter, head
//@ 0.000 0.00 0.00 0.0 0.0
//@ 0.040 0.50 0.00 0.1 1.0
//@ 0.090 0.95 0.00 0.15 1.0
//@ 0.230 0.95 0.00 0.3 1.0
//@ 0.250 0.95 0.00 0.5 0.0
//@ 0.260 0.9 0.02 1.0 0.0
//@ 0.300 0.80 0.15 1.0 0.0
//@ 0.370 0.45 0.55 0.9 0.0
//@ 0.450 0.15 0.80 0.8 0.0
//@ 0.600 0.03 0.90 0.6 0.0
//@ 0.750 0.00 0.95 0.3 0.0
//@ 0.950 0.00 1.00 0.0 0.0
//@p2 cyan haze over the whole frame
//@ 0.00 0.0
//@ 0.25 0.0
//@ 0.45 1.0
//@ 0.50 0.72
//@ 0.60 0.66
//@ 0.67 0.56
//@ 0.725 0.5
//@ 0.725 0.0
//@p16 path points
//@ 0 -1.05 0.2 -0.406 0.059
//@p17
//@ 0 0.269 -0.10 0.98 -0.43
//@p1 band offset x, y
//@ 0.00 0.0 0.02
fn comet() -> Comet {
    return Comet(16, 4, 0.03, 0.225, 1.0);
}

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let k = u.p[0];
    let c = comet();
    let p = to_c(uv);
    let sh = a1_comet_s(c, t);
    let near = a1_comet_near(c, p, max(sh, 1e-3));
    // the band hangs below the path
    let nb = a1_comet_near(c, p - u.p[1].xy, max(sh, 1e-3));
    // the clip switch follows the path: early parts of the band reveal first
    let w = smoothstep(nb.y * 0.45, nb.y * 0.45 + 0.55, k.y);
    var col = mix(a1_da(uv), a1_db(uv), w);
    // glow band, thicker behind the head
    // the wake widens and drifts up-right as it ages
    let age0 = max(t - c.t1, 0.0) * 2.0;
    let drift = vec2f(0.7, -2.0);
    let nb2 = a1_comet_near(c, p - u.p[1].xy - drift * age0, max(sh, 1e-3));
    let age = max(sh - nb2.y, 0.0) * 1.5 + age0;
    let band = k.x * a1_gauss(nb2.x, 0.2 + 0.15 * age) * smoothstep(0.0, 0.08, nb2.y + 0.02);
    col = mix(col, vec3f(0.28, 0.89, 0.76), clamp(band, 0.0, 1.0));
    col = a1_add(col, vec3f(0.25, 0.6, 0.55) * k.x * a1_gauss(near.x, 0.07));
    col = col + vec3f(0.005, 0.125, 0.035) * u.p[2].x;
    // head
    let hp = a1_comet_at(c, sh);
    let hd = length(p - hp);
    col = a1_add(col, k.w * (vec3f(1.0) * a1_gauss(hd, 0.035) + vec3f(0.4, 1.0, 0.9) * 0.6 * a1_halo(hd, 0.06)));
    // glitter
    let g = a1_comet_glitter(c, p, t, 200, 1.0, 0.55, 0.3, vec2f(0.3, -0.4), 0.9, 0.2, 70);
    col = a1_add(col, k.z * (vec3f(1.0, 1.0, 0.95) * g.x + vec3f(0.2, 1.0, 0.95) * 1.6 * g.y));
    return a1_out(col);
}

// Twin Comet Burst: two golden comets race along the diagonal from opposite
// corners, cross mid-frame and burst into a shower of amber bokeh that clears
// on the new clip.
//@p0 wake opacity, reveal, bokeh, heads
//@ 0.000 0.00 0.00 0.0 0.0
//@ 0.050 0.20 0.00 0.0 1.0
//@ 0.200 0.90 0.00 0.4 1.0
//@ 0.370 0.95 0.00 1.0 1.0
//@ 0.390 0.90 0.00 1.1 0.0
//@ 0.420 0.75 0.20 1.1 0.0
//@ 0.460 0.50 0.60 1.0 0.0
//@ 0.500 0.30 0.90 1.0 0.0
//@ 0.550 0.15 1.10 1.0 0.0
//@ 0.620 0.05 1.20 0.8 0.0
//@ 0.700 0.00 1.20 0.4 0.0
//@ 0.850 0.00 1.20 0.0 0.0
//@p16 path of the first comet (upper right to lower left)
//@ 0 0.98 -0.5 -0.75 0.48
//@p18 path of the second comet (lower left to upper right)
//@ 0 -0.98 0.48 0.75 -0.5
fn comet_a() -> Comet { return Comet(16, 2, 0.06, 0.38, 0.7); }
fn comet_b() -> Comet { return Comet(18, 2, 0.06, 0.38, 0.7); }

fn wake(c: Comet, p: vec2f, t: f32) -> f32 {
    let sh = a1_comet_s(c, t);
    let near = a1_comet_near(c, p, max(sh, 1e-3));
    let age = max(sh - near.y, 0.0) + max(t - c.t1, 0.0);
    let width = 0.04 + 0.15 * age;
    let speck = 0.6 + 0.4 * vnoise(p * 80.0 + vec2f(0.0, 15.0 * t));
    return (0.8 * a1_disc(near.x, width, 0.6 * width) + 0.35 * a1_gauss(near.x, 2.5 * width)) * speck
        * smoothstep(0.0, 0.05, near.y + 0.01);
}

fn head(c: Comet, p: vec2f, t: f32) -> vec3f {
    let hd = length(p - a1_comet_at(c, a1_comet_s(c, t)));
    return vec3f(1.0, 1.0, 0.85) * a1_gauss(hd, 0.035) + vec3f(1.0, 0.6, 0.1) * 0.8 * a1_halo(hd, 0.06);
}

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let k = u.p[0];
    let p = to_c(uv);
    let ca = comet_a();
    let cb = comet_b();
    // the new clip opens out from the diagonal, in noisy patches
    let dd = abs(dot(p, normalize(vec2f(0.5, 0.88)))) + 0.15 * (vnoise(p * 6.0) - 0.5);
    let w = clamp((k.y - dd * 0.8) * 3.0, 0.0, 1.0);
    var col = mix(a1_da(uv), a1_db(uv), w);
    let wk = k.x * max(wake(ca, p, t), wake(cb, p, t));
    col = mix(col, vec3f(0.98, 0.96, 0.82), clamp(wk, 0.0, 1.0));
    // amber bokeh shower from both paths
    let b1 = a1_comet_bokeh(ca, p, t, 200, 5.0, 0.55, 0.45, vec2f(0.0), 0.7, 0.02, 0.06);
    let b2 = a1_comet_bokeh(cb, p, t, 200, 6.0, 0.55, 0.45, vec2f(0.0), 0.7, 0.02, 0.06);
    let b = select(b2, b1, b1.x > b2.x);
    let bc = mix(vec3f(0.98, 0.78, 0.15), vec3f(1.0, 0.92, 0.45), b.y);
    col = mix(col, bc, clamp(0.85 * k.z * b.x, 0.0, 1.0));
    let g = a1_comet_glitter(ca, p, t, 120, 7.0, 0.1, 0.05, vec2f(0.0), 0.3, 0.1, 8)
        + a1_comet_glitter(cb, p, t, 120, 8.0, 0.1, 0.05, vec2f(0.0), 0.3, 0.1, 8);
    col = a1_add(col, k.x * vec3f(1.0, 0.95, 0.7) * (g.x + 1.3 * g.y));
    col = a1_add(col, k.w * (head(ca, p, t) + head(cb, p, t)));
    return a1_out(col);
}

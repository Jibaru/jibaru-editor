// Pink Sparkler Rise: a sparkler zig-zags up and down across the frame from
// left to right, trailing white sparks and pink bokeh; the new clip sweeps in
// from the left behind it.
//@p0 trail opacity, reveal sweep, bokeh, heads
//@ 0.000 0.00 -0.3 0.0 0.0
//@ 0.040 0.70 -0.3 0.1 1.0
//@ 0.200 1.00 -0.1 0.6 1.0
//@ 0.270 1.00 0.1 0.8 1.0
//@ 0.340 1.00 0.35 0.9 1.0
//@ 0.400 0.95 0.5 1.0 1.0
//@ 0.470 0.85 0.6 1.0 0.5
//@ 0.500 0.60 0.68 1.0 0.0
//@ 0.540 0.45 0.85 1.0 0.0
//@ 0.680 0.15 1.4 0.7 0.0
//@ 0.850 0.00 1.2 0.2 0.0
//@ 0.950 0.00 1.4 0.0 0.0
//@p1 pink haze lingering where the sparkler passed last
//@ 0.00 0.0
//@ 0.30 0.15
//@ 0.50 0.45
//@ 0.65 0.35
//@ 0.80 0.15
//@ 0.95 0.0
//@p16 zig-zag path of the sparkler
//@ 0 -0.6 -0.65 -0.75 0.45
//@p17
//@ 0 0.05 -0.48 0.3 0.25
//@p18
//@ 0 0.55 -0.48 0.85 0.3

fn trail(c: Comet, p: vec2f, t: f32) -> f32 {
    let sh = a1_comet_s(c, t);
    if (sh <= 0.0) { return 0.0; }
    let near = a1_comet_near(c, p, sh);
    let age = max(sh - near.y, 0.0) + max(t - c.t1, 0.0) * 2.0;
    let width = 0.04 + 0.25 * age;
    let fade = exp(-1.5 * age);
    let speck = 0.55 + 0.45 * vnoise(p * 70.0 + vec2f(0.0, 25.0 * t));
    return fade * speck * (a1_disc(near.x, width, 0.7 * width) + 0.4 * a1_gauss(near.x, 3.0 * width));
}

fn head(c: Comet, p: vec2f, t: f32) -> vec3f {
    let s = a1_comet_s(c, t);
    if (s <= 0.0 || s >= 1.0) { return vec3f(0.0); }
    let hd = length(p - a1_comet_at(c, s));
    return vec3f(1.0) * a1_gauss(hd, 0.025) + vec3f(1.0, 0.15, 0.5) * 1.2 * a1_halo(hd, 0.045);
}

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let k = u.p[0];
    let p = to_c(uv);
    let c1 = Comet(16, 6, 0.0, 0.45, 1.0);
    let w = clamp((k.y - uv.x) * 2.5 + 0.1 * (vnoise(p * 5.0) - 0.5), 0.0, 1.0);
    var col = mix(a1_da(uv), a1_db(uv), w);
    let hz = u.p[1].x * smoothstep(0.15, 0.95, uv.x) * (0.7 + 0.3 * vnoise(p * 3.0 + vec2f(0.0, t)));
    col = mix(col, vec3f(0.95, 0.7, 0.88), hz);
    let tr = k.x * trail(c1, p, t);
    col = mix(col, vec3f(1.0, 0.93, 0.97), clamp(tr, 0.0, 1.0));
    let b = a1_comet_bokeh(c1, p, t, 260, 11.0, 0.35, 0.3, vec2f(0.0, -0.1), 1.0, 0.012, 0.035);
    let bc = mix(vec3f(0.95, 0.45, 0.75), vec3f(1.0, 0.75, 0.9), b.y);
    col = mix(col, bc, clamp(k.z * b.x, 0.0, 1.0));
    col = a1_add(col, k.w * head(c1, p, t));
    return a1_out(col);
}

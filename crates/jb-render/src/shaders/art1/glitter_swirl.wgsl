// Glitter Swirl: a pink spark spirals out from the middle of the frame and
// winds a widening vortex of white glitter that fills the picture; the new
// clip opens from the centre of the vortex as the glitter scatters away.
//@base jb.sparkle_trail_wash_03
//@p0 vortex opacity, reveal radius, scatter, head
//@ 0.000 0.00 -0.2 0.0 0.0
//@ 0.050 0.60 -0.2 0.0 1.0
//@ 0.300 0.95 -0.2 0.2 1.0
//@ 0.420 0.95 0.15 0.5 1.0
//@ 0.470 0.85 0.32 0.8 0.0
//@ 0.540 0.60 0.50 1.0 0.0
//@ 0.600 0.35 0.68 1.0 0.0
//@ 0.670 0.15 0.95 0.8 0.0
//@ 0.740 0.05 1.30 0.5 0.0
//@ 0.900 0.00 1.30 0.0 0.0
//@p16 spiral path
//@ 0 -0.099 0.057 -0.234 0.033
//@p17
//@ 0 -0.108 -0.142 0.23 0.021
//@p18
//@ 0 -0.085 0.346 -0.625 0.044
//@p19
//@ 0 -0.122 -0.43 0.621 0.011
//@p20
//@ 0 -0.071 0.635 -1.016 0.054
//@p21
//@ 0 -0.135 -0.719
fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let k = u.p[0];
    let p = to_c(uv);
    let c = Comet(16, 11, 0.05, 0.467, 1.0);
    let sh = a1_comet_s(c, t);
    // new clip opens from the vortex centre
    let rc = length((p - vec2f(-0.1, 0.03)) * vec2f(0.75, 1.0));
    let w = clamp((k.y - rc) * 5.0 + 0.3 * (vnoise(p * 8.0) - 0.5), 0.0, 1.0);
    var col = mix(a1_da(uv), a1_db(uv), w);
    // glitter vortex: band along the spiral, widening and grainy with age
    let near = a1_comet_near(c, p, max(sh, 1e-3));
    let age = max(sh - near.y, 0.0) + max(t - c.t1, 0.0);
    let width = 0.1 + 0.16 * age;
    // short sparks streaming outwards from the vortex centre (polar noise)
    let pc = p - vec2f(-0.1, 0.03);
    let ang = atan2(pc.y, pc.x);
    let rad = length(pc);
    let streak = vnoise(vec2f(ang * 30.0, rad * 26.0 - 12.0 * t)) * vnoise(vec2f(ang * 70.0 + 3.0, rad * 18.0 - 7.0 * t));
    let grain = max(smoothstep(0.18, 0.45, streak), 0.6 * smoothstep(0.3, 0.6, vnoise(p * 110.0 + vec2f(13.0 * t, -9.0 * t))));
    let band = a1_disc(near.x, width, 0.8 * width) * smoothstep(0.0, 0.03, near.y + 0.01);
    let v = k.x * band * (0.1 + 1.1 * grain);
    col = mix(col, vec3f(1.0, 0.96, 0.97), clamp(v, 0.0, 1.0));
    col = a1_add(col, k.x * vec3f(0.12, 0.08, 0.1) * a1_gauss(near.x, 2.0 * width) * smoothstep(0.0, 0.03, near.y));
    // scattered glitter and pink bokeh flying outwards
    let b = a1_comet_bokeh(c, p, t, 220, 21.0, 0.15, 0.9, vec2f(0.0), 0.7, 0.006, 0.022);
    col = mix(col, mix(vec3f(1.0, 0.8, 0.88), vec3f(1.0, 0.97, 0.98), b.y), clamp(k.z * b.x, 0.0, 1.0));
    let g = a1_comet_glitter(c, p, t, 260, 22.0, 0.15, 0.7, vec2f(0.0), 0.6, 0.1, 30);
    col = a1_add(col, k.z * vec3f(1.0, 0.95, 0.97) * (g.x + 1.5 * g.y));
    // head: white core with a hot pink glow
    let hd = length(p - a1_comet_at(c, sh));
    col = a1_add(col, k.w * (vec3f(1.0) * a1_gauss(hd, 0.04) + vec3f(1.0, 0.15, 0.45) * 1.3 * a1_halo(hd, 0.06)));
    // the fitted colour field of the stand-in carries the average colour
    return a1_out(mix(a1_base(uv), col, 0.75));
}

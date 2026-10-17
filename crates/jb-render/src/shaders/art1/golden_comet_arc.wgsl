// Golden Comet Arc: a golden comet loops over the top of the frame and back
// along the bottom, leaving a dense white glitter wake that breaks into warm
// bokeh; the new clip appears along the wake in the order the comet passed.
//@p0 wake opacity, reveal, bokeh, head
//@ 0.000 0.00 0.00 0.0 0.0
//@ 0.050 0.30 0.00 0.0 1.0
//@ 0.150 0.85 0.05 0.3 1.0
//@ 0.300 0.85 0.35 0.8 1.0
//@ 0.420 0.75 0.75 1.0 1.0
//@ 0.440 0.50 0.80 1.0 0.0
//@ 0.550 0.15 0.92 1.0 0.0
//@ 0.700 0.0 0.98 0.6 0.0
//@ 0.800 0.00 1.00 0.2 0.0
//@ 0.900 0.00 1.00 0.0 0.0
//@p16 path points
//@ 0 -1.05 0.02 -0.62 -0.24
//@p17
//@ 0 -0.05 -0.36 0.5 -0.27
//@p18
//@ 0 0.68 0.02 0.38 0.3
//@p19
//@ 0 -0.05 0.4 -0.95 0.47
fn comet() -> Comet { return Comet(16, 8, 0.055, 0.43, 0.8); }

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let k = u.p[0];
    let c = comet();
    let p = to_c(uv);
    let sh = a1_comet_s(c, t);
    let near = a1_comet_near(c, p, max(sh, 1e-3));
    // the new clip sweeps in from the left under the wake
    let w = clamp((k.y - 0.5) * 2.5 + 0.5 - (uv.x - 0.5) * 1.2, 0.0, 1.0);
    var col = mix(a1_da(uv), a1_db(uv), w);
    // wake: soft white band whose width grows with age, speckled
    let age = max(sh - near.y, 0.0) + max(t - c.t1, 0.0);
    let width = 0.06 + 0.22 * age;
    let speck = 0.6 + 0.4 * vnoise(p * 90.0 + vec2f(0.0, 20.0 * t));
    let wake = k.x * (0.85 * a1_disc(near.x, width, 0.6 * width) + 0.3 * a1_gauss(near.x, 2.5 * width)) * speck * smoothstep(0.0, 0.05, near.y + 0.01);
    col = mix(col, vec3f(0.95, 0.93, 0.88), clamp(wake, 0.0, 1.0));
    // warm bokeh and glitter
    let b = a1_comet_bokeh(c, p, t, 150, 3.0, 0.2, 0.25, vec2f(0.15, 0.0), 0.6, 0.015, 0.05);
    let bc = mix(vec3f(1.0, 0.9, 0.6), vec3f(1.0, 0.98, 0.9), b.y);
    col = mix(col, bc, clamp(1.2 * k.z * b.x, 0.0, 1.0));
    let g = a1_comet_glitter(c, p, t, 220, 2.0, 0.12, 0.08, vec2f(0.0, 0.0), 0.35, 0.12, 12);
    col = a1_add(col, k.x * (vec3f(1.0, 0.97, 0.9) * g.x + 1.4 * vec3f(1.0, 0.98, 0.92) * g.y));
    // head: yellow-white core with an orange halo
    let hd = length(p - a1_comet_at(c, sh));
    col = a1_add(col, k.w * (vec3f(1.0, 1.0, 0.9) * a1_gauss(hd, 0.03) + vec3f(1.0, 0.75, 0.2) * 0.7 * a1_halo(hd, 0.05)));
    return a1_out(col);
}

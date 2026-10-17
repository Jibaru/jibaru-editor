// Gold Bokeh Beam: a star of light flares in the middle of the old clip and
// a thin vertical beam drops through it; then the frame floods with golden
// out-of-focus highlights while a bright frame outline tumbles past, and the
// gold thins out on the new clip. The fitted colour field of the stand-in
// (`//@base`) carries the measured broad tints and timing; the star, beam,
// outline and bokeh are drawn here around its mean.
//@base jb.gold_sparkle_burst
//@p0 star, beam, bokeh, outline
//@ 0.000 0.0 0.0 0.0 0.0
//@ 0.070 0.0 0.0 0.0 0.0
//@ 0.110 0.8 0.3 0.0 0.0
//@ 0.180 1.0 0.8 0.0 0.0
//@ 0.240 0.6 1.0 0.2 0.6
//@ 0.300 0.2 1.0 0.7 1.0
//@ 0.400 0.0 1.0 1.0 0.0
//@ 0.700 0.0 0.9 1.0 0.0
//@ 0.800 0.0 0.5 0.6 0.0
//@ 0.870 0.0 0.0 0.0 0.0
//@p1 outline: centre x, y, half size, angle
//@ 0.20 0.70 -0.55 0.30 0.6
//@ 0.40 0.30 0.05 0.45 1.4

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let t = a1_t();
    let p = to_c(uv);
    var col = a1_base(uv);
    // the flood hides the picture: wash the base out to its local mean
    if (k.z > 0.0) {
        var m = vec3f(0.0);
        for (var j = 0; j < 16; j = j + 1) {
            let o = vec2f(f32(j % 4) - 1.5, f32(j / 4) - 1.5) * 0.06;
            m = m + a1_base(uv + o);
        }
        col = mix(col, m / 16.0, k.z);
    }
    // golden bokeh, modulated around the base so its mean stays put
    if (k.z > 0.0) {
        let b = a1_bokeh_field(p, t, 60, 14.0, vec2f(0.0), vec2f(0.5 * aspect() + 0.1, 0.6), vec2f(0.0, -0.15), 0.06, 0.15, 0.3);
        let bc = mix(vec3f(1.0, 0.85, 0.3), vec3f(1.0, 0.97, 0.75), b.y);
        col = col + 0.45 * k.z * (bc * b.x - vec3f(0.3, 0.26, 0.12)) * (1.0 - 0.3 * col);
    }
    // vertical beam through the middle
    let beam = k.y * (a1_gauss(p.x, 0.006) + 0.35 * a1_gauss(p.x, 0.04));
    col = a1_add(col, vec3f(1.0, 0.95, 0.6) * beam);
    // star flare at the centre
    col = a1_add(col, k.x * vec3f(1.0, 0.92, 0.55) * a1_starflare(p, 0.5, 0.52));
    // tumbling frame outline
    let o = u.p[1];
    let q = rot2(p - o.xy, o.w);
    let fr = abs(a1_rbox(q, vec2f(o.z * 1.2, o.z * 0.8), 0.01));
    col = a1_add(col, k.w * vec3f(1.0, 0.95, 0.8) * (1.0 - smoothstep(0.003, 0.008, fr)));
    return a1_out(col);
}

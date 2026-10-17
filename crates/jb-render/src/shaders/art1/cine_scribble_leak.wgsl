// Cine Scribble Leak: hand-drawn loops of cream light scrawl across the
// picture, a new scribble every few frames, between washes of pink light, a
// dark smeared band with glowing film-strip squares, and a rainbow flare at
// the right before it clears. Rhythm, placement and tints follow Filmora's
// Movie Light Effects 4 (measured on its export); the scribbles and flares
// are drawn here.
//@time sec 2.002
//@p0 scribble: on, seed, centre x, y
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.4495 0.000 0.000 0.000 0.000
//@ 0.4505 1.000 1.000 0.450 -0.300
//@ 0.5495 1.000 1.000 0.450 -0.300
//@ 0.5505 0.000 0.000 0.000 0.000
//@ 0.7164 0.000 0.000 0.000 0.000
//@ 0.7174 1.000 2.000 0.500 0.050
//@ 0.8165 1.000 2.000 0.500 0.050
//@ 0.8175 1.000 3.000 0.300 0.000
//@ 0.9499 1.000 3.000 0.300 0.000
//@ 0.9509 1.000 4.000 0.000 0.000
//@ 1.0834 1.000 4.000 0.000 0.000
//@ 1.0844 1.000 5.000 -0.050 0.050
//@ 1.2169 1.000 5.000 -0.050 0.050
//@ 1.2179 1.000 6.000 0.250 0.000
//@ 1.2836 1.000 6.000 0.250 0.000
//@ 1.2846 1.000 7.000 0.100 0.150
//@ 1.4505 1.000 7.000 0.100 0.150
//@ 1.4515 1.000 8.000 0.100 0.200
//@ 1.5172 1.000 8.000 0.100 0.200
//@ 1.5182 1.000 9.000 0.450 0.300
//@ 1.6173 1.000 9.000 0.450 0.300
//@ 1.6183 0.000 0.000 0.000 0.000
//@p1 pink wash, whiteness
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.0824 0.000 0.000 0.000 0.000
//@ 0.0834 0.250 0.000 0.000 0.000
//@ 0.2493 0.250 0.000 0.000 0.000
//@ 0.2503 0.000 0.000 0.000 0.000
//@ 0.5495 0.000 0.000 0.000 0.000
//@ 0.5505 0.750 0.000 0.000 0.000
//@ 0.6497 0.750 0.000 0.000 0.000
//@ 0.6507 0.850 1.000 0.000 0.000
//@ 0.7164 0.850 1.000 0.000 0.000
//@ 0.7174 0.000 0.000 0.000 0.000
//@ 1.2836 0.000 0.000 0.000 0.000
//@ 1.2846 0.700 0.600 0.000 0.000
//@ 1.4505 0.700 0.600 0.000 0.000
//@ 1.4515 0.000 0.000 0.000 0.000
//@p2 dark band
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.9499 0.000 0.000 0.000 0.000
//@ 0.9509 1.000 0.000 0.000 0.000
//@ 1.2169 1.000 0.000 0.000 0.000
//@ 1.2179 0.000 0.000 0.000 0.000
//@p3 rainbow flare
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 1.6173 0.000 0.000 0.000 0.000
//@ 1.6183 1.000 0.000 0.000 0.000
//@ 1.8509 1.000 0.000 0.000 0.000
//@ 1.8518 0.000 0.000 0.000 0.000

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let p = to_c(uv);
    var col = a1_da(uv);
    // pink light washing in from the left, bleaching towards white
    let w = u.p[1];
    let pink = mix(vec3f(0.95, 0.35, 0.55), vec3f(1.0, 0.9, 0.92), w.y);
    col = mix(col, pink, w.x * (0.55 + 0.45 * smoothstep(0.7, -0.7, p.x)));
    // dark smeared band across the middle with glowing film-strip squares
    let bd = u.p[2].x;
    if (bd > 0.0) {
        let by = a1_gauss(p.y + 0.02 + 0.03 * sin(p.x * 3.0 + t * 4.0), 0.15);
        col = mix(col, vec3f(0.08, 0.03, 0.02), 0.95 * bd * by);
        let cell = vec2f(fract((p.x + 0.9 - t * 0.6) * 4.5) - 0.5, (p.y - 0.17) * 4.5);
        let sq = a1_rbox(cell, vec2f(0.32, 0.32), 0.08);
        let strip = bd * smoothstep(-0.2, -0.55, p.x) * (1.0 - smoothstep(-0.02, 0.02, sq)) * step(abs(p.y - 0.17), 0.12);
        col = mix(col, vec3f(1.0, 0.62, 0.35), 0.8 * strip);
    }
    // rainbow flare down the right
    let rb = u.p[3].x;
    if (rb > 0.0) {
        let st = a1_streak(p + vec2f(0.0, t * 0.3), vec2f(0.0, 1.0), 8.0, 9.0, 3);
        let hue = clamp((uv.x - 0.55) * 2.4 + 0.25 * (st - 0.5), 0.0, 1.0);
        let rc = a1_ramp(hue, vec3f(0.5, 0.45, 1.0), vec3f(0.3, 0.95, 0.85), vec3f(0.95, 1.0, 0.4));
        let a = rb * smoothstep(0.5, 0.85, uv.x) * (0.55 + 0.6 * st);
        col = mix(col, a1_screen(col, rc), clamp(a, 0.0, 1.0));
    }
    // the scribble
    let s = u.p[0];
    if (s.x > 0.0) {
        let seed = s.y * 7.0 + 3.0;
        let ang = 6.283 * a1_r(seed, 5.0);
        let c = a1_scribble(p, seed, s.zw, 1.1 + 0.4 * a1_r(seed, 6.0), ang, 1.5 + floor(2.0 * a1_r(seed, 7.0)), 0.2, 0.032);
        col = mix(col, vec3f(1.0, 0.97, 0.74), s.x * c);
    }
    return a1_out_a(col, a1_alpha(uv));
}

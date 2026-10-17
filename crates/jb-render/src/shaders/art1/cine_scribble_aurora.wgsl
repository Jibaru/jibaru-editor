// Cine Scribble Aurora: washes of pink light, then the picture drifts
// slightly larger and soft orange smears of film edge glow along the top and
// bottom while big hand-drawn loops of cream light scrawl across it and
// curtains of aurora light (cyan to lime) ripple through. Rhythm, placement
// and tints follow Filmora's Movie Light Effects 5 (measured on its export);
// the scribbles, smears and aurora are drawn here.
//@time sec 2.002
//@p0 scribble: on, seed, centre x, y
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.0491 0.000 0.000 0.000 0.000
//@ 0.0501 1.000 1.000 0.600 -0.450
//@ 0.1492 1.000 1.000 0.600 -0.450
//@ 0.1502 0.000 0.000 0.000 0.000
//@ 0.4161 0.000 0.000 0.000 0.000
//@ 0.4171 1.000 2.000 0.550 0.100
//@ 0.5495 1.000 2.000 0.550 0.100
//@ 0.5505 1.000 3.000 0.450 -0.100
//@ 0.7164 1.000 3.000 0.450 -0.100
//@ 0.7174 1.000 4.000 0.250 0.000
//@ 0.8165 1.000 4.000 0.250 0.000
//@ 0.8175 1.000 5.000 0.000 0.250
//@ 0.9499 1.000 5.000 0.000 0.250
//@ 0.9509 1.000 6.000 0.150 -0.200
//@ 1.2169 1.000 6.000 0.150 -0.200
//@ 1.2179 1.000 7.000 0.000 -0.200
//@ 1.2836 1.000 7.000 0.000 -0.200
//@ 1.2846 1.000 8.000 -0.150 0.050
//@ 1.4505 1.000 8.000 -0.150 0.050
//@ 1.4515 1.000 9.000 0.300 0.150
//@ 1.6173 1.000 9.000 0.300 0.150
//@ 1.6183 1.000 10.000 0.400 0.300
//@ 1.7508 1.000 10.000 0.400 0.300
//@ 1.7518 0.000 0.000 0.000 0.000
//@p1 pink wash, whiteness
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.1492 0.000 0.000 0.000 0.000
//@ 0.1502 0.800 0.000 0.000 0.000
//@ 0.2493 0.800 0.000 0.000 0.000
//@ 0.2503 0.800 0.400 0.000 0.000
//@ 0.3160 0.800 0.400 0.000 0.000
//@ 0.3170 0.850 0.700 0.000 0.000
//@ 0.4161 0.850 0.700 0.000 0.000
//@ 0.4171 0.000 0.000 0.000 0.000
//@p2 film look (zoom, edge smears)
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.4161 0.000 0.000 0.000 0.000
//@ 0.4171 1.000 0.000 0.000 0.000
//@ 1.7508 1.000 0.000 0.000 0.000
//@ 1.7518 0.000 0.000 0.000 0.000
//@p3 aurora: strength, centre x
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.9499 0.000 0.000 0.000 0.000
//@ 0.9509 1.000 0.150 0.000 0.000
//@ 1.2169 1.000 0.150 0.000 0.000
//@ 1.2179 0.000 0.000 0.000 0.000
//@ 1.6173 0.000 0.000 0.000 0.000
//@ 1.6183 0.800 0.450 0.000 0.000
//@ 1.7508 0.800 0.450 0.000 0.000
//@ 1.7518 0.000 0.000 0.000 0.000

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let fl = u.p[2].x;
    // the picture drifts a little larger during the film look
    let zuv = 0.5 + (uv - 0.5) / (1.0 + 0.08 * fl);
    let p = to_c(uv);
    var col = a1_da(zuv);
    // soft orange smears of film edge along the bottom and top
    if (fl > 0.0) {
        // out-of-focus film-strip squares glowing orange between dark gaps
        let sq = 0.5 + 0.5 * cos(6.283 * (p.x * 3.2 - t * 0.7));
        let sm = a1_streak(p + vec2f(t * 0.8, 0.0), vec2f(1.0, 0.0), 6.0, 5.0, 3);
        let edge = 0.8 * smoothstep(0.22, 0.42, p.y) + 0.5 * smoothstep(-0.36, -0.5, p.y);
        let smear = mix(vec3f(0.3, 0.1, 0.05), vec3f(1.0, 0.6, 0.35), clamp(0.35 + 0.3 * sq * (0.6 + 0.8 * sm), 0.0, 1.0));
        col = mix(col, smear, fl * clamp(edge, 0.0, 1.0) * 0.9);
    }
    // pink light washing in from the left, bleaching towards white
    let w = u.p[1];
    let pink = mix(vec3f(0.95, 0.35, 0.55), vec3f(1.0, 0.9, 0.92), w.y * smoothstep(0.6, -0.6, p.x));
    col = mix(col, pink, w.x * (0.6 + 0.4 * smoothstep(0.7, -0.7, p.x)));
    // aurora: rippling vertical curtains, cyan at the core, lime at the fringe
    let au = u.p[3];
    if (au.x > 0.0) {
        let st = a1_streak(p + vec2f(0.15 * sin(p.y * 4.0 + t * 3.0), t * 0.4), vec2f(0.0, 1.0), 7.0, 8.0, 3);
        let d = abs(uv.x - (0.5 + au.y * 0.5)) / 0.32;
        let core = 1.0 - smoothstep(0.0, 1.0, d);
        let ac = mix(vec3f(0.85, 1.0, 0.35), vec3f(0.3, 0.9, 0.95), smoothstep(0.2, 0.75, core));
        col = mix(col, a1_screen(col, ac), clamp(au.x * core * (0.6 + 0.6 * st), 0.0, 1.0));
    }
    // the scribble
    let s = u.p[0];
    if (s.x > 0.0) {
        let seed = s.y * 7.0 + 5.0;
        let ang = 6.283 * a1_r(seed, 5.0);
        let c = a1_scribble(p, seed, s.zw, 1.1 + 0.4 * a1_r(seed, 6.0), ang, 1.5 + floor(2.0 * a1_r(seed, 7.0)), 0.2, 0.032);
        col = mix(col, vec3f(1.0, 0.95, 0.72), s.x * c);
    }
    return a1_out_a(col, a1_alpha(uv));
}

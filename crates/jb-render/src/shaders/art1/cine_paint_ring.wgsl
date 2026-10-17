// Cine Paint Ring: a magenta flash, a splash of white paint, then the print
// bleaches to pastel with the edge of the film strip along the bottom while a
// rough ring of dark paint opens a window back onto the unbleached picture,
// and a last white flare streaked with cyan and magenta. Rhythm, placement
// and tints follow Filmora's Movie Light Effects 8 (measured on its export);
// the paint, ring and flares are drawn here.
//@time sec 2.002
//@p0 magenta flash
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.0824 0.000 0.000 0.000 0.000
//@ 0.0834 0.750 0.000 0.000 0.000
//@ 0.2493 0.750 0.000 0.000 0.000
//@ 0.2503 0.000 0.000 0.000 0.000
//@p1 white splash: on, edge x
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.5495 0.000 0.000 0.000 0.000
//@ 0.5505 1.000 -0.550 0.000 0.000
//@ 0.6497 1.000 -0.550 0.000 0.000
//@ 0.6507 0.000 0.000 0.000 0.000
//@ 1.4505 0.000 0.000 0.000 0.000
//@ 1.4515 1.000 -0.100 0.000 0.000
//@ 1.5172 1.000 -0.100 0.000 0.000
//@ 1.5182 0.000 0.000 0.000 0.000
//@p2 bleach
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.6497 0.000 0.000 0.000 0.000
//@ 0.6507 0.550 0.000 0.000 0.000
//@ 1.0501 0.550 0.000 0.000 0.000
//@ 1.0511 0.500 0.000 0.000 0.000
//@ 1.2169 0.500 0.000 0.000 0.000
//@ 1.2179 0.550 0.000 0.000 0.000
//@ 1.2836 0.550 0.000 0.000 0.000
//@ 1.2846 0.620 0.000 0.000 0.000
//@ 1.4505 0.620 0.000 0.000 0.000
//@ 1.4515 0.000 0.000 0.000 0.000
//@p3 ring: x, y, radius, on
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 1.0501 0.000 0.000 0.000 0.000
//@ 1.0511 0.350 -0.150 0.450 1.000
//@ 1.2836 0.350 -0.150 0.450 1.000
//@ 1.2846 -0.500 0.250 0.550 1.000
//@ 1.4505 -0.500 0.250 0.550 1.000
//@ 1.4515 0.000 0.000 0.000 0.000
//@p4 white flare
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 1.6840 0.000 0.000 0.000 0.000
//@ 1.6850 0.850 0.000 0.000 0.000
//@ 1.8509 0.850 0.000 0.000 0.000
//@ 1.8518 0.000 0.000 0.000 0.000

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let tq = floor(t * 12.0) / 12.0;
    let p = to_c(uv);
    let pic = a1_da(uv);
    var col = pic;
    // magenta flash
    col = mix(col, a1_screen(col, vec3f(0.9, 0.25, 0.75)), u.p[0].x);
    // bleached pastel print with the film strip's edge along the bottom
    let bl = u.p[2].x;
    if (bl > 0.0) {
        var b = 1.0 - (1.0 - col) * (1.0 - clamp(1.6 * bl, 0.0, 1.0) * vec3f(0.92, 0.86, 0.92));
        let hole = a1_rbox(vec2f(fract((p.x + tq * 0.3) * 5.0) - 0.5, (p.y - 0.465) * 5.0), vec2f(0.2, 0.16), 0.06);
        b = mix(b, vec3f(0.45, 0.3, 0.55), 0.7 * (1.0 - smoothstep(-0.02, 0.02, hole)));
        // the ring of paint opens a window onto the unbleached picture
        let r = u.p[3];
        var win = 0.0;
        if (r.w > 0.0) {
            let q = p - r.xy;
            let a = atan2(q.y, q.x);
            let rough = 0.012 * (a1_fbm(vec2f(a * 3.0, tq * 5.0), 3) - 0.5) * 4.0;
            let d = length(q) - r.z + rough;
            win = 1.0 - smoothstep(-0.01, 0.01, d);
            b = mix(b, mix(col, b, 0.35), win);
            let rim = (1.0 - smoothstep(0.004, 0.016, abs(d + 0.005))) * (0.6 + 0.4 * a1_fbm(vec2f(a * 9.0, 1.0), 2));
            b = mix(b, vec3f(0.3, 0.12, 0.38), rim);
        }
        col = b;
    }
    // splash of white paint from the left with a ragged edge
    let sp = u.p[1];
    if (sp.x > 0.0) {
        let e = sp.y + 0.12 * (a1_fbm(vec2f(p.y * 4.0, tq * 3.0), 3) - 0.5) * 2.0;
        col = mix(col, vec3f(1.0), sp.x * (1.0 - smoothstep(e - 0.01, e + 0.01, p.x)));
    }
    // white flare streaked with cyan and magenta
    let wf = u.p[4].x;
    if (wf > 0.0) {
        let st = a1_streak(p + vec2f(0.0, t * 0.5), vec2f(0.0, 1.0), 6.0, 6.0, 3);
        var c = vec3f(1.0);
        c = mix(c, vec3f(0.2, 0.95, 1.0), smoothstep(0.5, 0.7, st) * smoothstep(-0.3, 0.2, p.x));
        c = mix(c, vec3f(1.0, 0.45, 0.95), smoothstep(0.55, 0.75, 1.0 - st) * smoothstep(0.1, 0.5, -p.y));
        col = mix(col, c, wf);
    }
    return a1_out_a(col, a1_alpha(uv));
}

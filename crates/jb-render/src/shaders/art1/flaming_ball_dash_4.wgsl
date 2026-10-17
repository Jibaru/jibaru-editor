// Flaming Ball Dash 4: a ball flies straight at the camera out of the middle of the frame
// in a ring of kicked-up dust, swelling until it fills the frame, then cuts to the new clip.
// The ball, its fire and dust are drawn here; its path, size and the cut follow
// the timing of the matching Filmora Burning Dash transition (measured on its export).
//@p0 ball x, y, radius, spin
//@ 0.0000 0.000 0.000 0.000 0.000
//@ 0.1000 0.000 0.000 0.014 0.000
//@ 0.1167 0.000 0.000 0.022 0.100
//@ 0.2167 0.000 0.000 0.043 0.400
//@ 0.3333 0.000 0.000 0.072 0.800
//@ 0.4333 0.000 0.000 0.131 1.200
//@ 0.5500 0.000 0.000 0.232 1.600
//@ 0.6500 0.000 -0.050 0.435 2.000
//@ 0.7333 0.000 0.000 1.160 2.300
//@ 0.7833 0.050 0.000 2.320 2.500
//@ 0.7915 0.050 0.000 2.320 2.500
//@ 0.7917 0.050 0.000 0.000 2.500
//@p1 fire strength, direction x, y, length (radii)
//@ 0.0000 0.000 1.000 0.000 1.000
//@p2 new clip, dust, wake
//@ 0.0000 0.000 0.000
//@ 0.1000 0.000 0.900
//@ 0.6500 0.000 0.900
//@ 0.7333 0.000 0.300
//@ 0.7917 0.000 0.000
//@ 0.8000 1.000 0.000

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let p = to_c(uv);
    let k = u.p[2];
    var col = mix(a1_da(uv), a1_db(uv), k.x);
    let b = u.p[0];
    let f = u.p[1];
    // dust kicked up around the ball
    if (k.y > 0.0) {
        let q = (p - b.xy) / max(b.z, 1e-4);
        let d = length(q);
        let n = a1_fbm(q * 2.5 + vec2f(t * 3.0, 0.0), 4);
        let ring = smoothstep(1.9, 1.0, d) * smoothstep(0.9, 1.05, d) * (0.4 + 0.9 * n);
        col = mix(col, vec3f(0.97, 0.95, 0.93), clamp(k.y * ring, 0.0, 1.0));
    }
    if (f.x > 0.0) {
        col = a1_add(col, f.x * a1_ball_fire(p, b, normalize(f.yz), f.w, t));
    }
    // (items with a base) the whip after the ball has passed
    if (k.z > 0.0) { col = mix(col, a1_base(uv), k.z); }
    if (b.z > 0.0) {
        let ball = a1_ball(p, b);
        col = mix(col, ball.rgb, ball.a);
    }
    return a1_out(col);
}

// Flaming Ball Dash 3: a small ball wrapped in fire streaks in from the distance with its
// flames streaming back to the left, grows until it fills the frame and cuts to the new clip.
// The ball, its fire and dust are drawn here; its path, size and the cut follow
// the timing of the matching Filmora Burning Dash transition (measured on its export).
//@p0 ball x, y, radius, spin
//@ 0.0000 -0.200 0.020 0.000 0.000
//@ 0.0833 -0.120 0.000 0.036 0.100
//@ 0.1167 -0.090 0.000 0.058 0.200
//@ 0.2167 0.000 0.000 0.145 0.600
//@ 0.3333 0.000 0.000 0.203 1.000
//@ 0.4333 0.000 0.000 0.290 1.400
//@ 0.5500 0.040 0.000 0.406 1.800
//@ 0.6500 0.180 -0.050 0.652 2.200
//@ 0.7667 0.300 0.300 1.740 2.600
//@ 0.8500 0.350 0.400 2.610 2.900
//@ 0.8582 0.350 0.400 2.610 2.900
//@ 0.8583 0.350 0.400 0.000 2.900
//@p1 fire strength, direction x, y, length (radii)
//@ 0.0000 1.000 1.000 0.250 6.000
//@ 0.6500 1.000 1.000 0.250 4.000
//@ 0.7667 0.500 1.000 0.250 3.000
//@ 0.8500 0.300 1.000 0.250 3.000
//@p2 new clip, dust, wake
//@ 0.0000 0.000 0.000
//@ 0.8583 0.000 0.000
//@ 0.8667 1.000 0.000

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

// Flaming Ball Dash 2: a ball with a faint heat trail bounces up from the lower left and
// rushes at the camera until it fills the frame, cutting to the new clip.
// The ball, its fire and dust are drawn here; its path, size and the cut follow
// the timing of the matching Filmora Burning Dash transition (measured on its export).
//@p0 ball x, y, radius, spin
//@ 0.0000 -1.100 0.550 0.000 0.000
//@ 0.0667 -0.950 0.500 0.043 0.000
//@ 0.0833 -0.830 0.430 0.058 0.100
//@ 0.1167 -0.520 0.250 0.102 0.300
//@ 0.2167 -0.440 0.130 0.174 0.700
//@ 0.3333 -0.270 0.050 0.290 1.100
//@ 0.4333 -0.120 0.000 0.406 1.500
//@ 0.5500 -0.090 -0.050 0.609 1.900
//@ 0.6500 0.000 0.200 1.450 2.300
//@ 0.7500 0.050 0.300 2.320 2.600
//@ 0.7582 0.050 0.300 2.320 2.600
//@ 0.7583 0.050 0.300 0.000 2.600
//@p1 fire strength, direction x, y, length (radii)
//@ 0.0000 0.350 0.850 -0.500 3.000
//@ 0.5500 0.250 0.850 -0.500 2.000
//@ 0.6667 0.000 0.850 -0.500 2.000
//@p2 new clip, dust, wake
//@ 0.0000 0.000 0.000
//@ 0.7583 0.000 0.000
//@ 0.7667 1.000 0.000

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

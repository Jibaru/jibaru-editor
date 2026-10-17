// Flaming Ball Dash 1: a ball trailing a long tail of fire flies in from the left and
// swells towards the camera until it fills the frame, then whips away to the new clip.
// The ball, its fire and dust are drawn here; its path, size and the cut follow
// the timing of the matching Filmora Burning Dash transition (measured on its export).
// After the ball has passed, the fitted colour field of the stand-in
// (`//@base`) carries the whip to the new clip.
//@base jb.fireball_dash_1
//@p0 ball x, y, radius, spin
//@ 0.0000 -1.000 0.500 0.040 0.000
//@ 0.0500 -0.760 0.390 0.070 0.180
//@ 0.0833 -0.580 0.250 0.130 0.300
//@ 0.1167 -0.370 0.160 0.170 0.420
//@ 0.1667 -0.220 0.070 0.210 0.600
//@ 0.2167 -0.130 0.050 0.250 0.780
//@ 0.2833 -0.090 0.040 0.290 1.020
//@ 0.3333 -0.070 0.030 0.330 1.200
//@ 0.4000 -0.060 0.030 0.380 1.440
//@ 0.4667 0.000 0.050 0.450 1.680
//@ 0.5333 0.050 0.050 0.600 1.920
//@ 0.6000 0.200 0.100 0.850 2.160
//@ 0.6667 0.300 0.200 1.200 2.400
//@ 0.7500 0.350 0.300 1.800 2.700
//@ 0.7582 0.350 0.300 1.800 2.700
//@ 0.7583 0.350 0.300 0.000 2.700
//@p1 fire strength, direction x, y, length (radii)
//@ 0.0000 1.000 1.000 -0.150 5.000
//@ 0.5500 1.000 1.000 -0.150 4.000
//@ 0.6500 0.600 1.000 -0.150 3.000
//@ 0.7667 0.300 1.000 -0.600 3.000
//@ 0.8833 0.000 1.000 -0.600 3.000
//@p2 new clip, dust, wake
//@ 0.0000 0.000 0.000 0.000
//@ 0.6000 0.000 0.000 0.000
//@ 0.6667 0.000 0.000 0.400
//@ 0.7500 0.000 0.000 0.400
//@ 0.7667 1.000 0.000 1.000

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

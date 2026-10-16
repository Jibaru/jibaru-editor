// Parametric soft-mask wipe (lane1_mask.rs).
//
//   P = (uv - 0.5) * (aspect, 1) - centre;  Q = R(-r) P
//   d = Q.x (linear) | |Q| (circle) | angle of Q / 2pi (sweep) |
//       fract(blades * angle) (pinwheel) | star radius (star) | |Q.x| (band)
//   m = clamp(sgn * (c - d) / w + 0.5, 0, 1), optionally smoothstepped,
// and the frame is mix(A, B, m) in display space plus a glow along the edge
// (e = |c - d|): core.rgb * exp(-(e / wc)^2) + halo.rgb * exp(-e / wh).
//
// Uniforms: p[0] = (c, w, r, _) at this progress, p[1] = (shape, sgn, cx, cy),
// p[2] = (blades or star notch radius, profile, wc, wh), p[3] = core, p[4] = halo,
// p[5] = torn edge (amplitude, frequency, rim width): d is displaced by fbm noise
// along Q.y and a white paper rim is drawn just inside the incoming side.

// |q| over the radius of the unit five-point star (tip up at radius 1,
// notches at `rf`, straight edges) in the direction of q.
fn star_r(q: vec2f, rf: f32) -> f32 {
    let seg = PI / 5.0;
    let ang = atan2(q.x, -q.y);
    let th = abs(ang + seg - 2.0 * seg * floor((ang + seg) / (2.0 * seg)) - seg);
    let e = vec2f(rf * cos(seg) - 1.0, rf * sin(seg));
    let t = e.y / (cos(th) * e.y - sin(th) * e.x);
    return length(q) / t;
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let row = u.p[0];
    let shape = i32(u.p[1].x);
    let p = (in.uv - 0.5) * vec2f(aspect(), 1.0) - u.p[1].zw;
    let cr = cos(-row.z);
    let sr = sin(-row.z);
    let q = vec2f(cr * p.x - sr * p.y, sr * p.x + cr * p.y);
    let ang = fract(atan2(q.x, -q.y) / TAU + 1.0);
    var d = q.x;
    if (shape == 1) { d = length(q); }
    if (shape == 2) { d = ang; }
    if (shape == 3) { d = fract(ang * u.p[2].x); }
    if (shape == 4) { d = star_r(q, u.p[2].x); }
    if (shape == 5) { d = abs(q.x); }
    let tear = u.p[5];
    if (tear.x > 0.0) {
        let n = fbm(vec2f(q.y * tear.y, select(3.7, 11.3, q.x < 0.0))) * 0.8 + 0.2 * vnoise(vec2f(q.y * tear.y * 9.0, 5.1));
        d = d + tear.x * (n - 0.5) * 2.0;
    }
    var m = clamp(u.p[1].y * (row.x - d) / max(row.y, 1e-4) + 0.5, 0.0, 1.0);
    if (u.p[2].y > 0.5) { m = m * m * (3.0 - 2.0 * m); }
    let a = to_disp(sa(in.uv));
    let b = to_disp(sb(in.uv));
    var c = mix(a, b, m);
    if (tear.z > 0.0) {
        let inside = u.p[1].y * (row.x - d);
        let rim = smoothstep(-0.0015, 0.0, inside) * (1.0 - smoothstep(tear.z, tear.z + 0.0015, inside));
        c = vec4f(mix(c.rgb, vec3f(0.96, 0.96, 0.95), rim), c.a);
    }
    if (any(u.p[3].rgb != vec3f(0.0)) || any(u.p[4].rgb != vec3f(0.0))) {
        let e = abs(row.x - d);
        let g = u.p[3].rgb * exp(-pow(e / max(u.p[2].z, 1e-5), 2.0)) + u.p[4].rgb * exp(-e / max(u.p[2].w, 1e-5));
        c = vec4f(clamp(c.rgb + g, vec3f(0.0), vec3f(1.0)), c.a);
    }
    return from_disp(c);
}

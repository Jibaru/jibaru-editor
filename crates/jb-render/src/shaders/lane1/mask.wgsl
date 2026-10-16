// Parametric soft-mask wipe (lane1_mask.rs).
//
//   P = (uv - 0.5) * (aspect, 1) - centre;  Q = R(-r) P
//   d = Q.x (linear) | |Q| (circle) | angle of Q / 2pi (sweep) |
//       fract(blades * angle) (pinwheel) | star radius (star)
//   m = clamp(sgn * (c - d) / w + 0.5, 0, 1), optionally smoothstepped,
// and the frame is mix(A, B, m) in display space.
//
// Uniforms: p[0] = (c, w, r, _) at this progress, p[1] = (shape, sgn, cx, cy),
// p[2] = (blades, profile, _, _).

fn star_r(q: vec2f) -> f32 {
    let ang = atan2(q.x, -q.y);
    let k = abs(fract(ang * 5.0 / TAU) - 0.5) * 2.0;
    let rin = 0.45;
    return length(q) / (rin + (1.0 - rin) * k);
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
    if (shape == 4) { d = star_r(q); }
    var m = clamp(u.p[1].y * (row.x - d) / max(row.y, 1e-4) + 0.5, 0.0, 1.0);
    if (u.p[2].y > 0.5) { m = m * m * (3.0 - 2.0 * m); }
    let a = to_disp(sa(in.uv));
    let b = to_disp(sb(in.uv));
    return from_disp(mix(a, b, m));
}

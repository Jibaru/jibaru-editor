// Golden Bokeh Band: a broad band of golden light full of out-of-focus
// highlights sweeps across the frame from left to right; the new clip is
// revealed behind it.
//@p0 band centre (fraction of width), band half width, intensity, reveal front
//@ 0.00 -0.25 0.12 0.00 -0.20
//@ 0.10 -0.10 0.15 0.40 -0.20
//@ 0.22 0.10 0.22 0.85 -0.20
//@ 0.33 0.32 0.26 1.00 -0.10
//@ 0.38 0.45 0.27 1.00 0.00
//@ 0.45 0.65 0.27 1.00 0.20
//@ 0.55 0.88 0.25 1.00 0.45
//@ 0.67 1.05 0.20 0.90 0.75
//@ 0.78 1.20 0.16 0.50 1.05
//@ 0.85 1.30 0.12 0.00 1.20
//@p1 warm tint over the whole frame
//@ 0.00 0.00
//@ 0.13 0.25
//@ 0.40 0.35
//@ 0.60 0.20
//@ 0.75 0.00

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let t = a1_t();
    let p = to_c(uv);
    let asp = u.size.x * u.inv_size.y;
    // reveal: a soft, slightly ragged vertical front moving right
    let edge = k.w - uv.x + 0.04 * (vnoise(vec2f(uv.y * 6.0, 1.0)) - 0.5);
    let w = smoothstep(-0.06, 0.06, edge);
    var col = mix(a1_da(uv), a1_db(uv), w);
    col = mix(col, col * vec3f(1.05, 0.95, 0.6) + vec3f(0.12, 0.08, 0.0), u.p[1].x);
    // warm afterglow trailing the band
    let trail = k.z * smoothstep(k.x + 0.05, k.x - 0.15, uv.x) * exp(-max(k.x - uv.x, 0.0) / 0.35);
    col = mix(col, col * vec3f(1.1, 0.75, 0.35) + vec3f(0.25, 0.12, 0.0), 0.8 * trail);
    // the band: a soft vertical glow, a little slanted
    let bx = uv.x - k.x + 0.08 * (uv.y - 0.5);
    let band = k.z * a1_gauss(bx, k.y);
    col = mix(col, vec3f(1.0, 0.86, 0.45), clamp(0.6 * band, 0.0, 1.0));
    col = a1_add(col, vec3f(1.0, 0.95, 0.75) * 0.7 * band * band);
    // bokeh inside the band
    let cx = (k.x - 0.5) * asp;
    let b = a1_bokeh_field(p, t, 40, 9.0, vec2f(cx, 0.0), vec2f(k.y * asp * 1.2, 0.55), vec2f(0.0, -0.1), 0.05, 0.14, 0.25);
    let bc = mix(vec3f(1.0, 0.92, 0.6), vec3f(1.0, 1.0, 0.92), b.y);
    col = mix(col, bc, clamp(1.1 * k.z * b.x, 0.0, 1.0));
    return a1_out(col);
}

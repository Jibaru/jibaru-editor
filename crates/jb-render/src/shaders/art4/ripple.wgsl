// Ripple (see RIPPLE in fx/art4_models.rs).
fn bump_field(p: vec2f, f: f32, ph: f32, s: i32) -> f32 {
    let q = p * f + vec2f(ph, 0.37 * ph);
    return inoise(q, s) + 0.5 * inoise(q * 2.03 + vec2f(5.2, 1.3), s + 1);
}
fn bump_disp(p: vec2f, f: f32, ph: f32, s: i32) -> vec2f {
    let e = 0.15 / f;
    let gx = bump_field(p + vec2f(e, 0.0), f, ph, s) - bump_field(p - vec2f(e, 0.0), f, ph, s);
    let gy = bump_field(p + vec2f(0.0, e), f, ph, s) - bump_field(p - vec2f(0.0, e), f, ph, s);
    return vec2f(gx, gy) / (2.0 * e * f);
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let p = to_c(in.uv);
    let s = i32(c(1));
    let d = bump_disp(p, c(0), k(2), s);
    // Radial ring burst about c(4..6): radius k4, gaussian width k6, amplitude k5.
    let r = length(p - c2(4));
    let x = r - k(4);
    let ring = k(5) * sin(x * c(3)) * exp(-(x * x) / max(k(6) * k(6), 1e-6));
    let dir = select(vec2f(0.0), (p - c2(4)) / max(r, 1e-4), r > 1e-4);
    let pa = (p - k(0) * d - ring * dir) / max(k(3), 1e-3);
    let pb = select(p, pa, c(2) > 0.5);
    return mix(sat(from_c(pa)), sbt(from_c(pb)), clamp(k(1), 0.0, 1.0));
}

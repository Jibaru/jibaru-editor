// Swirl (see SWIRL in fx/art4_models.rs).
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let sx = max(c(0), 1e-3);
    let q = (in.uv - 0.5) * vec2f(sx, 1.0);
    let r = length(q);
    let t = max(0.0, 1.0 - r / max(k(1), 1e-3));
    let th = k(0) * pow(t, max(k(2), 0.05)) + k(5);
    let qq = rot2(q, th) / max(k(3), 1e-3);
    let uv = qq / vec2f(sx, 1.0) + 0.5;
    return mix(sat(uv), sbt(uv), clamp(k(4), 0.0, 1.0));
}

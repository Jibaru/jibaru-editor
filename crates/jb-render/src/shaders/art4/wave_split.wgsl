// Wave split (see WAVE_SPLIT in fx/art4_models.rs).
fn ws_uv(uv: vec2f, sh: f32) -> vec2f {
    let dy = k(0) * sin(TAU * c(0) * uv.x + k(1)) + k(6) * (uv.y - 0.5) * sin(TAU * c(2) * uv.x + 1.7 * k(1));
    let dx = k(2) * sin(TAU * c(1) * uv.y + k(3));
    return vec2f(uv.x + dx + sh, uv.y + dy);
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let s = k(4);
    let ur = ws_uv(in.uv, s);
    let ug = ws_uv(in.uv, 0.0);
    let ub = ws_uv(in.uv, -s);
    let a = vec4f(sat(ur).r, sat(ug).g, sat(ub).b, sat(ug).a);
    let b = vec4f(sbt(ur).r, sbt(ug).g, sbt(ub).b, sbt(ug).a);
    return mix(a, b, clamp(k(5), 0.0, 1.0));
}

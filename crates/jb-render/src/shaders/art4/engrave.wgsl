// Banknote engraving (see ENGRAVE in fx/art4_models.rs).
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let a = disp(sa(in.uv));
    var l = dot(a, vec3f(0.2126, 0.7152, 0.0722));
    l = clamp(pow(clamp(l * c(8) + c(9), 0.0, 1.0), max(c(11), 0.05)), 0.0, 1.0);
    // Pixel position in 1080p units.
    let p = vec2f(in.uv.x * aspect(), in.uv.y) * 1080.0;
    let ctr = vec2f(c(0) * aspect(), c(1)) * 1080.0;
    let r = length(p - ctr);
    let s1 = 0.5 + 0.5 * cos(6.2831853 * r / max(c(2), 1.0));
    // Second family: lines at angle c3 (radians), period c4, waving by c5 px at c6 cycles / 1000 px.
    let dir = vec2f(cos(c(3)), sin(c(3)));
    let nrm = vec2f(-dir.y, dir.x);
    let t = dot(p, nrm) + c(5) * sin(6.2831853 * dot(p, dir) * c(6) / 1000.0);
    let s2 = 0.5 + 0.5 * cos(6.2831853 * t / max(c(4), 1.0));
    // Ink where the screen is below the darkness; c7 sharpness.
    let ink1 = clamp((1.0 - l - s1) * c(7) + 0.5, 0.0, 1.0);
    let ink2 = clamp((1.0 - l - s2) * c(7) + 0.5, 0.0, 1.0);
    let ink = max(ink1, ink2 * c(12));
    let o = mix(vec3f(l), vec3f(1.0 - ink), clamp(c(10), 0.0, 1.0));
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

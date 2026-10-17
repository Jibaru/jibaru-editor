// Unsharp mask with fringe, desaturation and scanlines (see UNSHARP in
// fx/art4_models.rs). tx = blurred A.
fn sharp_at(uv: vec2f) -> vec3f {
    let a = disp(sa(uv));
    let bl = disp(sx(uv));
    return a + c(1) * (a - bl);
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let px = px1080();
    var o = sharp_at(in.uv);
    // Coloured fringes: red / blue sharpened from taps shifted by ±(c2, c6) px.
    let d = vec2f(c(2), c(6)) * px;
    let r = sharp_at(in.uv + d).r;
    let b = sharp_at(in.uv - d).b;
    o.r = mix(o.r, r, c(3));
    o.b = mix(o.b, b, c(3));
    let l = dot(o, vec3f(0.2126, 0.7152, 0.0722));
    o = mix(vec3f(l), o, 1.0 + c(7));
    // Scanlines: period c9 px, amplitude c8, phase c10 (in periods).
    let y = in.uv.y * 1080.0 / max(c(9), 0.5) + c(10);
    o *= 1.0 - c(8) * (0.5 + 0.5 * cos(6.2831853 * y));
    o = (o - 0.5) * c(4) + 0.5 + c(5) + vec3f(c(11), c(12), c(13));
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

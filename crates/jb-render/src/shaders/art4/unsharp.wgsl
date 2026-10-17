// Unsharp mask with fringe (see UNSHARP in fx/art4_models.rs). tx = blurred A.
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let px = px1080();
    let a = disp(sa(in.uv));
    let bl = disp(sx(in.uv));
    var o = a + c(1) * (a - bl);
    // Coloured fringes: red / blue sharpened from horizontally shifted taps.
    let ar = disp(sa(in.uv + vec2f(c(2) * px.x, 0.0))).r;
    let br = disp(sx(in.uv + vec2f(c(2) * px.x, 0.0))).r;
    let ab = disp(sa(in.uv - vec2f(c(2) * px.x, 0.0))).b;
    let bb = disp(sx(in.uv - vec2f(c(2) * px.x, 0.0))).b;
    o.r = mix(o.r, ar + c(1) * (ar - br), c(3));
    o.b = mix(o.b, ab + c(1) * (ab - bb), c(3));
    o = (o - 0.5) * c(4) + 0.5 + c(5);
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

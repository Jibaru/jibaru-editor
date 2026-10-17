// Signal glitch: frame jitter, channel split, displaced bands and bars
// (see JITTER_BARS in fx/art4_models.rs).
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let px = px1080();
    let beat = i32(k(4));
    var uv = in.uv + vec2f(k(0), k(1)) * px;
    // Displaced horizontal bands: band height c0 px, fraction k9, shift ±k8 px.
    let band = i32(floor(in.uv.y * 1080.0 / max(c(0), 1.0)));
    if (rnd3(band, beat, 11) < k(9)) {
        uv.x += (rnd3(band, beat, 12) * 2.0 - 1.0) * k(8) * px.x;
    }
    let d = vec2f(k(2) * px.x, 0.0);
    var o = vec3f(disp(sam(uv + d)).r, disp(sam(uv)).g, disp(sam(uv - d)).b);
    // Bars: up to 8 random rectangles per beat, k3 of them shown.
    let n = i32(clamp(k(3), 0.0, 8.0));
    for (var i = 0; i < n; i++) {
        let y0 = rnd3(i, beat, 21);
        let h = (c(1) + c(2) * rnd3(i, beat, 22)) * px.y;
        let x0 = rnd3(i, beat, 23);
        let w = c(3) + c(4) * rnd3(i, beat, 24);
        if (in.uv.y > y0 && in.uv.y < y0 + h && in.uv.x > x0 && in.uv.x < x0 + w) {
            let cyan = rnd3(i, beat, 25) < c(5);
            o = select(vec3f(1.0), vec3f(0.35, 0.95, 1.0), cyan) * k(7) + o * (1.0 - k(7));
        }
    }
    o = o * (1.0 + k(5)) + k(6);
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

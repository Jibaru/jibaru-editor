// Mosaic (see MOSAIC in fx/art4_models.rs).
fn mosaic_tap(uv: vec2f) -> vec4f {
    return mix(sa(uv), sb(uv), clamp(k(4), 0.0, 1.0));
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let n = max(vec2f(k(0), k(0) * select(1.0, k(1), k(1) > 0.0)), vec2f(1.0));
    let o = k2(2);
    let cell = floor((in.uv - 0.5) * n + o);
    let uv = clamp((cell + c2(0) - o) / n + 0.5, vec2f(0.0), vec2f(1.0));
    var col = mosaic_tap(uv);
    // Optional cell average (c2 weight), 4x4 taps.
    if (c(2) > 0.0) {
        var acc = vec4f(0.0);
        for (var j = 0; j < 4; j++) {
            for (var i = 0; i < 4; i++) {
                let t = (cell + (vec2f(f32(i), f32(j)) + 0.5) / 4.0 - o) / n + 0.5;
                acc += mosaic_tap(clamp(t, vec2f(0.0), vec2f(1.0)));
            }
        }
        col = mix(col, acc / 16.0, clamp(c(2), 0.0, 1.0));
    }
    let d = disp(col) * k(5) + k(6);
    return work(clamp(d, vec3f(0.0), vec3f(1.0)));
}

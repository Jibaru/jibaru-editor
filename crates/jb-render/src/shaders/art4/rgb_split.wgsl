// Channel split (see RGB_SPLIT in fx/art4_models.rs).
fn smear(uv: vec2f, len: f32) -> vec3f {
    if (len <= 0.0) { return disp(sam(uv)); }
    var acc = vec3f(0.0);
    for (var i = 0; i < 9; i++) {
        let t = (f32(i) / 8.0 - 0.5) * len;
        acc += disp(sam(uv + vec2f(t, 0.0)));
    }
    return acc / 9.0;
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let len = k(6) * px1080().x;
    let r = smear(in.uv + k2(0), len).r;
    let g = smear(in.uv + k2(2), len).g;
    let b = smear(in.uv + k2(4), len).b;
    let base = disp(sa(in.uv));
    var o = mix(base, vec3f(r, g, b), select(1.0, k(9), k(9) > 0.0));
    o = o * (1.0 + k(7)) + k(8);
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

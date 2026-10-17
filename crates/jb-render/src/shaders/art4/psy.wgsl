// Colour glitch (see PSY in fx/art4_models.rs).
fn rainbow(h: f32) -> vec3f {
    let t = fract(h) * 6.0;
    return clamp(vec3f(abs(t - 3.0) - 1.0, 2.0 - abs(t - 2.0), 2.0 - abs(t - 4.0)), vec3f(0.0), vec3f(1.0));
}
fn psy_src(uv: vec2f) -> vec3f {
    return disp(mix(sam(uv), sbm(uv), clamp(k(14), 0.0, 1.0)));
}
fn psy_tap(uv0: vec2f) -> vec3f {
    // Row wave: horizontal sine offset of k15 (uv), c1 cycles down the frame, phase c2 + progress·c3.
    let uv = uv0 + vec2f(k(15) * sin(6.2831853 * (uv0.y * c(1) + c(2) + u.progress * c(3))), 0.0);
    let d = vec2f(k(11) * px1080().x, 0.0);
    return vec3f(psy_src(uv + d).r, psy_src(uv).g, psy_src(uv - d).b);
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let len = k(5) * px1080().x;
    var o = vec3f(0.0);
    if (len > 0.0) {
        for (var i = 0; i < 12; i++) {
            o += psy_tap(in.uv + vec2f((f32(i) / 11.0 - 0.5) * len, 0.0));
        }
        o /= 12.0;
    } else {
        o = psy_tap(in.uv);
    }
    if (k(8) >= 2.0) { o = floor(o * k(8) + 0.5) / k(8); }
    let l = dot(o, vec3f(0.2126, 0.7152, 0.0722));
    let cyc = select(c(0), k(12), k(12) > 0.0);
    let w = clamp(k(6), 0.0, 1.0) * smoothstep(k(13) - 0.05, k(13) + 0.05, l);
    o = mix(o, rainbow(l * cyc + k(7)), w);
    o = mix(o, 1.0 - o, clamp(k(0), 0.0, 1.0));
    let l2 = dot(o, vec3f(0.2126, 0.7152, 0.0722));
    o = mix(o, vec3f(l2), clamp(k(1), 0.0, 1.0));
    o = o * (1.0 + vec3f(k(2), k(3), k(4))) * (1.0 + k(9)) + k(10);
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

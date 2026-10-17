// Soft glow / dreamy blur (see GLOW in fx/art4_models.rs). tx = blurred A.
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let uv = zoom_uv(in.uv, max(k(5), 1e-3), vec2f(0.5));
    let a = disp(sam(uv));
    let bl = disp(textureSampleLevel(tx, smp, mirror_uv(uv), 0.0));
    // Soft mix of the blurred copy, then a screen-blended glow of it.
    var o = mix(a, bl, clamp(k(1), 0.0, 1.0));
    let g = bl * k(4);
    o = 1.0 - (1.0 - o) * (1.0 - clamp(g, vec3f(0.0), vec3f(1.0)));
    o = o * k(2) + k(3) + vec3f(k(6), k(7), k(8));
    o = saturate3(o, 1.0 + k(9));
    return work(clamp(o, vec3f(0.0), vec3f(1.0)));
}

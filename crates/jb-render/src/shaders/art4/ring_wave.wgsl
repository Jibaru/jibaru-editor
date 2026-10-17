// Ring wave (see RING_WAVE in fx/art4_models.rs).
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let sx = c(0);
    let q = (in.uv - 0.5) * vec2f(sx, 1.0);
    let r = length(q);
    let dir = select(vec2f(0.0), q / max(r, 1e-4), r > 1e-4);
    // Front at radius k0 with a soft edge k5; the rings travel with it.
    let inside = 1.0 - smoothstep(k(0) - k(5), k(0) + k(5), r);
    let x = max(k(0) - r, 0.0);
    let ph = c(1) * x - c(2) - k(1);
    let disp = k(2) * sin(ph) * inside;
    let qq = (q - dir * disp) / max(k(6), 1e-3);
    let uv = qq / vec2f(sx, 1.0) + 0.5;
    // Weight of the incoming clip: base k3, ring contrast k4, growth k7 per unit behind the front.
    let w = clamp(k(3) + k(4) * cos(ph) + k(7) * x, 0.0, 1.0) * inside;
    return mix(sat(uv), sbt(uv), w);
}

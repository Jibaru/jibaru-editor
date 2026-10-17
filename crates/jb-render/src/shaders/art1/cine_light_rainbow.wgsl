// Cine Light Rainbow: an old-cinema look (lifted blacks, fine grain and a
// faint orange glint of the projector gate at the left edge) with two light
// events: a soft pink defocus bloom, and late on, a rainbow of streaky
// prismatic light (yellow, green, cyan) flaring down the left of the frame.
// Timing and tints follow Filmora's Movie Light Effects 1 (measured on its
// export); the look and the flares are drawn here.
//@time sec 2.002
//@pre 0.008
//@p0 bloom (blur), pink lift
//@ 0.0000 0.00 0.00
//@ 0.6507 0.00 0.00
//@ 0.6673 0.75 0.60
//@ 0.7341 1.00 0.75
//@ 0.8508 1.00 0.75
//@ 0.8675 0.00 0.00
//@ 1.6183 0.00 0.00
//@ 1.6350 0.00 0.00
//@p1 rainbow flare
//@ 0.0000 0.00
//@ 1.6183 0.00
//@ 1.6350 0.90
//@ 1.8518 1.00
//@ 1.8685 0.00

fn pre(uv: vec2f) -> vec4f { return sa(uv); }

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let p = to_c(uv);
    let k = u.p[0];
    var col = a1_da(uv);
    // pink defocus bloom
    let soft = to_disp(vec4f(sx(uv).rgb, 1.0)).rgb;
    col = mix(col, soft, k.x);
    col = mix(col, col * vec3f(1.12, 0.86, 1.04) + vec3f(0.03, 0.0, 0.02), k.y);
    // old-cinema print: lifted blacks, slightly softened whites, grain
    col = col * 0.94 + vec3f(0.05) * smoothstep(vec3f(0.0), vec3f(0.25), col) + vec3f(0.02);
    col = col + 0.035 * (hash12(floor(uv * vec2f(480.0, 270.0)) + floor(t * 24.0) * 7.3) - 0.5);
    // projector gate glint: a thin orange bracket at the left edge
    let g = a1_rbox(p - vec2f(-0.5 * aspect() - 0.01, -0.01), vec2f(0.045, 0.14), 0.04);
    col = a1_add(col, vec3f(0.9, 0.5, 0.15) * 0.5 * a1_gauss(g, 0.003));
    // rainbow flare: vertical prismatic streaks fading towards the right
    let r = u.p[1].x;
    if (r > 0.0) {
        let x = uv.x;
        let st = a1_streak(p + vec2f(0.0, t * 0.3), vec2f(0.0, 1.0), 8.0, 9.0, 3);
        let hue = clamp(x * 1.8 - 0.1 + 0.25 * (st - 0.5), 0.0, 1.0);
        let rc = a1_ramp(hue, vec3f(0.95, 1.0, 0.4), vec3f(0.35, 0.95, 0.7), vec3f(0.5, 0.75, 1.0));
        let a = r * (0.55 * smoothstep(0.62, 0.1, x) + 0.5 * smoothstep(0.2, 0.0, x)) * (0.55 + 0.6 * st);
        col = mix(col, a1_screen(col, rc), clamp(a, 0.0, 1.0));
    }
    return a1_out_a(col, a1_alpha(uv));
}

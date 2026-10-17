// A swarm of bats gathers over the outgoing clip, then one bat flies at the
// camera until its glowing eyes fill the frame and the incoming clip cuts
// in. Table `bs`: [p, swarm (number of bats, 0..BS_N), big bat scale,
// eye zoom, side | glow, -, -, -].

const BS_N: f32 = 46.0;
const BS_INK: vec3f = vec3f(0.1, 0.1, 0.11);
const BS_EYE: vec3f = vec3f(0.42, 0.9, 0.95);

fn bs_swarm(c: vec2f, count: f32, p: f32, px: f32) -> f32 {
    var cov = 0.0;
    for (var i = 0.0; i < BS_N; i = i + 1.0) {
        if (i >= count) { break; }
        let h = hash22(vec2f(i, 3.7));
        let h2 = hash12(vec2f(i, 9.1));
        let s = 0.04 + 0.1 * h2 * h2 + 0.02 * h.y;
        let dir = select(-1.0, 1.0, h.x > 0.5);
        let pos = vec2f(-0.95 + 1.9 * fract(h.x * 7.3 + dir * p * (0.4 + 0.3 * h.y)), -0.45 + 0.9 * h.y);
        let q = (c - pos) / s;
        if (abs(q.x) > 1.2 || abs(q.y) > 0.8) { continue; }
        let flap = sin(p * 60.0 + i * 1.7);
        let d = spr_bat_d(vec2f(q.x * dir, q.y), flap) * s;
        cov = max(cov, clamp(0.5 - d / px, 0.0, 1.0));
    }
    return cov;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_bs(p);
    let k1 = kf_bs_1(p);
    let px = u.inv_size.y;
    let c = to_c(uv);
    if (k.w > 0.5) { return l3_out(l3_db(uv)); }
    var col = l3_da(uv);
    col = ak_over(col, BS_INK, bs_swarm(c, k.x, p, px));
    // The big bat; its eyes sit at (+-0.035, -0.135) of the sprite.
    let s = k.y;
    if (s > 0.0) {
        let z = k.z;
        let eye_c = vec2f(0.0, -0.135);
        let q = c / (s * z) + eye_c;
        let d = (spr_bat_d(q, 0.7 + 0.2 * sin(p * 25.0)) - 0.03) * s * z;
        col = ak_over(col, BS_INK, clamp(0.5 - d / (px + 0.002 * z), 0.0, 1.0));
        let e = vec2f(abs(q.x) - 0.05, q.y + 0.135);
        let de = sd_ellipse(rot2(e, 0.3), vec2f(0.036, 0.022)) * s * z;
        let glow = exp(-max(de, 0.0) / (0.012 * s * z + 0.004)) * 0.6;
        col = ak_over(col, BS_EYE, max(clamp(0.5 - de / (px + 0.01 * s * z * k1.x), 0.0, 1.0), glow));
    }
    return l3_out(col);
}

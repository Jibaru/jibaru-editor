// Lantern roll: a huge dark jack-o'-lantern rolls in from the right, its
// carved eyes, nose and jagged grin glowing red from inside, covers the
// frame (the clips switch behind it) and rolls out to the left. Table `lr`:
// [p, centre x (frame heights from the centre), switch (0 outgoing, 1
// incoming), roll angle, -].

const LR_R: f32 = 0.93;

fn lr_glow(d: f32, px: f32) -> f32 { return clamp(0.5 - d / px, 0.0, 1.0); }

fn fx(uv: vec2f) -> vec4f {
    let k = kf_lr(prog());
    let c = to_c(uv);
    var col = select(l3_da(uv), l3_db(uv), k.y > 0.5);
    let ctr = vec2f(k.x, 0.0);
    let q0 = c - ctr;
    let d = length(q0) - LR_R;
    if (d < 0.03) {
        let px = ak_px();
        let q = rot2(q0, -k.z) / LR_R;
        // Dark rind with ribs and a faint red rim.
        let rib = 0.5 + 0.5 * cos(q.x * 9.0 / max(sqrt(1.0 - min(q.y * q.y, 0.95)), 0.2));
        var s = vec3f(0.035, 0.018, 0.015) * (0.7 + 0.5 * rib);
        let rim = smoothstep(-0.12, 0.0, d);
        s = mix(s, vec3f(0.30, 0.06, 0.05), rim * 0.6);
        // Carved face: two slanted eyes, a nose and a jagged grin.
        let le = sd_tri(q, vec2f(-0.86, -0.62), vec2f(-0.56, -0.34), vec2f(-0.86, -0.30));
        let re = sd_tri(q, vec2f(0.10, -0.62), vec2f(-0.18, -0.30), vec2f(0.16, -0.20));
        let nose = sd_tri(q, vec2f(-0.40, -0.14), vec2f(-0.28, 0.02), vec2f(-0.46, 0.02));
        let ym = 0.36 + 0.05 * abs(fract(q.x * 4.5) - 0.5) * 2.0;
        let yt = 0.50 - 0.09 * abs(fract(q.x * 4.5 + 0.25) - 0.5) * 2.0;
        let mouth = max(max(ym - q.y, q.y - yt), abs(q.x + 0.15) - 0.78);
        let face = min(min(le, re), min(nose, mouth));
        let inner = smoothstep(0.0, -0.08, face);
        let hot = mix(vec3f(0.85, 0.16, 0.12), vec3f(1.0, 0.78, 0.72), inner);
        s = mix(s, hot, lr_glow(face, px * 2.0 / LR_R));
        s = l3_screen(s, vec3f(0.5, 0.06, 0.04) * exp(-max(face, 0.0) / 0.05) * 0.5);
        col = mix(col, s, clamp(0.5 - d / px, 0.0, 1.0));
    }
    return l3_out(col);
}

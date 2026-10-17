// A big ghost pops up over the outgoing clip with little ghosts circling
// it, then the camera dives into its open mouth; a white flash follows and
// the white closes up into a wedge at the top over the incoming clip.
// Table `bz`: [p, ghost y (uv centre), ghost scale, orbit, zoom | side,
// white, wedge height (v), gain].

fn bz_mini(q0: vec2f, px: f32) -> vec4f {
    // Little ghost: a white drop with dot eyes.
    var acc = vec4f(0.0);
    let d = op_smin(sd_circle(q0, 0.5), sd_tri(q0, vec2f(-0.45, 0.2), vec2f(0.45, 0.2), vec2f(-0.9, 1.1)), 0.15);
    acc = spr_layer(acc, ak_rgb(250.0, 250.0, 248.0), spr_cov(d, px));
    let e = min(sd_circle(q0 - vec2f(-0.18, -0.05), 0.11), sd_circle(q0 - vec2f(0.18, -0.05), 0.11));
    acc = spr_layer(acc, ak_rgb(26.0, 26.0, 30.0), spr_cov(e, px));
    return acc;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_bz(p);
    let k1 = kf_bz_1(p);
    let px = u.inv_size.y;
    let c = to_c(uv);
    var col = select(l3_da(uv), l3_db(uv) * k1.w, k1.x > 0.5);
    if (k1.x < 0.5) {
        // Zoom into the mouth: the ghost is scaled about its mouth centre.
        let zs = k.y * k.w;
        let centre = vec2f(0.0, k.x - 0.5);
        let mouth = centre + vec2f(0.0, 0.0) * k.y;
        let q = (c - mouth) / zs + vec2f(0.0, 0.0);
        if (abs(q.x) < 0.7 && abs(q.y) < 0.7) {
            let g = spr_boo(q, px / zs);
            col = ak_over(col, g.rgb, g.a);
        }
        // Little ghosts orbiting.
        if (k.z > 0.0 && k.w < 1.5) {
            for (var i = 0.0; i < 3.0; i = i + 1.0) {
                let a = p * 9.0 + i * TAU / 3.0;
                let pos = centre + vec2f(0.52 * cos(a), 0.32 * sin(a) + 0.1) * k.z;
                let s = 0.1 * k.z;
                let m = bz_mini(rot2(c - pos, -0.3 * sin(a)) / s, px / s);
                col = ak_over(col, m.rgb, m.a);
            }
        }
    }
    // White flash, then a wedge of white closing up to the top centre.
    col = mix(col, vec3f(1.0), k1.y);
    let h = k1.z;
    if (h > 0.0) {
        let w = 0.8 * h;
        let d = (abs(c.x) / max(w, 1e-3)) - (h - uv.y) / max(h, 1e-3);
        col = mix(col, vec3f(1.0), clamp(0.5 - d * 6.0, 0.0, 1.0));
    }
    return l3_out(col);
}

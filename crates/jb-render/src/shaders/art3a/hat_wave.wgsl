// A witch hat drops onto the outgoing clip, slides off into the bottom-left
// corner and comes back riding a wavy wipe that floods up over the frame
// with the incoming clip. Table `hw`: [p, x, y (uv of the hat centre), rot,
// scale | right edge of the flood (u), top of the flood (v), gain of B, -].

fn hw_flood(uv: vec2f, xr: f32, vt: f32) -> f32 {
    let wave = 0.035 * sin(uv.x * 19.0 + 1.3) + 0.02 * sin(uv.x * 41.0);
    let side = 0.04 * sin(uv.y * 17.0);
    return max((vt + wave) - uv.y, uv.x - (xr + side));
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_hw(p);
    let k1 = kf_hw_1(p);
    let a = l3_da(uv);
    let b = l3_db(uv) * k1.z;
    let d = hw_flood(uv, k1.x, k1.y);
    let px = u.inv_size.y;
    var col = mix(a, b, clamp(0.5 - d / px, 0.0, 1.0));
    // Dark purple crest along the flood front.
    col = ak_over(col, ak_rgb(92.0, 50.0, 128.0), clamp(0.5 - (abs(d + 0.015) - 0.015) / px, 0.0, 1.0) * step(-0.5, k1.y));
    let c = to_c(uv);
    let hc = to_c(vec2f(k.x, k.y));
    let s = max(k.w, 1e-3);
    let q = ak_local(c, hc, k.z, s);
    if (abs(q.x) < 0.8 && abs(q.y) < 0.8) {
        let h = spr_witch_hat(q, px / s);
        col = ak_over(col, h.rgb, h.a);
    }
    return l3_out(col);
}

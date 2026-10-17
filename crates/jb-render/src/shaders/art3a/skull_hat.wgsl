// A skull rolls in from the left along the lower half while a witch hat
// glides in from the right along the upper half; each drags the incoming
// clip behind it. Table `sk`: [p, skull x, skull y, hat x, hat y | lower
// front (u, B to its left), upper front (u, B to its right), gain B, hat rot].

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_sk(p);
    let k1 = kf_sk_1(p);
    let px = u.inv_size.y;
    let a = l3_da(uv);
    let b = l3_db(uv) * k1.z;
    let wobble = 0.012 * sin(uv.y * 23.0);
    let d = select(uv.x - (k1.x + wobble), (k1.y + wobble) - uv.x, uv.y < 0.5);
    var col = mix(a, b, clamp(0.5 - d * u.size.x / u.size.y / px, 0.0, 1.0));
    let c = to_c(uv);
    // Skull, spinning slowly as it rolls.
    let sc = to_c(k.xy);
    let ss = 0.5;
    let qs = ak_local(c, sc, 0.25 * sin(p * 9.0), ss);
    if (abs(qs.x) < 0.6 && abs(qs.y) < 0.6) {
        let s = spr_skull(qs, px / ss);
        col = ak_over(col, s.rgb, s.a);
    }
    let hc = to_c(k.zw);
    let hs = 0.62;
    let qh = ak_local(c, hc, k1.w * 0.3, hs);
    if (abs(qh.x) < 0.8 && abs(qh.y) < 0.8) {
        let h = spr_witch_hat(qh, px / hs);
        col = ak_over(col, h.rgb, h.a);
    }
    return l3_out(col);
}

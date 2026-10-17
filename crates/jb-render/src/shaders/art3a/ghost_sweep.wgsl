// A sheet ghost floats in from the left and sweeps across, the incoming
// clip following it behind a rounded front. Table `gs`: [p, ghost x,
// ghost y, front (u at mid height), gain B | tilt, -, -, -].

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_gs(p);
    let k1 = kf_gs_1(p);
    let px = u.inv_size.y;
    let a = l3_da(uv);
    let b = l3_db(uv) * k.w;
    let yy = (uv.y - 0.4) / 0.5;
    let front = k.z - 0.2 * yy * yy;
    let d = (uv.x - front) * aspect();
    var col = mix(a, b, clamp(0.5 - d / px, 0.0, 1.0));
    let c = to_c(uv);
    let s = 0.62;
    let q = ak_local(c, to_c(k.xy), k1.x, s);
    if (abs(q.x) < 0.6 && abs(q.y) < 0.6) {
        let g = spr_ghost(q, px / s);
        col = ak_over(col, g.rgb, g.a);
    }
    return l3_out(col);
}

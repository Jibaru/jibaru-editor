// A crowd of sheet ghosts rises from the bottom until it fills the frame,
// then keeps drifting up and its ragged hems uncover the incoming clip.
// Table `gc`: [p, top (v of the front row line), hem (v of the hems), gain B, -].

fn gc_ghost(c: vec2f, cell: f32, row: f32, base: f32, w: f32, px: f32) -> vec4f {
    let h = hash12(vec2f(cell, row * 7.1));
    let cx = (cell + 0.5 + 0.35 * (h - 0.5)) * w;
    let cy = base + 0.08 * (hash12(vec2f(row, cell * 3.3)) - 0.5);
    let r = w * (0.36 + 0.08 * h);
    let q = c - vec2f(cx, cy);
    let d = op_smin(sd_circle(q, r), sd_box(q - vec2f(0.0, 2.0), vec2f(r * (1.05 + 0.25 * h), 2.0)), r * 0.4);
    var acc = vec4f(0.0);
    let fold = 0.5 + 0.5 * sin(q.x / r * 5.0 + h * 6.0 + q.y * 3.0);
    let lit = clamp(0.62 - 0.35 * q.x / r + 0.12 * fold - 0.25 * smoothstep(0.0, 1.0, q.y), 0.0, 1.0);
    let sheet = mix(ak_rgb(160.0, 152.0, 152.0), ak_rgb(212.0, 204.0, 202.0), lit);
    acc = spr_layer(acc, ak_rgb(130.0, 125.0, 130.0), spr_cov(d, px));
    acc = spr_layer(acc, sheet, spr_cov(d + 0.004, px));
    // Hollow eyes with red glints.
    let e = vec2f(abs(q.x) - r * 0.32, q.y + r * 0.05);
    let eye = sd_ellipse(e, vec2f(r * 0.13, r * 0.18));
    acc = spr_layer(acc, ak_rgb(40.0, 34.0, 38.0), spr_cov(eye, px) * acc.a);
    acc = spr_layer(acc, ak_rgb(235.0, 40.0, 40.0), spr_cov(length(e - vec2f(0.0, r * 0.03)) - r * 0.06, px) * acc.a);
    return acc;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_gc(p);
    let px = u.inv_size.y;
    let c = to_c(uv);
    let a = l3_da(uv);
    let b = l3_db(uv) * k.z;
    // Ragged hem: points of sheets hanging at different lengths.
    let hw = 0.11;
    let hc = floor(c.x / hw);
    let hf = fract(c.x / hw);
    let hem = (k.y - 0.5) + 0.12 * (hash11(hc * 1.7) - 0.5) + 0.07 * (1.0 - abs(hf - 0.5) * 2.0);
    var col = select(a, b, c.y > hem);
    if (c.y > hem + 0.01) { return l3_out(col); }
    var crowd = vec4f(0.0);
    let top = k.x - 0.5;
    for (var j = 0.0; j < 7.0; j = j + 1.0) {
        let w = 0.13 + 0.025 * j;
        let base = top + 0.05 + 0.14 * j;
        let cell = floor(c.x / w + 0.5 * fract(j * 0.5));
        let off = 0.5 * fract(j * 0.5);
        for (var dc = -1.0; dc <= 1.0; dc = dc + 1.0) {
            let g = gc_ghost(c + vec2f(off * w, 0.0), cell + dc, j, base, w, px);
            crowd = spr_layer(crowd, g.rgb, g.a);
        }
    }
    // The hems cut the crowd off at the bottom.
    let cut = clamp(0.5 - (c.y - hem) / px, 0.0, 1.0);
    col = ak_over(col, crowd.rgb, crowd.a * cut);
    return l3_out(col);
}

// A giant bat with raised wings flies straight up through the frame; the
// incoming clip rises behind it under a band of dark purple puffs.
// Table `bt`: [p, tip (v of the wing tips), gain of B, -].

const BT_TIPX: f32 = 0.82;

// Wing parameter: 0 at the shoulder, 1 at the tip.
fn bt_s(x: f32) -> f32 { return clamp((abs(x) - 0.1) / (BT_TIPX - 0.1), 0.0, 1.0); }
fn bt_upper(x: f32, tip: f32) -> f32 {
    let s = bt_s(x);
    return tip + 0.42 * pow(1.0 - s, 0.9) + 0.07 * sin(PI * s);
}
fn bt_lower(x: f32, tip: f32) -> f32 {
    let s = bt_s(x);
    return tip + 0.04 + 0.64 * pow(1.0 - s, 0.8) - 0.07 * abs(sin(3.0 * PI * s));
}
// Front of the incoming clip behind the bat, with puffs.
fn bt_front(x: f32, tip: f32) -> f32 {
    let s = clamp((abs(x) - 0.05) / (BT_TIPX + 0.05), 0.0, 1.0);
    return tip + 0.08 + 0.66 * pow(1.0 - s, 1.2);
}

fn bt_bat(c: vec2f, tip: f32, px: f32) -> vec4f {
    let ink = ak_rgb(26.0, 24.0, 30.0);
    let rib = ak_rgb(95.0, 92.0, 104.0);
    var acc = vec4f(0.0);
    let x = c.x;
    let ax = abs(x);
    // Wings.
    let dw = max(max(bt_upper(x, tip) - c.y, c.y - bt_lower(x, tip)), ax - BT_TIPX);
    let wing = select(1e3, dw, ax > 0.06);
    // Body, head and ears.
    let body = sd_ellipse(c - vec2f(0.0, tip + 0.58), vec2f(0.11, 0.18));
    let head = sd_circle(c - vec2f(0.0, tip + 0.43), 0.09);
    let ear = sd_tri(vec2f(ax, c.y), vec2f(0.02, tip + 0.39), vec2f(0.09, tip + 0.41), vec2f(0.075, tip + 0.29));
    let d = min(min(wing, body), min(head, ear));
    acc = spr_layer(acc, ink, spr_cov(d, px));
    // Light rim and wing bones.
    acc = spr_layer(acc, rib, spr_cov(abs(d + 0.004) - 0.0025, px) * acc.a);
    let sh = vec2f(0.1, tip + 0.5);
    let p = vec2f(ax, c.y);
    var bones = 1e3;
    for (var k = 1.0; k < 3.5; k = k + 1.0) {
        let s = k / 3.0;
        let xe = 0.1 + s * (BT_TIPX - 0.1);
        bones = min(bones, sd_seg(p, sh, vec2f(xe, bt_lower(xe, tip) - 0.005)));
    }
    acc = spr_layer(acc, rib, spr_cov(bones - 0.002, px) * step(0.06, ax) * spr_cov(wing, px));
    // Eyes and fangs.
    let eye = sd_ellipse(rot2(vec2f(ax, c.y) - vec2f(0.034, tip + 0.425), 0.4), vec2f(0.02, 0.011));
    acc = spr_layer(acc, ak_rgb(250.0, 210.0, 70.0), spr_cov(eye, px));
    let fang = sd_tri(vec2f(ax, c.y), vec2f(0.012, tip + 0.475), vec2f(0.032, tip + 0.475), vec2f(0.02, tip + 0.505));
    acc = spr_layer(acc, vec3f(0.96), spr_cov(fang, px));
    return acc;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_bt(p);
    let c = to_c(uv);
    let tip = k.x - 0.5;
    let px = u.inv_size.y;
    let a = l3_da(uv);
    let b = l3_db(uv) * k.y;
    // Puffs along the front.
    let fr = bt_front(c.x, tip);
    let puff = 0.025 * abs(sin(c.x * 26.0)) + 0.012 * sin(c.x * 61.0 + 1.0);
    let df = (fr - puff) - c.y;
    var col = mix(a, b, clamp(0.5 - df / px, 0.0, 1.0));
    col = ak_over(col, ak_rgb(70.0, 40.0, 92.0), clamp(0.5 - abs(df + 0.03) / (0.03 + px), 0.0, 1.0) * 0.85 * step(df, 0.0));
    let bat = bt_bat(c, tip, px);
    col = ak_over(col, bat.rgb, bat.a);
    return l3_out(col);
}

// Camera iris (original drawing): IR_N blades whose straight inner edges are
// tangent to the opening of inradius r, their seams curving outwards as
// spirals, inside a dark lens housing, with thin viewfinder marks on top.
// The style constants (IR_*) are emitted by the item; table `ir` rows are
// [p, r (fraction of height), rot (rad), side (0 A, 1 B), hud, rings, -, -].

fn ir_open(c: vec2f, r: f32, rot: f32) -> f32 {
    var d = -1e9;
    for (var k = 0.0; k < IR_N; k = k + 1.0) {
        d = max(d, dot(c, ak_dir(rot + k * TAU / IR_N)) - r);
    }
    // Slightly curved blade edges round the opening off.
    return mix(d, length(c) - r, IR_ROUND);
}

// Blade colour at c (display rgb) for an opening of inradius r.
fn ir_blades(c: vec2f, r: f32, rot: f32) -> vec3f {
    let rho = max(length(c), 1e-4);
    let a = atan2(c.y, c.x);
    let tw = IR_TWIST * log(max(rho, r + 0.02) / max(r + 0.02, 1e-3));
    let s = (a - rot - tw) / (TAU / IR_N);
    let f = fract(s);
    let k = floor(s);
    // Each blade darkens from its lit leading edge to the seam under the next.
    var g = IR_DARK + (IR_LIGHT - IR_DARK) * pow(f, 1.6);
    g = g * (0.92 + 0.16 * hash11(k + 7.0));
    // Thin dark seam where blades overlap, with a faint highlight beside it.
    let dseam = min(f, 1.0 - f) * TAU / IR_N * rho;
    var col = vec3f(g);
    col = ak_over(col, vec3f(IR_DARK * 0.45), ak_line(dseam, 0.0015) * 0.9);
    col = ak_over(col, vec3f(IR_LIGHT * 1.35), ak_fill(abs(dseam - 0.006) - 0.0012) * 0.35 * step(0.5, f));
    // Lit lip along the opening.
    return col;
}

fn ir_housing(c: vec2f, col0: vec3f) -> vec3f {
    let rho = length(c);
    var col = col0;
    // Outside the housing the lens body is darker, with a soft bevel.
    let hb = rho - IR_HOUSING;
    col = ak_over(col, vec3f(IR_BODY), ak_soft(-hb, 0.01));
    col = ak_over(col, vec3f(IR_LIGHT * 1.2), ak_line(hb, 0.002) * 0.6);
    return col;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_ir(p);
    let r = k.x;
    let rot = k.y;
    let c = to_c(uv);
    var col = ak_base(uv, k.z);
    let dopen = ir_open(c, r, rot);
    if (dopen > -0.004) {
        var blade = ir_housing(c, ir_blades(c, r, rot));
        // Bright lip of the opening edge.
        blade = ak_over(blade, vec3f(IR_LIGHT * 1.6), ak_line(dopen - 0.002, 0.0012) * 0.5);
        col = ak_over(col, blade, ak_fill(-dopen));
    }
    col = ir_hud(c, col, k.w, kf_ir_1(p).x, r);
    return l3_out(col);
}

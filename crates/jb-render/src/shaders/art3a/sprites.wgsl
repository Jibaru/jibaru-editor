// Original cartoon sprites shared by the art3a items. Each `spr_*` takes a
// point in the sprite's own frame (centred units, sprite about 1 unit tall
// before scaling, y down) and the size of one output pixel in that frame,
// and returns straight colour + coverage.

fn spr_layer(acc: vec4f, col: vec3f, a: f32) -> vec4f {
    // Paints `col` with coverage `a` over the accumulated sprite.
    let aa = clamp(a, 0.0, 1.0);
    return vec4f(mix(acc.rgb, col, aa), acc.a + aa * (1.0 - acc.a));
}
fn spr_cov(d: f32, px: f32) -> f32 { return clamp(0.5 - d / px, 0.0, 1.0); }

// Cartoon skull facing right: round cranium, jaw, dark sockets with glowing
// orange irises, a small nose and a row of teeth.
fn spr_skull(q: vec2f, px: f32) -> vec4f {
    let bone = ak_rgb(250.0, 248.0, 240.0);
    let shade = ak_rgb(214.0, 208.0, 196.0);
    let ink = ak_rgb(40.0, 34.0, 36.0);
    var acc = vec4f(0.0);
    let cran = sd_circle(q - vec2f(0.0, -0.12), 0.36);
    let cheek = sd_rbox(q - vec2f(0.05, 0.12), vec2f(0.25, 0.14), 0.1);
    let jaw = sd_rbox(q - vec2f(0.08, 0.3), vec2f(0.17, 0.09), 0.07);
    let skull = op_smin(op_smin(cran, cheek, 0.06), jaw, 0.04);
    acc = spr_layer(acc, ink, spr_cov(skull - 0.014, px));
    acc = spr_layer(acc, mix(bone, shade, smoothstep(-0.1, 0.35, q.y - 0.4 * q.x)), spr_cov(skull, px));
    // Eye sockets with glowing irises.
    for (var k = 0.0; k < 2.0; k = k + 1.0) {
        let ec = vec2f(-0.06 + 0.25 * k, 0.03);
        let sock = sd_ellipse(q - ec, vec2f(0.095, 0.105));
        acc = spr_layer(acc, ink, spr_cov(sock, px));
        let ir = length(q - ec - vec2f(0.015, 0.0));
        acc = spr_layer(acc, mix(ak_rgb(255.0, 170.0, 60.0), ak_rgb(230.0, 70.0, 30.0), smoothstep(0.0, 0.05, ir)), spr_cov(ir - 0.05, px));
        acc = spr_layer(acc, ink, spr_cov(ir - 0.018, px));
    }
    // Nose and teeth.
    let nose = sd_tri(q, vec2f(0.08, 0.13), vec2f(0.13, 0.2), vec2f(0.04, 0.2));
    acc = spr_layer(acc, ink, spr_cov(nose, px));
    let tq = q - vec2f(0.08, 0.3);
    let teeth = max(abs(tq.y) - 0.045, abs(fract(tq.x / 0.06 + 0.5) - 0.5) * 0.06 - 0.006);
    acc = spr_layer(acc, ink, spr_cov(max(teeth, abs(tq.x) - 0.15), px));
    acc = spr_layer(acc, ink, spr_cov(max(abs(tq.y) - 0.006, abs(tq.x) - 0.16), px));
    return acc;
}

// Friendly sheet ghost drifting right: round head, a body that flares into
// a wavy hem trailing to the left, a little arm, dot eyes and blush.
fn spr_ghost(q: vec2f, px: f32) -> vec4f {
    let sheet = ak_rgb(252.0, 248.0, 244.0);
    let shade = ak_rgb(222.0, 214.0, 214.0);
    let ink = ak_rgb(48.0, 30.0, 34.0);
    var acc = vec4f(0.0);
    let head = sd_circle(q - vec2f(0.06, -0.22), 0.24);
    // Body: a tapered capsule leaning back, hem waves at the bottom.
    let body = sd_taper(q, vec2f(0.04, -0.15), vec2f(-0.12, 0.3), 0.23, 0.2);
    let hem = q.y - (0.4 + 0.05 * sin(q.x * 22.0 + 1.0));
    let tail = sd_taper(q, vec2f(-0.1, 0.25), vec2f(-0.42, 0.42), 0.14, 0.02);
    var g = op_smin(head, max(body, hem), 0.06);
    g = op_smin(g, tail, 0.05);
    let arm = sd_taper(q, vec2f(0.12, 0.0), vec2f(0.33, 0.05), 0.06, 0.035);
    g = op_smin(g, arm, 0.03);
    acc = spr_layer(acc, mix(sheet, shade, smoothstep(0.0, 0.45, q.y - 0.5 * q.x + 0.1)), spr_cov(g, px));
    // Face.
    let fq = q - vec2f(0.12, -0.24);
    let eyes = min(sd_ellipse(fq - vec2f(-0.07, 0.0), vec2f(0.022, 0.03)), sd_ellipse(fq - vec2f(0.07, 0.0), vec2f(0.022, 0.03)));
    acc = spr_layer(acc, ink, spr_cov(eyes, px));
    let blush = min(sd_ellipse(fq - vec2f(-0.1, 0.06), vec2f(0.035, 0.018)), sd_ellipse(fq - vec2f(0.1, 0.06), vec2f(0.035, 0.018)));
    acc = spr_layer(acc, ak_rgb(246.0, 160.0, 160.0), spr_cov(blush, px) * 0.8);
    let smile = max(abs(length(fq - vec2f(0.0, 0.02)) - 0.04) - 0.006, -(fq.y - 0.04));
    acc = spr_layer(acc, ink, spr_cov(smile, px));
    return acc;
}

// Big "boo" ghost facing the viewer, about 1 unit tall: domed head, a
// skirt flaring to a wavy hem, little arms, tall oval eyes high on the head
// and a wide open grin with a teal tongue hanging out. Mouth centre at
// (0, -0.07).
fn spr_boo_mouth(q: vec2f) -> f32 {
    let qm = q - vec2f(0.0, -0.1);
    return max(sd_ellipse(qm, vec2f(0.17, 0.15)), -qm.y);
}
fn spr_boo(q: vec2f, px: f32) -> vec4f {
    let sheet = ak_rgb(252.0, 252.0, 250.0);
    let shade = ak_rgb(214.0, 220.0, 226.0);
    let ink = ak_rgb(26.0, 26.0, 30.0);
    let tongue = ak_rgb(150.0, 205.0, 210.0);
    var acc = vec4f(0.0);
    let dome = sd_circle(q - vec2f(0.0, -0.24), 0.23);
    let skirt = sd_taper(q, vec2f(0.0, -0.2), vec2f(0.0, 0.35), 0.21, 0.29);
    let hem = q.y - (0.44 + 0.05 * sin(q.x * 25.0));
    var g = op_smin(dome, max(skirt, hem), 0.05);
    let arms = sd_taper(vec2f(abs(q.x), q.y), vec2f(0.2, 0.08), vec2f(0.36, 0.2), 0.07, 0.04);
    g = op_smin(g, arms, 0.04);
    acc = spr_layer(acc, mix(sheet, shade, smoothstep(0.1, 0.5, q.y) * 0.5 + 0.35 * smoothstep(0.12, 0.3, abs(q.x))), spr_cov(g, px));
    let eye = sd_ellipse(vec2f(abs(q.x), q.y) - vec2f(0.1, -0.3), vec2f(0.065, 0.085));
    acc = spr_layer(acc, ink, spr_cov(eye, px));
    acc = spr_layer(acc, sheet, spr_cov(sd_circle(vec2f(abs(q.x), q.y) - vec2f(0.115, -0.33), 0.02), px));
    let m = spr_boo_mouth(q);
    acc = spr_layer(acc, ink, spr_cov(m, px));
    let t = sd_ellipse(q - vec2f(0.0, 0.08), vec2f(0.075, 0.16));
    acc = spr_layer(acc, tongue, spr_cov(max(t, -(q.y + 0.02)), px));
    acc = spr_layer(acc, ak_rgb(110.0, 165.0, 172.0), spr_cov(max(abs(q.x) - 0.004, max(t, -(q.y - 0.02))), px));
    return acc;
}

// Bat silhouette, wingspan 2 units (x -1..1), body at the origin. `flap`
// in -1..1 raises (+) or lowers (-) the wing tips. Returns the signed
// distance (negative inside) so callers can colour or glow it.
fn spr_bat_d(q: vec2f, flap: f32) -> f32 {
    let ax = abs(q.x);
    let s = clamp((ax - 0.08) / 0.92, 0.0, 1.0);
    let lift = -0.6 * flap * s * s;
    let up = lift - 0.12 * (1.0 - s) + 0.06 * sin(PI * s) * 0.5;
    let lo = lift + 0.1 + 0.18 * (1.0 - s) - 0.08 * abs(sin(3.0 * PI * s));
    let wing = max(max(up - q.y, q.y - lo), ax - 1.0);
    let body = sd_ellipse(q - vec2f(0.0, 0.05), vec2f(0.1, 0.17));
    let head = sd_circle(q - vec2f(0.0, -0.13), 0.1);
    let ear = sd_tri(vec2f(ax, q.y), vec2f(0.015, -0.17), vec2f(0.075, -0.17), vec2f(0.07, -0.27));
    return min(min(wing, body), min(head, ear));
}

// Witch hat: a bent cone over a wide brim, with a band and a square buckle.
fn spr_witch_hat(q: vec2f, px: f32) -> vec4f {
    let purple = ak_rgb(112.0, 62.0, 150.0);
    let purple_d = ak_rgb(74.0, 38.0, 108.0);
    let band = ak_rgb(240.0, 120.0, 40.0);
    let gold = ak_rgb(250.0, 220.0, 120.0);
    var acc = vec4f(0.0);
    // Brim: flat ellipse.
    let brim = sd_ellipse(q - vec2f(0.0, 0.32), vec2f(0.62, 0.14));
    acc = spr_layer(acc, purple_d, spr_cov(brim, px));
    acc = spr_layer(acc, purple, spr_cov(sd_ellipse(q - vec2f(0.0, 0.29), vec2f(0.56, 0.1)), px));
    // Cone: a triangle whose tip bends to the right.
    let bend = 0.18 * smoothstep(0.1, -0.5, q.y) * smoothstep(0.1, -0.5, q.y);
    let qc = vec2f(q.x - bend, q.y);
    let cone = sd_tri(qc, vec2f(-0.3, 0.3), vec2f(0.3, 0.3), vec2f(0.02, -0.55));
    let shade = smoothstep(-0.05, 0.25, qc.x);
    acc = spr_layer(acc, mix(purple, purple_d, shade), spr_cov(cone - 0.02, px));
    // Band and buckle.
    let bandd = max(cone - 0.02, abs(q.y - 0.19) - 0.06);
    acc = spr_layer(acc, band, spr_cov(bandd, px));
    let buckle = sd_box(q - vec2f(-0.02, 0.19), vec2f(0.1, 0.075));
    let hole = sd_box(q - vec2f(-0.02, 0.19), vec2f(0.055, 0.035));
    acc = spr_layer(acc, gold, spr_cov(op_sub(buckle, hole), px));
    // Dark outline around everything.
    let outline = min(min(brim, cone - 0.02), 1e9);
    acc = spr_layer(acc, purple_d * 0.6, spr_cov(abs(outline) - 0.012, px) * acc.a);
    return acc;
}

// Candy spiral: a striped candy spiral (white, salmon, blue, mint and
// orange bands widening outwards) winds out from the centre over the
// outgoing clip with a mint gem spinning at its heart and a spotted cartoon
// blob bouncing past; it then unwinds, opening from the centre on the
// incoming clip while its outer turns slide off. Table `cs`: [p, cover
// radius, hole radius, outer radius, spin | gem scale, blob x, blob y, blob scale].

fn cs_band(w: f32) -> vec3f {
    var E = array<f32, 10>(0.135, 0.236, 0.337, 0.404, 0.472, 0.618, 0.697, 0.775, 0.888, 1.0);
    var C = array<vec3f, 10>(
        vec3f(0.99, 0.99, 0.99), vec3f(0.92, 0.50, 0.51), vec3f(0.34, 0.47, 0.73), vec3f(0.45, 0.95, 0.85),
        vec3f(0.92, 0.50, 0.51), vec3f(0.98, 0.74, 0.40), vec3f(0.92, 0.50, 0.51), vec3f(0.45, 0.95, 0.85),
        vec3f(0.34, 0.47, 0.73), vec3f(0.92, 0.50, 0.51));
    let f = fract(w);
    for (var i = 0; i < 10; i = i + 1) { if (f < E[i]) { return C[i]; } }
    return C[9];
}

// Mint gem: a hexagon with facet lines.
fn cs_gem(q: vec2f, px: f32) -> vec4f {
    let d = sd_ngon(q, 0.5, 6.0);
    if (d > 0.05) { return vec4f(0.0); }
    var col = vec3f(0.48, 0.96, 0.86);
    let inner = sd_ngon(q, 0.26, 6.0);
    col = mix(col, vec3f(0.62, 1.0, 0.92), ak_fill(inner / 1.0) * 0.6);
    let a = atan2(q.x, -q.y);
    let spoke = abs(fract(a / (PI / 3.0)) - 0.5) * length(q);
    col = mix(col, vec3f(0.15, 0.35, 0.33), clamp(0.5 - (abs(inner) - 0.012) / px, 0.0, 1.0) * 0.8);
    col = mix(col, vec3f(0.15, 0.35, 0.33), clamp(0.5 - (spoke - 0.01) / px, 0.0, 1.0) * step(0.26, length(q)) * 0.6);
    col = mix(col, vec3f(0.12, 0.25, 0.25), clamp(0.5 - (abs(d) - 0.02) / px, 0.0, 1.0));
    return vec4f(col, clamp(0.5 - (d - 0.02) / px, 0.0, 1.0));
}

// Spotted cartoon blob with a yellow rim.
fn cs_blob(q: vec2f, px: f32) -> vec4f {
    let d = sd_ellipse(q, vec2f(0.5, 0.42));
    if (d > 0.08) { return vec4f(0.0); }
    var col = vec3f(0.50, 0.80, 0.66);
    let s1 = min(sd_circle(q - vec2f(-0.2, -0.15), 0.07), sd_circle(q - vec2f(0.18, -0.22), 0.05));
    let s2 = min(sd_circle(q - vec2f(0.25, 0.12), 0.08), sd_circle(q - vec2f(-0.15, 0.2), 0.06));
    col = mix(col, vec3f(0.26, 0.40, 0.75), clamp(0.5 - min(s1, s2) / px, 0.0, 1.0));
    col = mix(col, vec3f(0.98, 0.78, 0.30), clamp(0.5 - (abs(d - 0.02) - 0.025) / px, 0.0, 1.0));
    col = mix(col, vec3f(0.15, 0.2, 0.2), clamp(0.5 - (abs(d - 0.05) - 0.008) / px, 0.0, 1.0));
    return vec4f(col, clamp(0.5 - (d - 0.058) / px, 0.0, 1.0));
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_cs(p);
    let k1 = kf_cs_1(p);
    let c0 = to_c(uv);
    let c = c0 - vec2f(-0.08, 0.04);
    let r = max(length(c), 1e-4);
    let turn = atan2(c.y, c.x) / TAU;
    // Edges follow the spiral arm.
    let re = r + 0.12 * (fract(turn - k.w) - 0.5);
    let w = r / 0.45 - turn + k.w;
    let px = ak_px();
    let cover = clamp(0.5 - (re - k.x) / px, 0.0, 1.0)
        * clamp(0.5 - (k.y - re) / px, 0.0, 1.0)
        * clamp(0.5 - (re - k.z) / px, 0.0, 1.0);
    let opened = clamp(0.5 - (re - k.y) / px, 0.0, 1.0) * step(0.001, k.y);
    var col = mix(l3_da(uv), l3_db(uv), max(opened, step(k.z, re) * step(0.01, k.y)));
    // Thin dark lines between the bands.
    let edge = abs(fract(w * 10.0 / 1.0) - 0.5);
    col = mix(col, cs_band(w), cover);
    if (k1.x > 0.0) {
        let g = cs_gem(rot2(c0, p * 6.0) / k1.x, px / k1.x);
        col = ak_over(col, g.rgb, g.a);
    }
    if (k1.w > 0.0) {
        let b = cs_blob((c0 - to_c(k1.yz)) / k1.w, px / k1.w);
        col = ak_over(col, b.rgb, b.a);
    }
    return l3_out(col);
}

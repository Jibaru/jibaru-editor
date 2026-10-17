// Edge Glow Pulse: a soft vertical band sweeps across the picture from left
// to right, over and over, and wherever it passes the edges of the picture
// light up as glowing white lines. The pre pass finds the edges; its blur is
// the glow.
//@time sec 1.3013
//@pre 0.006
//@p0 band centre (frame widths), half width, strength
//@ 0.0000 0.10 0.20 0.00
//@ 0.0667 0.10 0.20 0.00
//@ 0.2669 0.11 0.20 0.60
//@ 0.4338 0.24 0.20 1.00
//@ 0.9009 0.75 0.20 1.00
//@ 1.0677 0.80 0.20 0.45
//@ 1.2012 0.86 0.20 0.00
//@ 1.3013 0.86 0.20 0.00

fn edges(uv: vec2f) -> f32 {
    let px = vec2f(1.0 / 960.0, 1.0 / 540.0) * 1.5;
    let l = luma(a1_da(uv - vec2f(px.x, 0.0)));
    let r = luma(a1_da(uv + vec2f(px.x, 0.0)));
    let t = luma(a1_da(uv - vec2f(0.0, px.y)));
    let b = luma(a1_da(uv + vec2f(0.0, px.y)));
    return smoothstep(0.1, 0.3, length(vec2f(r - l, b - t)));
}

fn pre(uv: vec2f) -> vec4f {
    let e = edges(uv);
    return vec4f(e, e, e, 1.0);
}

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    var col = a1_da(uv);
    let band = k.z * (1.0 - smoothstep(0.5 * k.y, k.y, abs(uv.x - k.x)));
    if (band > 0.0) {
        let e = edges(uv);
        let glow = sx(uv).r;
        col = a1_add(col, band * (vec3f(1.0) * 0.6 * e + vec3f(0.95, 0.95, 1.0) * 1.0 * glow));
    }
    return a1_out_a(col, a1_alpha(uv));
}

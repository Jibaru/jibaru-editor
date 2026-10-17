// Focus Pull Bokeh: the lens racks out of focus and back. The picture
// softens while its highlights swell into soft glowing discs, holds, then
// snaps back to sharp. The pre pass keeps the picture (rgb) and its
// highlight mask (alpha); both are blurred into the aux texture.
//@time sec 2.002
//@pre 0.01
//@p0 defocus, highlight swell, brighten
//@ 0.000 0.00 0.0 0.0
//@ 0.600 1.00 1.0 1.0
//@ 1.100 1.00 1.0 1.0
//@ 1.800 0.00 0.0 0.0

fn pre(uv: vec2f) -> vec4f {
    let c = a1_da(uv);
    let hl = smoothstep(0.8, 0.95, luma(c));
    return vec4f(sa(uv).rgb, hl);
}

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let sharp = a1_da(uv);
    let bx = sx(uv);
    let soft = to_disp(vec4f(bx.rgb, 1.0)).rgb;
    var col = mix(sharp, soft, 0.9 * k.x);
    // highlights spread into discs: the blurred mask is pushed up to full
    // strength wherever enough bright pixels are near
    let disc = smoothstep(0.015, 0.05, bx.a) * k.y;
    let hcol = mix(soft, vec3f(1.0, 0.95, 1.0), 0.8);
    col = max(col, mix(col, hcol, disc * 0.95));
    col = a1_add(col, k.z * (0.0 * soft + 0.9 * vec3f(1.0, 0.9, 1.0) * bx.a));
    return a1_out_a(col, a1_alpha(uv));
}

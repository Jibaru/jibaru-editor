// Ember Flash Leak: a warm red leak builds over the old clip and bleaches the
// frame white; the white then contracts towards a hot spot at the upper right
// with a crisp rim, through yellow into a deep ember red, which clears on the
// new clip. The fitted colour field of the stand-in (`//@base`) carries the
// broad timing and tints; the contracting flare is drawn here.
//@base jb.leak_wash_01
//@p0 unused, flare opacity, white radius, yellow radius
//@ 0.000 0.00 0.00 3.0 4.0
//@ 0.483 0.00 0.00 3.0 4.0
//@ 0.500 0.00 1.00 1.75 2.6
//@ 0.517 0.00 1.00 1.25 2.2
//@ 0.533 0.00 1.00 0.10 1.5
//@ 0.550 0.00 1.00 0.00 1.2
//@ 0.567 0.00 1.00 0.00 0.6
//@ 0.583 0.00 0.00 0.00 0.4
//@p1 hot spot x, y
//@ 0.00 0.70 -0.65
//@ 0.533 0.55 -0.55
//@ 0.567 0.15 -0.35

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let p = to_c(uv);
    var col = a1_base(uv);
    // the flare: a white disc with a crisp rim shrinking into a yellow body,
    // orange then ember red outside
    let d = length((p - u.p[1].xy) * vec2f(0.85, 1.0)) + 0.06 * (a1_fbm(p * 2.5 + vec2f(a1_t() * 2.0, 0.0), 3) - 0.5);
    let ry = max(k.w, 1e-3);
    var fl = mix(vec3f(1.0, 0.98, 0.75), vec3f(1.0, 0.93, 0.18), smoothstep(0.0, 0.5 * ry, d));
    fl = mix(fl, vec3f(1.0, 0.55, 0.04), smoothstep(0.55 * ry, ry, d));
    fl = mix(fl, vec3f(0.93, 0.2, 0.06), smoothstep(ry, 1.5 * ry, d));
    fl = mix(vec3f(1.0, 1.0, 0.98), fl, smoothstep(k.z - 0.12, k.z + 0.06, d));
    col = mix(col, fl, k.y);
    return a1_out(col);
}

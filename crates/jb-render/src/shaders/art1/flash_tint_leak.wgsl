// Flash Tint Leak: the picture is printed a little darker, like an aged
// print, and part-way through a soft light leak washes over it, lavender,
// then bleached cream, peach and a red burn that fades into a faint red cast.
// Timing and tints follow Filmora's Aesthetic Light Leak 04 (measured on its
// export); the leak is drawn here.
//@time sec 2.002
//@p1 leak colour, opacity
//@ 0.0000 0.80 0.50 0.70 0.00
//@ 0.6507 0.80 0.50 0.70 0.00
//@ 0.6673 0.80 0.55 0.72 0.38
//@ 0.7508 0.80 0.55 0.72 0.38
//@ 0.7674 0.93 0.90 0.93 0.86
//@ 0.8508 0.93 0.90 0.93 0.86
//@ 0.8675 0.97 0.78 0.72 0.82
//@ 0.9176 0.97 0.78 0.72 0.82
//@ 0.9343 0.97 0.58 0.42 0.78
//@ 0.9509 0.97 0.58 0.42 0.78
//@ 0.9676 0.88 0.26 0.20 0.62
//@ 1.0177 0.88 0.26 0.20 0.62
//@ 1.0344 0.80 0.22 0.28 0.32
//@ 1.1178 0.80 0.22 0.28 0.32
//@ 1.1345 0.80 0.40 0.50 0.06
//@ 1.3347 0.80 0.40 0.50 0.00

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let p = to_c(uv);
    var col = a1_da(uv) * vec3f(0.83, 0.86, 0.83);
    let l = u.p[1];
    if (l.w > 0.0) {
        // the leak is uneven: brighter where its soft folds gather
        let n = a1_fbm(p * 1.3 + vec2f(0.4 * t, -0.2 * t), 3);
        let a = clamp(l.w * (0.8 + 0.5 * (n - 0.5) + 0.15 * smoothstep(0.6, -0.6, p.x - p.y)), 0.0, 1.0);
        col = mix(col, l.rgb * (0.95 + 0.1 * n), a);
    }
    return a1_out_a(col, a1_alpha(uv));
}

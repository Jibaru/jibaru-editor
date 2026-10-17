// Sunlit Haze Leak: a low sun flares in at the upper left and throws a broad
// diagonal shaft of white light across the top of the old clip, which warms
// to orange; the new clip rises from the bottom of the frame up to the shaft,
// which fades last.
//@p0 light intensity, shaft half width, warm tint, reveal threshold
//@ 0.00 0.00 0.15 0.00 2.0
//@ 0.07 0.30 0.15 0.25 2.0
//@ 0.15 0.55 0.25 0.45 2.0
//@ 0.25 1.00 0.38 0.90 2.0
//@ 0.35 1.00 0.36 0.55 1.4
//@ 0.45 1.00 0.32 0.30 1.0
//@ 0.55 0.92 0.26 0.15 0.80
//@ 0.67 0.90 0.24 0.10 0.50
//@ 0.78 0.85 0.22 0.05 0.15
//@ 0.85 0.65 0.20 0.00 -0.10
//@ 0.95 0.15 0.12 0.00 -0.50
//@ 1.00 0.00 0.12 0.00 -1.00
//@p1 sun x, y, shaft slope
//@ 0.00 -0.95 0.25 -0.28
//@ 0.20 -0.90 0.10 -0.28
//@ 0.35 -0.86 -0.10 -0.26
//@ 0.50 -0.85 -0.20 -0.25
//@ 1.00 -0.85 -0.25 -0.22
//@p2 second, reflected glow at the lower right: opacity
//@ 0.40 0.0
//@ 0.50 0.7
//@ 0.60 0.8
//@ 0.70 0.5
//@ 0.80 0.0

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let t = a1_t();
    let p = to_c(uv);
    let s = u.p[1].xy;
    let dir = normalize(vec2f(1.0, u.p[1].z));
    let rel = p - s;
    // signed distance below the shaft's axis, ragged
    let below = dot(rel, vec2f(-dir.y, dir.x)) + 0.06 * (a1_fbm(p * 2.5 + vec2f(t, 0.0), 3) - 0.5);
    let w = smoothstep(-0.08, 0.08, below - k.w);
    var col = mix(a1_da(uv), a1_db(uv), w);
    // warm orange cast, strongest away from the new clip
    col = mix(col, vec3f(1.0, 0.68, 0.25) * (0.45 + 0.75 * luma(col)), 0.8 * k.z * (1.0 - 0.6 * w));
    // the shaft and the sun
    let along = max(dot(rel, dir), 0.0);
    let shaft = a1_gauss(below, k.y * (1.0 + 0.6 * along)) * exp(-along / 2.2);
    let sun = a1_halo(length(rel * vec2f(0.8, 1.0)), 0.18);
    let light = k.x * clamp(shaft + 0.8 * sun, 0.0, 1.2);
    col = mix(col, vec3f(1.0, 0.96, 0.75), clamp(light, 0.0, 1.0));
    col = a1_add(col, vec3f(1.0, 0.8, 0.3) * 0.45 * k.x * a1_gauss(below, 3.0 * k.y));
    let g2 = u.p[2].x * a1_gauss(length((p - vec2f(0.75, 0.3)) * vec2f(0.7, 1.0)), 0.35);
    col = mix(col, vec3f(0.95, 0.93, 0.8), clamp(g2, 0.0, 1.0));
    return a1_out(col);
}

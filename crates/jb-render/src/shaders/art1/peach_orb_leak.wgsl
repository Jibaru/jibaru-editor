// Peach Orb Leak: big out-of-focus orange orbs drift in from the corners of
// the old clip, swell and melt into a cream-white flash, which warms to gold
// at the edges and clears on the new clip. The fitted colour field of the
// stand-in (`//@base`) carries the broad timing and tints; the orbs are drawn
// here; the cream wash and the late reveal follow the timing measured on
// Filmora's export.
//@p0 cream wash, golden edge, new clip, warmth of the wash
//@ 0.00 0.00 0.00 0.00 1.0
//@ 0.15 0.08 0.00 0.00 1.0
//@ 0.20 0.30 0.00 0.00 1.0
//@ 0.25 0.55 0.00 0.00 0.9
//@ 0.30 0.72 0.05 0.00 0.7
//@ 0.35 0.84 0.05 0.03 0.5
//@ 0.42 0.93 0.10 0.06 0.2
//@ 0.50 1.00 0.10 0.08 0.0
//@ 0.55 0.97 0.25 0.12 0.0
//@ 0.62 0.90 0.60 0.20 0.0
//@ 0.70 0.78 0.80 0.35 0.0
//@ 0.77 0.62 0.80 0.50 0.0
//@ 0.85 0.42 0.70 0.70 0.0
//@ 0.90 0.25 0.50 0.85 0.0
//@ 0.95 0.08 0.25 0.95 0.0
//@ 1.00 0.00 0.00 1.00 0.0
//@p1 orb 1 (upper right): x, y, radius, opacity
//@ 0.00 1.05 -0.55 0.18 0.0
//@ 0.07 0.90 -0.50 0.20 0.9
//@ 0.20 0.72 -0.40 0.25 1.0
//@ 0.33 0.62 -0.36 0.28 0.45
//@ 0.45 0.55 -0.30 0.32 0.0
//@p2 orb 2 (lower left): x, y, radius, opacity
//@ 0.00 -1.00 0.45 0.20 0.0
//@ 0.07 -0.90 0.40 0.22 0.8
//@ 0.20 -0.70 0.20 0.30 1.0
//@ 0.33 -0.60 0.10 0.34 0.45
//@ 0.45 -0.50 0.05 0.38 0.0
//@p3 orb 3 (centre left): x, y, radius, opacity
//@ 0.00 -0.50 -0.10 0.15 0.0
//@ 0.15 -0.45 -0.05 0.18 0.0
//@ 0.27 -0.40 0.00 0.25 0.7
//@ 0.40 -0.35 0.00 0.30 0.6
//@ 0.47 -0.30 0.00 0.34 0.0

fn fx(uv: vec2f) -> vec4f {
    let p = to_c(uv);
    let k = u.p[0];
    var col = mix(a1_da(uv), a1_db(uv), k.z);
    // cream-white flood, brightest at the upper left, warming to gold at the edges
    let vig = max(smoothstep(0.3, 0.85, length(p * vec2f(0.8, 1.2) - vec2f(-0.05, 0.1))), smoothstep(-0.2, -0.5, p.y));
    let lift = 0.5 + 0.5 * a1_gauss(length(p - vec2f(-0.4, -0.35)), 0.6);
    var wash = mix(vec3f(0.98, 0.93, 0.84), vec3f(1.0, 0.99, 0.95), lift);
    wash = mix(wash, vec3f(1.0, 0.82, 0.5), k.w);
    wash = mix(wash, vec3f(0.93, 0.6, 0.15), k.y * vig);
    col = mix(col, wash, k.x);
    let o1 = a1_orb(p, u.p[1], vec3f(1.0, 0.62, 0.38), vec3f(1.0, 0.5, 0.25));
    let o2 = a1_orb(p, u.p[2], vec3f(1.0, 0.85, 0.35), vec3f(1.0, 0.52, 0.25));
    let o3 = a1_orb(p, u.p[3], vec3f(1.0, 0.75, 0.62), vec3f(1.0, 0.62, 0.5));
    col = mix(col, o1.rgb, clamp(o1.a, 0.0, 1.0));
    col = mix(col, o2.rgb, clamp(o2.a, 0.0, 1.0));
    col = mix(col, o3.rgb, clamp(0.8 * o3.a, 0.0, 1.0));
    // a hot streak inside the lower orb
    let q = rot2(p - u.p[2].xy - vec2f(0.05, -0.02), 0.35);
    let streak = a1_gauss(q.y, 0.05 * u.p[2].z / 0.3) * a1_gauss(q.x, 0.6 * u.p[2].z);
    col = a1_add(col, vec3f(1.0, 0.95, 0.6) * 0.6 * u.p[2].w * streak);
    return a1_out(col);
}

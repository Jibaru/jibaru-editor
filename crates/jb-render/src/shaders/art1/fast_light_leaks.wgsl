// Fast Light Leaks: a short burst of burnt film light at the start of the clip.
// For about half a second the picture is replaced by churning cream-pink
// light with dark maroon smoke between the flares (animated at 15 frames
// per second, like hand-cranked film), it bleaches nearly white, collapses
// back to smoke and then thins out over a slightly rippling picture.
//@time sec 2.002
//@p0 opacity, bright coverage, ripple, warm streak
//@ 0.000 0.00 0.00 0.00 0.0
//@ 0.070 0.00 0.00 0.00 0.0
//@ 0.120 0.10 0.00 0.00 0.0
//@ 0.185 0.30 0.00 0.00 0.0
//@ 0.186 1.00 0.66 0.00 0.0
//@ 0.330 1.00 0.68 0.00 0.0
//@ 0.331 1.00 0.82 0.00 0.6
//@ 0.400 1.00 0.93 0.00 0.2
//@ 0.630 1.00 0.98 0.00 0.0
//@ 0.631 1.00 0.70 0.00 0.0
//@ 0.697 1.00 0.70 0.00 0.0
//@ 0.698 0.45 0.60 1.00 0.0
//@ 0.800 0.30 0.58 1.00 0.0
//@ 0.934 0.15 0.55 0.60 0.0
//@ 0.968 0.00 0.00 0.00 0.0

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let t = a1_t();
    let tq = floor(t * 15.0) / 15.0;
    let p = to_c(uv);
    // the picture ripples a little as the leak thins out
    let rip = k.z * 0.006 * vec2f(sin(p.y * 40.0 + t * 30.0), sin(p.x * 30.0 - t * 25.0));
    let base = a1_da(uv + rip);
    // churning light: vertical flares drifting sideways, re-drawn every other frame
    let w = vec2f(a1_fbm(p * 2.0 + vec2f(tq * 3.0, 1.7), 3), a1_fbm(p * 2.0 + vec2f(4.1, tq * 2.0), 3));
    let n = a1_fbm(p * vec2f(1.8, 0.9) + 0.9 * (w - 0.5) + vec2f(tq * 2.2, -tq * 1.3), 4);
    let cov = clamp(k.y, 0.001, 0.999);
    let thr = 0.5 + 0.13 * log((1.0 - cov) / cov);
    let lit = smoothstep(thr - 0.05, thr + 0.05, n);
    let smoke = mix(vec3f(0.16, 0.05, 0.04), vec3f(0.42, 0.18, 0.15), smoothstep(thr - 0.25, thr, n));
    let glow = mix(vec3f(1.0, 0.86, 0.85), vec3f(1.0, 0.97, 0.95), smoothstep(thr, thr + 0.2, n));
    var leak = mix(smoke, glow, lit);
    // a warm amber streak through the first bleach
    let st = a1_gauss(p.x + 0.35 - 0.3 * p.y, 0.12) * k.w;
    leak = mix(leak, vec3f(0.85, 0.55, 0.12), clamp(st * (1.0 - 0.6 * lit), 0.0, 1.0));
    // film vignette
    leak = leak * (1.0 - 0.15 * smoothstep(0.45, 0.75, length(p * vec2f(0.75, 1.0))));
    // the tail is a pale veil rather than flares
    leak = mix(leak, vec3f(0.78, 0.68, 0.7) + 0.15 * (n - 0.5), 0.45 * k.z);
    // film grain and a few vertical scratches, new every other frame
    let g = hash12(floor(uv * vec2f(640.0, 360.0)) + tq * 91.0) - 0.5;
    let sx = a1_r(tq * 15.0, 3.0) * 1.8 - 0.9;
    let scratch = a1_gauss(p.x - sx, 0.002) * step(0.4, a1_r(tq * 15.0, 4.0));
    leak = leak + 0.06 * g - 0.25 * scratch * leak;
    let col = mix(base, leak, k.x);
    return a1_out_a(col, a1_alpha(uv));
}

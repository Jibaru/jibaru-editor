// Cine Film Burn: a reel of burning film flickers over the picture: full
// frames of orange, amber and bleached-white burn with churning heat
// texture, a deep red flash-back in the middle, and the edge of a film strip
// with its sprocket holes sliding past during the white burns. The rhythm
// and tints of the frames follow Filmora's Movie Light Effects 2 (measured
// on its export); the burn texture and film strip are drawn here.
//@time sec 2.002
//@p1
//@ 0.0334 0.00 0.00 0.00 0.00
//@ 0.0991 0.00 0.00 0.00 0.00
//@ 0.1001 0.94 0.52 0.37 1.00
//@ 0.1658 0.94 0.52 0.37 1.00
//@ 0.1668 0.94 0.56 0.35 1.00
//@ 0.2659 0.94 0.56 0.35 1.00
//@ 0.2669 0.94 0.60 0.45 1.00
//@ 0.3327 0.94 0.60 0.45 1.00
//@ 0.3337 0.94 0.62 0.49 1.00
//@ 0.4328 0.94 0.62 0.49 1.00
//@ 0.4338 0.00 0.00 0.00 0.00
//@ 0.8999 0.00 0.00 0.00 0.00
//@ 0.9009 0.97 0.71 0.40 1.00
//@ 0.9666 0.97 0.71 0.40 1.00
//@ 0.9676 0.95 0.50 0.27 1.00
//@ 1.0667 0.95 0.50 0.27 1.00
//@ 1.0677 0.94 0.49 0.28 1.00
//@ 1.1335 0.94 0.49 0.28 1.00
//@ 1.1345 0.95 0.47 0.24 1.00
//@ 1.2336 0.95 0.47 0.24 1.00
//@ 1.2346 0.94 0.28 0.14 1.00
//@ 1.3003 0.94 0.28 0.14 1.00
//@ 1.3013 1.00 0.94 0.83 1.00
//@ 1.3670 1.00 0.94 0.83 1.00
//@ 1.3680 1.00 0.94 0.79 1.00
//@ 1.4671 1.00 0.94 0.79 1.00
//@ 1.4681 0.99 0.88 0.71 1.00
//@ 1.5339 0.99 0.88 0.71 1.00
//@ 1.5349 0.99 0.87 0.69 1.00
//@ 1.6340 0.99 0.87 0.69 1.00
//@ 1.6350 0.99 0.88 0.70 1.00
//@ 1.7007 0.99 0.88 0.70 1.00
//@ 1.7017 0.97 0.65 0.39 1.00
//@ 1.7674 0.97 0.65 0.39 1.00
//@ 1.7684 0.97 0.70 0.41 1.00
//@ 1.8675 0.97 0.70 0.41 1.00
//@ 1.8685 0.91 0.37 0.13 1.00
//@ 1.9343 0.91 0.37 0.13 1.00
//@ 1.9353 0.00 0.00 0.00 0.00
//@p2
//@ 0.0334 0.00
//@ 0.4328 0.00
//@ 0.4338 1.00
//@ 0.8332 1.00
//@ 0.8342 0.00

fn fx(uv: vec2f) -> vec4f {
    let t = a1_t();
    let tq = floor(t * 10.0) / 10.0;
    let p = to_c(uv);
    var col = a1_da(uv);
    // red flash-back: the picture printed through a red filter
    let tint = u.p[2].x;
    col = mix(col, col * vec3f(1.3, 0.42, 0.55) + vec3f(0.12, 0.0, 0.02), tint);
    // burn: heat texture around the frame's tint, hotter (whiter) in the swirls
    let f = u.p[1];
    if (f.w > 0.0) {
        let w = vec2f(a1_fbm(p * 1.5 + vec2f(tq * 2.0, 0.0), 3), a1_fbm(p * 1.5 + vec2f(3.3, tq * 2.0), 3));
        let n = a1_fbm(p * 1.4 + 1.2 * (w - 0.5) + vec2f(tq, -tq), 4);
        // the frame's tint is split into saturated fire and bleached white
        // patches whose mix has that mean
        let c = clamp((min(f.r, min(f.g, f.b)) - 0.05) / 0.95, 0.0, 0.95);
        let fire = clamp((f.rgb - c) / (1.0 - c), vec3f(0.0), vec3f(1.0));
        let cov = clamp(c, 0.001, 0.999);
        let thr = 0.5 + 0.13 * log((1.0 - cov) / cov);
        // fire -> yellow -> white across the edge of the hot patches
        let e = clamp((n - thr) / 0.1 * 0.5 + 0.5, 0.0, 1.0);
        let yel = vec3f(1.0, 0.85, 0.35);
        var burn = mix(mix(fire, yel, smoothstep(0.0, 0.5, e)), vec3f(1.0, 0.99, 0.95), smoothstep(0.5, 1.0, e));
        // sprocket holes of a film strip along the left edge in the pale burns
        let pale = smoothstep(0.75, 0.85, f.g);
        let sx = -0.5 * aspect() + 0.09;
        let hy = fract((p.y + t * 0.9) * 5.0) - 0.5;
        let hole = a1_rbox(vec2f(p.x - sx, hy / 5.0), vec2f(0.035, 0.028), 0.012);
        burn = mix(burn, vec3f(0.85, 0.35, 0.12), pale * (1.0 - smoothstep(0.0, 0.006, hole)));
        col = mix(col, burn, 0.92 * f.w);
    }
    return a1_out_a(col, a1_alpha(uv));
}

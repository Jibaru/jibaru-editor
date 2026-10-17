// Shatter (see SHATTER in fx/art4_models.rs). Cells of a jittered Voronoi
// diagram (sites hashed per cell, seed c16) break off and shrink / drift.
fn site(ci: vec2i) -> vec2f {
    let s = i32(c(16));
    return (vec2f(ci) + 0.15 + 0.7 * vec2f(rnd3(ci.x, ci.y, s), rnd3(ci.x, ci.y, s + 17))) ;
}
// Nearest site cell of point q (cell units), and distance to the border.
fn nearest(q: vec2f) -> vec3f {
    let b = vec2i(floor(q));
    var best = vec2i(0);
    var d1 = 1e9;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let ci = b + vec2i(i, j);
            let d = distance(q, site(ci));
            if (d < d1) { d1 = d; best = ci; }
        }
    }
    // Distance to the nearest bisector.
    var e = 1e9;
    let sb = site(best);
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let ci = b + vec2i(i, j);
            if (ci.x == best.x && ci.y == best.y) { continue; }
            let so = site(ci);
            let m = 0.5 * (sb + so);
            let n = normalize(so - sb);
            e = min(e, dot(m - q, n));
        }
    }
    return vec3f(f32(best.x), f32(best.y), e);
}
fn release(ci: vec2i, cell: f32) -> f32 {
    let s = i32(c(16));
    let st = site(ci) * cell; // 1080p px
    var x = st.x / (1080.0 * aspect());
    if (c(7) > 0.5) {
        x = length((st - vec2f(c(8) * aspect(), c(9)) * 1080.0) / 1080.0);
    }
    return c(1) + c(2) * x + c(3) * rnd3(ci.x, ci.y, s + 5);
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let s = i32(c(16));
    let cell = max(c(0), 4.0);
    let p = vec2f(in.uv.x * aspect(), in.uv.y) * 1080.0 / cell;
    let t = u.progress;
    let base = vec2i(floor(p));
    var col = vec4f(0.0, 0.0, 0.0, 1.0);
    var found = false;
    // Is p still on its own unreleased cell?
    let n0 = nearest(p);
    let c0 = vec2i(n0.xy);
    let a0 = t - release(c0, cell);
    if (a0 < 0.0) {
        // Cracks: black borders of width c5 cells from c4 progress before the release.
        let crack = select(0.0, c(5), a0 > -c(4));
        if (n0.z > crack) {
            return sa(in.uv);
        }
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    // Released shards nearby that may cover p (drawn on black).
    for (var j = -2; j <= 2; j++) {
        for (var i = -2; i <= 2; i++) {
            let ci = base + vec2i(i, j);
            let age = t - release(ci, cell);
            if (age < 0.0) { continue; }
            let life = c(13) + c(14) * rnd3(ci.x, ci.y, s + 9);
            if (age > life) { continue; }
            let sc = mix(1.0, c(6), smoothstep(0.0, max(c(10), 1e-3), age));
            let ang = 6.2831853 * rnd3(ci.x, ci.y, s + 3);
            let off = (vec2f(cos(ang), sin(ang)) * c(11) * (0.5 + rnd3(ci.x, ci.y, s + 4)) + vec2f(0.0, c(12))) * age / cell;
            let sj = site(ci);
            let q = sj + (p - sj - off) / max(sc, 1e-3);
            let nq = nearest(q);
            if (i32(nq.x) == ci.x && i32(nq.y) == ci.y && nq.z > 0.0) {
                let uv = vec2f(q.x * cell / (1080.0 * aspect()), q.y * cell / 1080.0);
                col = sa(clamp(uv, vec2f(0.0), vec2f(1.0)));
                col = vec4f(col.rgb * c(15), col.a);
                found = true;
            }
        }
    }
    return col;
}

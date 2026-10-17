// Arrow stack: six right-pointing chevron slabs, alternately carrying the
// outgoing and the incoming clip, shoot in from the left one after another
// and decelerate off the right edge, each casting a soft shadow ahead of
// its edge and trimmed with a thin light rim. The last slab leaves the
// incoming clip. Animated on a 25 fps clock like the original pack.

const AS_SLOPE: f32 = 0.58;
const AS_N: i32 = 6;

fn as_path(i: i32) -> f32 {
    var P = array<f32, 12>(-0.86, -0.78, -0.58, -0.18, 0.21, 0.48, 0.68, 0.82, 0.93, 1.02, 1.09, 1.14);
    if (i < 0) { return -1.2; }
    if (i > 11) { return 1.4; }
    return P[i];
}

fn fx(uv: vec2f) -> vec4f {
    let f = prog() * 60.0;
    let n = i32(floor((f + 0.5) * 0.8342));
    let c = to_c(uv);
    let s = c.x + AS_SLOPE * abs(c.y);
    let a = l3_da(uv);
    let b = l3_db(uv);
    var col = a;
    for (var i = 0; i < AS_N; i = i + 1) {
        let e = as_path(n - (1 + 4 * i));
        let d = s - e;
        let content = select(a, b, (i % 2) == 1);
        let cover = clamp(0.5 - d / ak_px(), 0.0, 1.0);
        let shade = 1.0 - 0.68 * exp(-pow(max(d, 0.0) / 0.1, 2.0));
        col = mix(col * shade, content, cover);
        // Light rim along the slab edge.
        col = ak_over(col, vec3f(1.0), ak_line(d, 0.0012) * 0.45 * step(-1.0, e) * step(e, 1.3));
    }
    return l3_out(col);
}

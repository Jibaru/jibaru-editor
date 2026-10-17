// Comic page: the camera pulls back from the outgoing clip until it is one
// panel of a comic page on black, pans across the page and up to a panel
// holding the incoming clip, then pushes into it. Panels show their clip
// cover-fitted with a printed vignette. Table `cp`: [p, zoom, offset x,
// offset y] (screen uv = page * zoom + offset; the outgoing panel is the
// unit page square).

const CP_N: i32 = 13;

fn cp_panel(i: i32) -> vec4f {
    var P = array<vec4f, 13>(
        vec4f(0.0, 0.0, 1.0, 1.0),      // outgoing clip
        vec4f(-1.05, 0.0, -0.05, 0.97),
        vec4f(1.05, 0.0, 1.93, 0.58),
        vec4f(1.05, 0.63, 1.93, 1.45),
        vec4f(1.98, 0.0, 2.9, 1.08),
        vec4f(-0.72, -1.1, 0.32, -0.1),
        vec4f(0.34, -1.1, 1.36, -0.07),
        vec4f(1.39, -1.15, 2.47, -0.07), // incoming clip
        vec4f(2.53, -1.15, 3.5, -0.07),
        vec4f(0.0, 1.07, 1.0, 2.1),
        vec4f(-1.05, 1.03, -0.05, 2.1),
        vec4f(1.98, 1.13, 2.9, 2.1),
        vec4f(-1.6, -1.1, -0.76, 0.97),
    );
    return P[i];
}

// 1 when the panel carries the incoming clip.
fn cp_side(i: i32) -> f32 {
    var S = array<f32, 13>(0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0);
    return S[i];
}

fn cp_clip(q: vec2f, b: bool) -> vec3f { return select(l3_da(q), l3_db(q), b); }

fn fx(uv: vec2f) -> vec4f {
    let k = kf_cp(prog());
    let page = (uv - k.yz) / k.x;
    var col = vec3f(0.0);
    for (var i = 0; i < CP_N; i = i + 1) {
        let r = cp_panel(i);
        if (page.x >= r.x && page.x < r.z && page.y >= r.y && page.y < r.w) {
            let sz = r.zw - r.xy;
            let local = (page - r.xy) / sz;
            // The two hero panels frame their clip exactly; the others show
            // a centred crop of theirs at page scale.
            let s = max(sz.x, sz.y);
            let q = (local - 0.5) * sz / s + 0.5;
            let hero = i == 0 || i == 7;
            var clip = cp_clip(q, cp_side(i) > 0.5);
            if (!hero) {
                // Printed panels: softened like the halftone print.
                let o = 0.006;
                clip = 0.4 * clip + 0.15 * (cp_clip(q + vec2f(o, 0.0), cp_side(i) > 0.5) + cp_clip(q - vec2f(o, 0.0), cp_side(i) > 0.5)
                    + cp_clip(q + vec2f(0.0, o * 1.78), cp_side(i) > 0.5) + cp_clip(q - vec2f(0.0, o * 1.78), cp_side(i) > 0.5));
            }
            let e = max(abs(local.x - 0.5), abs(local.y - 0.5));
            col = clip * (1.0 - 0.24 * smoothstep(0.25, 0.5, e));
        }
    }
    // The end frames are the clips themselves, without vignette.
    if (k.x >= 0.999 && abs(k.y) < 1e-3 && abs(k.z) < 1e-3) { col = l3_da(uv); }
    if (k.x >= 0.925 && k.y < -1.28) { col = l3_db(uv); }
    return l3_out(col);
}

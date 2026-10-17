// Two warped faces (see PATCH2 in fx/art4_models.rs). Face f's keys start
// at 19·f: corners TL, TR, BR, BL (uv), edge bulges top, right, bottom,
// left (uv vectors), perspective along s and t, shade.
fn fk2(f: i32, i: i32) -> vec2f { return k2(19 * f + i); }
fn patch_at(f: i32, st: vec2f) -> vec2f {
    let s = st.x;
    let t = st.y;
    let p = mix(mix(fk2(f, 0), fk2(f, 2), s), mix(fk2(f, 6), fk2(f, 4), s), t);
    let bs = 4.0 * s * (1.0 - s);
    let bt = 4.0 * t * (1.0 - t);
    return p + bs * ((1.0 - t) * fk2(f, 8) + t * fk2(f, 12)) + bt * ((1.0 - s) * fk2(f, 14) + s * fk2(f, 10));
}
// Inverse of the patch by Newton iterations; z = 1 when inside.
fn patch_inv(f: i32, x: vec2f) -> vec3f {
    var st = vec2f(0.5);
    let h = 1e-3;
    for (var i = 0; i < 10; i++) {
        let p = patch_at(f, st);
        let ps = (patch_at(f, st + vec2f(h, 0.0)) - p) / h;
        let pt = (patch_at(f, st + vec2f(0.0, h)) - p) / h;
        let det = ps.x * pt.y - ps.y * pt.x;
        if (abs(det) < 1e-9) { break; }
        let r = x - p;
        let d = vec2f(r.x * pt.y - r.y * pt.x, ps.x * r.y - ps.y * r.x) / det;
        st = clamp(st + d, vec2f(-0.5), vec2f(1.5));
    }
    let err = length(patch_at(f, st) - x);
    let inside = all(st >= vec2f(0.0)) && all(st <= vec2f(1.0)) && err < 2e-3;
    return vec3f(st, select(0.0, 1.0, inside));
}
fn persp(s: f32, e: f32) -> f32 {
    let g = exp(e);
    return s * g / (1.0 + s * (g - 1.0));
}
fn face_col(f: i32, x: vec2f) -> vec4f {
    let r = patch_inv(f, x);
    if (r.z < 0.5) { return vec4f(0.0); }
    let uv = vec2f(persp(r.x, k(19 * f + 16)), persp(r.y, k(19 * f + 17)));
    var col = select(sb(uv), sa(uv), f == 0);
    let sh = 1.0 + k(19 * f + 18);
    return vec4f(col.rgb * sh, col.a);
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let a = face_col(0, in.uv);
    let b = face_col(1, in.uv);
    // k38 > 0.5: the outgoing face is in front.
    var top = b;
    var bot = a;
    if (k(38) > 0.5) { top = a; bot = b; }
    return top + bot * (1.0 - top.a);
}

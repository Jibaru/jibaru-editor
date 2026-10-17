// Block dissolve (see BLOCK_DISSOLVE in fx/art4_models.rs).
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let s = i32(c(12));
    let nx = max(c(0), 1.0);
    let n = vec2f(nx, max(round(nx / aspect()), 1.0));
    let ci = vec2i(floor(in.uv * n));
    let ctr = (vec2f(ci) + 0.5) / n;
    var bias = ctr.x;
    if (c(11) > 0.5) { bias = 1.0 - 2.0 * abs(ctr.x - select(0.5, c(13), c(13) > 0.0)); }
    let r = rnd3(ci.x, ci.y, s);
    let r2 = rnd3(ci.x, ci.y, s + 1);
    let ta = c(5) + c(6) * r + c(7) * bias;
    let tb = c(8) + c(9) * r2 + c(10) * bias;
    let p = u.progress;
    if (p < ta) { return sa(in.uv); }
    if (p > tb) { return sb(in.uv); }
    return work(vec3f(c(2), c(3), c(4)));
}

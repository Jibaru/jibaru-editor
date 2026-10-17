// Gold chevron sets: from each side a set of three polished gold chevrons
// (a thick bar led in front, two thin rules trailing) sweeps towards the
// centre with the incoming clip behind it; the sets cross and carry on
// outwards as nested diamond frames. Table `gc`: [p, lead position of each
// set (frame heights, along x + M|y|), light on the gold, -, -].

const GC_M: f32 = 0.97;

fn gc_bar(col0: vec3f, d: f32, d0: f32, d1: f32, along: f32, t: f32, lit: f32) -> vec3f {
    let w = d1 - d0;
    let dd = max(d - d1, d0 - d);
    let across = clamp((d - d0) / w * 2.0 - 1.0, -1.0, 1.0);
    return ak_over(col0, ak_gold_lit(along, across, t, lit), ak_fill(dd));
}

fn gc_set(col0: vec3f, d: f32, along: f32, t: f32, lit: f32) -> vec3f {
    var col = col0;
    col = gc_bar(col, d, -0.56, -0.51, along + 0.7, t, lit * 0.8);
    col = gc_bar(col, d, -0.38, -0.30, along + 0.3, t, lit * 0.8);
    col = gc_bar(col, d, -0.185, 0.0, along, t, lit);
    return col;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_gc(p);
    let c = to_c(uv);
    let ay = abs(c.y);
    let d1 = (c.x + GC_M * ay) - k.x;
    let d2 = (-c.x + GC_M * ay) - k.x;
    let behind = clamp(0.5 - (min(d1, d2) + 0.21) / ak_px(), 0.0, 1.0);
    var col = mix(l3_da(uv), l3_db(uv), behind);
    let along = c.y * 1.2;
    col = gc_set(col, d2, along + c.x * 0.3, p, k.y);
    col = gc_set(col, d1, -along + c.x * 0.3, p + 0.3, k.y);
    return l3_out(col);
}

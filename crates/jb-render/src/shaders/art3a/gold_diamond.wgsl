// Gold diamond: two polished gold chevrons slide in from both sides, cross
// in an X at the centre and open into a diamond frame that grows past the
// edges with the incoming clip inside. Each chevron is a thick bar with a
// thin inner rule. Table `gd`: [p, vertex position of the right-hand chevron
// (frame heights from the centre), light on the gold, -, -].

const GD_M: f32 = 1.0;
const GD_THICK: f32 = 0.135;
const GD_GAP: f32 = 0.015;
const GD_THIN: f32 = 0.035;

// Paints one chevron bar whose leading edge is at d = 0 (d < 0 behind it).
fn gd_bar(col0: vec3f, d: f32, along: f32, t: f32, lit: f32) -> vec3f {
    var col = col0;
    let thick = max(d, -GD_THICK - d);
    let thin = max(d + GD_THICK + GD_GAP, -(GD_THICK + GD_GAP + GD_THIN) - d);
    let across = clamp(-d / GD_THICK * 2.0 - 1.0, -1.0, 1.0);
    col = ak_over(col, ak_gold_lit(along, across, t, lit * 0.7), ak_fill(thin));
    col = ak_over(col, ak_gold_lit(along * 1.1 + 0.4, across, t, lit), ak_fill(thick));
    return col;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_gd(p);
    let xv = k.x;
    let c = to_c(uv);
    let ay = abs(c.y);
    // Signed distances (horizontal) to the leading edges of both chevrons.
    let d1 = (c.x + GD_M * ay) - xv;
    let d2 = (-c.x + GD_M * ay) - xv;
    let back = -(GD_THICK + GD_GAP + GD_THIN);
    let inside = clamp(0.5 - (max(d1, d2) - back) / ak_px(), 0.0, 1.0);
    var col = mix(l3_da(uv), l3_db(uv), inside);
    let along = c.y * 1.2;
    col = gd_bar(col, d2, along + c.x * 0.3, p, k.y);
    col = gd_bar(col, d1, -along + c.x * 0.3, p + 0.3, k.y);
    return l3_out(col);
}

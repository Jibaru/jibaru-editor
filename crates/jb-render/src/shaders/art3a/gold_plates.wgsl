// Gold plates: a broad diagonal band made of stacked polished gold plates
// (edge-on, like a tumbling roll of coins) sweeps across the frame, its
// ragged plate ends leading, with the incoming clip following behind it and
// warm sparks drifting around it. Consts `GP_ANG` (axis of the sweep) and
// `GP_DIR` (+1/-1, direction along it). Table `gp`: [p, front, back, -, -]
// (positions along the axis, frame heights from the centre).

const GP_PLATE: f32 = 0.03;

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_gp(p);
    let c = to_c(uv);
    let sn = sin(GP_ANG);
    let cs = cos(GP_ANG);
    let q = GP_DIR * (c.x * sn + c.y * cs);
    let t = c.x * cs - c.y * sn;
    // Plate under this pixel, with its own ragged ends and finish.
    let i = floor(t / GP_PLATE);
    let h1 = hash11(i * 1.37 + 3.1);
    let h2 = hash11(i * 2.71 + 7.3);
    let h3 = hash11(i * 0.91 + 1.9);
    let front = k.x + (h1 - 0.5) * 0.07;
    let back = k.y + (h2 - 0.5) * 0.07;
    let behind = clamp(0.5 - (q - k.y) / ak_px(), 0.0, 1.0);
    var col = mix(l3_da(uv), l3_db(uv), behind);
    let inband = clamp(0.5 - max(q - front, back - q) / ak_px(), 0.0, 1.0);
    if (inband > 0.0) {
        let ft = fract(t / GP_PLATE);
        // Edge-on plate: dark rim, lit face, a glint that runs along it.
        let face = smoothstep(0.0, 0.25, ft) * (1.0 - smoothstep(0.7, 1.0, ft));
        let glint = pow(0.5 + 0.5 * sin(q * 7.0 + h3 * 6.28 + p * 9.0), 8.0);
        let base = vec3f(0.47, 0.33, 0.11);
        var g = base * (0.45 + 0.75 * face * (0.6 + 0.8 * h3));
        g = mix(g, vec3f(0.98, 0.84, 0.45), clamp(glint * face * (0.3 + 0.7 * h1), 0.0, 1.0));
        col = mix(col, g, inband);
    }
    // Sparks around the band.
    let cell = floor(c * 26.0);
    let r = hash22(cell + vec2f(floor(p * 30.0), 0.0));
    let sp = fract(c * 26.0) - r;
    let near = exp(-pow((q - 0.5 * (k.x + k.y)) / 0.35, 2.0)) * step(-1.5, k.x) * step(k.y, 1.2);
    let spark = step(0.86, hash12(cell * 1.7 + 0.3)) * exp(-dot(sp, sp) / 0.012) * near;
    col = l3_screen(col, vec3f(1.0, 0.62, 0.18) * spark * 0.9);
    return l3_out(col);
}

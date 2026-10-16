// Compositor passes. Appended to prelude.wgsl.

// ---------------------------------------------------------------- prep
// Resamples a source (RGBA8 / NV12 / P010 / working texture) with crop into a
// layer texture in the working space.
//   e0 = crop rect in source uv (u0, v0, u1, v1)
//   e1 = (format, matrix, full_range, transfer)
//        format: 0 rgba8 straight sRGB, 1 nv12, 2 p010, 3 working premultiplied, 4 solid color (e2)
//        matrix: 0 bt709, 1 bt601, 2 bt2020
//        transfer: 0 sdr, 1 pq, 2 hlg
//   e2 = solid color (linear rgba) for format 4
//   e3.x = flip_h, e3.y = flip_v

fn ycbcr_to_rgb(y0: f32, cb0: f32, cr0: f32, matrix: i32, full: bool) -> vec3f {
    var y = y0;
    var cb = cb0 - 0.5;
    var cr = cr0 - 0.5;
    if (!full) {
        y = (y0 - 16.0 / 255.0) * (255.0 / 219.0);
        cb = (cb0 - 128.0 / 255.0) * (255.0 / 224.0);
        cr = (cr0 - 128.0 / 255.0) * (255.0 / 224.0);
    }
    var kr = 0.2126;
    var kb = 0.0722;
    if (matrix == 1) { kr = 0.299; kb = 0.114; }
    if (matrix == 2) { kr = 0.2627; kb = 0.0593; }
    let kg = 1.0 - kr - kb;
    let r = y + 2.0 * (1.0 - kr) * cr;
    let b = y + 2.0 * (1.0 - kb) * cb;
    let g = (y - kr * r - kb * b) / kg;
    return vec3f(r, g, b);
}

fn pq_eotf(e: vec3f) -> vec3f {
    // SMPTE ST 2084, returns linear light where 1.0 = 10000 nits.
    let m1 = 0.1593017578125;
    let m2 = 78.84375;
    let c1 = 0.8359375;
    let c2 = 18.8515625;
    let c3 = 18.6875;
    let ep = pow(max(e, vec3f(0.0)), vec3f(1.0 / m2));
    return pow(max(ep - c1, vec3f(0.0)) / (c2 - c3 * ep), vec3f(1.0 / m1));
}

fn hlg_inv_oetf(e: vec3f) -> vec3f {
    let a = 0.17883277;
    let b = 0.28466892;
    let c = 0.55991073;
    let lo = e * e / 3.0;
    let hi = (exp((e - c) / a) + b) / 12.0;
    return select(hi, lo, e <= vec3f(0.5));
}

// HDR (BT.2020 PQ/HLG) -> SDR BT.709 display-referred.
fn hdr_to_sdr(rgb: vec3f, transfer: i32) -> vec3f {
    var lin: vec3f;
    if (transfer == 1) {
        // Scale so that 203 nits (reference white) maps to 1.0.
        lin = pq_eotf(rgb) * (10000.0 / 203.0);
    } else {
        // HLG with a 1000-nit system gamma; reference white at 75% signal.
        let scene = hlg_inv_oetf(rgb);
        let ys = dot(scene, vec3f(0.2627, 0.6780, 0.0593));
        lin = scene * pow(max(ys, 1e-6), 0.2) * (1000.0 / 203.0);
    }
    // BT.2020 -> BT.709 primaries.
    let m = mat3x3f(
        vec3f(1.6605, -0.1246, -0.0182),
        vec3f(-0.5876, 1.1329, -0.1006),
        vec3f(-0.0728, -0.0083, 1.1187),
    );
    var c = max(m * lin, vec3f(0.0));
    // Extended Reinhard on luminance, white point at 4x reference.
    let l = luma(c);
    let lw = 4.0;
    let ln = l * (1.0 + l / (lw * lw)) / (1.0 + l);
    c = c * (ln / max(l, 1e-6));
    return lin_to_srgb(min(c, vec3f(1.0)));
}

fn prep_sample(suv: vec2f) -> vec4f {
    let fmt = i32(u.e1.x);
    if (fmt == 4) { return lin_color(u.e2); }
    if (fmt == 3) { return sa(suv); }
    if (fmt == 0) {
        let c = sa(suv);
        var rgb = c.rgb;
        if (is_linear()) { rgb = srgb_to_lin(rgb); }
        return vec4f(rgb * c.a, c.a);
    }
    var y: f32;
    var cbcr: vec2f;
    if (fmt == 1) {
        y = sa(suv).r;
        cbcr = sb(suv).rg;
    } else {
        // P010: 16-bit little-endian words uploaded as bytes; bilinear filtering of
        // the low and high bytes separately is exact because filtering is linear.
        let yy = sa(suv).rg;
        y = (yy.x * 255.0 + yy.y * 65280.0) / 65535.0;
        let c = sb(suv);
        cbcr = vec2f(c.r * 255.0 + c.g * 65280.0, c.b * 255.0 + c.a * 65280.0) / 65535.0;
    }
    var rgb = clamp(ycbcr_to_rgb(y, cbcr.x, cbcr.y, i32(u.e1.y), u.e1.z > 0.5), vec3f(0.0), vec3f(1.0));
    let transfer = i32(u.e1.w);
    if (transfer != 0) { rgb = hdr_to_sdr(rgb, transfer); }
    if (is_linear()) { rgb = srgb_to_lin(rgb); }
    return vec4f(rgb, 1.0);
}

@fragment
fn fs_prep(in: VsOut) -> @location(0) vec4f {
    var uv = in.uv;
    if (u.e3.x > 0.5) { uv.x = 1.0 - uv.x; }
    if (u.e3.y > 0.5) { uv.y = 1.0 - uv.y; }
    let suv = mix(u.e0.xy, u.e0.zw, uv);
    // Downscaling by more than 1.5x: average 4 bilinear taps (approx. box filter).
    let ratio = (u.e0.zw - u.e0.xy) * u.a_size * u.inv_size;
    if (max(ratio.x, ratio.y) > 1.5 && i32(u.e1.x) != 4) {
        let o = 0.25 * (u.e0.zw - u.e0.xy) * u.inv_size;
        return 0.25 * (prep_sample(suv + vec2f(-o.x, -o.y)) + prep_sample(suv + vec2f(o.x, -o.y))
            + prep_sample(suv + vec2f(-o.x, o.y)) + prep_sample(suv + vec2f(o.x, o.y)));
    }
    return prep_sample(suv);
}

// ---------------------------------------------------------------- composite
// Draws a layer texture (A) as a transformed quad over the accumulator.
//   e0 = (center_x, center_y, half_w, half_h) in target pixels (after scale)
//   e1 = (cos, sin, opacity, blend mode)
//   e2 = (expand px, 0, 0, 0)
// Blend modes other than normal read the backdrop copy (B) and replace.

struct QuadOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) local: vec2f,
};

@vertex
fn vs_quad(@builtin(vertex_index) i: u32) -> QuadOut {
    let corner = vec2f(f32(i & 1u), f32((i >> 1u) & 1u)) * 2.0 - 1.0;
    let half = u.e0.zw;
    // Grow the quad by one pixel so the anti-aliased edge has room.
    let local = corner * (half + vec2f(u.e2.x));
    let r = vec2f(local.x * u.e1.x - local.y * u.e1.y, local.x * u.e1.y + local.y * u.e1.x);
    let p = u.e0.xy + r;
    var o: QuadOut;
    o.pos = vec4f(p.x * u.inv_size.x * 2.0 - 1.0, 1.0 - p.y * u.inv_size.y * 2.0, 0.0, 1.0);
    o.local = local;
    o.uv = local / max(half, vec2f(1e-6)) * 0.5 + 0.5;
    return o;
}

fn blend_ch(mode: i32, b: vec3f, s: vec3f) -> vec3f {
    switch mode {
        case 1: { return b * s; }
        case 2: { return b + s - b * s; }
        case 3: { // overlay
            return select(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, b <= vec3f(0.5));
        }
        case 4: { return min(b, s); }
        case 5: { return max(b, s); }
        case 6: { // color dodge
            return select(min(vec3f(1.0), b / max(1.0 - s, vec3f(1e-5))), vec3f(1.0), s >= vec3f(1.0)) * select(vec3f(1.0), vec3f(0.0), b <= vec3f(0.0));
        }
        case 7: { // color burn
            let r = 1.0 - min(vec3f(1.0), (1.0 - b) / max(s, vec3f(1e-5)));
            return select(select(r, vec3f(0.0), s <= vec3f(0.0)), vec3f(1.0), b >= vec3f(1.0));
        }
        case 8: { // hard light
            return select(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, s <= vec3f(0.5));
        }
        case 9: { // soft light (W3C)
            let d = select(sqrt(b), ((16.0 * b - 12.0) * b + 4.0) * b, b <= vec3f(0.25));
            return select(b + (2.0 * s - 1.0) * (d - b), b - (1.0 - 2.0 * s) * b * (1.0 - b), s <= vec3f(0.5));
        }
        case 10: { return abs(b - s); }
        case 11: { return b + s - 2.0 * b * s; }
        case 12: { return min(b + s, vec3f(1.0)); }
        case 13: { return max(b + s - 1.0, vec3f(0.0)); } // linear burn
        case 14: { // darker color
            return select(b, s, dot(s, vec3f(0.299, 0.587, 0.114)) < dot(b, vec3f(0.299, 0.587, 0.114)));
        }
        case 15: { // lighter color
            return select(b, s, dot(s, vec3f(0.299, 0.587, 0.114)) > dot(b, vec3f(0.299, 0.587, 0.114)));
        }
        case 16: { // vivid light
            let burn = 1.0 - min(vec3f(1.0), (1.0 - b) / max(2.0 * s, vec3f(1e-5)));
            let dodge = min(vec3f(1.0), b / max(2.0 * (1.0 - s), vec3f(1e-5)));
            return select(dodge, burn, s <= vec3f(0.5));
        }
        case 17: { return clamp(b + 2.0 * s - 1.0, vec3f(0.0), vec3f(1.0)); } // linear light
        case 18: { // pin light
            return select(max(b, 2.0 * s - 1.0), min(b, 2.0 * s), s <= vec3f(0.5));
        }
        case 19: { return select(vec3f(0.0), vec3f(1.0), b + s >= vec3f(1.0)); } // hard mix
        case 20: { return max(b - s, vec3f(0.0)); } // subtract
        case 21: { return min(b / max(s, vec3f(1e-5)), vec3f(1.0)); } // divide
        default: { return s; }
    }
}

@fragment
fn fs_composite(in: QuadOut) -> @location(0) vec4f {
    let half = u.e0.zw;
    let d = half - abs(in.local);
    let cover = clamp(min(d.x, d.y) + 0.5, 0.0, 1.0);
    let s = sa(clamp(in.uv, vec2f(0.0), vec2f(1.0))) * (u.e1.z * cover);
    let mode = i32(u.e1.w);
    if (mode == 0) { return s; }
    let bd = textureLoad(tb, vec2i(in.pos.xy), 0);
    // Blend in display space: W3C separable blending on unpremultiplied colors.
    let cb = to_disp(bd);
    let cs = to_disp(s);
    var mixed = blend_ch(mode, clamp(cb.rgb, vec3f(0.0), vec3f(1.0)), clamp(cs.rgb, vec3f(0.0), vec3f(1.0)));
    if (is_linear()) { mixed = srgb_to_lin(mixed); }
    let ab = bd.a;
    let as_ = s.a;
    let rgb = s.rgb * (1.0 - ab) + bd.rgb * (1.0 - as_) + as_ * ab * mixed;
    return vec4f(rgb, as_ + ab * (1.0 - as_));
}

// ---------------------------------------------------------------- encode
// Working space -> straight-alpha sRGB-encoded output.
@fragment
fn fs_encode(in: VsOut) -> @location(0) vec4f {
    let c = to_disp(sa(in.uv));
    // p[0].rgb: output level offset (RenderOptions::output_bias), applied to
    // what is visible: premultiplied over black, so transparent stays clear.
    let rgb = c.rgb + u.p[0].rgb * c.a;
    return vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), clamp(c.a, 0.0, 1.0));
}

// Fills the target with a working-space color given in linear RGBA (e0).
@fragment
fn fs_fill(in: VsOut) -> @location(0) vec4f {
    return lin_color(u.e0);
}

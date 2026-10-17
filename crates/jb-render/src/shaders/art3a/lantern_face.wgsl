// Close-up of a lit jack-o'-lantern (original drawing): the camera pulls
// back out of its nose hole (the outgoing clip seen through it), the eyes
// glow, the jagged grin lights up and the camera dives into it, the light
// washing over the incoming clip.
// Table `lf`: [p, zoom, pivot (0 nose, 1 mouth), mouth light, eye glow |
// clip over the whole frame, B mix, clip through the grin, tint of the clip].

const LF_LIGHT: vec3f = vec3f(1.0, 0.8, 0.01);
const LF_HOT: vec3f = vec3f(1.0, 0.9, 0.2);

fn lf_tri(x: f32) -> f32 { return abs(fract(x) - 0.5) * 2.0; }
fn lf_eye(q0: vec2f) -> f32 {
    // Angry slanted eye: a wedge from a sharp inner corner up to the brow.
    let q = vec2f(abs(q0.x), q0.y);
    return sd_tri(q, vec2f(0.16, -0.16), vec2f(0.74, -0.46), vec2f(0.56, -0.16)) - 0.025;
}
fn lf_nose(q: vec2f) -> f32 { return sd_tri(q, vec2f(0.0, -0.13), vec2f(0.055, -0.04), vec2f(-0.055, -0.04)); }
// Grin: the band between a jagged upper lip and a curved lower lip.
fn lf_mouth(q: vec2f, open: f32) -> f32 {
    let ax = abs(q.x);
    let up = 0.24 + 0.07 * (1.0 - lf_tri(q.x / 0.17)) - 0.26 * (ax / 0.72) * (ax / 0.72);
    let lo = up + 0.012 + open * (0.26 * sqrt(max(1.0 - (ax / 0.75) * (ax / 0.75), 0.0)) - 0.06 * lf_tri(q.x / 0.17 + 0.5));
    return max(max(up - q.y, q.y - lo), ax - 0.75);
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_lf(p);
    let k1 = kf_lf_1(p);
    let c = to_c(uv);
    let px = u.inv_size.y;
    let pivot = mix(vec2f(0.0, -0.06), vec2f(0.0, 0.4), k.y);
    let z = max(k.x, 1e-3);
    let q = (c - pivot) / z + pivot;
    let qpx = px / z;
    // Pumpkin skin: ribs and a dark vignette.
    let rib = abs(fract(q.x * 1.6 * (1.0 + 0.25 * q.y * q.y)) - 0.5);
    var skin = mix(vec3f(0.66, 0.36, 0.01), vec3f(0.55, 0.29, 0.01), smoothstep(0.42, 0.5, rib));
    let vig = smoothstep(0.2, 1.0, length(q * vec2f(0.72, 1.15)));
    skin = mix(skin, vec3f(0.12, 0.06, 0.0), vig);
    // Seen from up close (before the eyes light) the rind is a bright orange.
    skin = skin * mix(1.4, 1.0, k.w);
    var col = skin;
    // Warm light from inside around the holes.
    let de = lf_eye(q);
    let dn = lf_nose(q);
    let dm = lf_mouth(q, k.z);
    let g = k.w;
    let halo = exp(-max(min(de, dn), 0.0) / 0.06) * 0.55 * g + exp(-max(dm, 0.0) / 0.05) * 0.5 * k.z;
    col = ak_over(col, LF_LIGHT, halo);
    col = ak_over(col, mix(LF_LIGHT, LF_HOT, 0.4), clamp(0.5 - de / qpx, 0.0, 1.0) * g);
    // Carved mouth line, glowing when lit.
    col = ak_over(col, vec3f(0.3, 0.14, 0.0), clamp(0.5 - dm / qpx, 0.0, 1.0) * (1.0 - k.z));
    col = ak_over(col, LF_LIGHT, clamp(0.5 - dm / qpx, 0.0, 1.0) * k.z);
    // The nose hole shows the outgoing clip at first.
    let nose_in = clamp(0.5 - dn / qpx, 0.0, 1.0);
    let ap = uv;
    col = ak_over(col, mix(LF_LIGHT, LF_HOT, 0.3), nose_in * g);
    col = ak_over(col, l3_da(ap), nose_in * (1.0 - g));
    // Inside the lit grin the camera finds the clips, tinted by the light:
    // the outgoing one first, then the incoming one.
    let clip = mix(l3_da(uv), l3_db(uv), k1.y);
    let tinted = mix(clip, clip * vec3f(1.15, 1.0, 0.25) + vec3f(0.15, 0.1, 0.0), k1.w);
    col = mix(col, tinted, clamp(0.5 - dm / qpx, 0.0, 1.0) * k1.z);
    col = mix(col, tinted, k1.x);
    return l3_out(col);
}

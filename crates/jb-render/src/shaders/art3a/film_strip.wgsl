// Film strip: a dark film strip with rounded frames closes in over the
// clip and races sideways (its frame edges motion-blurred) while the
// picture seen through its frames stays put; the frames that pass after
// the first carry the incoming clip, and the strip opens up again on it. Table `fs`: [p, zoom, strip offset (uv x of the first cell's
// left gap centre), frame width (uv), blur (uv) | vertical centre, -, -, -].

const FS_P: f32 = 0.692;

// Coverage of the strip's frame windows and the index of the frame under x.
fn fs_mask(uv: vec2f, k: vec4f, yc: f32) -> vec2f {
    let x = uv.x - k.y - FS_P * 0.5;
    let n = floor(x / FS_P + 0.5);
    let cx = k.y + FS_P * 0.5 + n * FS_P;
    let hw = select(k.z * 0.5, 2.0, k.z >= 0.999);
    let hh = 0.5 * k.x;
    let asp = aspect();
    let q = vec2f((uv.x - cx) * asp, uv.y - yc);
    let d = sd_rbox(q, vec2f(hw * asp, hh), 0.035 * k.x);
    return vec2f(clamp(0.5 - d / ak_px(), 0.0, 1.0), n);
}

fn fx(uv: vec2f) -> vec4f {
    // The strip is animated at 25 fps: every sixth frame repeats the last.
    let fr = floor(prog() * 60.0 + 0.5);
    let pp = select(prog(), (fr - 1.0) / 60.0, abs(fract((fr - 3.0) / 6.0)) < 0.01);
    let k = kf_fs(pp);
    let yc = kf_fs_1(pp).x;
    var n = fs_mask(uv, k, yc).y;
    if (k.z >= 0.999) { n = select(0.0, 2.0, pp > 0.5); }
    let clip = select(l3_da(uv), l3_db(uv), n >= 1.0);
    var m = 0.0;
    let taps = 9;
    for (var i = 0; i < taps; i = i + 1) {
        let o = (f32(i) / f32(taps - 1) - 0.5) * k.w;
        m = m + fs_mask(uv + vec2f(o, 0.0), k, yc).x;
    }
    m = m / f32(taps);
    return l3_out(mix(vec3f(0.03), clip, m));
}

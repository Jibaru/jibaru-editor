// Diagonal stripe close/open: two walls of black and blue stripes close on
// the outgoing clip along a diagonal band, leave a pale card crossed by two
// orange rules, then open again from the centre line on the incoming clip.
// A row of pale-blue chevrons drifts left along the bottom.
// Table `sc`: [p, clip half-width, black edge, blue edge, side | chevron lead (u), soft, -, -]
// (half-widths measured across the stripes, in frame heights).

const SC_ANG: f32 = 0.6283; // 36 degrees
const SC_PALE: vec3f = vec3f(0.875, 0.906, 0.988);
const SC_BLUE: vec3f = vec3f(0.087, 0.330, 0.980);
const SC_BLACK: vec3f = vec3f(0.103, 0.104, 0.104);
const SC_ORANGE: vec3f = vec3f(0.992, 0.678, 0.137);
const SC_CHEV: vec3f = vec3f(0.694, 0.776, 0.969);

fn sc_band(r: f32, e: f32, soft: f32) -> f32 { return clamp(0.5 - (r - e) / max(soft, ak_px()), 0.0, 1.0); }

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_sc(p);
    let k1 = kf_sc_1(p);
    let c = to_c(uv);
    let sn = sin(SC_ANG);
    let cs = cos(SC_ANG);
    let s = c.x * sn + c.y * cs;
    let t = c.x * cs - c.y * sn;
    let r = abs(s);
    let soft = k1.y;
    let clip = select(l3_da(uv), l3_db(uv), k.w > 0.5);
    // From the outside in: pale card, blue wall, black wall, clip.
    var col = SC_PALE;
    col = mix(col, SC_BLUE, sc_band(r, k.z, soft));
    col = mix(col, SC_BLACK, sc_band(r, k.y, soft));
    col = mix(col, clip, sc_band(r, k.x, soft));
    // Two orange rules sliding along the stripes in opposite directions.
    let f = p * 60.0;
    let r1 = 0.43 - 0.072 * (f - 30.0);
    let r2 = -0.63 + 0.072 * (f - 2.0);
    let len = 1.79;
    let o1 = max(abs(s + 0.074) - 0.019, max(t - r1, (r1 - len) - t));
    let o2 = max(abs(s - 0.093) - 0.019, max(t - r2, (r2 - len) - t));
    col = ak_over(col, SC_ORANGE, ak_fill(min(o1, o2)));
    // Chevrons "<" along the bottom.
    let asp = aspect();
    var dch = 1e3;
    for (var i = 0.0; i < 7.0; i = i + 1.0) {
        let tip = to_c(vec2f(k1.x + 0.016 + i * 0.0757, 0.915));
        let arm = vec2f(0.0625 * asp, 0.125);
        dch = min(dch, min(sd_seg(c, tip, tip + arm), sd_seg(c, tip, tip + vec2f(arm.x, -arm.y))) - 0.026);
    }
    col = ak_over(col, SC_CHEV, ak_soft(dch, soft * 1.5));
    return l3_out(col);
}

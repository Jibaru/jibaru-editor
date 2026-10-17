// Giant jack-o'-lantern grin (original drawing). The camera starts inside
// the open mouth (the outgoing clip seen through it) and pulls back until
// the carved face fills the frame; the jaw snaps shut and the head flies
// up, uncovering the incoming clip.
// Table `pk` rows: [p, scale, dy, jaw, side, gain of B, -, -]; the face is
// scaled about the mouth centre PK_PIVOT and shifted down by dy.

const PK_PIVOT: vec2f = vec2f(0.0, 0.17);
const PK_SKIN: vec3f = vec3f(0.98, 0.67, 0.14);
const PK_SKIN_DARK: vec3f = vec3f(0.93, 0.55, 0.1);
const PK_RIM: vec3f = vec3f(1.0, 0.86, 0.52);
const PK_HOLE: vec3f = vec3f(0.2, 0.05, 0.03);

// Upper edge of the mouth: a wide smile curve with square teeth hanging.
fn pk_upper(x: f32) -> f32 {
    var y = 0.1 - 0.41 * (x / 0.89) * (x / 0.89);
    if (abs(abs(x) - 0.27) < 0.09) { y = y + 0.035; }
    return y;
}
// Lower edge of the mouth (deep arc meeting the smile at the corners).
fn pk_lower(x: f32, jaw: f32) -> f32 {
    let k = sqrt(clamp(1.0 - (x / 0.86) * (x / 0.86), 0.0, 1.0));
    return mix(pk_upper(x), 0.45 - 0.06 * jaw, k);
}
// Lower edge of the window through the mouth (dark flesh below it).
fn pk_window(x: f32, jaw: f32) -> f32 {
    var y = pk_upper(x) + 0.13 * (1.0 - jaw);
    // Two small teeth rise from the lower jaw.
    if (abs(abs(x) - 0.42) < 0.07) { y = y - 0.04 * (1.0 - jaw); }
    return y;
}

// Distance (negative inside) to the mouth hole.
fn pk_mouth(q: vec2f, jaw: f32) -> f32 {
    let du = pk_upper(q.x) - q.y;
    let dl = q.y - pk_lower(q.x, jaw);
    let dx = abs(q.x) - 0.9;
    return max(max(du, dl), dx) * 0.7;
}
// Eyes: slanted almond holes under the brow.
fn pk_eye(q0: vec2f) -> f32 {
    let q = vec2f(abs(q0.x), q0.y);
    return sd_ellipse(rot2(q - vec2f(0.52, -0.33), -0.15), vec2f(0.25, 0.15));
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let k = kf_pk(p);
    let s = k.x;
    let c = to_c(uv);
    var col = ak_base(uv, k.w);
    if (k.w > 0.5) { col = col * kf_pk_1(p).x; }
    // Face coordinates.
    let q = (c - PK_PIVOT - vec2f(0.0, k.y)) / s + PK_PIVOT;
    let px = ak_px() / s;
    // Head outline: a wide pumpkin whose bottom edge passes under the chin.
    let head = sd_ellipse(q - vec2f(0.0, -0.05), vec2f(1.35, 0.62));
    if (head > 0.03) { return l3_out(col); }
    let jaw = k.z;
    // Skin with vertical ribs.
    let rib = abs(fract(q.x * 1.6 + 0.5) - 0.5);
    var skin = mix(PK_SKIN, PK_SKIN_DARK, 0.3 * smoothstep(0.44, 0.5, rib));
    let dm = pk_mouth(q, jaw);
    let de = pk_eye(q);
    // Light catches the carved edges.
    skin = ak_over(skin, PK_RIM, clamp(0.5 - (abs(min(dm, de) - 0.012) - 0.01) / px, 0.0, 1.0) * 0.85);
    // Nose: three short carved notches.
    let qn = q - vec2f(0.0, -0.36);
    let nose = min(sd_capsule(qn, vec2f(-0.07, -0.03), vec2f(-0.03, 0.02), 0.006),
               min(sd_capsule(qn, vec2f(0.07, -0.03), vec2f(0.03, 0.02), 0.006),
                   sd_capsule(qn, vec2f(0.0, -0.05), vec2f(0.0, 0.0), 0.005)));
    skin = ak_over(skin, PK_HOLE, clamp(0.5 - nose / px, 0.0, 1.0) * 0.8);
    var face = skin;
    face = ak_over(face, PK_HOLE, clamp(0.5 - de / px, 0.0, 1.0));
    // Mouth: the clip shows through the upper part, dark flesh below.
    let inside = clamp(0.5 - dm / px, 0.0, 1.0);
    let win = clamp(0.5 - (q.y - pk_window(q.x, jaw)) / px, 0.0, 1.0);
    face = ak_over(face, PK_HOLE, inside);
    face = ak_over(face, col, inside * win);
    col = ak_over(col, face, clamp(0.5 - head / px, 0.0, 1.0));
    return l3_out(col);
}

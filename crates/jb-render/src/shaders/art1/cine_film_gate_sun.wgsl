// Cine Film Gate Sun: the picture becomes a frame of film in a projector
// gate: a rounded frame on black with the sprocket holes of the strip down
// the left side, a burning white sun crescent printed on the film, a hot
// cream burn that swells from the left, a rainbow edge flare, and halfway
// through the frame slips and rolls down through the gate (frame line and
// all) before it settles, flashes pink and clears.
// Keys are at frame times (29.97 fps) of Filmora's Movie Light Effects 6,
// whose rhythm and layout this follows; all shapes are drawn here.
//@time sec 2.002
//@p0 gate, roll (frames), red glow, pink wash
//@ 0.0000 0.0 0.00 0.0 0.0
//@ 0.1230 0.0 0.00 0.0 0.0
//@ 0.1330 1.0 0.00 0.6 0.0
//@ 0.7330 1.0 0.00 0.6 0.0
//@ 0.7430 1.0 0.10 0.6 0.0
//@ 0.8130 1.0 0.18 0.6 0.0
//@ 0.8830 1.0 0.40 0.6 0.0
//@ 0.9830 1.0 0.62 0.6 0.0
//@ 1.0530 1.0 0.82 0.6 0.0
//@ 1.1330 1.0 0.98 0.6 0.0
//@ 1.2230 1.0 1.00 0.6 0.0
//@ 1.2330 1.0 0.00 0.8 0.0
//@ 1.6730 1.0 0.00 0.8 0.0
//@ 1.6930 1.0 0.00 0.5 0.7
//@ 1.8030 0.6 0.00 0.2 1.0
//@ 1.9100 0.5 0.00 0.0 1.0
//@ 1.9300 0.0 0.00 0.0 0.0
//@p1 sun on the film: x, y (frame fractions), radius (heights), brightness
//@ 0.0000 0.52 0.25 0.19 0.0
//@ 0.1230 0.52 0.25 0.19 0.0
//@ 0.1330 0.53 0.24 0.19 1.0
//@ 0.4330 0.53 0.24 0.19 1.0
//@ 0.4930 0.50 0.75 0.19 1.0
//@ 0.5930 0.52 0.62 0.19 1.0
//@ 0.7130 0.53 0.62 0.19 1.0
//@ 0.7330 0.53 0.24 0.19 1.0
//@ 1.7830 0.53 0.24 0.19 1.0
//@ 1.8030 0.53 0.24 0.19 0.0
//@p2 cream burn: x, y (frame fractions), radius (heights), opacity
//@ 0.0000 0.10 0.40 0.20 0.0
//@ 0.4130 0.10 0.40 0.20 0.0
//@ 0.4330 0.12 0.40 0.36 1.0
//@ 0.4930 0.10 0.60 0.40 1.0
//@ 0.5930 0.25 0.55 0.48 1.0
//@ 0.7130 0.25 0.55 0.42 1.0
//@ 0.7330 0.25 0.55 0.42 0.0
//@ 0.8730 0.25 0.55 0.42 0.0
//@ 0.8930 0.30 0.30 0.50 1.0
//@ 1.0830 0.28 0.45 0.50 1.0
//@ 1.1330 0.20 0.50 0.45 1.0
//@ 1.2530 0.25 0.50 0.45 1.0
//@ 1.2730 0.25 0.50 0.45 0.0
//@p3 rainbow edge flare
//@ 0.0000 0.0
//@ 0.4130 0.0
//@ 0.4330 1.0
//@ 0.6130 1.0
//@ 0.6330 0.0

// Picture of the film frame at frame position q (0..1, y down), with the
// art printed on the film: sun crescent and cream burn.
fn film(q: vec2f) -> vec3f {
    var col = a1_da(q);
    let asp = aspect();
    // red cast from the left of the frame
    col = mix(col, col * vec3f(1.25, 0.55, 0.6) + vec3f(0.2, 0.0, 0.03), u.p[0].z * smoothstep(0.55, 0.0, q.x));
    // cream burn with an orange rim
    let b = u.p[2];
    let db = length((q - b.xy) * vec2f(asp, 1.0)) / max(b.z, 1e-3);
    let burn = b.w * (1.0 - smoothstep(0.92, 1.0, db));
    let rim = b.w * a1_gauss(db - 1.0, 0.07) * (1.0 - burn);
    col = mix(col, vec3f(1.0, 0.45, 0.1), clamp(rim, 0.0, 1.0));
    col = mix(col, vec3f(1.0, 0.97, 0.86), burn);
    // the sun: a bright disc with a dark bite on its right, an ember beside it
    let s = u.p[1];
    let ps = (q - s.xy) * vec2f(asp, 1.0);
    let disc = a1_disc(length(ps), s.z, 0.006);
    let bite = a1_disc(length(ps - vec2f(0.8 * s.z, -0.05 * s.z)), 0.75 * s.z, 0.01);
    let crescent = disc * (1.0 - bite);
    let speck = a1_gauss(length(ps - vec2f(1.15 * s.z, -0.15 * s.z)), 0.012);
    col = mix(col, vec3f(1.0, 0.96, 0.85) * (0.9 + 0.1 * hash12(floor(q * 300.0))), s.w * crescent);
    col = mix(col, vec3f(1.0, 0.4, 0.1), s.w * clamp(speck + 0.5 * a1_gauss(length(ps) - s.z, 0.01) * bite, 0.0, 1.0));
    return col;
}

fn fx(uv: vec2f) -> vec4f {
    let k = u.p[0];
    let base = a1_da(uv);
    if (k.x <= 0.0) { return a1_out_a(base, a1_alpha(uv)); }
    let asp = aspect();
    // the film strip rolls down by `roll` frames; frames are separated by a black line
    let fy = uv.y - k.y;
    let fi = floor(fy);
    let q = vec2f(uv.x, fy - fi);
    // gate opening: a rounded frame inset from the edges, wider margin on the left
    let gc = (q - vec2f(0.535, 0.5)) * vec2f(asp, 1.0);
    let gate = a1_rbox(gc, vec2f(0.465 * asp - 0.012, 0.43), 0.05);
    let open = 1.0 - smoothstep(-0.006, 0.006, gate);
    var col = film(q) * open;
    // sprocket holes: rounded white squares on the strip, left of the frame
    let hy = fract(q.y * 3.0 + 0.1) - 0.5;
    let hole = a1_rbox(vec2f((q.x - 0.075) * asp, hy / 3.0), vec2f(0.06, 0.065), 0.022);
    col = mix(col, vec3f(1.0, 0.98, 0.95), 1.0 - smoothstep(-0.004, 0.004, hole));
    // a dark ghost of the gate edge at the far left
    col = col + vec3f(0.12) * a1_gauss(abs(uv.x - 0.012) + 0.0 * uv.y, 0.004) * smoothstep(0.5, 0.2, abs(uv.y - 0.5));
    // rainbow flare along the right edge
    let rb = u.p[3].x * smoothstep(0.7, 1.0, uv.x);
    let hue = clamp((uv.x - 0.75) * 4.0 + 0.3 * uv.y, 0.0, 1.0);
    let rainbow = a1_ramp(hue, vec3f(0.6, 0.3, 0.9), vec3f(0.2, 0.8, 0.9), vec3f(0.9, 1.0, 0.3));
    col = mix(col, rainbow, 0.75 * rb);
    // pink wash as the effect lets go
    col = mix(col, vec3f(1.0, 0.75, 0.82), 0.55 * k.w * (0.7 + 0.3 * uv.x));
    col = mix(base, col, k.x);
    return a1_out_a(col, a1_alpha(uv));
}

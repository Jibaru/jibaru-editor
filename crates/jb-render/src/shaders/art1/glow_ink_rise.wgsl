// Glow Ink Rise: the new clip wells up from the bottom of the frame like ink
// soaking upward: a first bubble at the lower left, then a ragged front that
// surges up the right side and spreads left, its edge burning with a thin
// white glow. The arrival time of the front across the frame follows the
// coverage measured on Filmora's export of Glow Ink Up; the ink edge and
// glow are drawn here.
//@p0 glow strength
//@ 0.00 0.0
//@ 0.27 0.0
//@ 0.29 1.0
//@ 0.62 1.0
//@ 0.75 0.0

// Frame (of 60) at which the front reaches column x (0..1) at mid height.
fn arrive(x: f32) -> f32 {
    let t = array<f32, 8>(35.0, 34.0, 33.0, 31.0, 30.0, 30.0, 24.5, 23.5);
    let fx = clamp(x * 8.0 - 0.5, 0.0, 7.0);
    let i = min(i32(floor(fx)), 6);
    return mix(t[i], t[i + 1], fx - f32(i));
}

// Arrival frame of the ink at uv.
fn field(uv: vec2f) -> f32 {
    let p = to_c(uv);
    let up = 1.0 - uv.y;
    let n = a1_fbm(p * 5.0, 4) - 0.5;
    var f = arrive(uv.x) + (up - 0.5) * 11.0 + 4.0 * n;
    // the top-left corner soaks through last
    f = f + 9.0 * smoothstep(0.75, 1.0, up) * smoothstep(0.25, 0.0, uv.x);
    // the first bubble at the lower left
    let bub = length((p - vec2f(-0.45, 0.5)) * vec2f(1.0, 1.0)) - 0.13 - 0.02 * n;
    f = min(f, 16.5 + bub * 70.0);
    return f;
}

fn fx(uv: vec2f) -> vec4f {
    let fr = prog() * 60.0;
    let d = field(uv) - fr;
    // inside the ink (d < 0) the new clip shows
    let w = 1.0 - smoothstep(-0.25, 0.25, d);
    var col = mix(a1_da(uv), a1_db(uv), w);
    let g = u.p[0].x;
    let line = a1_gauss(d, 0.35);
    let glow = a1_gauss(d, 1.6);
    col = a1_add(col, g * (vec3f(1.0) * 0.9 * line + vec3f(0.55, 1.0, 0.75) * 0.35 * glow));
    return a1_out(col);
}

// Shared helpers of the art-4 models (see fx/art4_kit.rs for the uniform
// layout: constants c(i) in u.p[0..16], keyframe row k(i) in u.p[16..20]).

fn c(i: i32) -> f32 { return u.p[i / 4][i % 4]; }
fn k(i: i32) -> f32 { return u.p[16 + i / 4][i % 4]; }
fn c2(i: i32) -> vec2f { return vec2f(c(i), c(i + 1)); }
fn k2(i: i32) -> vec2f { return vec2f(k(i), k(i + 1)); }

// Integer hash (PCG): deterministic on every GPU, unlike sin-based hashes.
fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}
fn rnd1(a: i32) -> f32 { return f32(pcg(u32(a)) >> 8u) / 16777216.0; }
fn rnd2(a: i32, b: i32) -> f32 { return f32(pcg(pcg(u32(a)) ^ u32(b) * 2654435761u) >> 8u) / 16777216.0; }
fn rnd3(a: i32, b: i32, s: i32) -> f32 {
    return f32(pcg(pcg(pcg(u32(a)) ^ u32(b) * 2654435761u) ^ u32(s) * 2246822519u) >> 8u) / 16777216.0;
}

// Smooth value noise with an integer hash.
fn inoise(p: vec2f, s: i32) -> f32 {
    let i = vec2i(floor(p));
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let a = rnd3(i.x, i.y, s);
    let b = rnd3(i.x + 1, i.y, s);
    let cc = rnd3(i.x, i.y + 1, s);
    let d = rnd3(i.x + 1, i.y + 1, s);
    return mix(mix(a, b, w.x), mix(cc, d, w.x), w.y);
}

// Mirrored-repeat sampling (no black borders when a warp reaches outside).
fn mirror_uv(uv: vec2f) -> vec2f {
    let m = abs(uv - 2.0 * floor((uv + 1.0) * 0.5) );
    return 1.0 - abs(1.0 - m);
}
fn sam(uv: vec2f) -> vec4f { return sa(mirror_uv(uv)); }
fn sbm(uv: vec2f) -> vec4f { return sb(mirror_uv(uv)); }

// Display-space helpers.
fn disp(c: vec4f) -> vec3f { return to_disp(c).rgb; }
fn work(rgb: vec3f) -> vec4f { return from_disp(vec4f(rgb, 1.0)); }

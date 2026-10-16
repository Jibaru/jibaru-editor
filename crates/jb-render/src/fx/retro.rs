//! Retro, film, light and frame effects (Filmora's Retro / Film / Light Leaks /
//! Frames packs), ported from the agent-editor prototype catalog.
//!
//! Everything is procedural: film scratches, dust, flicker, scanlines, light
//! leaks and frame masks are computed analytically in the shaders. Sizes
//! defined at 1080p are converted with `px1080()`, so previews and 4K finals
//! look alike. Animated noise is quantized to film frame rates.

use super::{info, shader, CustomFx, FxCx};
use crate::registry::{FxIo, FxKind::Effect, ParamSpec, Registry};

/// WGSL helpers shared by the shaders of this module.
const RLIB: &str = r#"
fn rt_frame(fps: f32) -> f32 { return floor(u.time * fps); }
// Uniform per-pixel noise in -1..1, new pattern every frame.
fn rt_noise(uv: vec2f, fr: f32) -> f32 {
    return hash12(floor(uv * u.size) + vec2f(fr * 37.17, fr * 11.31)) * 2.0 - 1.0;
}
// Roughly gaussian per-pixel noise (sum of three uniforms), ~unit range.
fn rt_gnoise(uv: vec2f, fr: f32, salt: f32) -> f32 {
    let p = floor(uv * u.size) + vec2f(fr * 13.7 + salt * 71.3, fr * 7.9 + salt * 19.1);
    return (hash12(p) + hash12(p + 0.37) + hash12(p + 0.71)) / 1.5 - 1.0;
}
// ffmpeg-style natural vignette: cos^4(angle * normalized distance).
fn rt_vignette(uv: vec2f, angle: f32) -> f32 {
    let d = length((uv - 0.5) * u.size) / length(0.5 * u.size);
    let c = cos(min(angle * d, 1.5707963));
    return c * c * c * c;
}
fn rt_sepia(c: vec3f, a: f32) -> vec3f {
    let s = vec3f(dot(c, vec3f(0.393, 0.769, 0.189)), dot(c, vec3f(0.349, 0.686, 0.168)), dot(c, vec3f(0.272, 0.534, 0.131)));
    return mix(c, s, a);
}
fn rt_contrast(c: vec3f, k: f32) -> vec3f { return (c - 0.5) * k + 0.5; }
// ffmpeg eq order: contrast, gamma, saturation.
fn rt_eq(c: vec3f, contrast: f32, sat: f32, gamma: f32) -> vec3f {
    var x = clamp(rt_contrast(c, contrast), vec3f(0.0), vec3f(1.0));
    x = pow(x, vec3f(1.0 / gamma));
    return saturate3(x, sat);
}
// Color balance per tonal range (shadows / midtones / highlights).
fn rt_balance(c: vec3f, s: vec3f, m: vec3f, h: vec3f) -> vec3f {
    let l = luma(c);
    let ws = 1.0 - smoothstep(0.0, 0.55, l);
    let wh = smoothstep(0.45, 1.0, l);
    let wm = clamp(1.0 - ws - wh, 0.0, 1.0);
    return c + 0.5 * (s * ws + m * wm + h * wh);
}
// Vertical projector scratches; returns a brightness multiplier.
fn rt_lines(uv: vec2f, fr: f32, num: i32, width_px: f32, darker: f32, lighter: f32) -> f32 {
    var m = 1.0;
    let seg = floor(fr / 6.0);
    for (var i = 0; i < num; i = i + 1) {
        let fi = f32(i);
        let x = hash11(fi * 7.3 + seg * 1.71) + (hash11(fi * 3.1 + fr * 0.37) - 0.5) * 0.004;
        let w = max(width_px * px1080().x * (0.5 + hash11(fi * 5.7 + seg)), 0.75 * u.inv_size.x);
        let prof = 1.0 - smoothstep(0.3, 1.0, abs(uv.x - x) / w);
        let y0 = hash11(fi * 2.3 + seg * 4.1) * 0.5;
        let y1 = y0 + 0.5 + hash11(fi * 9.1 + seg) * 0.5;
        let vy = smoothstep(y0, y0 + 0.05, uv.y) * (1.0 - smoothstep(y1 - 0.05, y1, uv.y));
        let k = prof * vy * (0.5 + 0.5 * hash11(fi * 4.4 + fr * 0.13));
        if (hash11(fi * 1.9 + seg * 0.3) < 0.5) {
            m = m * (1.0 - k * darker / 100.0);
        } else {
            m = m * (1.0 + k * lighter / 100.0);
        }
    }
    return m;
}
// Dust specks and hairs; returns a darkening amount 0..1. `maxd` is the
// largest particle size in percent of the frame width.
fn rt_dust(uv: vec2f, fr: f32, count: i32, maxd: f32) -> f32 {
    var dark = 0.0;
    let pc = to_c(uv);
    for (var i = 0; i < count; i = i + 1) {
        let fi = f32(i);
        let seed = vec2f(fi * 1.37 + fr * 0.917, fr * 0.311 + fi * 2.11);
        if (hash12(seed + 3.3) > 0.65) { continue; }
        let p = to_c(hash22(seed));
        let r = maxd / 100.0 * aspect() * 0.5 * (0.3 + 0.7 * hash12(seed + 1.7));
        let q = pc - p;
        let wob = 1.0 + 0.35 * (vnoise(q / max(r, 1e-4) * 2.5 + seed) - 0.5);
        if (hash12(seed + 5.1) < 0.6) {
            // Small irregular blob.
            let d = length(q) / (r * 0.18) * wob;
            dark = max(dark, 1.0 - smoothstep(0.6, 1.0, d));
        } else {
            // Hair: a thin arc of a circle.
            let ang = atan2(q.y, q.x) - hash12(seed + 8.0) * TAU;
            let along = fract(ang / TAU);
            let thick = max(r * 0.03, 1.2 * u.inv_size.y);
            let d = abs(length(q) - r * 0.6) / thick * wob;
            let arc = 1.0 - smoothstep(0.25, 0.35, along);
            dark = max(dark, (1.0 - smoothstep(0.5, 1.0, d)) * arc);
        }
    }
    return dark;
}
// oldfilm: vertical frame jumps (fraction of height).
fn rt_jump(fr: f32, delta_px: f32, every: f32) -> f32 {
    if (hash11(fr * 1.31 + 0.5) < every / 100.0) {
        return (hash11(fr * 2.17 + 5.0) * 2.0 - 1.0) * delta_px / 1080.0;
    }
    return 0.0;
}
// oldfilm: brightness flicker.
fn rt_flicker(fr: f32, up: f32, down: f32, every: f32) -> f32 {
    if (hash11(fr * 0.71 + 3.0) < every / 100.0) {
        return 1.0 + (hash11(fr * 1.93 + 7.0) * (up + down) - down) / 100.0;
    }
    return 1.0;
}
// oldfilm: uneven development, a slow drifting brightness gradient.
fn rt_uneven(uv: vec2f, up: f32, down: f32) -> f32 {
    let amp = 0.3 * (up + down) / 200.0;
    let w = sin(uv.y * 2.6 + u.time * 1.7) * 0.6 + sin(uv.x * 1.9 - u.time * 1.1) * 0.4;
    return 1.0 + amp * w;
}
// Pixelized sample: average of a 4x4 grid inside the block containing uv.
fn rt_block(uv: vec2f, block: vec2f) -> vec4f {
    let b0 = floor(uv / block) * block;
    var acc = vec4f(0.0);
    for (var j = 0; j < 4; j = j + 1) {
        for (var i = 0; i < 4; i = i + 1) {
            acc = acc + sa(b0 + (vec2f(f32(i), f32(j)) + 0.5) * 0.25 * block);
        }
    }
    return acc / 16.0;
}
fn rt_posterize(c: vec3f, levels: f32) -> vec3f {
    let n = max(levels - 1.0, 1.0);
    return clamp(round(c * n) / n, vec3f(0.0), vec3f(1.0));
}
// Quadratic through (0, a), (0.5, b), (1, c): a smooth 3-point curve.
fn rt_curve3(x: f32, a: f32, b: f32, c: f32) -> f32 {
    let t = clamp(x, 0.0, 1.0);
    return a * (2.0 * t - 1.0) * (t - 1.0) + b * 4.0 * t * (1.0 - t) + c * t * (2.0 * t - 1.0);
}
"#;

fn lib(body: &str) -> String {
    format!("{RLIB}\n{body}")
}

const INTENSITY: fn(f32) -> ParamSpec = |d| ParamSpec::float("intensity", "Intensity", 0.0, 100.0, d);

pub fn register(r: &mut Registry) {
    // ---------------------------------------------------------------- Retro
    r.register(CustomFx::new(
        info(
            "jb.vhs",
            "VHS",
            "Retro",
            Effect,
            "VHS tape: bleeding colors, noise, scanlines and soft focus. Filmora equivalent: VHS.",
            &[INTENSITY(70.0)],
        ),
        &[&lib(VHS)],
        encode_vhs,
    ));
    shader(
        r,
        info(
            "jb.blanco_negro",
            "Black & White",
            "Retro",
            Effect,
            "Monochrome with adjustable contrast. Filmora equivalent: Black & White.",
            &[ParamSpec::float("contrast", "Contrast", 0.0, 100.0, 40.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let s = to_disp(sa(uv));
    let g = rt_contrast(vec3f(luma(s.rgb)), 1.0 + 0.6 * p_contrast() / 100.0);
    return from_disp(vec4f(clamp(g, vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );
    shader(
        r,
        info(
            "jb.sepia",
            "Sepia",
            "Retro",
            Effect,
            "Brown tone of an old photograph. Filmora equivalent: Sepia.",
            &[INTENSITY(80.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let s = to_disp(sa(uv));
    return from_disp(vec4f(clamp(rt_sepia(s.rgb, p_intensity() / 100.0), vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );
    shader(
        r,
        info("jb.negativo", "Invert", "Retro", Effect, "Inverts the colors like a photographic negative. Filmora equivalent: Invert.", &[]),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let s = to_disp(sa(uv));
    return from_disp(vec4f(1.0 - clamp(s.rgb, vec3f(0.0), vec3f(1.0)), s.a));
}"#,
    );
    shader(
        r,
        info(
            "jb.posterizar",
            "Posterize",
            "Artistic",
            Effect,
            "Reduces the image to flat color bands. Filmora equivalent: Posterize.",
            &[ParamSpec::int("levels", "Levels", 2.0, 16.0, 5.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let s = to_disp(sa(uv));
    return from_disp(vec4f(rt_posterize(s.rgb, max(2.0, round(p_levels()))), s.a));
}"#),
    );
    r.register(CustomFx::new(
        info(
            "jb.comic",
            "Comic",
            "Artistic",
            Effect,
            "Cartoon look: flat colors with dark outlines. Filmora equivalent: Comic / Cartoon.",
            &[INTENSITY(70.0)],
        ),
        &[&lib(COMIC)],
        encode_comic,
    ));
    shader(
        r,
        info(
            "jb.pixel_retro",
            "8-bit Pixel",
            "Retro",
            Effect,
            "Big pixels and a reduced palette, like an 8-bit video game. Filmora equivalent: 8-bit.",
            &[ParamSpec::float("size", "Pixel size", 2.0, 60.0, 16.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let block = max(p_size() * px1080(), u.inv_size);
    let s = to_disp(rt_block(uv, block));
    var c = rt_eq(s.rgb, 1.0, 1.6, 1.3);
    // ffmpeg lutrgb with step 255/4: five levels per channel.
    c = rt_posterize(c, 5.0);
    return from_disp(vec4f(c, s.a));
}"#),
    );

    // ---------------------------------------------------------------- Film
    shader(
        r,
        info(
            "jb.pelicula_antigua",
            "Old Film",
            "Film",
            Effect,
            "Silent-movie film: sepia, flicker, frame jumps, dust and scratches. Filmora equivalent: Old Film.",
            &[INTENSITY(70.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let a = p_intensity() / 100.0;
    let fr = rt_frame(18.0);
    let suv = uv - vec2f(0.0, rt_jump(fr, 4.0 + 14.0 * a, 10.0 + 30.0 * a));
    let s = to_disp(sa(suv));
    var c = rt_sepia(s.rgb, min(1.0, 0.7 + 0.3 * a));
    c = rt_contrast(c, 1.1 + 0.2 * a);
    c = c * rt_flicker(fr, 15.0, 25.0, 30.0 + 50.0 * a) * rt_uneven(uv, 40.0, 20.0);
    c = c * (1.0 - 0.85 * rt_dust(uv, fr, i32(round(5.0 + 20.0 * a)), 2.0 + 4.0 * a));
    c = c * rt_lines(uv, fr, i32(round(2.0 + 6.0 * a)), 2.0, 50.0, 20.0);
    c = c + rt_noise(uv, fr) * (8.0 + 12.0 * a) / 255.0;
    c = c * rt_vignette(uv, 0.5 + 0.3 * a);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );
    shader(
        r,
        info(
            "jb.grano_pelicula",
            "Film Grain",
            "Film",
            Effect,
            "Fine analog film grain. Filmora equivalent: Film Grain.",
            &[INTENSITY(50.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let a = p_intensity() / 100.0;
    let fr = rt_frame(24.0);
    let s = to_disp(sa(uv));
    let ln = rt_gnoise(uv, fr, 0.0) * (12.0 + 68.0 * a) / 255.0 * 0.6;
    let cb = rt_gnoise(uv, fr, 1.0) * (2.0 + 8.0 * a) / 255.0;
    let cr = rt_gnoise(uv, fr, 2.0) * (2.0 + 8.0 * a) / 255.0;
    // Luma grain plus a little chroma grain (BT.709 Cb/Cr directions).
    let c = s.rgb + vec3f(ln) + vec3f(1.5748 * cr, -0.1873 * cb - 0.4681 * cr, 1.8556 * cb);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );
    shader(
        r,
        info(
            "jb.polvo_rayones",
            "Dust & Scratches",
            "Film",
            Effect,
            "Dust specks and vertical projector scratches. Filmora equivalent: Dust & Scratches.",
            &[INTENSITY(60.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let a = p_intensity() / 100.0;
    let fr = rt_frame(24.0);
    let s = to_disp(sa(uv));
    var c = s.rgb * (1.0 - 0.85 * rt_dust(uv, fr, i32(round(8.0 + 30.0 * a)), 2.0 + 3.0 * a));
    c = c * rt_lines(uv, fr, i32(round(2.0 + 8.0 * a)), 3.0, 60.0, 30.0);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );
    shader(
        r,
        info(
            "jb.super8",
            "Super 8",
            "Film",
            Effect,
            "Home movie from the 70s: warm, faded, grainy, with vignette. Filmora equivalent: 8mm Film.",
            &[INTENSITY(70.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let a = p_intensity() / 100.0;
    let fr = rt_frame(18.0);
    let suv = uv - vec2f(0.0, rt_jump(fr, 2.0 + 6.0 * a, 15.0));
    let s = to_disp(sa(suv));
    var c = vec3f(
        rt_curve3(s.r, 0.08, 0.56, 0.95),
        rt_curve3(s.g, 0.05, 0.50, 0.90),
        rt_curve3(s.b, 0.12, 0.42, 0.78),
    );
    c = rt_eq(c, 1.05, 1.0 - 0.25 * a, 1.0);
    c = c * rt_flicker(fr, 10.0, 10.0, 60.0) * rt_uneven(uv, 20.0, 10.0);
    c = c + rt_noise(uv, fr) * (8.0 + 16.0 * a) / 255.0;
    c = c * rt_vignette(uv, 0.6 + 0.3 * a);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );

    // ---------------------------------------------------------------- Light
    shader(
        r,
        info(
            "jb.vineta",
            "Vignette",
            "Light",
            Effect,
            "Gently darkens the edges of the frame. Filmora equivalent: Vignette.",
            &[INTENSITY(50.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let s = to_disp(sa(uv));
    return from_disp(vec4f(s.rgb * rt_vignette(uv, 0.3 + 1.1 * p_intensity() / 100.0), s.a));
}"#),
    );
    shader(
        r,
        info(
            "jb.fuga_luz",
            "Light Leak",
            "Light",
            Effect,
            "Warm orange glow sweeping across the frame. Filmora equivalent: Light Leak.",
            &[INTENSITY(60.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let level = p_intensity() / 10.0;
    let s = to_disp(sa(uv));
    var c = rt_balance(s.rgb, vec3f(0.0), vec3f(0.06 * level / 10.0, 0.0, -0.06 * level / 10.0), vec3f(0.0));
    let x = uv.x;
    let y = uv.y;
    // A slow drift keeps the leak alive over the clip.
    let drift = 0.04 * sin(u.time * 0.9);
    let warm = pow(clamp(1.0 - length(vec2f((x + 0.05 + drift) * 1.5, y - 0.25)) / 0.95, 0.0, 1.0), 1.5);
    let pink = pow(clamp(1.0 - length(vec2f((x - 1.02 - drift) * 1.8, y - 0.9)) / 0.7, 0.0, 1.0), 2.0);
    let alpha = clamp((warm + 0.7 * pink) * (0.3 + 0.09 * level), 0.0, 1.0);
    let leak = vec3f(1.0, (0.45 + 0.45 * warm) * (1.0 - 0.5 * pink), 0.15 + 0.25 * warm * warm * warm + 0.5 * pink);
    let cc = clamp(c, vec3f(0.0), vec3f(1.0));
    let screen = 1.0 - (1.0 - cc) * (1.0 - clamp(leak, vec3f(0.0), vec3f(1.0)));
    c = mix(cc, screen, alpha);
    return from_disp(vec4f(c, s.a));
}"#),
    );
    shader(
        r,
        info(
            "jb.destello_estroboscopico",
            "Strobe Flash",
            "Light",
            Effect,
            "Rhythmic white flashes. Filmora equivalent: Strobe / Flash.",
            &[ParamSpec::float("speed", "Flashes per second", 1.0, 10.0, 2.0)],
        ),
        lib(r#"
fn fx(uv: vec2f) -> vec4f {
    let fps = 30.0;
    let period = max(3.0, round(fps / p_speed()));
    let decay = clamp(floor(period / 3.0), 1.0, 4.0);
    let f = floor(u.time * fps);
    let k = f - floor(f / period) * period;
    let level = mix(2.4, 1.0, min(1.0, k / decay));
    let s = to_disp(sa(uv));
    return from_disp(vec4f(clamp(s.rgb * level, vec3f(0.0), vec3f(1.0)), s.a));
}"#),
    );
    r.register(CustomFx::new(
        info(
            "jb.neon_bordes",
            "Neon",
            "Light",
            Effect,
            "Glowing neon edges over a dark background. Filmora equivalent: Neon / Edge Glow.",
            &[INTENSITY(70.0)],
        ),
        &[&lib(NEON_EDGES), &lib(NEON_GLOW)],
        encode_neon,
    ));

    // ---------------------------------------------------------------- Frames
    shader(
        r,
        info(
            "jb.barras_cine",
            "Cinematic Bars",
            "Frames",
            Effect,
            "Black widescreen bars (2.39:1 at half the slider). Filmora equivalent: Cinematic Bars.",
            &[ParamSpec::float("height", "Height", 0.0, 100.0, 50.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let frac = 0.254 * p_height() / 100.0;
    let px = u.inv_size.y;
    // Anti-aliased bar edges.
    let top = clamp((frac - uv.y) / px + 0.5, 0.0, 1.0);
    let bot = clamp((uv.y - (1.0 - frac)) / px + 0.5, 0.0, 1.0);
    let k = max(top, bot);
    let s = sa(uv);
    return mix(s, vec4f(0.0, 0.0, 0.0, 1.0), k);
}"#,
    );
    shader(
        r,
        info(
            "jb.marco_polaroid",
            "Polaroid Frame",
            "Frames",
            Effect,
            "White instant-photo border, wider at the bottom. Filmora equivalent: Polaroid Frame.",
            &[ParamSpec::float("size", "Thickness", 0.0, 100.0, 40.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let b = 0.01 + 0.07 * p_size() / 100.0;
    let side = b * 9.0 / 16.0;
    let r0 = vec2f(side, b);
    let r1 = vec2f(1.0 - side, 1.0 - 3.0 * b);
    let inner = (uv - r0) / (r1 - r0);
    let paper = from_disp(vec4f(245.0 / 255.0, 242.0 / 255.0, 234.0 / 255.0, 1.0));
    let e = min(min(uv.x - r0.x, r1.x - uv.x) * u.size.x, min(uv.y - r0.y, r1.y - uv.y) * u.size.y);
    let k = clamp(e + 0.5, 0.0, 1.0);
    return mix(paper, over(sa(clamp(inner, vec2f(0.0), vec2f(1.0))), paper), k);
}"#,
    );
    shader(
        r,
        info(
            "jb.sombra_interior",
            "Inner Shadow",
            "Frames",
            Effect,
            "Soft shadow hugging the edges, like a recessed frame. Filmora equivalent: Inner Shadow.",
            &[INTENSITY(60.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let level = p_intensity() / 10.0;
    let d = min(min(uv.x, 1.0 - uv.x) * aspect(), min(uv.y, 1.0 - uv.y));
    let t = 1.0 - clamp(d / 0.12, 0.0, 1.0);
    let alpha = clamp(t * t * (0.3 + 0.06 * level), 0.0, 1.0);
    return over(vec4f(0.0, 0.0, 0.0, alpha), sa(uv));
}"#,
    );
    shader(
        r,
        info(
            "jb.marco_redondeado",
            "Rounded Frame",
            "Frames",
            Effect,
            "Rounded corners over black, or with a white border. Filmora equivalent: Rounded Frame.",
            &[ParamSpec::float("radius", "Radius", 0.0, 100.0, 40.0), ParamSpec::boolean("white", "White border", false)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let r = p_radius() / 10.0;
    let white = p_white() >= 0.5;
    let rad = 0.02 + 0.018 * r;
    let border = select(0.0, 0.03, white);
    let p = vec2f((uv.x - 0.5) * aspect(), uv.y - 0.5);
    let hs = vec2f(aspect() * 0.5 - border - rad, 0.5 - border - rad);
    let q = max(abs(p) - hs, vec2f(0.0));
    let sd = length(q) - rad;
    let alpha = clamp(sd * u.size.y + 0.5, 0.0, 1.0);
    let col = select(vec4f(0.0, 0.0, 0.0, 1.0), vec4f(1.0), white);
    return mix(sa(uv), col, alpha);
}"#,
    );
}

/// Blurred, color-shifted, noisy pass of the VHS effect (A = softened source).
const VHS: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let uv = in.uv;
    let a = p_intensity() / 100.0;
    let px = px1080();
    let fr = rt_frame(30.0);
    // Slight per-band horizontal tracking wobble.
    let band = floor(uv.y * 54.0);
    let wob = (hash11(band * 3.1 + fr * 0.7) - 0.5) * 1.5 * px.x * a;
    let shift = (6.0 + 14.0 * a) * px.x;
    let base = sa(uv + vec2f(wob, 0.0));
    let rr = to_disp(sa(uv + vec2f(shift + wob, 0.0))).r;
    let gg = to_disp(sa(uv + vec2f(wob, -2.0 * px.y))).g;
    let bb = to_disp(sa(uv + vec2f(wob - shift, 0.0))).b;
    var c = rt_eq(vec3f(rr, gg, bb), 1.05, 1.25 + 0.2 * a, 1.05);
    c = rt_balance(c, vec3f(0.06, 0.0, 0.08), vec3f(0.0, -0.03, 0.0), vec3f(0.0));
    c = c + rt_noise(uv, fr) * (10.0 + 22.0 * a) / 255.0;
    // Scanlines: a 1px dark line every 4px at 1080p (never finer than 2 output px).
    let period = max(4.0 * u.size.y / 1080.0, 2.0);
    let y = uv.y * u.size.y;
    let ph = (y - floor(y / period) * period) / period;
    let line = 1.0 - smoothstep(0.2, 0.3, ph);
    c = c * (1.0 - line * (0.25 + 0.3 * a));
    c = c * rt_lines(uv, fr, i32(round(1.0 + 3.0 * a)), 2.0, 20.0, 40.0);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), base.a));
}
"#;

fn encode_vhs(fx: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let a = io.params.get("intensity") / 100.0;
    let k = io.a.height as f32 / 1080.0;
    let soft = cx.blur(io.a, (0.6 + 1.6 * a) * k, 0.3 * k);
    cx.run(&fx.shaders[0], &soft, None, io.out, CustomFx::uniforms(io));
}

/// Comic: A = lightly smoothed source, B = wider blur for the unsharp mask.
const COMIC: &str = r#"
fn comic_tone(c0: vec3f, a: f32) -> vec3f {
    let c = rt_eq(c0, 1.0, 1.6 + 0.8 * a, 1.4);
    let step = (48.0 + 32.0 * (1.0 - a)) / 255.0;
    let l = luma(c);
    let lq = clamp(round(l / step) * step + 16.0 / 255.0, 16.0 / 255.0, 235.0 / 255.0);
    return c * (lq / max(l, 1e-3));
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let a = p_intensity() / 100.0;
    let s = to_disp(sa(in.uv));
    let c = comic_tone(s.rgb, a);
    let b = comic_tone(to_disp(sb(in.uv)).rgb, a);
    let amount = 2.0 + 1.5 * a;
    var o = c + amount * (c - b);
    // Ink outlines from the luma gradient of the smoothed image.
    let d = max(u.inv_size, 1.5 * px1080());
    let gx = luma(to_disp(sa(in.uv + vec2f(d.x, 0.0))).rgb) - luma(to_disp(sa(in.uv - vec2f(d.x, 0.0))).rgb);
    let gy = luma(to_disp(sa(in.uv + vec2f(0.0, d.y))).rgb) - luma(to_disp(sa(in.uv - vec2f(0.0, d.y))).rgb);
    let ink = smoothstep(0.06, 0.2, length(vec2f(gx, gy))) * (0.6 + 0.4 * a);
    o = o * (1.0 - ink);
    return from_disp(vec4f(clamp(o, vec3f(0.0), vec3f(1.0)), s.a));
}
"#;

fn encode_comic(fx: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let k = io.a.height as f32 / 1080.0;
    let smooth = cx.blur(io.a, 2.5 * k, 2.5 * k);
    let wide = cx.blur(&smooth, 3.0 * k, 3.0 * k);
    cx.run(&fx.shaders[0], &smooth, Some(&wide), io.out, CustomFx::uniforms(io));
}

/// Neon pass 1: thin edges (white on black).
const NEON_EDGES: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let d = max(u.inv_size, px1080());
    var l: array<f32, 9>;
    for (var j = 0; j < 3; j = j + 1) {
        for (var i = 0; i < 3; i = i + 1) {
            l[j * 3 + i] = luma(to_disp(sa(in.uv + vec2f(f32(i - 1), f32(j - 1)) * d)).rgb);
        }
    }
    let gx = (l[2] + 2.0 * l[5] + l[8]) - (l[0] + 2.0 * l[3] + l[6]);
    let gy = (l[6] + 2.0 * l[7] + l[8]) - (l[0] + 2.0 * l[1] + l[2]);
    let e = smoothstep(0.04, 0.12, length(vec2f(gx, gy)) * 0.25);
    return vec4f(vec3f(e), 1.0);
}
"#;

/// Neon pass 2: A = blurred edges, B = sharp edges.
const NEON_GLOW: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let a = p_intensity() / 100.0;
    let gain = 2.0 + 2.0 * a;
    let col = vec3f(1.1, 0.3, 1.3) * gain;
    let glow = sa(in.uv).r;
    let edge = sb(in.uv).r;
    let c = clamp(glow * col + 0.6 * edge * col, vec3f(0.0), vec3f(1.0));
    return from_disp(vec4f(c, 1.0));
}
"#;

fn encode_neon(fx: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let a = io.params.get("intensity") / 100.0;
    let edges = cx.temp_like(io.a);
    let u = CustomFx::uniforms(io);
    cx.run(&fx.shaders[0], io.a, None, &edges, u);
    let s = (0.8 + 1.5 * a) * io.a.height as f32 / 540.0;
    let glow = cx.blur(&edges, s, s);
    cx.run(&fx.shaders[1], &glow, Some(&edges), io.out, u);
}

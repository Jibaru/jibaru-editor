//! Effects behind the Properties panel modules (category "Panel", hidden from
//! the Effects library): corner radius, chroma key, shape masks, video
//! denoise, lens correction, stabilization, color wheels, and the two
//! canvas-space modules (background fill and drop shadow) that the compositor
//! draws around the clip (see `render.rs`).
//!
//! Sizes given in "1080p pixels" are scaled by the layer height / 1080.

use super::{info, param_accessors, shader, CustomFx, FxCx, ShaderSrc, Uniforms};
use crate::fx::{Bind, Pass};
use crate::registry::{Effect, FxInfo, FxIo, FxKind::Effect as E, ParamSpec as P, Registry};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const CAT: &str = "Panel";

/// Canvas-space background fill (Filmora Video > Background).
pub const BACKGROUND: &str = "jb.background";
/// Canvas-space drop shadow (Filmora Video > Shadow).
pub const SHADOW: &str = "jb.shadow";

/// Shape masks; all share [`MASK_PARAMS`].
pub const MASK_SHAPES: &[&str] = &["rectangle", "ellipse", "single_line", "double_line", "heart", "star"];

/// (shape, id, label) of every shape mask.
const MASKS: [(&str, &str, &str); 6] = [
    ("rectangle", "jb.mask.rectangle", "Rectangle Mask"),
    ("ellipse", "jb.mask.ellipse", "Circle Mask"),
    ("single_line", "jb.mask.single_line", "Single Line Mask"),
    ("double_line", "jb.mask.double_line", "Double Line Mask"),
    ("heart", "jb.mask.heart", "Heart Mask"),
    ("star", "jb.mask.star", "Star Mask"),
];

const fn wheel(name: &'static str, label: &'static str) -> P {
    P::float(name, label, -100.0, 100.0, 0.0)
}

const WHEEL_PARAMS: [P; 12] = [
    wheel("global_x", "Global X"),
    wheel("global_y", "Global Y"),
    wheel("global_lum", "Global Luminance"),
    wheel("shadows_x", "Shadows X"),
    wheel("shadows_y", "Shadows Y"),
    wheel("shadows_lum", "Shadows Luminance"),
    wheel("midtones_x", "Midtones X"),
    wheel("midtones_y", "Midtones Y"),
    wheel("midtones_lum", "Midtones Luminance"),
    wheel("highlights_x", "Highlights X"),
    wheel("highlights_y", "Highlights Y"),
    wheel("highlights_lum", "Highlights Luminance"),
];

const MASK_PARAMS: [P; 9] = [
    P::float("x", "Position X", -100.0, 100.0, 0.0).unit("%"),
    P::float("y", "Position Y", -100.0, 100.0, 0.0).unit("%"),
    P::float("width", "Width", 0.0, 200.0, 60.0).unit("%"),
    P::float("height", "Height", 0.0, 200.0, 60.0).unit("%"),
    P::float("rotation", "Rotation", -360.0, 360.0, 0.0).unit("°"),
    P::float("roundness", "Roundness", 0.0, 100.0, 0.0),
    P::float("feather", "Blur Strength", 0.0, 100.0, 0.0),
    P::float("opacity", "Opacity", 0.0, 100.0, 100.0).unit("%"),
    P::boolean("invert", "Invert Mask", false),
];

/// Signed distance (layer pixels) of each shape; `q` local coordinates
/// (y down), `hs` half size.
fn mask_sdf(shape: &str) -> &'static str {
    match shape {
        "rectangle" => {
            "fn mask_d(q: vec2f, hs: vec2f) -> f32 {\n\
                 let r = p_roundness() / 100.0 * min(hs.x, hs.y);\n\
                 let k = abs(q) - hs + r;\n\
                 return length(max(k, vec2f(0.0))) + min(max(k.x, k.y), 0.0) - r;\n\
             }\n"
        }
        "ellipse" => "fn mask_d(q: vec2f, hs: vec2f) -> f32 { return (length(q / hs) - 1.0) * min(hs.x, hs.y); }\n",
        "single_line" => "fn mask_d(q: vec2f, hs: vec2f) -> f32 { return q.y; }\n",
        "double_line" => "fn mask_d(q: vec2f, hs: vec2f) -> f32 { return abs(q.y) - hs.y; }\n",
        "heart" => {
            "fn dot2(v: vec2f) -> f32 { return dot(v, v); }\n\
             fn sd_heart(p0: vec2f) -> f32 {\n\
                 let p = vec2f(abs(p0.x), p0.y);\n\
                 if (p.y + p.x > 1.0) { return sqrt(dot2(p - vec2f(0.25, 0.75))) - sqrt(2.0) / 4.0; }\n\
                 return sqrt(min(dot2(p - vec2f(0.0, 1.0)), dot2(p - 0.5 * max(p.x + p.y, 0.0)))) * sign(p.x - p.y);\n\
             }\n\
             fn mask_d(q: vec2f, hs: vec2f) -> f32 {\n\
                 let n = q / hs;\n\
                 let hp = vec2f(n.x * 0.62, (1.0 - n.y) * 0.53);\n\
                 return sd_heart(hp) * min(hs.x / 0.62, hs.y / 0.53);\n\
             }\n"
        }
        "star" => {
            "fn sd_star5(p0: vec2f, r: f32, rf: f32) -> f32 {\n\
                 let k1 = vec2f(0.809016994375, -0.587785252292);\n\
                 let k2 = vec2f(-k1.x, k1.y);\n\
                 var p = vec2f(abs(p0.x), p0.y);\n\
                 p = p - 2.0 * max(dot(k1, p), 0.0) * k1;\n\
                 p = p - 2.0 * max(dot(k2, p), 0.0) * k2;\n\
                 p = vec2f(abs(p.x), p.y - r);\n\
                 let ba = rf * vec2f(-k1.y, k1.x) - vec2f(0.0, 1.0);\n\
                 let h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);\n\
                 return length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y);\n\
             }\n\
             fn mask_d(q: vec2f, hs: vec2f) -> f32 {\n\
                 let n = q / hs;\n\
                 let rr = p_roundness() / 100.0 * 0.3;\n\
                 return (sd_star5(vec2f(n.x, -n.y), 1.0 - rr, 0.45) - rr) * min(hs.x, hs.y);\n\
             }\n"
        }
        _ => unreachable!("unknown mask shape {shape}"),
    }
}

const MASK_BODY: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let sz = u.size;
    let c = sz * 0.5 + vec2f(p_x() / 100.0 * sz.x * 0.5, p_y() / 100.0 * sz.y * 0.5);
    let hs = max(vec2f(p_width() / 100.0 * sz.x * 0.5, p_height() / 100.0 * sz.y * 0.5), vec2f(0.5));
    let q = rot2(uv * sz - c, -radians(p_rotation()));
    let d = mask_d(q, hs);
    let f = 1.0 + p_feather() / 100.0 * 0.25 * min(sz.x, sz.y);
    var m = clamp(0.5 - d / f, 0.0, 1.0);
    if (p_invert() > 0.5) { m = 1.0 - m; }
    m = mix(1.0, m, clamp(p_opacity() / 100.0, 0.0, 1.0));
    return sa(uv) * m;
}
"#;

/// Chroma key: YCbCr helpers shared by the key pass.
const KEY_LIB: &str = r#"
fn ycc(c: vec3f) -> vec3f {
    let y = luma(c);
    return vec3f(y, (c.b - y) / 1.8556, (c.r - y) / 1.5748);
}
fn rgb_of(y: f32, cb: f32, cr: f32) -> vec3f {
    let r = y + 1.5748 * cr;
    let b = y + 1.8556 * cb;
    let g = (y - 0.2126 * r - 0.0722 * b) / 0.7152;
    return vec3f(r, g, b);
}
"#;

/// Pass 0: straight display color with spill removed, alpha = matte × source alpha.
const KEY_PASS: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let d = to_disp(sa(in.uv));
    let kc = ycc(lin_to_srgb(p_color().rgb)).yz;
    let px = ycc(clamp(d.rgb, vec3f(0.0), vec3f(1.0)));
    var pc = px.yz;
    let dist = length(pc - kc);
    let tol = p_tolerance() / 100.0 * 0.35;
    let soft = 0.01 + p_offset() / 100.0 * 0.3;
    let matte = smoothstep(tol, tol + soft, dist);
    let kl = length(kc);
    if (kl > 1e-4) {
        let kd = kc / kl;
        let proj = dot(pc, kd);
        if (proj > 0.0) { pc = pc - kd * proj * clamp(p_spill() / 100.0, 0.0, 1.0); }
    }
    let rgb = clamp(rgb_of(px.x, pc.x, pc.y), vec3f(0.0), vec3f(1.0));
    return vec4f(rgb, matte * d.a);
}
"#;

/// Pass 1: separable erode (e0.z < 0) / dilate of the alpha along e0.xy.
const MORPH_PASS: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let c = sa(in.uv);
    let r = i32(min(abs(u.e0.z), 48.0));
    var a = c.a;
    for (var i = -r; i <= r; i = i + 1) {
        let s = sa(in.uv + u.e0.xy * f32(i)).a;
        if (u.e0.z < 0.0) { a = min(a, s); } else { a = max(a, s); }
    }
    return vec4f(c.rgb, a);
}
"#;

/// Pass 2: color from A (straight), alpha from B (processed matte).
const KEY_FINAL: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let c = sa(in.uv);
    let m = clamp(sb(in.uv).a, 0.0, 1.0);
    if (p_alpha_channel() > 0.5) { return from_disp(vec4f(vec3f(m), 1.0)); }
    return from_disp(vec4f(c.rgb, m));
}
"#;

fn chroma_key(me: &CustomFx, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let k = io.a.height as f32 / 1080.0;
    let keyed = cx.temp_like(io.a);
    cx.run(&me.shaders[0], io.a, None, &keyed, CustomFx::uniforms(io));
    let mut matte = keyed.clone();
    let thick = io.params.get("edge_thickness") * k;
    if thick.abs() >= 0.5 {
        for dir in [[1.0 / matte.width as f32, 0.0], [0.0, 1.0 / matte.height as f32]] {
            let out = cx.temp_like(&matte);
            let mut u = CustomFx::uniforms(io);
            u.e[0] = [dir[0], dir[1], thick.round(), 0.0];
            cx.run(&me.shaders[1], &matte, None, &out, u);
            matte = out;
        }
    }
    let feather = io.params.get("edge_feather") * k;
    if feather > 0.05 {
        matte = cx.blur(&matte, feather, feather);
    }
    cx.run(&me.shaders[2], &keyed, Some(&matte), io.out, CustomFx::uniforms(io));
}

/// A canvas-space module: nothing happens in the layer pass, the compositor
/// draws it (see `render.rs`).
struct CanvasFx {
    info: FxInfo,
}

impl Effect for CanvasFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn has_pixels(&self) -> bool {
        false
    }
}

/// Shaders the compositor uses for the canvas-space modules.
pub struct CanvasShaders {
    /// Colored, padded alpha silhouette: e0.xy padding (px), e1 linear color,
    /// e2.x opacity.
    pub silhouette: ShaderSrc,
    /// Solid linear color e0 (premultiplied over).
    pub fill: ShaderSrc,
}

pub fn canvas_shaders() -> &'static CanvasShaders {
    static S: std::sync::OnceLock<CanvasShaders> = std::sync::OnceLock::new();
    S.get_or_init(|| CanvasShaders {
        silhouette: ShaderSrc::new(
            "jb.internal.silhouette",
            r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let p = in.uv * u.size - u.e0.xy;
    let a = sat(p / u.a_size).a;
    return lin_color(vec4f(u.e1.rgb, u.e1.a * a * u.e2.x));
}
"#,
        ),
        fill: ShaderSrc::new(
            "jb.internal.fill",
            "@fragment fn fs_main(in: VsOut) -> @location(0) vec4f { return lin_color(u.e0); }\n",
        ),
    })
}

/// Parsed `jb.stabilize` data: per-frame offsets and the auto zoom.
struct Track {
    fps: f32,
    offsets: Vec<[f32; 2]>,
    max: f32,
}

fn parse_track(s: &str) -> Option<Track> {
    let (fps, rest) = s.split_once('|')?;
    let fps: f32 = fps.trim().parse().ok().filter(|f: &f32| *f > 0.0)?;
    let offsets: Vec<[f32; 2]> = rest
        .split(';')
        .filter_map(|p| {
            let (x, y) = p.split_once(',')?;
            Some([x.trim().parse().ok()?, y.trim().parse().ok()?])
        })
        .collect();
    if offsets.is_empty() {
        return None;
    }
    let max = offsets.iter().map(|o| o[0].abs().max(o[1].abs())).fold(0.0, f32::max);
    Some(Track { fps, offsets, max })
}

struct StabilizeFx {
    info: FxInfo,
    shader: ShaderSrc,
    tracks: Mutex<HashMap<String, Option<Arc<Track>>>>,
}

impl Effect for StabilizeFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let data = io.params.text("data").trim();
        let track = if data.is_empty() {
            None
        } else {
            let mut m = self.tracks.lock().unwrap();
            if m.len() > 16 {
                m.clear();
            }
            m.entry(data.to_string()).or_insert_with(|| parse_track(data).map(Arc::new)).clone()
        };
        let Some(t) = track else {
            cx.copy(io.a, io.out);
            return;
        };
        let f = (io.time.max(0.0) * t.fps).min((t.offsets.len() - 1) as f32);
        let i = f.floor() as usize;
        let j = (i + 1).min(t.offsets.len() - 1);
        let w = f - i as f32;
        let off =
            [t.offsets[i][0] * (1.0 - w) + t.offsets[j][0] * w, t.offsets[i][1] * (1.0 - w) + t.offsets[j][1] * w];
        let zoom = io.params.get("zoom");
        let scale = if zoom > 0.0 { 1.0 + zoom / 100.0 } else { 1.0 + 2.0 * t.max };
        let mut u = Uniforms::with_params(io.params);
        u.time = io.time;
        u.e[0] = [off[0], off[1], scale, 0.0];
        let mut pass = Pass::full(&self.shader, io.out, u);
        pass.a = Bind::Tex(io.a);
        cx.draw(pass);
    }
}

pub fn register(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.corner_radius",
            "Corner Radius",
            CAT,
            E,
            "Rounds the clip's corners (radius in % of half the shorter side). Filmora: Transform > Corner Radius.",
            &[P::float("radius", "Corner Radius", 0.0, 100.0, 0.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let sz = u.size;
    let r = p_radius() / 100.0 * 0.5 * min(sz.x, sz.y);
    let k = abs(uv * sz - sz * 0.5) - (sz * 0.5 - r);
    let d = length(max(k, vec2f(0.0))) + min(max(k.x, k.y), 0.0) - r;
    return sa(uv) * clamp(0.5 - d, 0.0, 1.0);
}
"#,
    );

    r.register(CustomFx::new(
        info(
            "jb.chroma_key",
            "Chroma Key",
            CAT,
            E,
            "Keys out a background color (green/blue screen) by its chroma, with spill removal and matte edge controls. Filmora: Matting > Chroma Key.",
            &[
                P::color("color", "Key Color", [0.0, 1.0, 0.0, 1.0]),
                P::float("tolerance", "Tolerance", 0.0, 100.0, 20.0),
                P::float("offset", "Offset", 0.0, 100.0, 0.0),
                P::float("edge_thickness", "Edge Thickness", -10.0, 10.0, 0.0),
                P::float("edge_feather", "Edge Feather", 0.0, 10.0, 0.0),
                P::float("spill", "Spill Removal", 0.0, 100.0, 50.0),
                P::boolean("alpha_channel", "Alpha Channel", false),
            ],
        ),
        &[&format!("{KEY_LIB}{KEY_PASS}"), MORPH_PASS, KEY_FINAL],
        chroma_key,
    ));

    for (shape, id, label) in MASKS {
        shader(
            r,
            info(
                id,
                label,
                CAT,
                E,
                "Shape mask: keeps the clip inside the shape. Filmora: Video > Mask.",
                &MASK_PARAMS,
            ),
            format!("{}{MASK_BODY}", mask_sdf(shape)),
        );
    }

    shader(
        r,
        info(
            "jb.denoise",
            "Video Denoise",
            CAT,
            E,
            "Edge-preserving spatial noise reduction (bilateral filter). Filmora: Video Denoise.",
            &[P::float("strength", "Strength", 0.0, 100.0, 50.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let s = p_strength() / 100.0;
    let c0 = sa(uv);
    if (s <= 0.001) { return c0; }
    let d0 = to_disp(c0);
    let st = max(1.0, u.size.y / 1080.0) * (1.0 + s) * u.inv_size;
    let sr = 0.03 + 0.12 * s;
    var acc = vec3f(0.0);
    var wsum = 0.0;
    for (var i = -3; i <= 3; i = i + 1) {
        for (var j = -3; j <= 3; j = j + 1) {
            let o = vec2f(f32(i), f32(j));
            let c = to_disp(sa(uv + o * st)).rgb;
            let dc = c - d0.rgb;
            let w = exp(-dot(o, o) / 8.0) * exp(-dot(dc, dc) / (2.0 * sr * sr));
            acc = acc + c * w;
            wsum = wsum + w;
        }
    }
    return from_disp(vec4f(mix(d0.rgb, acc / wsum, s), d0.a));
}
"#,
    );

    shader(
        r,
        info(
            "jb.lens_correction",
            "Lens Correction",
            CAT,
            E,
            "Radial distortion correction: positive levels straighten the barrel distortion of wide-angle and action cameras. Filmora: Lens Correction.",
            &[P::float("level", "Adjust Level", -100.0, 100.0, 0.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let k = p_level() / 100.0 * 0.5;
    if (abs(k) < 1e-5) { return sa(uv); }
    let c = to_c(uv);
    let rn = length(c) / length(vec2f(0.5 * aspect(), 0.5));
    let s = max(1.0, 1.0 + k);
    return sa(from_c(c * (1.0 + k * rn * rn) / s));
}
"#,
    );

    r.register(CanvasFx {
        info: info(
            BACKGROUND,
            "Background",
            CAT,
            E,
            "Fills the canvas around a clip that does not cover it: its own image blurred (mode 0) or a solid color (mode 1). Filmora: Video > Background.",
            &[
                P::int("mode", "Type", 0.0, 1.0, 0.0),
                P::float("blur", "Blur", 0.0, 100.0, 50.0),
                P::color("color", "Color", [0.0, 0.0, 0.0, 1.0]),
            ],
        ),
    });
    r.register(CanvasFx {
        info: info(
            SHADOW,
            "Shadow",
            CAT,
            E,
            "Drop shadow under the clip (distance and blur in 1080p pixels, angle in degrees, 0 = right, 90 = down). Filmora: Video > Shadow.",
            &[
                P::float("distance", "Distance", 0.0, 100.0, 10.0),
                P::float("angle", "Angle", 0.0, 360.0, 135.0).unit("°"),
                P::float("blur", "Blur", 0.0, 100.0, 20.0),
                P::float("opacity", "Opacity", 0.0, 100.0, 50.0).unit("%"),
                P::color("color", "Color", [0.0, 0.0, 0.0, 1.0]),
            ],
        ),
    });

    let stab_params = [
        P::float("smoothness", "Smooth Level", 0.0, 100.0, 50.0),
        P::float("zoom", "Zoom", 0.0, 50.0, 0.0).unit("%"),
        P::text("data", "Analysis", ""),
    ];
    r.register(StabilizeFx {
        shader: ShaderSrc::new(
            "jb.stabilize",
            format!(
                "{}\n@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return sa((in.uv - 0.5) / u.e0.z + 0.5 - u.e0.xy); }}\n",
                param_accessors(&stab_params)
            ),
        ),
        info: info(
            "jb.stabilize",
            "Stabilization",
            CAT,
            E,
            "Applies the camera-shake correction measured by the stabilization analysis (`data`: \"fps|dx,dy;...\" offsets in fractions of the frame) and zooms in to hide the moving borders. Filmora: Stabilization.",
            &stab_params,
        ),
        tracks: Mutex::new(HashMap::new()),
    });

    shader(
        r,
        info(
            "jb.color.wheels",
            "Color Wheels",
            CAT,
            E,
            "Lift/gamma/gain style color wheels: x, y push the tint of the global, shadow, midtone and highlight ranges toward a hue; lum offsets their luminance.",
            &WHEEL_PARAMS,
        ),
        r#"
fn wheel(x: f32, y: f32) -> vec3f {
    let r = min(length(vec2f(x, y)) / 100.0, 1.0);
    if (r < 1e-5) { return vec3f(0.0); }
    let rgb = hsv2rgb(vec3f(fract(atan2(-y, x) / TAU), 1.0, 1.0));
    return (rgb - vec3f(luma(rgb))) * r;
}
fn fx(uv: vec2f) -> vec4f {
    let d = to_disp(sa(uv));
    var c = clamp(d.rgb, vec3f(0.0), vec3f(1.0));
    let l = clamp(luma(c), 0.0, 1.0);
    let ws = (1.0 - l) * (1.0 - l);
    let wh = l * l;
    let wm = 1.0 - ws - wh;
    let tint = wheel(p_global_x(), p_global_y()) + ws * wheel(p_shadows_x(), p_shadows_y())
        + wm * wheel(p_midtones_x(), p_midtones_y()) + wh * wheel(p_highlights_x(), p_highlights_y());
    let lum = p_global_lum() + ws * p_shadows_lum() + wm * p_midtones_lum() + wh * p_highlights_lum();
    c = c + 0.25 * tint + vec3f(0.25 * lum / 100.0);
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), d.a));
}
"#,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stabilize_track_parses() {
        let t = parse_track("30|0.01,-0.02;0,0;-0.03,0.005").unwrap();
        assert_eq!(t.fps, 30.0);
        assert_eq!(t.offsets.len(), 3);
        assert!((t.max - 0.03).abs() < 1e-6);
        assert!(parse_track("").is_none());
        assert!(parse_track("0|1,1").is_none());
        assert!(parse_track("30|").is_none());
    }

    #[test]
    fn panel_effects_are_registered() {
        let r = Registry::builtin();
        for id in [
            "jb.corner_radius",
            "jb.chroma_key",
            "jb.denoise",
            "jb.lens_correction",
            BACKGROUND,
            SHADOW,
            "jb.stabilize",
            "jb.color.wheels",
        ] {
            assert_eq!(r.get(id).map(|e| e.info().category), Some(CAT), "{id}");
        }
        for s in MASK_SHAPES {
            assert!(r.get(&format!("jb.mask.{s}")).is_some());
        }
    }
}

//! Color-correction tools (Filmora "Color" panel), kind = Effect.
//!
//! All sliders are 0 = neutral. Grading happens on display-referred values
//! (`to_disp` / `from_disp`), except exposure and white balance which are
//! applied to linear light as a camera would.

use super::filters::{curve_image, identity_curve};
use super::{info, param_accessors, shader, FxCx, ShaderSrc, Uniforms};
use crate::fx::{Bind, Pass};
use crate::lut::Lut3d;
use crate::registry::{Effect, FxInfo, FxIo, FxKind::Effect as E, ParamSpec as P, Registry};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const CAT: &str = "Color";

/// WGSL building blocks shared by the color tools. Values in display space 0..1.
const LIB: &str = r#"
fn cc_exposure(c: vec3f, stops: f32) -> vec3f {
    return lin_to_srgb(srgb_to_lin(c) * exp2(stops));
}
// temperature/tint in -1..1: white-balance gains on linear light, luminance kept.
fn cc_white_balance(c: vec3f, temp: f32, tint: f32) -> vec3f {
    let g = vec3f(1.0 + 0.3 * temp, 1.0 - 0.25 * tint, 1.0 - 0.3 * temp) * vec3f(1.0 + 0.1 * tint, 1.0, 1.0 + 0.1 * tint);
    let gn = g / dot(g, LUMA);
    return lin_to_srgb(srgb_to_lin(c) * gn);
}
// Midtone brightness (-1..1): gamma curve keeping black and white fixed.
fn cc_brightness(c: vec3f, b: f32) -> vec3f {
    return pow(clamp(c, vec3f(0.0), vec3f(1.0)), vec3f(exp2(-b)));
}
fn cc_contrast(c: vec3f, k: f32) -> vec3f {
    // k in -1..1; above 0 the slope grows up to 2.5x around mid grey.
    let s = select(1.0 + k, 1.0 + 1.5 * k, k > 0.0);
    return (c - 0.5) * s + 0.5;
}
fn cc_saturation(c: vec3f, s: f32) -> vec3f { return mix(vec3f(luma(c)), c, 1.0 + s); }
fn cc_vibrance(c: vec3f, v: f32) -> vec3f {
    let sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
    let sc = 1.0 + v * (1.0 - sign(v) * sat);
    return mix(vec3f(luma(c)), c, sc);
}
// Tone ranges (-1..1 each), applied to luma and added back to RGB.
fn cc_tones(c: vec3f, hi: f32, sh: f32, wh: f32, bl: f32) -> vec3f {
    let l = clamp(luma(c), 0.0, 1.0);
    let ws = exp(-pow((l - 0.25) / 0.22, 2.0));
    let wh_ = exp(-pow((l - 0.75) / 0.22, 2.0));
    let ww = l * l * l;
    let wb = (1.0 - l) * (1.0 - l) * (1.0 - l);
    let d = 0.3 * (sh * ws + hi * wh_) + 0.25 * (wh * ww + bl * wb);
    return c + vec3f(d);
}
// amount -1..1: negative darkens the edges, positive brightens them.
fn cc_vignette(c: vec3f, uv: vec2f, amount: f32) -> vec3f {
    let d = length(to_c(uv)) / length(vec2f(0.5 * aspect(), 0.5));
    let f = smoothstep(0.35, 1.05, d) * abs(amount);
    if (amount < 0.0) { return c * (1.0 - f); }
    return mix(c, vec3f(1.0), f);
}
fn cc_in(uv: vec2f) -> vec4f { let d = to_disp(sa(uv)); return vec4f(clamp(d.rgb, vec3f(0.0), vec3f(1.0)), d.a); }
fn cc_out(c: vec3f, a: f32) -> vec4f { return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), a)); }
"#;

fn body(code: &str) -> String {
    format!("{LIB}\nfn fx(uv: vec2f) -> vec4f {{\n    let d = cc_in(uv);\n    var c = d.rgb;\n{code}\n    return cc_out(c, d.a);\n}}\n")
}

const fn s100(name: &'static str, label: &'static str) -> P {
    P::float(name, label, -100.0, 100.0, 0.0)
}

const HSL_PARAMS: [P; 24] = [
    s100("red_hue", "Red hue"),
    s100("red_sat", "Red saturation"),
    s100("red_lum", "Red luminance"),
    s100("orange_hue", "Orange hue"),
    s100("orange_sat", "Orange saturation"),
    s100("orange_lum", "Orange luminance"),
    s100("yellow_hue", "Yellow hue"),
    s100("yellow_sat", "Yellow saturation"),
    s100("yellow_lum", "Yellow luminance"),
    s100("green_hue", "Green hue"),
    s100("green_sat", "Green saturation"),
    s100("green_lum", "Green luminance"),
    s100("aqua_hue", "Aqua hue"),
    s100("aqua_sat", "Aqua saturation"),
    s100("aqua_lum", "Aqua luminance"),
    s100("blue_hue", "Blue hue"),
    s100("blue_sat", "Blue saturation"),
    s100("blue_lum", "Blue luminance"),
    s100("purple_hue", "Purple hue"),
    s100("purple_sat", "Purple saturation"),
    s100("purple_lum", "Purple luminance"),
    s100("magenta_hue", "Magenta hue"),
    s100("magenta_sat", "Magenta saturation"),
    s100("magenta_lum", "Magenta luminance"),
];

const HSL_BODY: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let d = cc_in(uv);
    var hsv = rgb2hsv(d.rgb);
    // Band centres in degrees; weights interpolate linearly between neighbours (sum to 1).
    var centers = array<f32, 9>(0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 270.0, 300.0, 360.0);
    let h = fract(hsv.x) * 360.0;
    var wts = array<f32, 8>(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (var i = 0; i < 8; i = i + 1) {
        if (h >= centers[i] && h < centers[i + 1]) {
            let k = (h - centers[i]) / (centers[i + 1] - centers[i]);
            wts[i] = 1.0 - k;
            wts[(i + 1) % 8] = k;
        }
    }
    var dh = 0.0;
    var ds = 0.0;
    var dl = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
        dh = dh + wts[i] * u.p[i * 3].x;
        ds = ds + wts[i] * u.p[i * 3 + 1].x;
        dl = dl + wts[i] * u.p[i * 3 + 2].x;
    }
    // Greys have no hue: scale the adjustment by saturation.
    let w = smoothstep(0.0, 0.25, hsv.y);
    hsv.x = fract(hsv.x + w * dh / 100.0 * (30.0 / 360.0) + 1.0);
    hsv.y = clamp(hsv.y * (1.0 + w * ds / 100.0), 0.0, 1.0);
    var c = hsv2rgb(hsv);
    c = c * (1.0 + w * dl / 100.0 * 0.5);
    return cc_out(c, d.a);
}
"#;

/// Curve texture lookup + optional LUT shaders.
const CURVES_BODY: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let d = cc_in(uv);
    let n = 256.0;
    var c = d.rgb * ((n - 1.0) / n) + 0.5 / n;
    c = vec3f(sx(vec2f(c.r, 0.5)).r, sx(vec2f(c.g, 0.5)).g, sx(vec2f(c.b, 0.5)).b);
    c = c * ((n - 1.0) / n) + 0.5 / n;
    c = vec3f(sx(vec2f(c.r, 0.5)).a, sx(vec2f(c.g, 0.5)).a, sx(vec2f(c.b, 0.5)).a);
    return cc_out(mix(d.rgb, c, clamp(p_intensity() / 100.0, 0.0, 1.0)), d.a);
}
"#;

const LUT_BODY: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let d = cc_in(uv);
    let n = u.e0.x;
    let coord = d.rgb * ((n - 1.0) / n) + 0.5 / n;
    let g = textureSampleLevel(tlut, smp, coord, 0.0).rgb;
    return cc_out(mix(d.rgb, g, clamp(p_intensity() / 100.0, 0.0, 1.0)), d.a);
}
"#;

fn custom_src(id: &'static str, params: &[P], body: &str) -> ShaderSrc {
    ShaderSrc::new(
        id,
        format!(
            "{}\n{LIB}\n{body}\n@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return fx(in.uv); }}\n",
            param_accessors(params)
        ),
    )
}

/// Monotone cubic (Fritsch–Carlson) curve through "x/y x/y ..." points, sampled at 256 values.
pub fn monotone_curve(spec: &str) -> Vec<f32> {
    let mut pts: Vec<(f64, f64)> = spec
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter_map(|s| {
            let (x, y) = s.split_once('/')?;
            Some((x.trim().parse::<f64>().ok()?.clamp(0.0, 1.0), y.trim().parse::<f64>().ok()?.clamp(0.0, 1.0)))
        })
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
    if pts.len() < 2 {
        return identity_curve();
    }
    let n = pts.len();
    let dx: Vec<f64> = (0..n - 1).map(|i| pts[i + 1].0 - pts[i].0).collect();
    let sl: Vec<f64> = (0..n - 1).map(|i| (pts[i + 1].1 - pts[i].1) / dx[i]).collect();
    let mut m = vec![0f64; n];
    m[0] = sl[0];
    m[n - 1] = sl[n - 2];
    for i in 1..n - 1 {
        m[i] = if sl[i - 1] * sl[i] <= 0.0 { 0.0 } else { (sl[i - 1] + sl[i]) / 2.0 };
    }
    for i in 0..n - 1 {
        if sl[i] == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        } else {
            let (a, b) = (m[i] / sl[i], m[i + 1] / sl[i]);
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                m[i] = t * a * sl[i];
                m[i + 1] = t * b * sl[i];
            }
        }
    }
    (0..256)
        .map(|k| {
            let x = k as f64 / 255.0;
            let y = if x <= pts[0].0 {
                pts[0].1
            } else if x >= pts[n - 1].0 {
                pts[n - 1].1
            } else {
                let i = (0..n - 1).find(|&i| x < pts[i + 1].0).unwrap_or(n - 2);
                let h = dx[i];
                let t = (x - pts[i].0) / h;
                let (t2, t3) = (t * t, t * t * t);
                (2.0 * t3 - 3.0 * t2 + 1.0) * pts[i].1
                    + (t3 - 2.0 * t2 + t) * h * m[i]
                    + (-2.0 * t3 + 3.0 * t2) * pts[i + 1].1
                    + (t3 - t2) * h * m[i + 1]
            };
            y.clamp(0.0, 1.0) as f32
        })
        .collect()
}

struct CurvesFx {
    info: FxInfo,
    shader: ShaderSrc,
}

impl Effect for CurvesFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let p = io.params;
        let specs = [p.text("red"), p.text("green"), p.text("blue"), p.text("master")];
        let key = format!("curves:{}|{}|{}|{}", specs[0], specs[1], specs[2], specs[3]);
        let view = cx.aux_texture(&key, || {
            let c: Vec<Vec<f32>> = specs.iter().map(|s| monotone_curve(s)).collect();
            curve_image([&c[0], &c[1], &c[2], &c[3]])
        });
        let mut pass = Pass::full(&self.shader, io.out, Uniforms::with_params(p));
        pass.a = Bind::Tex(io.a);
        pass.aux = Bind::View(&view, [256, 1]);
        cx.draw(pass);
    }
}

struct LutFx {
    info: FxInfo,
    shader: ShaderSrc,
    luts: Mutex<HashMap<String, Result<Arc<Lut3d>, String>>>,
}

impl Effect for LutFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let path = io.params.text("path").trim().to_string();
        if path.is_empty() {
            cx.copy(io.a, io.out);
            return;
        }
        let lut = {
            let mut m = self.luts.lock().unwrap();
            m.entry(path.clone()).or_insert_with(|| Lut3d::load(&path).map(Arc::new).map_err(|e| e.to_string())).clone()
        };
        let lut = match lut {
            Ok(l) => l,
            Err(e) => {
                let msg = format!("jb.color.lut: cannot load {path}: {e}");
                if !cx.report.errors.contains(&msg) {
                    cx.report.errors.push(msg);
                }
                cx.copy(io.a, io.out);
                return;
            }
        };
        let view = cx.aux_texture(&format!("lut:{path}"), || lut.to_aux());
        let mut u = Uniforms::with_params(io.params);
        u.e[0] = [lut.size as f32, 0.0, 0.0, 0.0];
        let mut pass = Pass::full(&self.shader, io.out, u);
        pass.a = Bind::Tex(io.a);
        pass.lut = Some(&view);
        cx.draw(pass);
    }
}

pub fn register(r: &mut Registry) {
    let one = |r: &mut Registry, id, name, desc, params: &[P], code: &str| {
        shader(r, info(id, name, CAT, E, desc, params), body(code));
    };
    one(
        r,
        "jb.color.exposure",
        "Exposure",
        "Exposure in stops, applied to linear light.",
        &[P::float("stops", "Exposure", -4.0, 4.0, 0.0).unit("EV")],
        "    c = cc_exposure(c, p_stops());",
    );
    one(
        r,
        "jb.color.brightness",
        "Brightness",
        "Midtone brightness; black and white stay fixed.",
        &[s100("amount", "Brightness")],
        "    c = cc_brightness(c, p_amount() / 100.0);",
    );
    one(
        r,
        "jb.color.contrast",
        "Contrast",
        "Contrast around mid grey.",
        &[s100("amount", "Contrast")],
        "    c = cc_contrast(c, p_amount() / 100.0);",
    );
    one(
        r,
        "jb.color.saturation",
        "Saturation",
        "Color saturation (-100 = greyscale).",
        &[s100("amount", "Saturation")],
        "    c = cc_saturation(c, p_amount() / 100.0);",
    );
    one(
        r,
        "jb.color.vibrance",
        "Vibrance",
        "Saturation boost that protects already saturated colors.",
        &[s100("amount", "Vibrance")],
        "    c = cc_vibrance(c, p_amount() / 100.0);",
    );
    one(
        r,
        "jb.color.temperature",
        "White Balance",
        "Color temperature (blue <-> amber) and tint (green <-> magenta).",
        &[s100("temperature", "Temperature"), s100("tint", "Tint")],
        "    c = cc_white_balance(c, p_temperature() / 100.0, p_tint() / 100.0);",
    );
    one(
        r,
        "jb.color.tint",
        "Tint",
        "Green <-> magenta balance.",
        &[s100("amount", "Tint")],
        "    c = cc_white_balance(c, 0.0, p_amount() / 100.0);",
    );
    one(
        r,
        "jb.color.highlights_shadows",
        "Highlights & Shadows",
        "Tone range adjustments: highlights, shadows, whites and blacks.",
        &[
            s100("highlights", "Highlights"),
            s100("shadows", "Shadows"),
            s100("whites", "Whites"),
            s100("blacks", "Blacks"),
        ],
        "    c = cc_tones(c, p_highlights() / 100.0, p_shadows() / 100.0, p_whites() / 100.0, p_blacks() / 100.0);",
    );
    shader(
        r,
        info("jb.color.hsl", "HSL", CAT, E, "Hue, saturation and luminance per color band (8 bands).", &HSL_PARAMS),
        format!("{LIB}\n{HSL_BODY}"),
    );
    one(
        r,
        "jb.color.vignette",
        "Color Vignette",
        "Darkens (negative) or brightens (positive) the frame edges.",
        &[s100("amount", "Amount")],
        "    c = cc_vignette(c, uv, p_amount() / 100.0);",
    );
    one(
        r,
        "jb.color.adjust",
        "Color Correction",
        "Filmora-like Color panel: exposure, brightness, contrast, saturation, vibrance, white balance, tones and vignette.",
        &[
            s100("exposure", "Exposure"),
            s100("brightness", "Brightness"),
            s100("contrast", "Contrast"),
            s100("saturation", "Saturation"),
            s100("vibrance", "Vibrance"),
            s100("temperature", "Temperature"),
            s100("tint", "Tint"),
            s100("highlights", "Highlights"),
            s100("shadows", "Shadows"),
            s100("whites", "Whites"),
            s100("blacks", "Blacks"),
            s100("vignette", "Vignette"),
        ],
        r#"    c = cc_exposure(c, p_exposure() / 50.0);
    c = cc_white_balance(c, p_temperature() / 100.0, p_tint() / 100.0);
    c = clamp(cc_tones(c, p_highlights() / 100.0, p_shadows() / 100.0, p_whites() / 100.0, p_blacks() / 100.0), vec3f(0.0), vec3f(1.0));
    c = cc_brightness(c, p_brightness() / 100.0);
    c = clamp(cc_contrast(c, p_contrast() / 100.0), vec3f(0.0), vec3f(1.0));
    c = cc_saturation(c, p_saturation() / 100.0);
    c = clamp(cc_vibrance(c, p_vibrance() / 100.0), vec3f(0.0), vec3f(1.0));
    c = cc_vignette(c, uv, p_vignette() / 100.0);"#,
    );

    let curve_params = [
        P::text("master", "Master curve", "0/0 1/1"),
        P::text("red", "Red curve", "0/0 1/1"),
        P::text("green", "Green curve", "0/0 1/1"),
        P::text("blue", "Blue curve", "0/0 1/1"),
        P::float("intensity", "Intensity", 0.0, 100.0, 100.0).unit("%"),
    ];
    r.register(CurvesFx {
        shader: custom_src("jb.color.curves", &curve_params, CURVES_BODY),
        info: info(
            "jb.color.curves",
            "Curves",
            CAT,
            E,
            "RGB curves: points as \"x/y x/y ...\" in 0..1 per channel; channels first, then master.",
            &curve_params,
        ),
    });
    let lut_params =
        [P::text("path", "LUT file (.cube)", ""), P::float("intensity", "Intensity", 0.0, 100.0, 100.0).unit("%")];
    r.register(LutFx {
        shader: custom_src("jb.color.lut", &lut_params, LUT_BODY),
        info: info("jb.color.lut", "LUT", CAT, E, "Applies a 3D/1D .cube LUT to display-referred colors.", &lut_params),
        luts: Mutex::new(HashMap::new()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotone_identity_and_points() {
        let id = monotone_curve("0/0 1/1");
        assert!(id.iter().enumerate().all(|(i, v)| (v - i as f32 / 255.0).abs() < 1e-5));
        let c = monotone_curve("0/0.1 0.5/0.7 1/1");
        assert!((c[0] - 0.1).abs() < 1e-5 && (c[255] - 1.0).abs() < 1e-5);
        assert!(c.windows(2).all(|w| w[1] >= w[0]));
        assert_eq!(monotone_curve("garbage"), identity_curve());
    }
}

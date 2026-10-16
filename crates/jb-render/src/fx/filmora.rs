//! Filmora-compatible wrappers: effects whose parameters use Filmora's own
//! names (so imported `filmora:<id>` instances render without translation)
//! and one-knob effects with no generic equivalent.

use super::{info, shader, FxCx};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec, ParamValues, Registry};
use jb_core::param::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Exposes `inner` under another id with renamed parameters.
pub struct RemapFx {
    pub info: FxInfo,
    pub inner: Arc<dyn Effect>,
    /// (outer name, inner name, scale applied to the value).
    pub map: Vec<(&'static str, &'static str, f32)>,
}

impl RemapFx {
    fn inner_params(&self, p: &ParamValues) -> ParamValues {
        let mut given = BTreeMap::new();
        for (outer, inner, k) in &self.map {
            given.insert(inner.to_string(), Value::Float((p.get(outer) * k) as f64));
        }
        ParamValues::resolve(self.inner.info(), &given)
    }
}

impl Effect for RemapFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let params = self.inner_params(io.params);
        self.inner.encode(cx, &FxIo { params: &params, ..*io });
    }

    fn animate(&self, state: &mut crate::registry::AnimState, progress: f32, params: &ParamValues) {
        self.inner.animate(state, progress, &self.inner_params(params))
    }
}

/// Filmora "AdjustColor": `u_exposure` renders through Filmora's own
/// exposure curve ([`EXPOSURE`]), the other sliders through `jb.color.adjust`.
pub struct AdjustColorFx {
    pub info: FxInfo,
    pub exposure: Arc<dyn Effect>,
    pub adjust: RemapFx,
}

impl Effect for AdjustColorFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let ev = io.params.get("u_exposure");
        let others = ADJUST_COLOR.iter().any(|(outer, _, _)| io.params.get(outer) != 0.0);
        if ev == 0.0 {
            return self.adjust.encode(cx, io);
        }
        let given = BTreeMap::from([("exposure".to_string(), Value::Float(ev as f64))]);
        let params = ParamValues::resolve(self.exposure.info(), &given);
        if !others {
            return self.exposure.encode(cx, &FxIo { params: &params, ..*io });
        }
        let tmp = cx.temp_like(io.out);
        self.exposure.encode(cx, &FxIo { params: &params, out: &tmp, ..*io });
        self.adjust.encode(cx, &FxIo { a: &tmp, ..*io });
    }
}

/// Filmora's exposure slider, fitted on its exports of the user projects
/// (u_exposure 45, 46, 60, 92 and 100 against the same frames without the
/// effect): in display space `y = 1 - (1 - min(g x, 1))^k` with
/// `k = 1 + 0.01256 * min(s, 50)^1.244` (2.43 at 45, 2.55 from 50 on) and
/// `g = 1 + 0.0145 * max(s - 50, 0)` (1.15 at 60, 1.61 at 92, 1.73 at 100):
/// above 50 the slider stops bending the curve and scales the input
/// instead, so highlights clip (at x ≈ 0.58 for 100). The input already
/// carries the decode-side level offset (`RenderOptions::input_bias`),
/// which the gain amplifies per channel as in Filmora. Negative values
/// darken with the mirrored `x^k` (not observed).
const EXPOSURE: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let d = to_disp(sa(uv));
    let s = p_exposure();
    let a = abs(s);
    let k = 1.0 + 0.01256 * pow(min(a, 50.0), 1.244);
    let g = 1.0 + 0.0145 * max(a - 50.0, 0.0);
    let x = clamp(d.rgb, vec3f(0.0), vec3f(1.0));
    var y = pow(x, vec3f(k));
    if (s > 0.0) { y = 1.0 - pow(1.0 - min(g * x, vec3f(1.0)), vec3f(k)); }
    return from_disp(vec4f(y, d.a));
}
"#;

/// Filmora "AdjustColor" slider names -> `jb.color.adjust` parameters
/// (`u_exposure` is handled by [`EXPOSURE`]).
const ADJUST_COLOR: &[(&str, &str, f32)] = &[
    ("u_brightness", "brightness", 1.0),
    ("u_contrast", "contrast", 1.0),
    ("u_saturation", "saturation", 1.0),
    ("u_vibrance", "vibrance", 1.0),
    ("u_temperature", "temperature", 1.0),
    ("u_tint", "tint", 1.0),
    ("u_highlight", "highlights", 1.0),
    ("u_highlights", "highlights", 1.0),
    ("u_shadow", "shadows", 1.0),
    ("u_shadows", "shadows", 1.0),
    ("u_white", "whites", 1.0),
    ("u_whites", "whites", 1.0),
    ("u_black", "blacks", 1.0),
    ("u_blacks", "blacks", 1.0),
    ("u_vignette", "vignette", 1.0),
];

/// Filmora's Slide (store pack), fitted on its export of the user's project
/// (Colombia 1 at 96.6 s, 2 s, center): every frame of each side is the clip
/// mapped through `src = c + (a0, b0) + k R(phi) (uv - c)` in uv space
/// (c = frame centre; the rotation is in uv units, so it shears a 16:9 frame
/// slightly), sampled with mirrored repeat, then box-blurred along
/// ±`(bx, by)` uv (motion blur). Rows are `[p, a0, b0, k, phi, bx, by]` at the frame
/// times of the fitted export; the outgoing clip zooms in to ~1.07 and tilts
/// to 3.6° before it whips right (a0 → -0.23 on its last frame), the
/// incoming one lands at 1.10 / 5.0° shifted 0.29 to the left and settles
/// with an ease-out. Identity outside the rows.
#[allow(clippy::approx_constant)] // fitted data, not constants
const SLIDE_FIT: &[[f32; 7]] = &[
    [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
    [0.0833, 0.0004, 0.0000, 1.0008, 0.0006, 0.0000, 0.0000],
    [0.1000, 0.0006, 0.0000, 1.0009, 0.0010, 0.0000, 0.0000],
    [0.1167, 0.0010, 0.0001, 1.0007, 0.0016, 0.0000, 0.0000],
    [0.1333, 0.0013, 0.0001, 1.0005, 0.0021, 0.0003, 0.0001],
    [0.1500, 0.0017, 0.0002, 1.0003, 0.0027, 0.0004, 0.0001],
    [0.1667, 0.0022, 0.0002, 0.9998, 0.0034, 0.0004, 0.0003],
    [0.1833, 0.0027, 0.0002, 0.9992, 0.0042, 0.0005, 0.0005],
    [0.2000, 0.0033, 0.0002, 0.9984, 0.0051, 0.0005, 0.0005],
    [0.2167, 0.0040, 0.0003, 0.9976, 0.0061, 0.0006, 0.0005],
    [0.2333, 0.0047, 0.0003, 0.9966, 0.0072, 0.0006, 0.0003],
    [0.2500, 0.0055, 0.0003, 0.9958, 0.0085, 0.0008, 0.0004],
    [0.2667, 0.0065, 0.0005, 0.9943, 0.0101, 0.0008, 0.0008],
    [0.2833, 0.0075, 0.0006, 0.9929, 0.0117, 0.0010, 0.0009],
    [0.3000, 0.0087, 0.0007, 0.9912, 0.0135, 0.0010, 0.0011],
    [0.3167, 0.0100, 0.0008, 0.9893, 0.0155, 0.0010, 0.0013],
    [0.3333, 0.0114, 0.0009, 0.9869, 0.0177, 0.0013, 0.0013],
    [0.3500, 0.0130, 0.0010, 0.9844, 0.0202, 0.0014, 0.0014],
    [0.3667, 0.0148, 0.0012, 0.9815, 0.0230, 0.0015, 0.0013],
    [0.3833, 0.0169, 0.0015, 0.9782, 0.0264, 0.0015, 0.0014],
    [0.4000, 0.0192, 0.0017, 0.9744, 0.0300, 0.0016, 0.0015],
    [0.4167, 0.0167, 0.0020, 0.9700, 0.0342, 0.0017, 0.0019],
    [0.4333, 0.0086, 0.0024, 0.9649, 0.0390, 0.0023, 0.0018],
    [0.4500, -0.0131, 0.0029, 0.9595, 0.0449, 0.0033, 0.0020],
    [0.4667, -0.0684, 0.0036, 0.9501, 0.0524, 0.0053, 0.0029],
    [0.4833, -0.2263, 0.0044, 0.9366, 0.0634, 0.0063, 0.0041],
    [0.5000, 0.2877, 0.0075, 0.9091, 0.0872, 0.0074, -0.0010],
    [0.5167, 0.2099, 0.0075, 0.9093, 0.0871, 0.0057, -0.0016],
    [0.5333, 0.1786, 0.0075, 0.9098, 0.0869, 0.0051, -0.0019],
    [0.5500, 0.1560, 0.0075, 0.9099, 0.0863, 0.0049, -0.0016],
    [0.5667, 0.1379, 0.0074, 0.9103, 0.0856, 0.0048, -0.0015],
    [0.5833, 0.1226, 0.0072, 0.9109, 0.0848, 0.0045, -0.0014],
    [0.6000, 0.1095, 0.0071, 0.9119, 0.0838, 0.0044, -0.0013],
    [0.6167, 0.0980, 0.0070, 0.9131, 0.0824, 0.0043, -0.0011],
    [0.6333, 0.0876, 0.0067, 0.9144, 0.0813, 0.0041, -0.0008],
    [0.6500, 0.0784, 0.0065, 0.9161, 0.0795, 0.0040, -0.0006],
    [0.6667, 0.0700, 0.0063, 0.9180, 0.0774, 0.0037, -0.0005],
    [0.6833, 0.0625, 0.0060, 0.9203, 0.0751, 0.0035, -0.0003],
    [0.7000, 0.0557, 0.0058, 0.9228, 0.0725, 0.0033, -0.0003],
    [0.7167, 0.0495, 0.0055, 0.9256, 0.0697, 0.0030, 0.0004],
    [0.7333, 0.0443, 0.0051, 0.9289, 0.0668, 0.0027, 0.0004],
    [0.7500, 0.0401, 0.0048, 0.9324, 0.0634, 0.0024, 0.0004],
    [0.7667, 0.0379, 0.0043, 0.9363, 0.0599, 0.0020, 0.0005],
    [0.7833, 0.0355, 0.0040, 0.9405, 0.0558, 0.0019, 0.0004],
    [0.8000, 0.0328, 0.0035, 0.9449, 0.0517, 0.0018, 0.0005],
    [0.8167, 0.0300, 0.0032, 0.9496, 0.0472, 0.0015, 0.0004],
    [0.8333, 0.0271, 0.0027, 0.9547, 0.0426, 0.0013, 0.0003],
    [0.8500, 0.0240, 0.0023, 0.9597, 0.0379, 0.0010, 0.0004],
    [0.8667, 0.0210, 0.0020, 0.9651, 0.0329, 0.0008, 0.0003],
    [0.8833, 0.0179, 0.0016, 0.9705, 0.0280, 0.0005, 0.0003],
    [0.9000, 0.0148, 0.0013, 0.9756, 0.0231, 0.0004, 0.0001],
    [0.9167, 0.0119, 0.0010, 0.9808, 0.0184, 0.0003, 0.0001],
    [0.9333, 0.0090, 0.0007, 0.9854, 0.0140, 0.0000, 0.0000],
    [0.9500, 0.0063, 0.0005, 0.9898, 0.0099, 0.0000, 0.0000],
    [0.9667, 0.0040, 0.0003, 0.9938, 0.0061, 0.0000, 0.0000],
    [0.9833, 0.0019, 0.0002, 0.9972, 0.0027, 0.0000, 0.0000],
    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
];

/// WGSL for [`SLIDE_FIT`]: interpolates the rows of the current side.
fn slide_body() -> String {
    let n = SLIDE_FIT.len();
    let cut = SLIDE_FIT.iter().position(|r| r[0] >= 0.5).unwrap_or(n);
    let row = |r: &[f32; 7]| format!("vec4f({}, {}, {}, {})", r[1], r[2], r[3], r[4]);
    let geo: Vec<String> = SLIDE_FIT.iter().map(row).collect();
    let pb: Vec<String> = SLIDE_FIT.iter().map(|r| format!("vec4f({}, {}, {}, 0.0)", r[0], r[5], r[6])).collect();
    format!(
        r#"
const SLIDE_N: i32 = {n};
const SLIDE_CUT: i32 = {cut};
var<private> SLIDE_GEO: array<vec4f, {n}> = array<vec4f, {n}>({});
var<private> SLIDE_PB: array<vec4f, {n}> = array<vec4f, {n}>({});

fn mirror1(x: f32) -> f32 {{
    let f = x - 2.0 * floor(x * 0.5);
    return select(f, 2.0 - f, f > 1.0);
}}

// (a0, b0, k, phi) and the blur vector at progress p, interpolated within one side.
fn slide_at(p: f32, geo: ptr<function, vec4f>) -> vec2f {{
    var lo = 0;
    var hi = SLIDE_CUT - 1;
    if (p >= 0.5) {{ lo = SLIDE_CUT; hi = SLIDE_N - 1; }}
    if (p <= SLIDE_PB[lo].x) {{ *geo = SLIDE_GEO[lo]; return SLIDE_PB[lo].yz; }}
    if (p >= SLIDE_PB[hi].x) {{ *geo = SLIDE_GEO[hi]; return SLIDE_PB[hi].yz; }}
    for (var i = lo; i < hi; i = i + 1) {{
        if (p < SLIDE_PB[i + 1].x) {{
            let k = (p - SLIDE_PB[i].x) / max(SLIDE_PB[i + 1].x - SLIDE_PB[i].x, 1e-6);
            *geo = mix(SLIDE_GEO[i], SLIDE_GEO[i + 1], k);
            return mix(SLIDE_PB[i].yz, SLIDE_PB[i + 1].yz, k);
        }}
    }}
    *geo = SLIDE_GEO[hi];
    return SLIDE_PB[hi].yz;
}}

fn slide_sample(use_b: bool, q: vec2f) -> vec4f {{
    let m = vec2f(mirror1(q.x), mirror1(q.y));
    return select(sa(m), sb(m), use_b);
}}

fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    var g: vec4f;
    let blur = slide_at(p, &g) * p_blur();
    let d = uv - vec2f(0.5);
    let cs = cos(g.w);
    let sn = sin(g.w);
    let shift = vec2f(g.x * p_distance() / 0.25, g.y);
    let q = vec2f(0.5) + shift + g.z * vec2f(cs * d.x - sn * d.y, sn * d.x + cs * d.y);
    let use_b = p >= 0.5;
    if (dot(blur, blur) <= 1e-9) {{ return slide_sample(use_b, q); }}
    // Box blur along the motion, jittered taps.
    let j = hash12(uv * u.size);
    var acc = vec4f(0.0);
    for (var i = 0; i < 16; i = i + 1) {{
        let t = (f32(i) + j) / 16.0 * 2.0 - 1.0;
        acc = acc + slide_sample(use_b, q + t * blur);
    }}
    return acc / 16.0;
}}
"#,
        geo.join(", "),
        pb.join(", ")
    )
}

/// Filmora's Film Burn Transition 02, fitted on its export of the user's
/// project (Colombia 2 at 313.9 s, 2 s, center), procedurally: each frame of
/// either side is a radial zoom blur of the clip (scaled by `s` about
/// `(0.5 + cx, 0.5 + cy)`, rays averaged over a fraction `amt` towards that
/// centre) followed by a per-channel display-space affine
/// `g * c + o0 + o1 * (u - 0.5)`. The horizontal term carries the warm leak
/// (p ≈ 0.47) and the dark brown burn (p ≈ 0.48); sprocket holes are drawn
/// at both edges while the frame washes out. Rows are
/// `[p, s, amt, cx, cy, g.rgb, o0.rgb, o1.rgb]` at the fitted export's frame
/// times; offsets include Filmora's output level offset. Identity outside.
#[allow(clippy::approx_constant)] // fitted data, not constants
const BURN_FIT: &[[f32; 14]] = &[
    [0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0167, 1.0000, 0.0000, 0.0000, 0.0000, 1.0011, 0.9999, 0.9980, 0.0070, 0.0058, 0.0097, -0.0002, -0.0004, -0.0009],
    [0.0333, 1.0020, 0.0000, -0.0680, -0.0050, 1.0118, 1.0097, 1.0076, 0.0021, 0.0019, 0.0060, 0.0028, 0.0030, 0.0016],
    [0.0500, 1.0020, 0.0040, -0.3630, 0.2250, 1.0176, 1.0141, 1.0117, -0.0005, -0.0001, 0.0037, 0.0052, 0.0055, 0.0047],
    [0.0667, 1.0050, 0.0040, -0.4080, 0.2500, 1.0065, 1.0065, 1.0043, 0.0051, 0.0032, 0.0060, 0.0035, 0.0045, 0.0035],
    [0.0833, 1.0050, 0.0190, -0.2270, 0.2300, 1.0139, 1.0128, 1.0099, 0.0018, 0.0008, 0.0041, 0.0045, 0.0054, 0.0043],
    [
        0.1000, 1.0150, 0.0190, -0.2020, -0.0030, 0.9996, 1.0024, 0.9997, 0.0085, 0.0049, 0.0075, -0.0007, 0.0004,
        -0.0008,
    ],
    [0.1167, 1.0100, 0.0410, -0.2000, 0.0050, 1.0099, 1.0096, 1.0069, 0.0038, 0.0022, 0.0049, 0.0035, 0.0040, 0.0029],
    [0.1333, 1.0120, 0.0560, -0.1870, 0.0020, 1.0093, 1.0086, 1.0063, 0.0042, 0.0028, 0.0054, 0.0032, 0.0037, 0.0020],
    [0.1500, 1.0150, 0.0710, -0.1770, 0.0020, 1.0064, 1.0068, 1.0063, 0.0060, 0.0038, 0.0058, 0.0024, 0.0027, 0.0015],
    [0.1667, 1.0200, 0.0860, -0.1700, -0.0000, 1.0040, 1.0036, 1.0036, 0.0070, 0.0052, 0.0068, 0.0011, 0.0011, -0.0004],
    [
        0.1833, 1.0250, 0.1010, -0.1600, -0.0030, 1.0000, 0.9994, 0.9976, 0.0097, 0.0075, 0.0086, -0.0007, 0.0001,
        -0.0009,
    ],
    [0.2000, 1.0220, 0.1280, -0.1500, -0.0000, 1.0051, 1.0041, 1.0024, 0.0078, 0.0059, 0.0073, 0.0019, 0.0032, 0.0013],
    [
        0.2167, 1.0300, 0.1430, -0.1420, -0.0000, 0.9998, 0.9987, 0.9972, 0.0102, 0.0084, 0.0098, -0.0010, -0.0002,
        -0.0022,
    ],
    [0.2333, 1.0350, 0.1580, -0.1350, 0.0020, 0.9934, 0.9925, 0.9912, 0.0136, 0.0115, 0.0124, 0.0002, -0.0001, -0.0015],
    [0.2500, 1.0380, 0.1800, -0.1250, 0.0020, 0.9917, 0.9913, 0.9889, 0.0151, 0.0128, 0.0144, 0.0009, 0.0001, -0.0023],
    [
        0.2667, 1.0430, 0.1990, -0.1170, -0.0000, 0.9863, 0.9876, 0.9825, 0.0173, 0.0146, 0.0168, 0.0005, -0.0002,
        -0.0027,
    ],
    [0.2833, 1.0450, 0.2210, -0.1100, -0.0000, 0.9832, 0.9857, 0.9806, 0.0196, 0.0161, 0.0182, 0.0023, 0.0019, -0.0006],
    [0.3000, 1.0500, 0.2360, -0.1000, -0.0000, 0.9752, 0.9790, 0.9746, 0.0247, 0.0205, 0.0219, 0.0031, 0.0041, 0.0015],
    [0.3167, 1.0530, 0.2590, -0.0920, 0.0020, 0.9700, 0.9744, 0.9701, 0.0290, 0.0244, 0.0256, 0.0034, 0.0040, 0.0017],
    [0.3333, 1.0500, 0.2850, -0.0820, -0.0000, 0.9616, 0.9648, 0.9652, 0.0358, 0.0309, 0.0290, 0.0091, 0.0086, 0.0086],
    [
        0.3500, 1.0530, 0.3080, -0.0750, -0.0000, 0.7832, 0.7879, 0.8052, 0.0544, 0.0465, 0.0346, -0.0008, -0.0010,
        0.0073,
    ],
    [
        0.3667, 1.0500, 0.3300, -0.0650, -0.0000, 0.5976, 0.5894, 0.6202, 0.1019, 0.0966, 0.0691, -0.0264, -0.0429,
        -0.0384,
    ],
    [
        0.3833, 1.0550, 0.3300, -0.0570, -0.0000, 0.4128, 0.3869, 0.4477, 0.1970, 0.1976, 0.1557, -0.0168, -0.0272,
        -0.0368,
    ],
    [
        0.4000, 1.0600, 0.3080, -0.0520, 0.0020, 0.2471, 0.2307, 0.2535, 0.3300, 0.3264, 0.3106, -0.0243, -0.0288,
        -0.0205,
    ],
    [
        0.4167, 1.0680, 0.3080, -0.0420, 0.0020, 0.2374, 0.2201, 0.2433, 0.3371, 0.3333, 0.3170, -0.0260, -0.0309,
        -0.0227,
    ],
    [
        0.4333, 1.0650, 0.3680, -0.0350, -0.0050, 0.1841, 0.1895, 0.1918, 0.4706, 0.4609, 0.4621, -0.0115, -0.0120,
        -0.0110,
    ],
    [
        0.4500, 1.0530, 0.3900, -0.0250, -0.0080, 0.0686, 0.0767, 0.0727, 0.7597, 0.7508, 0.7588, -0.0022, -0.0010,
        -0.0023,
    ],
    [0.4667, 1.1030, 0.4000, -0.0220, -0.0100, 0.0866, 0.1605, 0.2030, 0.7022, 0.6491, 0.5883, -0.0001, 0.0698, 0.2676],
    [0.4833, 1.1030, 0.4000, -0.0220, -0.0100, -0.0219, 0.0023, 0.0333, 0.3137, 0.2227, 0.1698, 0.3396, 0.2545, 0.1896],
    [0.5000, 1.0000, 0.5000, 0.0200, 0.0000, 0.0675, 0.0758, 0.0525, 0.0630, 0.0440, 0.0638, -0.0187, -0.0177, -0.0164],
    [0.5167, 1.0000, 0.5000, 0.0200, 0.0000, 0.0774, 0.1084, 0.0956, 0.0602, 0.0252, 0.0391, -0.0142, -0.0143, -0.0137],
    [0.5333, 1.0000, 0.5000, 0.0200, 0.0000, 0.1224, 0.1237, 0.1434, 0.0499, 0.0360, 0.0274, -0.0129, -0.0147, -0.0109],
    [0.5500, 1.0000, 0.5000, 0.0200, 0.0000, 0.1581, 0.1533, 0.1847, 0.0422, 0.0331, 0.0166, -0.0124, -0.0130, -0.0108],
    [
        0.5667, 0.9970, 0.4530, 0.0280, -0.0030, 0.1963, 0.1879, 0.2150, 0.0363, 0.0300, 0.0167, -0.0106, -0.0100,
        -0.0108,
    ],
    [0.5833, 1.0170, 0.4040, 0.0370, 0.0000, 0.2353, 0.2022, 0.2426, 0.0267, 0.0388, 0.0151, -0.0076, -0.0089, -0.0092],
    [0.6000, 1.0270, 0.3890, 0.0480, 0.0000, 0.2637, 0.2394, 0.2769, 0.0274, 0.0341, 0.0119, -0.0062, -0.0076, -0.0068],
    [0.6167, 1.0350, 0.3550, 0.0550, 0.0000, 0.2634, 0.2419, 0.2793, 0.0290, 0.0340, 0.0116, -0.0047, -0.0058, -0.0051],
    [
        0.6333, 1.0350, 0.3330, 0.0650, -0.0030, 0.2982, 0.2737, 0.3164, 0.0252, 0.0328, 0.0068, -0.0037, -0.0051,
        -0.0039,
    ],
    [
        0.6500, 1.0400, 0.3180, 0.0720, -0.0030, 0.3470, 0.2905, 0.3587, 0.0079, 0.0392, -0.0044, -0.0034, -0.0035,
        -0.0044,
    ],
    [
        0.6667, 1.0470, 0.2870, 0.0820, -0.0030, 0.3844, 0.3091, 0.3890, 0.0008, 0.0468, -0.0058, -0.0024, -0.0037,
        -0.0035,
    ],
    [
        0.6833, 1.0400, 0.2720, 0.0900, 0.0000, 0.4126, 0.3443, 0.4061, -0.0014, 0.0405, -0.0006, -0.0010, -0.0038,
        -0.0022,
    ],
    [
        0.7000, 1.0450, 0.2420, 0.1000, -0.0030, 0.4487, 0.3863, 0.4356, -0.0072, 0.0309, -0.0015, -0.0010, -0.0050,
        -0.0017,
    ],
    [
        0.7167, 1.0450, 0.2240, 0.1070, 0.0000, 0.4565, 0.3985, 0.4392, -0.0126, 0.0226, -0.0033, -0.0004, -0.0053,
        -0.0013,
    ],
    [
        0.7333, 1.0370, 0.2090, 0.1150, 0.0000, 0.4971, 0.4532, 0.4855, -0.0242, 0.0004, -0.0177, -0.0004, -0.0067,
        -0.0009,
    ],
    [
        0.7500, 1.0350, 0.1860, 0.1250, -0.0030, 0.5198, 0.4852, 0.5057, -0.0201, -0.0025, -0.0120, -0.0005, -0.0073,
        0.0009,
    ],
    [
        0.7667, 1.0270, 0.1710, 0.1320, -0.0030, 0.5377, 0.5100, 0.5402, -0.0154, -0.0027, -0.0191, 0.0010, -0.0074,
        0.0005,
    ],
    [
        0.7833, 1.0250, 0.1490, 0.1400, -0.0030, 0.5904, 0.5603, 0.5700, -0.0334, -0.0181, -0.0201, 0.0012, -0.0064,
        0.0018,
    ],
    [
        0.8000, 1.0250, 0.1260, 0.1500, -0.0030, 0.6099, 0.5963, 0.6045, -0.0271, -0.0238, -0.0245, 0.0009, -0.0060,
        0.0012,
    ],
    [
        0.8167, 1.0220, 0.1070, 0.1570, -0.0050, 0.6111, 0.5927, 0.6036, -0.0279, -0.0211, -0.0239, 0.0008, -0.0051,
        0.0016,
    ],
    [
        0.8333, 1.0170, 0.0890, 0.1650, -0.0050, 0.6332, 0.6042, 0.6281, -0.0270, -0.0127, -0.0241, 0.0007, -0.0047,
        0.0014,
    ],
    [
        0.8500, 1.0150, 0.0740, 0.1750, -0.0050, 0.6630, 0.6475, 0.6541, -0.0283, -0.0244, -0.0231, 0.0016, -0.0038,
        0.0010,
    ],
    [
        0.8667, 1.0120, 0.0590, 0.1820, -0.0050, 0.6906, 0.6814, 0.6752, -0.0312, -0.0319, -0.0214, 0.0016, -0.0045,
        0.0007,
    ],
    [
        0.8833, 1.0100, 0.0440, 0.1900, -0.0070, 0.7208, 0.7196, 0.7014, -0.0332, -0.0396, -0.0206, 0.0015, -0.0042,
        0.0011,
    ],
    [
        0.9000, 1.0020, 0.0400, 0.1950, -0.0100, 0.7476, 0.7525, 0.7263, -0.0355, -0.0464, -0.0213, 0.0022, -0.0038,
        0.0018,
    ],
    [
        0.9167, 1.0020, 0.0250, 0.2180, -0.0100, 0.7436, 0.7453, 0.7220, -0.0325, -0.0410, -0.0185, 0.0023, -0.0041,
        0.0021,
    ],
    [
        0.9333, 1.0000, 0.0210, 0.2030, -0.0150, 0.7803, 0.7911, 0.7671, -0.0384, -0.0538, -0.0297, 0.0023, -0.0050,
        0.0023,
    ],
    [
        0.9500, 0.9970, 0.0140, 0.2180, -0.0350, 0.7903, 0.7951, 0.7742, -0.0282, -0.0394, -0.0179, 0.0011, -0.0053,
        0.0017,
    ],
    [
        0.9667, 0.9980, 0.0070, 0.1300, -0.0350, 0.8108, 0.8080, 0.8033, -0.0231, -0.0288, -0.0194, 0.0001, -0.0063,
        0.0003,
    ],
    [
        0.9833, 1.0000, 0.0000, 0.0000, 0.0000, 0.8251, 0.8109, 0.8107, -0.0144, -0.0117, -0.0065, -0.0016, -0.0070,
        -0.0005,
    ],
    [1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
];

/// WGSL for [`BURN_FIT`].
fn film_burn_body() -> String {
    let n = BURN_FIT.len();
    let cut = BURN_FIT.iter().position(|r| r[0] >= 0.5).unwrap_or(n);
    let v4 = |a: f32, b: f32, c: f32, d: f32| format!("vec4f({a}, {b}, {c}, {d})");
    let geo: Vec<String> = BURN_FIT.iter().map(|r| v4(r[1], r[2], r[3], r[4])).collect();
    let gain: Vec<String> = BURN_FIT.iter().map(|r| v4(r[5], r[6], r[7], r[0])).collect();
    let o0: Vec<String> = BURN_FIT.iter().map(|r| v4(r[8], r[9], r[10], 0.0)).collect();
    let o1: Vec<String> = BURN_FIT.iter().map(|r| v4(r[11], r[12], r[13], 0.0)).collect();
    format!(
        r#"
const BURN_N: i32 = {n};
const BURN_CUT: i32 = {cut};
var<private> BURN_GEO: array<vec4f, {n}> = array<vec4f, {n}>({});
var<private> BURN_GAIN: array<vec4f, {n}> = array<vec4f, {n}>({});
var<private> BURN_O0: array<vec4f, {n}> = array<vec4f, {n}>({});
var<private> BURN_O1: array<vec4f, {n}> = array<vec4f, {n}>({});

// Row index i and blend k for progress p within the current side.
fn burn_row(p: f32) -> vec2f {{
    var lo = 0;
    var hi = BURN_CUT - 1;
    if (p >= 0.5) {{ lo = BURN_CUT; hi = BURN_N - 1; }}
    if (p <= BURN_GAIN[lo].w) {{ return vec2f(f32(lo), 0.0); }}
    for (var i = lo; i < hi; i = i + 1) {{
        if (p < BURN_GAIN[i + 1].w) {{
            return vec2f(f32(i), (p - BURN_GAIN[i].w) / max(BURN_GAIN[i + 1].w - BURN_GAIN[i].w, 1e-6));
        }}
    }}
    return vec2f(f32(hi), 0.0);
}}

// Film-strip perforations at both side edges (display-space coverage).
fn sprockets(uv: vec2f) -> f32 {{
    let edge = max(1.0 - smoothstep(0.007, 0.011, uv.x), smoothstep(0.986, 0.990, uv.x));
    let cell = fract((uv.y - 0.087) / 0.105);
    let hole = 1.0 - smoothstep(0.55, 0.66, abs(cell - 0.31) / 0.31);
    return edge * hole;
}}

fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let r = burn_row(p);
    let i = i32(r.x);
    let j = min(i + 1, BURN_N - 1);
    let k = r.y;
    let geo = mix(BURN_GEO[i], BURN_GEO[j], k);
    let gain = mix(BURN_GAIN[i].xyz, BURN_GAIN[j].xyz, k);
    let o0 = mix(BURN_O0[i].xyz, BURN_O0[j].xyz, k);
    let o1 = mix(BURN_O1[i].xyz, BURN_O1[j].xyz, k);
    let use_b = p >= 0.5;
    let c = vec2f(0.5) + geo.zw;
    let base = c + (uv - c) / geo.x;
    var acc = vec4f(0.0);
    if (geo.y <= 1e-4) {{
        acc = select(sa(base), sb(base), use_b);
    }} else {{
        let jit = hash12(uv * u.size);
        for (var t = 0; t < 24; t = t + 1) {{
            let q = c + (base - c) * (1.0 - geo.y * (f32(t) + jit) / 24.0);
            acc = acc + select(sa(q), sb(q), use_b);
        }}
        acc = acc / 24.0;
    }}
    let d = to_disp(acc);
    var rgb = gain * d.rgb + o0 + o1 * (uv.x - 0.5);
    let holes = sprockets(uv) * smoothstep(0.36, 0.375, p) * (1.0 - smoothstep(0.425, 0.44, p));
    rgb = mix(rgb, vec3f(0.89, 0.88, 0.77), 0.9 * holes);
    return from_disp(vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), max(d.a, 1.0 - gain.g)));
}}
"#,
        geo.join(", "),
        gain.join(", "),
        o0.join(", "),
        o1.join(", ")
    )
}

/// Transitions calibrated frame by frame against Filmora's own preview
/// renders (timing, easing, direction); see docs/filmora-coverage.md.
fn register_transitions(r: &mut Registry) {
    shader(
        r,
        info(
            "jb.pan_slide",
            "Slide (Pan)",
            "Motion",
            FxKind::Transition,
            "Camera swing: the clip zooms in and tilts while it drifts, whips out to the right and the next \
             clip lands from the left already tilted and zoomed, then settles; the edges repeat mirrored. \
             Fitted frame by frame on Filmora's export of Slide (store pack).",
            &[
                ParamSpec::float("distance", "Distance", 0.0, 1.0, 0.25),
                ParamSpec::float("blur", "Blur", 0.0, 2.0, 1.0),
            ],
        ),
        slide_body(),
    );
    shader(
        r,
        info(
            "jb.film_burn_02",
            "Film Burn 02",
            "Light",
            FxKind::Transition,
            "Film-reel burn: the clip zooms into a growing radial blur, dims, washes out to white with \
             sprocket holes at the edges, flashes warm, burns to dark brown, and the next clip rises out of \
             the dark through a fading radial blur. Fitted frame by frame on Filmora's export of Film Burn \
             Transition 02.",
            &[],
        ),
        film_burn_body(),
    );
}

pub fn register(r: &mut Registry) {
    register_transitions(r);
    shader(
        r,
        info(
            "jb.filmora.exposure",
            "Exposure (Filmora)",
            "Color",
            FxKind::Effect,
            "Filmora's Color > Light > Exposure slider, fitted on Filmora exports.",
            &[ParamSpec::float("exposure", "Exposure", -100.0, 100.0, 0.0)],
        ),
        EXPOSURE,
    );
    if let (Some(adjust), Some(exposure)) = (r.get("jb.color.adjust").cloned(), r.get("jb.filmora.exposure").cloned()) {
        let mut params: Vec<ParamSpec> = vec![ParamSpec::float("u_exposure", "u_exposure", -100.0, 100.0, 0.0)];
        params.extend(ADJUST_COLOR.iter().map(|(outer, _, _)| ParamSpec::float(outer, outer, -100.0, 100.0, 0.0)));
        let info = info(
            "jb.filmora.adjust_color",
            "Adjust Color (Filmora)",
            "Color",
            FxKind::Effect,
            "Filmora's Color panel with its native u_* parameter names: exposure follows Filmora's curve, \
             the other sliders render through jb.color.adjust.",
            &params,
        );
        r.register(AdjustColorFx {
            adjust: RemapFx { info: info.clone(), inner: adjust, map: ADJUST_COLOR.to_vec() },
            exposure,
            info,
        });
    }
    shader(
        r,
        info(
            "jb.auto_enhance",
            "Auto Enhance",
            "Color",
            FxKind::Effect,
            "One-knob automatic enhancement: gentle levels stretch, local-contrast S-curve and a vibrance boost. Equivalent to Filmora: AutoEnhance.",
            &[ParamSpec::float("amount", "Amount", 0.0, 100.0, 50.0)],
        ),
        r#"
fn fx(uv: vec2f) -> vec4f {
    let d = to_disp(sa(uv));
    let k = p_amount() / 100.0;
    var c = d.rgb;
    // Levels: lift the black point and pull the white point in slightly.
    c = clamp((c - 0.03 * k) / (1.0 - 0.08 * k), vec3f(0.0), vec3f(1.0));
    // Mild S-curve around mid grey.
    let s = c * c * (3.0 - 2.0 * c);
    c = mix(c, s, 0.35 * k);
    // Vibrance: saturate muted colors more than saturated ones.
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let sat = mx - mn;
    c = saturate3(c, 1.0 + 0.45 * k * (1.0 - sat));
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), d.a));
}
"#,
    );
}

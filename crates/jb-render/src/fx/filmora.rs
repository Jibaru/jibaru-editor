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
/// effect): in display space `y = 1 - (1 - x)^k` with
/// `k = 1 + 0.01256 * s^1.244` (k = 2.43, 3.06, 4.47, 4.86 at 45, 60, 92, 100,
/// chosen so the mean level of each measured clip matches Filmora);
/// negative values darken with the mirrored `x^k` (not observed).
const EXPOSURE: &str = r#"
fn fx(uv: vec2f) -> vec4f {
    let d = to_disp(sa(uv));
    let s = p_exposure();
    let k = 1.0 + 0.01256 * pow(abs(s), 1.244);
    let x = clamp(d.rgb, vec3f(0.0), vec3f(1.0));
    var y = pow(x, vec3f(k));
    if (s > 0.0) { y = 1.0 - pow(1.0 - x, vec3f(k)); }
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

/// Transitions calibrated frame by frame against Filmora's own preview
/// renders (timing, easing, direction); see docs/filmora-coverage.md.
fn register_transitions(r: &mut Registry) {
    use super::transitions::{body, HBLUR, ZOOM_BLUR};
    shader(
        r,
        info(
            "jb.pan_slide",
            "Slide (Pan)",
            "Motion",
            FxKind::Transition,
            "Camera-pan slide: the clip drifts right and accelerates into a cut, the next clip arrives \
             from the left with a long ease-out. Timing matched to Filmora: Slide (store pack).",
            &[ParamSpec::float("distance", "Distance", 0.0, 1.0, 0.25), ParamSpec::float("blur", "Blur", 0.0, 2.0, 1.0)],
        ),
        body(&format!(
            "{HBLUR}{}",
            r#"
fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    let cut = 0.485;
    let d = p_distance();
    if (p < cut) {
        let u = seg(p, 0.1, cut);
        let e = u * u * u;
        let near = seg(p, 0.43, cut);
        return hblur(false, uv, vec2f(0.15 * d * e, 0.0), 1.0 + 0.15 * e, 0.03 * p_blur() * near * near);
    }
    let s = seg(p, cut, 1.0);
    let k = (1.0 - s) * (1.0 - s);
    let k0 = 1.0 - seg(p, cut, 0.6);
    return hblur(true, uv, vec2f(-0.5 * d * k, 0.0), 1.0 + 0.25 * k, 0.01 * p_blur() * k0 * k0);
}
"#
        )),
    );
    shader(
        r,
        info(
            "jb.film_burn_02",
            "Film Burn 02",
            "Light",
            FxKind::Transition,
            "Film-reel burn: the clip creeps in and softens with sprocket holes at the edges, washes out \
             to white, burns through warm orange to black, and the next clip rises out of the dark \
             through a radial zoom blur. Matched to Filmora: Film Burn Transition 02.",
            &[],
        ),
        body(&format!(
            "{ZOOM_BLUR}{}",
            r#"
// Bright perforations along the left and right edges of a film strip.
fn sprockets(uv: vec2f) -> f32 {
    let a = aspect();
    let x = min(uv.x, 1.0 - uv.x) * a; // distance to the side edge, in heights
    let cell = fract(uv.y * 5.0 + 0.5) - 0.5;
    let hole = (1.0 - smoothstep(0.02, 0.026, abs(x - 0.02))) * (1.0 - smoothstep(0.12, 0.15, abs(cell)));
    return hole;
}

fn blur_a(uv: vec2f, s: f32, r: f32) -> vec4f {
    // Small disc blur (8 taps on two rings) of A scaled by s.
    let base = zoom_uv(uv, s, vec2f(0.5));
    var acc = sa(base);
    for (var i = 0; i < 8; i = i + 1) {
        let ang = f32(i) * 0.785398;
        let o = h2uv(vec2f(cos(ang), sin(ang)) * r);
        acc = acc + sa(base + o) + sa(base + 0.5 * o);
    }
    return acc / 17.0;
}

fn fx(uv: vec2f) -> vec4f {
    let p = prog();
    if (p >= 0.999) { return sb(uv); }
    let cut = 0.5;
    if (p < cut) {
        let s = 1.0 + 0.3 * pow(seg(p, 0.0, cut), 1.2);
        let c = to_disp(blur_a(uv, s, 0.05 * smoothstep(0.18, 0.42, p)));
        // The frame dims a little as it softens, before washing out.
        var rgb = c.rgb * (1.0 - 0.18 * smoothstep(0.28, 0.36, p));
        // Sprocket holes flicker in while the frame softens.
        let sp = sprockets(uv) * seg(p, 0.33, 0.36) * (1.0 - seg(p, 0.44, 0.47));
        rgb = mix(rgb, vec3f(1.0, 0.97, 0.85), sp);
        // Wash out to a cold white...
        rgb = mix(rgb, vec3f(0.80, 0.82, 0.83), 0.92 * smoothstep(0.37, 0.455, p));
        // ...then a warm leak sweeping from the left burns through to dark brown.
        let leak = smoothstep(0.44, 0.47, p) * (1.0 - smoothstep(0.0, 0.8, uv.x));
        rgb = mix(rgb, vec3f(0.85, 0.8, 0.55), 0.45 * leak);
        let burn = smoothstep(0.465, 0.49, p);
        let ember = vec3f(0.2, 0.15, 0.1) + vec3f(0.45, 0.3, 0.15) * smoothstep(0.5, 1.0, uv.x);
        rgb = mix(rgb, ember, burn);
        return from_disp(vec4f(rgb, c.a));
    }
    // The next clip rises out of the dark through a radial zoom blur.
    let t = seg(p, cut, 1.0);
    let c = to_disp(zoom_blur(true, uv, 1.0 + 0.06 * (1.0 - t), 0.22 * (1.0 - t) + 0.03));
    let gain = mix(0.08, 0.82, t);
    let tint = mix(vec3f(0.6, 0.85, 0.8), vec3f(1.0), t);
    return from_disp(vec4f(c.rgb * gain * tint, c.a));
}
"#
        )),
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

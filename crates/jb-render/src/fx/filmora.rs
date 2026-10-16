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

/// Filmora "AdjustColor" slider names -> `jb.color.adjust` parameters.
const ADJUST_COLOR: &[(&str, &str, f32)] = &[
    ("u_exposure", "exposure", 1.0),
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

pub fn register(r: &mut Registry) {
    if let Some(adjust) = r.get("jb.color.adjust").cloned() {
        let params: Vec<ParamSpec> = ADJUST_COLOR
            .iter()
            .map(|(outer, _, _)| ParamSpec::float(outer, outer, -100.0, 100.0, 0.0))
            .collect();
        r.register(RemapFx {
            info: info(
                "jb.filmora.adjust_color",
                "Adjust Color (Filmora)",
                "Color",
                FxKind::Effect,
                "Filmora's Color panel with its native u_* parameter names; renders through jb.color.adjust.",
                &params,
            ),
            inner: adjust,
            map: ADJUST_COLOR.to_vec(),
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

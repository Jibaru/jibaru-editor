//! Editing helpers shared by the commands: parameter paths, keyframe
//! windows (used by split and trim), and keeping clip-relative data
//! (keyframes, ranged effects, animations, markers) attached to the content
//! when a clip's edges move.

use crate::model::*;
use crate::param::{Keyframe, Param, Value};
use crate::time::Ticks;

/// Parameter paths accepted by the keyframe commands.
pub const PARAM_PATHS: &str = "transform.position_x | transform.position_y | transform.scale_x | \
transform.scale_y | transform.scale (both axes) | transform.rotation | opacity | volume (audio.gain_db) | \
pan (audio.pan) | speed (rate ramp) | effect.<effect_id>.<param> (effects and animations)";

/// A resolved parameter path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamPath {
    PositionX,
    PositionY,
    ScaleX,
    ScaleY,
    /// Both scale axes at once.
    Scale,
    Rotation,
    Opacity,
    Volume,
    Pan,
    Speed,
    Effect {
        effect_id: String,
        param: String,
    },
}

impl ParamPath {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        Some(match s {
            "transform.position_x" | "position_x" | "position.x" => Self::PositionX,
            "transform.position_y" | "position_y" | "position.y" => Self::PositionY,
            "transform.scale_x" | "scale_x" | "scale.x" => Self::ScaleX,
            "transform.scale_y" | "scale_y" | "scale.y" => Self::ScaleY,
            "transform.scale" | "scale" => Self::Scale,
            "transform.rotation" | "rotation" => Self::Rotation,
            "opacity" => Self::Opacity,
            "volume" | "audio.gain_db" | "gain_db" | "audio.volume" => Self::Volume,
            "pan" | "audio.pan" | "balance" => Self::Pan,
            "speed" | "speed.ramp" | "speed.rate" => Self::Speed,
            _ => {
                let rest = s.strip_prefix("effect.").or_else(|| s.strip_prefix("effects."))?;
                let (effect_id, param) = rest.rsplit_once('.')?;
                if effect_id.is_empty() || param.is_empty() {
                    return None;
                }
                Self::Effect { effect_id: effect_id.to_string(), param: param.to_string() }
            }
        })
    }

    /// The parameters this path addresses on `clip` (two for `Scale`).
    /// Effect parameters that do not exist yet are created with `init`.
    pub fn params_mut<'a>(&self, clip: &'a mut Clip, init: Option<&Value>) -> Result<Vec<&'a mut Param>, String> {
        let t = &mut clip.transform;
        Ok(match self {
            Self::PositionX => vec![&mut t.position_x],
            Self::PositionY => vec![&mut t.position_y],
            Self::ScaleX => vec![&mut t.scale_x],
            Self::ScaleY => vec![&mut t.scale_y],
            Self::Scale => vec![&mut t.scale_x, &mut t.scale_y],
            Self::Rotation => vec![&mut t.rotation],
            Self::Opacity => vec![&mut clip.opacity],
            Self::Volume => vec![&mut clip.audio.gain_db],
            Self::Pan => vec![&mut clip.audio.pan],
            Self::Speed => {
                if clip.speed.ramp.is_none() {
                    clip.speed.ramp = Some(Param::float(clip.speed.rate));
                }
                vec![clip.speed.ramp.as_mut().expect("just set")]
            }
            Self::Effect { effect_id, param } => {
                let e = clip
                    .effects
                    .iter_mut()
                    .chain(clip.animations.iter_mut())
                    .find(|e| &e.id == effect_id)
                    .ok_or_else(|| format!("effect {effect_id}"))?;
                if !e.params.contains_key(param) {
                    match init {
                        Some(v) => {
                            e.params.insert(param.clone(), Param::constant(v.clone()));
                        }
                        None => return Err(format!("parameter {param} of effect {effect_id}")),
                    }
                }
                vec![e.params.get_mut(param).expect("present")]
            }
        })
    }
}

/// Inserts or replaces the keyframe at `kf.time`, keeping keyframes sorted.
pub fn upsert_keyframe(p: &mut Param, kf: Keyframe) {
    if p.keyframes.is_empty() {
        p.value = kf.value.clone();
    }
    match p.keyframes.iter().position(|k| k.time >= kf.time) {
        Some(i) if p.keyframes[i].time == kf.time => p.keyframes[i] = kf,
        Some(i) => p.keyframes.insert(i, kf),
        None => p.keyframes.push(kf),
    }
}

/// Removes keyframes at `time`; when the last one goes, its value stays as
/// the constant value. Returns whether one was removed.
pub fn remove_keyframe(p: &mut Param, time: Ticks) -> bool {
    let Some(i) = p.keyframes.iter().position(|k| k.time == time) else { return false };
    let k = p.keyframes.remove(i);
    if p.keyframes.is_empty() {
        p.value = k.value;
    }
    true
}

/// Drops keyframes, keeping the value at `at` as the constant value.
pub fn clear_keyframes(p: &mut Param, at: Ticks) {
    if !p.keyframes.is_empty() {
        p.value = p.sample(at);
        p.keyframes.clear();
    }
}

/// The part of a parameter curve between clip-local `from` and `to`
/// (`None` = open ended), re-based so that `from` becomes time 0. Boundary
/// keyframes are added where the window cuts through an animated segment,
/// so the curve inside the window is unchanged (exactly for linear and hold
/// segments). `from` may be negative, which just delays the curve.
pub fn window_param(p: &Param, from: Ticks, to: Option<Ticks>) -> Param {
    if p.keyframes.is_empty() {
        return p.clone();
    }
    let kfs = &p.keyframes;
    let interp_at = |t: Ticks| kfs.iter().rev().find(|k| k.time <= t).map(|k| k.interpolation).unwrap_or_default();
    let first = kfs[0].time;
    let last = kfs[kfs.len() - 1].time;
    if to.is_some_and(|e| e <= first) {
        // The window ends before the curve starts moving.
        return Param::constant(kfs[0].value.clone());
    }
    let mut out: Vec<Keyframe> = Vec::new();
    if from > first && !kfs.iter().any(|k| k.time == from) {
        out.push(Keyframe { time: 0, value: p.sample(from), interpolation: interp_at(from) });
    }
    for k in kfs.iter().filter(|k| k.time >= from && to.is_none_or(|e| k.time <= e)) {
        out.push(Keyframe { time: k.time - from, ..k.clone() });
    }
    if let Some(end) = to {
        if end < last && !kfs.iter().any(|k| k.time == end) {
            out.push(Keyframe { time: end - from, value: p.sample(end), interpolation: interp_at(end) });
        }
    }
    if out.is_empty() {
        // Window entirely after the last keyframe.
        return Param::constant(p.sample(from));
    }
    if out.len() == 1 {
        return Param::constant(out.remove(0).value);
    }
    Param { value: out[0].value.clone(), keyframes: out }
}

/// Visits every keyframable parameter of a clip, except the speed ramp
/// (which needs source-aware handling, see `crate::speed`).
pub fn for_each_param(clip: &mut Clip, mut f: impl FnMut(&mut Param)) {
    let t = &mut clip.transform;
    for p in [&mut t.position_x, &mut t.position_y, &mut t.scale_x, &mut t.scale_y, &mut t.rotation] {
        f(p);
    }
    f(&mut clip.opacity);
    f(&mut clip.audio.gain_db);
    f(&mut clip.audio.pan);
    for e in clip.effects.iter_mut().chain(clip.animations.iter_mut()) {
        for p in e.params.values_mut() {
            f(p);
        }
    }
    if let Some(tr) = &mut clip.transition_out {
        for p in tr.params.values_mut() {
            f(p);
        }
    }
}

/// Re-bases ranged effects, animations and markers after the clip's start moved by `offset`
/// (positive = later into the content) and its length became `len`:
/// keyframes, ranged effects and markers keep pointing at the same content.
pub fn rebase_local(clip: &mut Clip, offset: Ticks, len: Ticks) {
    if offset != 0 {
        for e in clip.effects.iter_mut().chain(clip.animations.iter_mut()).filter(|e| e.placement.is_none()) {
            if let Some((a, b)) = &mut e.range {
                *a = (*a - offset).max(0);
                *b -= offset;
            }
        }
        for m in &mut clip.markers {
            m.time -= offset;
        }
    }
    clip.effects.retain(|e| e.range.is_none_or(|(a, b)| b > 0 && a < len));
    clip.animations.retain(|e| e.range.is_none_or(|(a, b)| b > 0 && a < len) || e.placement.is_some());
    for e in clip.effects.iter_mut().chain(clip.animations.iter_mut()).filter(|e| e.placement.is_none()) {
        if let Some((a, b)) = &mut e.range {
            *a = (*a).clamp(0, len);
            *b = (*b).clamp(*a, len);
        }
    }
    clip.markers.retain(|m| m.time >= 0 && m.time < len);
    reanchor_animations(clip);
}

/// Keeps placed animations on their clip edges: entry animations start at 0,
/// exit animations end at the clip end, loop/combo span the clip. Durations
/// are capped at the clip length (entry + exit never overlap).
pub fn reanchor_animations(clip: &mut Clip) {
    let len = clip.duration().max(0);
    let dur = |e: &EffectInstance| e.range.map_or(len, |(a, b)| (b - a).max(0));
    let in_len = clip.animations.iter().find(|e| e.placement == Some(AnimationPlacement::In)).map(dur).unwrap_or(0);
    for e in &mut clip.animations {
        let d = e.range.map_or(len, |(a, b)| (b - a).max(0));
        e.range = match e.placement {
            None => continue,
            Some(AnimationPlacement::In) => Some((0, d.min(len))),
            Some(AnimationPlacement::Out) => {
                let d = d.min((len - in_len.min(len)).max(0)).min(len);
                Some((len - d, len))
            }
            Some(AnimationPlacement::Loop | AnimationPlacement::Combo) => Some((0, len)),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::Interpolation;

    fn kf(t: Ticks, v: f64) -> Keyframe {
        Keyframe { time: t, value: Value::Float(v), interpolation: Interpolation::Linear }
    }

    fn ramp() -> Param {
        Param { value: Value::Float(0.0), keyframes: vec![kf(0, 0.0), kf(100, 10.0), kf(200, 0.0)] }
    }

    #[test]
    fn window_keeps_the_curve() {
        let p = ramp();
        let r = window_param(&p, 50, None);
        for t in [0, 25, 50, 100, 150] {
            assert!((r.sample_f64(t) - p.sample_f64(t + 50)).abs() < 1e-9, "t={t}");
        }
        assert_eq!(r.keyframes[0].time, 0);
        let l = window_param(&p, 0, Some(150));
        for t in [0, 50, 100, 149] {
            assert!((l.sample_f64(t) - p.sample_f64(t)).abs() < 1e-9);
        }
        assert_eq!(l.keyframes.last().unwrap().time, 150);
        // Entirely past the last key: constant.
        let c = window_param(&p, 300, None);
        assert!(c.keyframes.is_empty() && c.value == Value::Float(0.0));
        // Negative offset delays the curve.
        let d = window_param(&p, -10, None);
        assert_eq!(d.keyframes[1].time, 110);
    }

    #[test]
    fn paths_parse() {
        assert_eq!(ParamPath::parse("transform.scale_x"), Some(ParamPath::ScaleX));
        assert_eq!(ParamPath::parse("volume"), Some(ParamPath::Volume));
        assert_eq!(
            ParamPath::parse("effect.abc-1.radius"),
            Some(ParamPath::Effect { effect_id: "abc-1".into(), param: "radius".into() })
        );
        assert_eq!(ParamPath::parse("nope"), None);
    }

    #[test]
    fn keyframe_edits() {
        let mut p = Param::float(1.0);
        upsert_keyframe(&mut p, kf(10, 2.0));
        upsert_keyframe(&mut p, kf(0, 1.0));
        upsert_keyframe(&mut p, kf(10, 3.0));
        assert_eq!(p.keyframes.len(), 2);
        assert_eq!(p.sample_f64(10), 3.0);
        assert!(remove_keyframe(&mut p, 0));
        assert!(remove_keyframe(&mut p, 10));
        assert_eq!(p, Param::float(3.0));
    }
}

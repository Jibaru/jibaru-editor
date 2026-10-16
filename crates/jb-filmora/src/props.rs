//! Shapes of the clip properties beyond the basics (see
//! `docs/filmora-format.md` §13): blend mode codes, parameter keyframes
//! (`paramMapList`), keyframe interpolation, animation presets and the
//! names of the crop/flip/fit/audio parameters. Shared by the importer and
//! the writer so both sides always agree.

use crate::json::*;
use jb_core::{AnimationPlacement, BlendMode, Interpolation, Keyframe, Param, Ticks};
use serde_json::{json, Value};

/// `pipBuf.BlendMode` codes, in the order of Filmora's blending menu.
pub const BLEND_MODES: [BlendMode; 22] = [
    BlendMode::Normal,
    BlendMode::Darken,
    BlendMode::Multiply,
    BlendMode::ColorBurn,
    BlendMode::LinearBurn,
    BlendMode::DarkerColor,
    BlendMode::Lighten,
    BlendMode::Screen,
    BlendMode::ColorDodge,
    BlendMode::Add,
    BlendMode::LighterColor,
    BlendMode::Overlay,
    BlendMode::SoftLight,
    BlendMode::HardLight,
    BlendMode::VividLight,
    BlendMode::LinearLight,
    BlendMode::PinLight,
    BlendMode::HardMix,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Subtract,
    BlendMode::Divide,
];

pub fn blend_from_code(code: i64) -> Option<BlendMode> {
    usize::try_from(code).ok().and_then(|i| BLEND_MODES.get(i).copied())
}

pub fn blend_code(m: BlendMode) -> i64 {
    BLEND_MODES.iter().position(|b| *b == m).unwrap_or(0) as i64
}

// -- parameter names -------------------------------------------------------------

/// Transform effect parameters.
pub const TF_ROTATION: &str = "Rotation";
pub const TF_FLIP_X: &str = "FlipX";
pub const TF_FLIP_Y: &str = "FlipY";
pub const TF_FIT: &str = "FitMode";
/// Crop insets on `video/effect/crop-pan-zoom`, fractions of the source.
pub const CROP_PARAMS: [&str; 4] = ["CropLeft", "CropTop", "CropRight", "CropBottom"];
/// Audio parameters.
pub const FX_PITCH: &str = "audio/effect/pitch";
pub const FX_CLIP_VOLUME: &str = "audio/effect/clip_volume";
pub const FX_EQUALIZER: &str = "audio/effect/equalizer";
pub const AU_BALANCE: &str = "Balance";
pub const AU_PITCH: &str = "Pitch";
pub const AU_MUTE: &str = "Mute";
pub const AU_NORMALIZE: &str = "AutoNormalize";
pub const AU_NORMALIZE_TARGET: &str = "NormalizeTarget";
pub const AU_LOUDNESS: &str = "MeasuredLoudness";
pub const EQ_PRESET: &str = "Preset";

pub fn eq_band(i: usize) -> String {
    format!("Band{i}")
}

pub fn fit_code(f: jb_core::FitMode) -> i64 {
    match f {
        jb_core::FitMode::Fit => 0,
        jb_core::FitMode::Fill => 1,
        jb_core::FitMode::Stretch => 2,
        jb_core::FitMode::None => 3,
    }
}

pub fn fit_from_code(c: i64) -> jb_core::FitMode {
    match c {
        1 => jb_core::FitMode::Fill,
        2 => jb_core::FitMode::Stretch,
        3 => jb_core::FitMode::None,
        _ => jb_core::FitMode::Fit,
    }
}

// -- keyframes ---------------------------------------------------------------------

/// Interpolation codes: 1 = linear (with slope derivatives), 7 = hold, 6 =
/// curve; a curve whose tangent is flat at the segment start eases in, flat
/// at its end eases out, flat at both eases in and out.
const KF_LINEAR: i64 = 1;
const KF_CURVE: i64 = 6;
const KF_HOLD: i64 = 7;

/// A keyframed parameter in Filmora units (`value * scale + offset`).
pub fn kf_param(p: &Param, scale: f64, offset: f64) -> KfParam {
    let keys = &p.keyframes;
    let v = |k: &Keyframe| k.value.as_f64().unwrap_or(0.0) * scale + offset;
    let mut out: Vec<Kf> = keys
        .iter()
        .map(|k| Kf {
            time: ticks_to_secs(k.time),
            interpolation: match k.interpolation {
                Interpolation::Linear => KF_LINEAR,
                Interpolation::Hold => KF_HOLD,
                _ => KF_CURVE,
            },
            value: v(k),
            left_derivative: None,
            right_derivative: None,
        })
        .collect();
    for i in 0..keys.len().saturating_sub(1) {
        let (a, b) = (&keys[i], &keys[i + 1]);
        let dt = ticks_to_secs(b.time - a.time).max(1e-9);
        let slope = (v(b) - v(a)) / dt;
        let (r, l) = match a.interpolation {
            Interpolation::Hold => continue,
            Interpolation::Linear => (slope, slope),
            Interpolation::EaseIn => (0.0, slope),
            Interpolation::EaseOut => (slope, 0.0),
            Interpolation::EaseInOut => (0.0, 0.0),
        };
        out[i].right_derivative = Some(r);
        out[i + 1].left_derivative = Some(l);
    }
    let mut k = KfParam::empty();
    k.keys = out;
    k
}

/// Keyframes back from Filmora units.
pub fn keyframes_from(kf: &KfParam, scale: f64, offset: f64) -> Vec<Keyframe> {
    let keys = &kf.keys;
    keys.iter()
        .enumerate()
        .map(|(i, k)| {
            let next = keys.get(i + 1);
            let interpolation = match k.interpolation {
                KF_HOLD => Interpolation::Hold,
                KF_CURVE => {
                    let flat_start = k.right_derivative == Some(0.0);
                    let flat_end = next.is_some_and(|n| n.left_derivative == Some(0.0));
                    let moves = next.is_some_and(|n| n.value != k.value);
                    match (moves, flat_start, flat_end) {
                        (true, true, true) => Interpolation::EaseInOut,
                        (true, true, false) => Interpolation::EaseIn,
                        (true, false, true) => Interpolation::EaseOut,
                        _ => Interpolation::Linear,
                    }
                }
                _ => Interpolation::Linear,
            };
            Keyframe {
                time: secs_to_ticks(k.time),
                value: jb_core::Value::Float((k.value - offset) / scale),
                interpolation,
            }
        })
        .collect()
}

/// Keyframe string of an effect parameter (`paramMapList` entry).
pub fn param_map<'a>(effect: &'a Value, name: &str) -> Option<&'a str> {
    effect
        .get("paramMapList")?
        .as_array()?
        .iter()
        .find(|e| str_of(e, "name") == Some(name))?
        .get("keyFrame")?
        .get("parameter")?
        .as_str()
}

/// Sets (or with `None` removes) the keyframes of an effect parameter.
pub fn set_param_map(effect: &mut Value, name: &str, text: Option<String>) {
    let has = effect.get("paramMapList").is_some_and(Value::is_array);
    if !has && text.is_none() {
        return;
    }
    if !has {
        put(effect, "paramMapList", json!([]));
    }
    let list = effect["paramMapList"].as_array_mut().expect("array");
    let pos = list.iter().position(|e| str_of(e, "name") == Some(name));
    match (text, pos) {
        (Some(t), Some(i)) => list[i]["keyFrame"] = json!({ "parameter": t }),
        (Some(t), None) => list.push(json!({ "key": 3, "keyFrame": { "parameter": t }, "name": name })),
        (None, Some(i)) => {
            list.remove(i);
        }
        (None, None) => {}
    }
    if effect["paramMapList"].as_array().is_some_and(Vec::is_empty) {
        if let Some(o) = effect.as_object_mut() {
            o.remove("paramMapList");
        }
    }
}

/// Reads a parameter with its keyframes from an effect, in model units.
pub fn effect_param_full(effect: &Value, name: &str, scale: f64, offset: f64) -> Option<Param> {
    let base = effect_param_f64(effect, name).map(|v| (v - offset) / scale);
    let keys = param_map(effect, name).and_then(KfParam::parse).map(|k| keyframes_from(&k, scale, offset));
    match (base, keys) {
        (None, None) => None,
        (b, k) => {
            let keyframes = k.unwrap_or_default();
            let value = b.or_else(|| keyframes.first().and_then(|k| k.value.as_f64())).unwrap_or(0.0);
            Some(Param { value: jb_core::Value::Float(value), keyframes })
        }
    }
}

/// Writes a parameter (value and keyframes) into an effect in Filmora units.
pub fn set_effect_param_full(effect: &mut Value, name: &str, p: &Param, scale: f64, offset: f64, ty: i64) {
    set_effect_param(effect, name, json!(p.value.as_f64().unwrap_or(0.0) * scale + offset), ty);
    let text = (!p.keyframes.is_empty()).then(|| kf_param(p, scale, offset).to_text());
    set_param_map(effect, name, text);
}

// -- animations --------------------------------------------------------------------

/// `AnimationScene` values of the "Animation" tab presets.
pub fn scene_name(p: AnimationPlacement) -> &'static str {
    match p {
        AnimationPlacement::In => "animation_scene_in",
        AnimationPlacement::Out => "animation_scene_out",
        AnimationPlacement::Loop => "animation_scene_loop",
        AnimationPlacement::Combo => "animation_scene_combo",
    }
}

pub fn scene_from(s: &str) -> Option<AnimationPlacement> {
    Some(match s {
        "animation_scene_in" => AnimationPlacement::In,
        "animation_scene_out" => AnimationPlacement::Out,
        "animation_scene_loop" => AnimationPlacement::Loop,
        "animation_scene_combo" => AnimationPlacement::Combo,
        _ => return None,
    })
}

/// Registry animations with an equivalent Filmora preset: (kind, preset id, title).
pub const ANIMATIONS: &[(&str, &str, &str)] = &[
    ("jb.anim.fade_in", "{C0BBEA86-C7CB-48ee-AB59-9CBE2FF697DD}", "Fade1"),
    ("jb.anim.fade_out", "{99C431D2-E9A2-41d1-AEF8-0425CCC13B82}", "Fade1"),
    ("jb.anim.zoom_in", "{C7A7DEE9-3239-4fc2-8049-FDB6639D51EF}", "Zoom in"),
    ("jb.anim.zoom_in_out", "{BE3FA86E-AF47-4489-8C8E-280D9EC67135}", "Zoom in and out"),
    ("jb.anim.spin_in", "{61096A73-D759-4d9d-AEE5-47433E4D2B25}", "Clockwise rotate"),
    ("jb.anim.drop_in", "{781D6912-0EF0-4f53-813D-0324E97FAAA7}", "Drop"),
    ("jb.anim.slide_in_left", "{3EA4BC06-1115-44a1-A272-4A3C99F36D38}", "Left Dir Insert"),
    ("jb.anim.slide_in_right", "{131C9392-CFC2-474c-AF2F-DA3246222949}", "Right Dir Insert"),
    ("jb.anim.slide_in_up", "{ABD0149B-5029-496a-AC67-0C152E296FEB}", "Up Dir Insert"),
    ("jb.anim.slide_in_down", "{3B33C7F0-2A40-4909-9701-1FFDDFB45DC7}", "Down Dir Insert"),
    ("jb.anim.slide_out_left", "{AFD054D9-051B-4705-B384-84E4F6FDF0B3}", "Left Dir Insert"),
    ("jb.anim.slide_out_right", "{13E8E2C2-BB5D-4a42-9A66-8483E783B517}", "Right Dir Insert"),
    ("jb.anim.slide_out_up", "{FDB0E26A-6B58-42aa-8751-BF8F0A574CEF}", "Up Dir Insert"),
    ("jb.anim.slide_out_down", "{8A5BBDD7-FDD4-4617-9794-D70BF6E39A37}", "Down Dir Insert"),
];

/// Filmora id and display name of an animation kind, if it has one.
pub fn animation_id(kind: &str) -> Option<(String, String)> {
    if let Some(id) = kind.strip_prefix("filmora:") {
        return Some((id.to_string(), id.to_string()));
    }
    ANIMATIONS.iter().find(|(k, _, _)| *k == kind).map(|(_, id, t)| (id.to_string(), t.to_string()))
}

/// Model kind of a Filmora animation id.
pub fn animation_kind(id: &str) -> String {
    ANIMATIONS
        .iter()
        .find(|(_, i, _)| i.eq_ignore_ascii_case(id))
        .map_or_else(|| format!("filmora:{id}"), |(k, _, _)| k.to_string())
}

pub fn ticks_range(e: &Value) -> Option<(Ticks, Ticks)> {
    Some((i64_of(e, "inPoint")?, i64_of(e, "outPoint")?)).filter(|(a, b)| b > a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jb_core::Value as V;

    #[test]
    fn interpolation_round_trips() {
        for interp in [
            Interpolation::Linear,
            Interpolation::Hold,
            Interpolation::EaseIn,
            Interpolation::EaseOut,
            Interpolation::EaseInOut,
        ] {
            let p = Param {
                value: V::Float(0.0),
                keyframes: vec![
                    Keyframe { time: 0, value: V::Float(0.0), interpolation: interp },
                    Keyframe { time: 10_000_000, value: V::Float(0.5), interpolation: Interpolation::Linear },
                ],
            };
            let text = kf_param(&p, 100.0, 0.0).to_text();
            let back = keyframes_from(&KfParam::parse(&text).unwrap(), 100.0, 0.0);
            assert_eq!(back, p.keyframes, "{interp:?}: {text}");
        }
    }

    #[test]
    fn blend_codes() {
        for (i, m) in BLEND_MODES.iter().enumerate() {
            assert_eq!(blend_from_code(i as i64), Some(*m));
            assert_eq!(blend_code(*m), i as i64);
        }
    }
}

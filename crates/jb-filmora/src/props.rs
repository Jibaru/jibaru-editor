//! Shapes of the clip properties beyond the basics, as Filmora 15.7.3
//! writes them (see `docs/filmora-format.md` §13): blend mode names,
//! parameter keyframes (`paramMapList`), keyframe time bases and
//! interpolation codes, flip effects, the crop rectangle, audio parameter
//! names, equalizer bands and Motion animations. Shared by the importer and
//! the writer so both sides always agree.

use crate::json::*;
use jb_core::{AnimationPlacement, BlendMode, Interpolation, Keyframe, Param, Ticks};
use serde_json::{json, Value};

// -- blend modes -------------------------------------------------------------------

/// `pipBuf.BlendMode` values. Filmora writes the integer `0` until a mode is
/// picked, then the menu name (`"Normal"` included). Its menu has these 18
/// modes; darker/lighter color, subtract and divide do not exist.
pub const BLEND_NAMES: [(BlendMode, &str); 18] = [
    (BlendMode::Normal, "Normal"),
    (BlendMode::Darken, "Darken"),
    (BlendMode::Multiply, "Multiply"),
    (BlendMode::ColorBurn, "Color Burn"),
    (BlendMode::LinearBurn, "Linear Burn"),
    (BlendMode::Lighten, "Lighten"),
    (BlendMode::Screen, "Screen"),
    (BlendMode::ColorDodge, "Color Dodge"),
    (BlendMode::Add, "Linear Dodge"),
    (BlendMode::Overlay, "Overlay"),
    (BlendMode::SoftLight, "Soft Light"),
    (BlendMode::HardLight, "Hard Light"),
    (BlendMode::VividLight, "Vivid Light"),
    (BlendMode::LinearLight, "Linear Light"),
    (BlendMode::PinLight, "Pin Light"),
    (BlendMode::HardMix, "Hard Mix"),
    (BlendMode::Difference, "Difference"),
    (BlendMode::Exclusion, "Exclusion"),
];

/// Blend mode of a `pipBuf.BlendMode` value (`0` or a menu name).
pub fn blend_from(v: &Value) -> Option<BlendMode> {
    match v {
        Value::Number(n) if n.as_f64() == Some(0.0) => Some(BlendMode::Normal),
        Value::String(s) => BLEND_NAMES.iter().find(|(_, n)| n.eq_ignore_ascii_case(s)).map(|(m, _)| *m),
        _ => None,
    }
}

/// Filmora's name of a blend mode, if Filmora has it.
pub fn blend_name(m: BlendMode) -> Option<&'static str> {
    BLEND_NAMES.iter().find(|(b, _)| *b == m).map(|(_, n)| *n)
}

// -- effect and parameter names ------------------------------------------------------

/// Flips are parameterless effects in the `BasicFollow` chain (Filmora's own
/// spelling); userData 101 = 1 while active, `enable: false` once reset.
pub const FX_FLIP_H: &str = "video/effect/horizontal_filp";
pub const FX_FLIP_V: &str = "video/effect/vertical_filp";
pub const UD_FLIP_ACTIVE: i64 = 101;
/// Rotation of `video/effect/transform`, degrees clockwise.
pub const TF_ROTATION: &str = "Rotation";
/// Crop and Pan & Zoom rectangles of `video/effect/crop-pan-zoom`: a JSON
/// list of `{OriTime, Time, X, Y, Width, Height}` (normalized source rect;
/// `Time` 0..1 over the clip, `OriTime` its length in ticks).
pub const CROP_SCRIPT: &str = "SettingScript";
/// crop-pan-zoom userData 15001 = 5 after "Crop to Fit".
pub const UD_CROP_MODE: i64 = 15001;
pub const CROP_TO_FIT: i32 = 5;
/// crop-pan-zoom userData 56 = 1 while Pan & Zoom is on.
pub const UD_PAN_ZOOM: i64 = 56;
/// Audio parameters, all on `audio/effect/volume` except the pitch shifter.
pub const FX_PITCH: &str = "audio/effect/pitchshifter";
pub const FX_PITCH_DISPLAY: &str = "pitch shifter";
pub const FX_CLIP_VOLUME: &str = "audio/effect/clip_volume";
pub const FX_EQUALIZER: &str = "audio/effect/equalizer";
pub const AU_GAIN: &str = "VolumeGain";
/// 0 (left) .. 0.5 (center) .. 1 (right).
pub const AU_BALANCE: &str = "Balance";
pub const AU_PITCH: &str = "Semitone";
/// Boolean (paramType 1).
pub const AU_MUTE: &str = "Mute";
/// Auto Normalization: `LoudnessGainEnable` (bool) and the linear gain
/// Filmora measured (`LoudnessGain`, double); the target LUFS is the clip's
/// userData 30307 (i32).
pub const AU_LOUDNESS_ON: &str = "LoudnessGainEnable";
pub const AU_LOUDNESS_GAIN: &str = "LoudnessGain";
pub const UD_LOUDNESS_TARGET: i64 = 30307;
/// Equalizer bands (dB, paramType 2); bands at 0 dB are left out.
pub const EQ_BANDS: [&str; 10] = ["31Hz", "63Hz", "125Hz", "250Hz", "500Hz", "1kHz", "2kHz", "4kHz", "8kHz", "16kHz"];

/// Filmora 15 equalizer presets as it writes them (dB per band). The
/// preset itself is not stored; Filmora recognizes it by its bands.
pub const EQ_PRESETS: &[(&str, [f64; 10])] = &[
    ("default", [0.0; 10]),
    ("classic", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.16, 0.36, -7.92, -10.08]),
    ("dance", [-0.72, 3.96, 5.04, 0.72, -1.08, -1.08, 0.0, 0.36, 3.96, 3.96]),
    ("low_bass", [2.88, 3.96, 2.88, 2.88, 2.16, -2.16, -7.92, -10.08, -12.96, -10.08]),
    ("sparkle", [-3.96, -3.96, -3.96, -2.16, -1.08, 2.16, 5.76, 7.92, 7.92, 7.92]),
    ("pop", [-1.08, -2.16, 3.96, 5.76, 5.76, 3.96, -1.08, -2.16, -2.16, -2.16]),
    ("rock_n_roll", [-1.08, 2.88, 2.16, 2.88, -1.08, -1.08, 0.0, 2.16, 2.16, 3.96]),
    ("soft", [0.0, 3.96, 2.16, -1.08, -2.16, -1.08, 3.96, 5.76, 5.76, 7.92]),
    ("techno", [-5.76, 1.08, 3.96, -1.08, -1.08, -2.16, 0.0, 0.0, 5.04, 5.04]),
    ("hard_rock", [-3.96, 0.0, 0.0, 0.0, 0.0, 0.0, 2.88, 0.0, 2.88, 1.08]),
    ("ska", [0.0, -2.16, -3.96, -2.16, 0.0, 3.96, 5.76, 7.92, 5.76, 5.76]),
    ("country", [0.0, -1.08, 0.0, 0.0, 2.16, 2.16, 0.0, 0.0, 2.88, 2.88]),
    ("acoustic", [0.0, 1.08, 2.16, 0.0, 0.0, 0.0, 0.0, 0.0, 2.16, 2.16]),
    ("folk", [-1.08, 0.0, 1.08, 0.0, 2.16, 0.0, 0.0, 0.0, 2.16, 0.0]),
    ("blues", [-1.44, 0.0, 2.16, 1.08, 0.0, 0.0, 0.0, 0.0, -1.08, -2.88]),
];

/// The Filmora preset with exactly these bands, if any.
pub fn eq_preset_of(bands: &[f64; 10]) -> Option<&'static str> {
    EQ_PRESETS.iter().find(|(_, b)| b.iter().zip(bands).all(|(x, y)| (x - y).abs() < 1e-3)).map(|(n, _)| *n)
}

// -- keyframes ---------------------------------------------------------------------

/// Where keyframe times live in a Filmora clip: the clip's own `inPoint`
/// timebase (`inPoint + clip-local time`, so trimming the start does not
/// move them), in seconds for effect parameters (`paramMapList`) and in
/// ticks for `pipBuf.OpacityKeyFrame` (verified in Filmora 15.7.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeBase {
    pub origin: Ticks,
    pub ticks: bool,
}

impl TimeBase {
    pub fn seconds(origin: Ticks) -> Self {
        Self { origin, ticks: false }
    }
    pub fn ticks(origin: Ticks) -> Self {
        Self { origin, ticks: true }
    }
    /// Clip-relative presets (Motion animations): seconds from 0.
    pub fn local() -> Self {
        Self::seconds(0)
    }
    fn file_of(self, local: Ticks) -> f64 {
        let t = local + self.origin;
        if self.ticks {
            t as f64
        } else {
            ticks_to_secs(t)
        }
    }
    fn local_of(self, x: f64) -> Ticks {
        let t = if self.ticks { x.round() as Ticks } else { secs_to_ticks(x) };
        t - self.origin
    }
}

/// Interpolation codes of a key (it shapes the segment that starts there):
/// 1 linear (derivatives = slope), 6 freeze (hold), 13 curve and 14
/// continuous curve (Bezier keys: `_leftDerivative`/`_rightDerivative` are
/// handle influences, 0.5; `_leftDerivativeY`/`_rightDerivativeY` handle
/// heights, absent = flat). "Ease In"/"Ease Out" in Filmora's menu are 13
/// keys with one non-flat handle. 9 appears on speed ramp keys.
pub const KF_LINEAR: i64 = 1;
pub const KF_HOLD: i64 = 6;
pub const KF_RAMP: i64 = 9;
pub const KF_CURVE: i64 = 13;
pub const KF_CONTINUOUS: i64 = 14;
const HANDLE: f64 = 0.5;

fn is_curve(code: i64) -> bool {
    code == KF_CURVE || code == KF_CONTINUOUS
}

/// A keyframed parameter in Filmora units (`value * scale + offset`).
/// A segment that eases at its start gets a flat right handle on its first
/// key, one that eases at its end a flat left handle on its last key.
pub fn kf_param(p: &Param, scale: f64, offset: f64, tb: TimeBase) -> KfParam {
    let keys = &p.keyframes;
    let v = |k: &Keyframe| k.value.as_f64().unwrap_or(0.0) * scale + offset;
    let n = keys.len();
    let seg = |i: usize| keys[i].interpolation;
    let flat_start = |i: usize| i + 1 < n && matches!(seg(i), Interpolation::EaseIn | Interpolation::EaseInOut);
    let flat_end = |i: usize| i > 0 && matches!(seg(i - 1), Interpolation::EaseOut | Interpolation::EaseInOut);
    let slope = |i: usize| {
        let dt = (tb.file_of(keys[i + 1].time) - tb.file_of(keys[i].time)).abs().max(1e-9);
        (v(&keys[i + 1]) - v(&keys[i])) / dt
    };
    let mut out = Vec::with_capacity(n);
    for (i, k) in keys.iter().enumerate() {
        let hold = i + 1 < n && seg(i) == Interpolation::Hold;
        let curve = !hold && (flat_start(i) || flat_end(i));
        let mut kf = Kf::new(tb.file_of(k.time), v(k));
        kf.interpolation = if hold {
            KF_HOLD
        } else if curve {
            KF_CURVE
        } else {
            KF_LINEAR
        };
        if curve {
            kf.left_derivative = Some(HANDLE);
            kf.right_derivative = Some(HANDLE);
            // Non-flat sides point along the neighbouring segment.
            if i > 0 && !flat_end(i) {
                kf.left_y = Some(-HANDLE * (v(k) - v(&keys[i - 1])));
            }
            if i + 1 < n && !flat_start(i) {
                kf.right_y = Some(HANDLE * (v(&keys[i + 1]) - v(k)));
            }
        } else if !hold {
            if i > 0 && seg(i - 1) != Interpolation::Hold {
                kf.left_derivative = Some(slope(i - 1));
            }
            if i + 1 < n {
                kf.right_derivative = Some(slope(i));
            }
        }
        out.push(kf);
    }
    let mut k = KfParam::params();
    k.keys = out;
    k
}

/// Keyframes back from Filmora units. The last key's code shapes nothing.
pub fn keyframes_from(kf: &KfParam, scale: f64, offset: f64, tb: TimeBase) -> Vec<Keyframe> {
    let keys = &kf.keys;
    let flat = |y: Option<f64>| y.is_none_or(|y| y.abs() < 1e-9);
    keys.iter()
        .enumerate()
        .map(|(i, k)| {
            let interpolation = match keys.get(i + 1) {
                None => Interpolation::Linear,
                Some(_) if k.interpolation == KF_HOLD => Interpolation::Hold,
                Some(n) => {
                    let start = is_curve(k.interpolation) && flat(k.right_y);
                    let end = is_curve(n.interpolation) && flat(n.left_y);
                    match (n.value != k.value, start, end) {
                        (true, true, true) => Interpolation::EaseInOut,
                        (true, true, false) => Interpolation::EaseIn,
                        (true, false, true) => Interpolation::EaseOut,
                        _ => Interpolation::Linear,
                    }
                }
            };
            Keyframe {
                time: tb.local_of(k.time),
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
/// `key` repeats the parameter's paramType (3 for doubles, 2 for floats).
pub fn set_param_map(effect: &mut Value, name: &str, text: Option<String>) {
    let has = effect.get("paramMapList").is_some_and(Value::is_array);
    if !has && text.is_none() {
        return;
    }
    let ty = effect_param(effect, name).and_then(|p| p.get("paramType")).and_then(Value::as_i64).unwrap_or(3);
    if !has {
        put(effect, "paramMapList", json!([]));
    }
    let list = effect["paramMapList"].as_array_mut().expect("array");
    let pos = list.iter().position(|e| str_of(e, "name") == Some(name));
    match (text, pos) {
        (Some(t), Some(i)) => list[i]["keyFrame"] = json!({ "parameter": t }),
        (Some(t), None) => list.push(json!({ "key": ty, "keyFrame": { "parameter": t }, "name": name })),
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
pub fn effect_param_full(effect: &Value, name: &str, scale: f64, offset: f64, tb: TimeBase) -> Option<Param> {
    let base = effect_param_f64(effect, name).map(|v| (v - offset) / scale);
    let keys = param_map(effect, name).and_then(KfParam::parse).map(|k| keyframes_from(&k, scale, offset, tb));
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
pub fn set_effect_param_full(
    effect: &mut Value,
    name: &str,
    p: &Param,
    scale: f64,
    offset: f64,
    ty: i64,
    tb: TimeBase,
) {
    set_effect_param(effect, name, json!(p.value.as_f64().unwrap_or(0.0) * scale + offset), ty);
    let text = (!p.keyframes.is_empty()).then(|| kf_param(p, scale, offset, tb).to_text());
    set_param_map(effect, name, text);
}

// -- crop --------------------------------------------------------------------------

/// One rectangle of a `SettingScript`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropKey {
    pub ori_time: i64,
    pub time: f64,
    pub rect: [f64; 4],
}

pub fn parse_crop_script(text: &str) -> Vec<CropKey> {
    let Ok(Value::Array(list)) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    list.iter()
        .map(|k| CropKey {
            ori_time: i64_of(k, "OriTime").unwrap_or(0),
            time: f64_of(k, "Time").unwrap_or(0.0),
            rect: [
                f64_of(k, "X").unwrap_or(0.0),
                f64_of(k, "Y").unwrap_or(0.0),
                f64_of(k, "Width").unwrap_or(1.0),
                f64_of(k, "Height").unwrap_or(1.0),
            ],
        })
        .collect()
}

/// Filmora's own layout: an indented JSON list with keys in this order.
pub fn crop_script(keys: &[CropKey]) -> String {
    let items: Vec<String> = keys
        .iter()
        .map(|k| {
            format!(
                "    {{\n        \"OriTime\": {},\n        \"Time\": {:?},\n        \"X\": {:?},\n        \"Y\": {:?},\n        \"Width\": {:?},\n        \"Height\": {:?}\n    }}",
                k.ori_time, k.time, k.rect[0], k.rect[1], k.rect[2], k.rect[3]
            )
        })
        .collect();
    format!("[\n{}\n]", items.join(",\n"))
}

/// Crop insets `[left, top, right, bottom]` of a normalized source rect.
pub fn insets_of(rect: [f64; 4]) -> [f64; 4] {
    let c = |x: f64| if x.abs() < 1e-9 { 0.0 } else { x };
    [c(rect[0]), c(rect[1]), c(1.0 - rect[0] - rect[2]), c(1.0 - rect[1] - rect[3])]
}

pub fn rect_of(insets: [f64; 4]) -> [f64; 4] {
    [insets[0], insets[1], 1.0 - insets[0] - insets[2], 1.0 - insets[1] - insets[3]]
}

// -- Motion animations ----------------------------------------------------------------

/// Filmora 15 "Motion Presets" live in the clip's `inAnimation` /
/// `outAnimation`: `{duration, effectChain: {effectList: [video/effect/motion],
/// name}, name, originLength, type: 6, userData}`. The motion effect keys
/// `Position_x/_y` (0.5 = in place), `Scale_x/_y` (%), `Rotation` (degrees)
/// and `Opacity` (%) relative to the clip, at seconds from the animation
/// start in its `originLength`; `duration` stretches that to the slot.
pub const ANIM_IN: &str = "inAnimation";
pub const ANIM_OUT: &str = "outAnimation";
pub const FX_MOTION: &str = "video/effect/motion";
/// Clip userData 13011: name of the last Motion preset applied (UI state).
pub const UD_ANIM_NAME: i64 = 13011;

pub fn anim_field(p: AnimationPlacement) -> Option<&'static str> {
    match p {
        AnimationPlacement::In => Some(ANIM_IN),
        AnimationPlacement::Out => Some(ANIM_OUT),
        _ => None,
    }
}

/// One keyed motion parameter: name, base value and (fraction 0..1 of the
/// animation, value, interpolation of the segment that starts there).
pub type MotionTrack = (&'static str, f64, &'static [(f64, f64, Interpolation)]);

use Interpolation::{EaseIn as EI, EaseInOut as EIO, EaseOut as EO, Linear as L};

/// Registry animations written as Motion presets: (kind, placement, name,
/// tracks). Shapes follow `jb-render`'s animations (ease-out cubic ≈ EaseOut,
/// smoothstep ≈ EaseInOut). Filmora's own "Fade" (in) is the same look as
/// `jb.anim.fade_in` and is imported as it.
pub const MOTIONS: &[(&str, AnimationPlacement, &str, &[MotionTrack])] = &[
    ("jb.anim.fade_in", AnimationPlacement::In, "Fade", &[("Opacity", 100.0, &[(0.0, 0.0, EIO), (1.0, 100.0, L)])]),
    (
        "jb.anim.fade_out",
        AnimationPlacement::Out,
        "Fade Out",
        &[("Opacity", 100.0, &[(0.0, 100.0, EIO), (1.0, 0.0, L)])],
    ),
    (
        "jb.anim.zoom_in",
        AnimationPlacement::In,
        "Zoom In",
        &[
            ("Scale_x", 100.0, &[(0.0, 30.0, EO), (1.0, 100.0, L)]),
            ("Scale_y", 100.0, &[(0.0, 30.0, EO), (1.0, 100.0, L)]),
            ("Opacity", 100.0, &[(0.0, 0.0, L), (0.4, 100.0, L), (1.0, 100.0, L)]),
        ],
    ),
    (
        "jb.anim.zoom_in_out",
        AnimationPlacement::Out,
        "Zoom In (Out)",
        &[
            ("Scale_x", 100.0, &[(0.0, 100.0, EI), (1.0, 160.0, L)]),
            ("Scale_y", 100.0, &[(0.0, 100.0, EI), (1.0, 160.0, L)]),
            ("Opacity", 100.0, &[(0.0, 100.0, L), (0.5, 100.0, L), (1.0, 0.0, L)]),
        ],
    ),
    (
        "jb.anim.spin_in",
        AnimationPlacement::In,
        "Spin In",
        &[
            ("Rotation", 0.0, &[(0.0, -360.0, EO), (1.0, 0.0, L)]),
            ("Scale_x", 100.0, &[(0.0, 0.0, EO), (1.0, 100.0, L)]),
            ("Scale_y", 100.0, &[(0.0, 0.0, EO), (1.0, 100.0, L)]),
        ],
    ),
    (
        "jb.anim.drop_in",
        AnimationPlacement::In,
        "Drop In",
        &[
            ("Position_y", 0.5, &[(0.0, 0.1, EO), (1.0, 0.5, L)]),
            ("Scale_x", 100.0, &[(0.0, 115.0, EO), (1.0, 100.0, L)]),
            ("Scale_y", 100.0, &[(0.0, 115.0, EO), (1.0, 100.0, L)]),
            ("Opacity", 100.0, &[(0.0, 0.0, L), (0.5, 100.0, L), (1.0, 100.0, L)]),
        ],
    ),
    (
        "jb.anim.slide_in_left",
        AnimationPlacement::In,
        "Slide In Left",
        &[("Position_x", 0.5, &[(0.0, -0.5, EO), (1.0, 0.5, L)])],
    ),
    (
        "jb.anim.slide_in_right",
        AnimationPlacement::In,
        "Slide In Right",
        &[("Position_x", 0.5, &[(0.0, 1.5, EO), (1.0, 0.5, L)])],
    ),
    (
        "jb.anim.slide_in_up",
        AnimationPlacement::In,
        "Slide In Up",
        &[("Position_y", 0.5, &[(0.0, -0.5, EO), (1.0, 0.5, L)])],
    ),
    (
        "jb.anim.slide_in_down",
        AnimationPlacement::In,
        "Slide In Down",
        &[("Position_y", 0.5, &[(0.0, 1.5, EO), (1.0, 0.5, L)])],
    ),
    (
        "jb.anim.slide_out_left",
        AnimationPlacement::Out,
        "Slide Out Left",
        &[("Position_x", 0.5, &[(0.0, 0.5, EI), (1.0, -0.5, L)])],
    ),
    (
        "jb.anim.slide_out_right",
        AnimationPlacement::Out,
        "Slide Out Right",
        &[("Position_x", 0.5, &[(0.0, 0.5, EI), (1.0, 1.5, L)])],
    ),
    (
        "jb.anim.slide_out_up",
        AnimationPlacement::Out,
        "Slide Out Up",
        &[("Position_y", 0.5, &[(0.0, 0.5, EI), (1.0, -0.5, L)])],
    ),
    (
        "jb.anim.slide_out_down",
        AnimationPlacement::Out,
        "Slide Out Down",
        &[("Position_y", 0.5, &[(0.0, 0.5, EI), (1.0, 1.5, L)])],
    ),
];

/// Model kind of a Motion preset name in a slot.
pub fn motion_kind(name: &str, placement: AnimationPlacement) -> String {
    MOTIONS
        .iter()
        .find(|(_, p, n, _)| *p == placement && n.eq_ignore_ascii_case(name))
        .map_or_else(|| format!("filmora:{name}"), |(k, _, _, _)| k.to_string())
}

/// A Motion animation object for a registry animation, if it has one.
pub fn motion_json(kind: &str, duration: Ticks) -> Option<Value> {
    let (_, placement, name, tracks) = MOTIONS.iter().find(|(k, _, _, _)| *k == kind)?;
    let placement = *placement;
    let secs = ticks_to_secs(duration.max(1));
    let mut effect = json!({
        "display": "motion", "id": FX_MOTION, "placeholder": false, "placeholderStatus": 0,
        "thisUId": new_uid(), "type": 3, "userData": [{ "data": "MQ==", "key": 50003, "size": 1 }]
    });
    for (param, base, keys) in tracks.iter() {
        set_effect_param(&mut effect, param, json!(base), 3);
        let p = Param {
            value: jb_core::Value::Float(*base),
            keyframes: keys
                .iter()
                .map(|(f, v, i)| Keyframe {
                    time: secs_to_ticks(f * secs),
                    value: jb_core::Value::Float(*v),
                    interpolation: *i,
                })
                .collect(),
        };
        let mut kf = kf_param(&p, 1.0, 0.0, TimeBase::local());
        kf.version = 2;
        set_param_map(&mut effect, param, Some(kf.to_text()));
    }
    let mut a = json!({
        "duration": duration, "effectChain": { "effectList": [effect], "name": name }, "name": name,
        "originLength": duration, "type": 6, "userData": []
    });
    // Filmora's animation panel reads the store record (userData 12) of a
    // Motion preset and crashes without one; ours are marked custom.
    let (sub, dir) = match placement {
        AnimationPlacement::Out => (2802, "Out"),
        _ => (2801, "In"),
    };
    let res = json!({
        "id": 0, "infoType": 7, "is_custom": 1, "smart_resource": 0, "sale_mode": 0, "source": 1,
        "res_sub_type": sub, "res_type": 28, "smart_res_relative_path": "", "related_slug": "",
        "name": format!("Motion_{dir}_{}", name.replace(' ', "_")), "user_string": {}
    });
    set_user_data(&mut a, crate::mapping::ud::RESOURCE, serde_json::to_string(&res).unwrap_or_default().as_bytes());
    set_user_data(&mut a, 112, b"{\"res\":[]}\0");
    Some(a)
}

/// Legacy (pre-15) animation effects: `AnimationScene` of the old
/// "Animation" chain presets.
pub fn scene_from(s: &str) -> Option<AnimationPlacement> {
    Some(match s {
        "animation_scene_in" => AnimationPlacement::In,
        "animation_scene_out" => AnimationPlacement::Out,
        "animation_scene_loop" => AnimationPlacement::Loop,
        "animation_scene_combo" => AnimationPlacement::Combo,
        _ => return None,
    })
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
            for tb in [TimeBase::seconds(6_339_667), TimeBase::ticks(6_339_667)] {
                let p = Param {
                    value: V::Float(0.0),
                    keyframes: vec![
                        Keyframe { time: 0, value: V::Float(0.0), interpolation: interp },
                        Keyframe { time: 10_000_000, value: V::Float(0.5), interpolation: Interpolation::Linear },
                        Keyframe { time: 20_000_000, value: V::Float(0.2), interpolation: Interpolation::Linear },
                    ],
                };
                let text = kf_param(&p, 100.0, 0.0, tb).to_text();
                let back = keyframes_from(&KfParam::parse(&text).unwrap(), 100.0, 0.0, tb);
                assert_eq!(back, p.keyframes, "{interp:?}: {text}");
            }
        }
    }

    /// Opacity keyframes as Filmora 15.7.3 writes them: ticks in the clip's
    /// inPoint timebase, linear keys with per-tick slopes.
    #[test]
    fn opacity_keyframes_match_filmora() {
        let text = "{\"Version\": 3,\"ParameterType\": 0,\"keyframeSets\": [{\"_time\": 18351667.0,\"Interpolation\": 1,\"_value\": 100.0,\"_rightDerivative\": -0.00000450},{\"_time\": 31698333.0,\"Interpolation\": 1,\"_value\": 40.0,\"_leftDerivative\": -0.00000450}],\"MD5\":\"b48551e89dcbeb07a7dea2ff3d099736\"}";
        let tb = TimeBase::ticks(6_339_667);
        let keys = keyframes_from(&KfParam::parse(text).unwrap(), 100.0, 0.0, tb);
        assert_eq!(keys[0].time, 12_012_000);
        assert_eq!(keys[1].value, V::Float(0.4));
        let p = Param { value: V::Float(1.0), keyframes: keys };
        assert_eq!(kf_param(&p, 100.0, 0.0, tb).to_text(), text);
    }

    #[test]
    fn transform_keyframes_match_filmora() {
        let text = "{\"Version\": 3,\"ParameterType\": 0,\"keyframeSets\": [{\"_time\": 0.96763330,\"Interpolation\": 1,\"_value\": 0.5,\"_rightDerivative\": 0.03902348},{\"_time\": 2.96963330,\"Interpolation\": 1,\"_value\": 0.57812500,\"_leftDerivative\": 0.03902348}],\"MD5\":\"b7b1fc236c938f4b96479e56819881fb\"}";
        let tb = TimeBase::seconds(0);
        let keys = keyframes_from(&KfParam::parse(text).unwrap(), 1.0, 0.5, tb);
        assert_eq!(keys[0].value, V::Float(0.0));
        let p = Param { value: V::Float(0.0), keyframes: keys };
        // Filmora hashed its unrounded internal value here; the text matches.
        let ours = kf_param(&p, 1.0, 0.5, tb).to_text();
        assert_eq!(ours.split("\"MD5\"").next(), text.split("\"MD5\"").next());
    }

    #[test]
    fn filmora_curve_keys_read_as_eases() {
        // "Curve" on the first key, linear second key: flat start.
        let curve = "{\"Version\": 3,\"ParameterType\": 0,\"keyframeSets\": [{\"_time\": 18351667.0,\"Interpolation\": 13,\"_value\": 100.0,\"_leftDerivative\": 0.5,\"_rightDerivative\": 0.5},{\"_time\": 31698333.0,\"Interpolation\": 1,\"_value\": 40.0,\"_leftDerivative\": -0.00000450}],\"MD5\":\"x\"}";
        let k = keyframes_from(&KfParam::parse(curve).unwrap(), 1.0, 0.0, TimeBase::ticks(0));
        assert_eq!(k[0].interpolation, Interpolation::EaseIn);
        // "Ease In" (a non-flat right handle) eases nothing at the start.
        let ease = curve
            .replace("\"_rightDerivative\": 0.5}", "\"_rightDerivative\": 0.5,\"_rightDerivativeY\": -44.95504720}");
        let k = keyframes_from(&KfParam::parse(&ease).unwrap(), 1.0, 0.0, TimeBase::ticks(0));
        assert_eq!(k[0].interpolation, Interpolation::Linear);
        let hold = curve.replace("\"Interpolation\": 13", "\"Interpolation\": 6");
        let k = keyframes_from(&KfParam::parse(&hold).unwrap(), 1.0, 0.0, TimeBase::ticks(0));
        assert_eq!(k[0].interpolation, Interpolation::Hold);
    }

    #[test]
    fn blend_names() {
        for (m, n) in BLEND_NAMES {
            assert_eq!(blend_from(&json!(n)), Some(m));
            assert_eq!(blend_name(m), Some(n));
        }
        assert_eq!(blend_from(&json!(0)), Some(BlendMode::Normal));
        assert_eq!(blend_name(BlendMode::Divide), None);
    }

    #[test]
    fn crop_script_layout() {
        let k = CropKey { ori_time: 0, time: 0.0, rect: [0.0, 0.341796875, 1.0, 0.31640625] };
        let text = crop_script(&[k]);
        assert_eq!(text, "[\n    {\n        \"OriTime\": 0,\n        \"Time\": 0.0,\n        \"X\": 0.0,\n        \"Y\": 0.341796875,\n        \"Width\": 1.0,\n        \"Height\": 0.31640625\n    }\n]");
        assert_eq!(parse_crop_script(&text), vec![k]);
        assert_eq!(rect_of(insets_of(k.rect)), k.rect);
    }

    #[test]
    fn eq_presets_are_recognized() {
        for (n, b) in EQ_PRESETS {
            assert_eq!(eq_preset_of(b), Some(*n));
        }
    }
}

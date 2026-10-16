//! Text clips (clip type 4). Their content lives in `scriptBuf`, a JSON
//! document stored as a string. The clip's transform effect mirrors the
//! text box: `Position_x/y` equal `PosX/PosY` and `Scale_x/y` equal
//! `ScaleX/ScaleY * 100` (the box size as a percentage of the frame).

use crate::json::*;
use crate::mapping::*;
use jb_core::{Shadow, Stroke, TitleData};
use serde_json::{json, Value};

/// Box height of Filmora's default text (fraction of frame height).
const DEFAULT_BOX_HEIGHT: f64 = 0.0666666701436;

fn basic(script: &Value) -> Option<&Value> {
    script.get("TextData")?.get(0)?.get("Basic")
}

fn first_text_data(script: &Value) -> Option<&Value> {
    script.get("TextData")?.get(0)
}

/// Glyph height (fraction of the canvas) of one Filmora font-size unit.
fn unit(script: &Value, font_size: f64) -> f64 {
    let fs = basic(script).and_then(|b| f64_of(b, "FontSize")).filter(|v| *v > 0.0).unwrap_or(24.0);
    font_size / fs
}

pub fn title_from_script(script: &Value, transform_scale_y: Option<f64>) -> TitleData {
    let b = basic(script).cloned().unwrap_or(Value::Null);
    let box_h = f64_of(script, "ScaleY").or(transform_scale_y.map(|s| s / 100.0)).unwrap_or(DEFAULT_BOX_HEIGHT);
    let font_size = box_h * TITLE_GLYPH_RATIO;
    let u = unit(script, font_size);
    let text_alpha = i64_of(&b, "TextAlpha").unwrap_or(255);
    let color = b
        .get("TextColor")
        .and_then(|c| c.get(0))
        .and_then(|c| i64_of(c, "Color"))
        .map_or([1.0; 4], |c| color_from_int(c, text_alpha));
    let td = first_text_data(script).cloned().unwrap_or(Value::Null);
    let stroke = td.get("Border").filter(|x| i64_of(x, "Enable") == Some(1)).map(|x| Stroke {
        color: color_from_int(i64_of(x, "Color").unwrap_or(0), i64_of(x, "Alpha").unwrap_or(255)),
        width: f64_of(x, "Size").unwrap_or(0.0) * u,
    });
    let shadow = td.get("Shadow").filter(|x| i64_of(x, "Enable") == Some(1)).map(|x| {
        // Direction semantics are not verified; Filmora's default shadow falls
        // down-right, which is what this approximates.
        let d = f64_of(x, "Distance").unwrap_or(0.0) * u / std::f64::consts::SQRT_2;
        Shadow {
            color: color_from_int(i64_of(x, "Color").unwrap_or(0), i64_of(x, "Alpha").unwrap_or(255)),
            offset: [d, d],
            blur: f64_of(x, "BlurRadius").unwrap_or(0.0) * u,
        }
    });
    let background = script
        .get("Background")
        .filter(|x| i64_of(x, "Enable") == Some(1))
        .map(|x| color_from_int(i64_of(x, "Color").unwrap_or(0), i64_of(x, "Alpha").unwrap_or(255)));
    TitleData {
        text: str_of(script, "Text").unwrap_or_default().to_string(),
        font_family: str_of(&b, "FontName").unwrap_or("Arial").to_string(),
        font_size,
        color,
        position: [f64_of(script, "PosX").unwrap_or(0.5), f64_of(script, "PosY").unwrap_or(0.5)],
        bold: i64_of(&b, "FontBold").unwrap_or(0) != 0,
        italic: i64_of(&b, "FontItalic").unwrap_or(0) != 0,
        stroke,
        shadow,
        background,
        ..Default::default()
    }
}

fn transform_effect_mut(clip: &mut Value) -> Option<&mut Value> {
    clip.get_mut("effectChainList")?
        .as_array_mut()?
        .iter_mut()
        .flat_map(|ch| ch.get_mut("effectList").and_then(Value::as_array_mut).into_iter().flatten())
        .find(|e| str_of(e, "id") == Some(FX_TRANSFORM))
}

/// Writes the fields of `new` that differ from `old` into a text clip.
pub fn patch_title_clip(clip: &mut Value, old: Option<&TitleData>, new: &TitleData) {
    let mut script = nested(clip, "scriptBuf").unwrap_or_else(default_script);
    let mut tf: Vec<(&str, f64)> = Vec::new();

    if old.is_none_or(|o| o.text != new.text) {
        let old_len = str_of(&script, "Text").map_or(0, |t| t.chars().count()).max(1);
        let ratio = new.text.chars().count().max(1) as f64 / old_len as f64;
        script["Text"] = json!(new.text);
        if let Some(td) = script.get_mut("TextData").and_then(|t| t.get_mut(0)) {
            td["CharData"] = json!(new.text);
        }
        // The box width follows the text; scale it with the character count.
        if let Some(sx) = f64_of(&script, "ScaleX") {
            script["ScaleX"] = json!(sx * ratio);
            tf.push(("Scale_x", sx * ratio * 100.0));
        }
    }
    if old.is_none_or(|o| o.font_size != new.font_size) {
        let box_h = new.font_size / TITLE_GLYPH_RATIO;
        let k = box_h / f64_of(&script, "ScaleY").filter(|v| *v > 0.0).unwrap_or(DEFAULT_BOX_HEIGHT);
        let sx = f64_of(&script, "ScaleX").unwrap_or(0.0) * k;
        script["ScaleY"] = json!(box_h);
        script["ScaleX"] = json!(sx);
        tf.retain(|(n, _)| *n != "Scale_x");
        tf.push(("Scale_x", sx * 100.0));
        tf.push(("Scale_y", box_h * 100.0));
    }
    if old.is_none_or(|o| o.position != new.position) {
        script["PosX"] = json!(new.position[0]);
        script["PosY"] = json!(new.position[1]);
        tf.push(("Position_x", new.position[0]));
        tf.push(("Position_y", new.position[1]));
    }
    let u = unit(&script, new.font_size);
    if let Some(b) = script.get_mut("TextData").and_then(|t| t.get_mut(0)).and_then(|t| t.get_mut("Basic")) {
        if old.is_none_or(|o| o.font_family != new.font_family) {
            b["FontName"] = json!(new.font_family);
        }
        if old.is_none_or(|o| o.bold != new.bold) {
            b["FontBold"] = json!(new.bold as i64);
        }
        if old.is_none_or(|o| o.italic != new.italic) {
            b["FontItalic"] = json!(new.italic as i64);
        }
        if old.is_none_or(|o| o.color != new.color) {
            let (c, a) = color_to_int(new.color);
            b["TextColor"] = json!([{ "Color": c }]);
            b["TextAlpha"] = json!(a);
        }
    }
    if let Some(td) = script.get_mut("TextData").and_then(|t| t.get_mut(0)) {
        if old.is_none_or(|o| o.stroke != new.stroke) {
            let border = td.as_object_mut().map(|o| o.entry("Border").or_insert_with(|| json!({})));
            if let Some(border) = border {
                match &new.stroke {
                    Some(s) => {
                        let (c, a) = color_to_int(s.color);
                        border["Enable"] = json!(1);
                        border["Color"] = json!(c);
                        border["Alpha"] = json!(a);
                        border["Size"] = json!((s.width / u.max(1e-9)).round() as i64);
                    }
                    None => border["Enable"] = json!(0),
                }
            }
        }
        if old.is_none_or(|o| o.shadow != new.shadow) {
            let shadow = td.as_object_mut().map(|o| o.entry("Shadow").or_insert_with(|| json!({})));
            if let Some(shadow) = shadow {
                match &new.shadow {
                    Some(s) => {
                        let (c, a) = color_to_int(s.color);
                        let d = (s.offset[0].powi(2) + s.offset[1].powi(2)).sqrt();
                        shadow["Enable"] = json!(1);
                        shadow["Color"] = json!(c);
                        shadow["Alpha"] = json!(a);
                        shadow["Distance"] = json!((d / u.max(1e-9)).round() as i64);
                        shadow["BlurRadius"] = json!((s.blur / u.max(1e-9)).round() as i64);
                    }
                    None => shadow["Enable"] = json!(0),
                }
            }
        }
    }
    if old.is_none_or(|o| o.background != new.background) {
        if let Some(bg) = script.as_object_mut().map(|o| o.entry("Background").or_insert_with(|| json!({}))) {
            match new.background {
                Some(c) => {
                    let (c, a) = color_to_int(c);
                    bg["Enable"] = json!(1);
                    bg["Color"] = json!(c);
                    bg["Alpha"] = json!(a);
                }
                None => bg["Enable"] = json!(0),
            }
        }
    }
    set_nested(clip, "scriptBuf", &script);
    if !tf.is_empty() {
        if let Some(e) = transform_effect_mut(clip) {
            for (name, v) in tf {
                set_effect_param(e, name, json!(v), 3);
            }
        }
    }
}

/// `scriptBuf` of a plain centered "Basic" title, matching the structure
/// Filmora writes (field set observed in user projects).
pub fn default_script() -> Value {
    let tex = || json!({"Enable": 0, "File": "", "FillMode": 0, "KeepRatio": 0, "LoopPath": "", "SegmentNum": 0});
    let grad = || {
        json!({"Angle": 0, "ColorPos": [{"Color": 16777215, "Pos": 0}, {"Color": 0, "Pos": 100}], "Enable": 0,
               "FillMode": 0, "GradType": 0, "GradVer": "2.0"})
    };
    json!({
        "Aspect": "16:9", "AutoLine": 0, "AutoLineLimited": 0, "AutoLineScale": 1.0, "AutoLineVer": 1,
        "Background": {"Alpha": 255, "Border": {"Alpha": 255, "Color": 0, "Enable": 0, "Gradient": grad(), "Size": 0},
            "Bottom": 2.0, "Color": 0, "Enable": 0, "FillMode": 0, "Gradient": grad(), "Left": 2.0, "OffsetX": 0,
            "OffsetY": 0, "Radian": 0, "Right": 2.0, "Texture": tex(), "Top": 2.0},
        "BasePixel": 0, "BoldVer": 1, "ChangeAlignMode": 0, "CleanAreaSize": 0,
        "Curve": {"Angle": 180, "Animation": {"Duration": 10000000, "Enable": 0, "Loop": 0}, "Distance": 0.0,
            "Enable": 0, "Mode": 0, "PathAnchorType": 0, "PathClose": 0, "PathType": 0, "VerticalPath": 1},
        "DirectX": 1, "DirectY": 1, "FeatureVersion": 1, "FillTextArea": 0,
        "Gradient": grad(), "HideChar": 0, "HideText": 0,
        "Layout": {"Angle": 0.0, "CharSpace": 0, "LineSpace": 0, "TextAlign": "center", "TextAlignMode": "horizontal",
            "TextAlignPos": "center", "Underline": 0, "UnderlineColor": -1},
        "NLEProjChangeStatus": false, "Offset": {"x": 0.0, "y": 0.0}, "OutputTextPath": 0, "PosX": 0.5, "PosY": 0.5,
        "Rotate3D": {"x": 0.0, "y": 0.0, "z": 0.0}, "ScaleX": 0.1, "ScaleY": DEFAULT_BOX_HEIGHT,
        "TemplatePath": "", "Text": "",
        "TextData": [{
            "Basic": {"BgTexture": tex(), "Effect": 0, "Enable": 1, "FontBold": 0, "FontHSize": 24.0, "FontItalic": 0,
                "FontName": "Arial", "FontSize": 24.0, "FontVSize": 24.0, "TextAlpha": 255, "TextBlurRadius": 0,
                "TextColor": [{"Color": 16777215}], "Texture": tex()},
            "Border": {"Alpha": 255, "Angle": 0, "BlurRadius": 0, "Color": 0, "Distance": 0, "Enable": 0,
                "Gradient": grad(), "Size": 5, "Texture": tex()},
            "CharCase": 0, "CharData": "", "CharSpace": 0, "LineSpace": 0,
            "Shadow": {"Alpha": 255, "BlurRadius": 3, "Color": 0, "Direction": 4, "Distance": 8, "Enable": 0,
                "Gradient": grad(), "InAlpha": 255, "InAngle": 0, "InBlurRadius": 5, "InColor": 0, "InDistance": 10,
                "InEnable": 0, "Texture": tex()},
            "Strikeout": 0, "StrikeoutColor": -1, "Underline": 0, "UnderlineColor": -1}],
        "TextMode": 1, "TextScale": 1.0, "TextVersion": 1.0, "Texture": tex(), "UseDefaultBaseline": 0,
        "UseDefaultSliderColor": 1, "ViewSize": "1920:1080"
    })
}

/// Rough text box width (fraction of frame width) for a new title.
pub fn estimate_box_width(text: &str, box_h: f64, aspect: f64) -> f64 {
    // ~0.55 em per character for proportional Latin fonts.
    (text.chars().count().max(1) as f64 * 0.55 * box_h * TITLE_GLYPH_RATIO / aspect).min(1.0)
}

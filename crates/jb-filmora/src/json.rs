//! Helpers for the JSON structures used inside Filmora documents: typed
//! field access, `userData` blobs, GUIDs, effect parameters and the
//! keyframe strings (`speedParam`, `OpacityKeyFrame`, ...).

use base64::Engine;
use jb_core::{Ticks, TICKS_PER_SECOND};
use md5::{Digest, Md5};
use serde_json::{json, Value};

pub fn i64_of(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f.round() as i64)))
}

pub fn f64_of(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

pub fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

pub fn ticks_to_secs(t: Ticks) -> f64 {
    t as f64 / TICKS_PER_SECOND as f64
}

pub fn secs_to_ticks(s: f64) -> Ticks {
    (s * TICKS_PER_SECOND as f64).round() as Ticks
}

/// Sets `key` on an object. New keys are inserted in alphabetical position,
/// which is how Filmora orders every object it writes.
pub fn put(v: &mut Value, key: &str, val: Value) {
    let Some(obj) = v.as_object_mut() else { return };
    if let Some(slot) = obj.get_mut(key) {
        *slot = val;
        return;
    }
    let pos = obj.keys().position(|k| k.as_str() > key).unwrap_or(obj.len());
    obj.shift_insert(pos, key.to_string(), val);
}

/// Removes `key` from an object (Filmora leaves out flags that are off).
pub fn remove_key(v: &mut Value, key: &str) {
    if let Some(o) = v.as_object_mut() {
        o.shift_remove(key);
    }
}

// ---------------------------------------------------------------------------
// userData:`[{ "data": <base64>, "key": <int>, "size": <byte len> }, ...]`

pub fn user_data(v: &Value, key: i64) -> Option<Vec<u8>> {
    let list = v.get("userData")?.as_array()?;
    let item = list.iter().find(|u| u.get("key").and_then(Value::as_i64) == Some(key))?;
    base64::engine::general_purpose::STANDARD.decode(item.get("data")?.as_str()?).ok()
}

/// A userData blob as text, with trailing NUL padding removed.
pub fn user_data_str(v: &Value, key: i64) -> Option<String> {
    let raw = user_data(v, key)?;
    let end = raw.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    Some(String::from_utf8_lossy(&raw[..end]).into_owned())
}

/// A 4-byte little-endian integer blob.
pub fn user_data_i32(v: &Value, key: i64) -> Option<i32> {
    let raw = user_data(v, key)?;
    let b: [u8; 4] = raw.get(..4)?.try_into().ok()?;
    Some(i32::from_le_bytes(b))
}

pub fn set_user_data(v: &mut Value, key: i64, data: &[u8]) {
    if v.get("userData").is_none_or(|u| !u.is_array()) {
        put(v, "userData", Value::Array(Vec::new()));
    }
    let Some(list) = v.get_mut("userData").and_then(Value::as_array_mut) else { return };
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    if let Some(item) = list.iter_mut().find(|u| u.get("key").and_then(Value::as_i64) == Some(key)) {
        item["data"] = json!(encoded);
        item["size"] = json!(data.len());
    } else {
        list.push(json!({ "data": encoded, "key": key, "size": data.len() }));
    }
}

pub fn remove_user_data(v: &mut Value, key: i64) {
    if let Some(list) = v.get_mut("userData").and_then(Value::as_array_mut) {
        list.retain(|u| u.get("key").and_then(Value::as_i64) != Some(key));
    }
}

/// GUID strings are stored NUL-padded to 64 bytes.
pub fn guid_blob(guid: &str) -> Vec<u8> {
    let mut b = guid.as_bytes().to_vec();
    b.resize(b.len().max(64), 0);
    b
}

/// A new GUID in Filmora's `XX-XX-...` (16 uppercase hex bytes) notation.
pub fn new_guid() -> String {
    to_guid(&uuid::Uuid::new_v4().to_string())
}

/// Converts any identifier into Filmora GUID notation. Values that already
/// are in that notation are kept; UUIDs are reformatted; anything else is
/// hashed so the mapping is stable.
pub fn to_guid(id: &str) -> String {
    let is_guid = id.len() == 47 && id.split('-').all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()));
    if is_guid {
        return id.to_string();
    }
    let bytes: [u8; 16] = match uuid::Uuid::parse_str(id) {
        Ok(u) => *u.as_bytes(),
        Err(_) => Md5::digest(id.as_bytes()).into(),
    };
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("-")
}

pub fn new_uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// Effect parameters: `paramList: [{ "fxParam": {"paramType": n, "unValue": v}, "name": s }]`
//
// Observed paramType values: 2 = float (f32 precision), 3 = double,
// 5 = integer / flag, 6 = string.

pub fn effect_param<'a>(effect: &'a Value, name: &str) -> Option<&'a Value> {
    effect
        .get("paramList")?
        .as_array()?
        .iter()
        .find(|p| p.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|p| p.get("fxParam"))
}

pub fn effect_param_f64(effect: &Value, name: &str) -> Option<f64> {
    effect_param(effect, name)?.get("unValue")?.as_f64()
}

/// Sets (or adds) a parameter, coercing the value to the stored paramType.
pub fn set_effect_param(effect: &mut Value, name: &str, value: Value, default_type: i64) {
    if effect.get("paramList").is_none_or(|u| !u.is_array()) {
        put(effect, "paramList", Value::Array(Vec::new()));
    }
    let Some(list) = effect.get_mut("paramList").and_then(Value::as_array_mut) else { return };
    if let Some(p) = list.iter_mut().find(|p| p.get("name").and_then(Value::as_str) == Some(name)) {
        let ty = p["fxParam"].get("paramType").and_then(Value::as_i64).unwrap_or(default_type);
        p["fxParam"]["unValue"] = coerce(value, ty);
    } else {
        list.push(
            json!({ "fxParam": { "paramType": default_type, "unValue": coerce(value, default_type) }, "name": name }),
        );
    }
}

pub fn remove_effect_param(effect: &mut Value, name: &str) {
    if let Some(list) = effect.get_mut("paramList").and_then(Value::as_array_mut) {
        list.retain(|p| p.get("name").and_then(Value::as_str) != Some(name));
    }
}

fn coerce(v: Value, ty: i64) -> Value {
    match (ty, &v) {
        (5, Value::Number(n)) => json!(n.as_f64().unwrap_or(0.0).round() as i64),
        (5, Value::Bool(b)) => json!(*b as i64),
        (2 | 3, Value::Bool(b)) => json!(if *b { 1.0 } else { 0.0 }),
        _ => v,
    }
}

// ---------------------------------------------------------------------------
// Keyframe strings:
// `{"Version": 3,"ParameterType": 0,"keyframeSets": [{"_time": 0.0,"Interpolation": 6,"_value": 1.0}],
//   "MD5": "<hex>","_totalTime": 105.83906670}`
//
// `MD5` is the MD5 of the little-endian f64 triples (time, interpolation,
// value) of every keyframe, in order (verified against every keyframe
// string in the corpus). Derivatives are not part of the hash.
//
// Speed curves write `"MD5": "…"` with a space and end with `_totalTime`
// (plus `"reverse": 1` for reversed clips and `_freeze` for freeze frames);
// parameter keyframes (`paramMapList`, `OpacityKeyFrame`, animations) write
// `"MD5":"…"` without it (verified in Filmora 15.7.3). Numbers that fit in
// one decimal are written short (`0.5`, `100.0`), all others with eight
// decimals (`0.57812500`, `-0.00000450`).

#[derive(Debug, Clone, PartialEq)]
pub struct Kf {
    pub time: f64,
    pub interpolation: i64,
    pub value: f64,
    pub left_derivative: Option<f64>,
    pub right_derivative: Option<f64>,
    /// Bezier handle heights (value offsets) of curve keys (`Interpolation`
    /// 13/14); absent = a flat handle.
    pub left_y: Option<f64>,
    pub right_y: Option<f64>,
}

impl Kf {
    pub fn new(time: f64, value: f64) -> Self {
        Self {
            time,
            interpolation: 6,
            value,
            left_derivative: None,
            right_derivative: None,
            left_y: None,
            right_y: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KfParam {
    pub version: i64,
    pub parameter_type: i64,
    pub keys: Vec<Kf>,
    pub total_time: Option<f64>,
    /// `"MD5":"…"` without the space (parameter keyframes).
    pub compact: bool,
    /// `"reverse": 1` after `_totalTime` (reversed speed curves).
    pub reverse: bool,
    /// Freeze frames of a speed curve: (source seconds, duration seconds).
    pub freeze: Vec<(f64, f64)>,
}

impl KfParam {
    pub fn parse(text: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(text).ok()?;
        let keys = v
            .get("keyframeSets")?
            .as_array()?
            .iter()
            .map(|k| Kf {
                time: f64_of(k, "_time").unwrap_or(0.0),
                interpolation: i64_of(k, "Interpolation").unwrap_or(6),
                value: f64_of(k, "_value").unwrap_or(0.0),
                left_derivative: f64_of(k, "_leftDerivative"),
                right_derivative: f64_of(k, "_rightDerivative"),
                left_y: f64_of(k, "_leftDerivativeY"),
                right_y: f64_of(k, "_rightDerivativeY"),
            })
            .collect();
        let freeze = v
            .get("_freeze")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|f| {
                let mut it = f.as_str()?.split_whitespace().map(|x| x.parse::<f64>().ok());
                Some((it.next()??, it.next()??))
            })
            .collect();
        Some(Self {
            version: i64_of(&v, "Version").unwrap_or(3),
            parameter_type: i64_of(&v, "ParameterType").unwrap_or(0),
            keys,
            total_time: f64_of(&v, "_totalTime"),
            compact: text.contains("\"MD5\":\""),
            reverse: i64_of(&v, "reverse").is_some_and(|r| r != 0),
            freeze,
        })
    }

    pub fn empty() -> Self {
        Self {
            version: 3,
            parameter_type: 0,
            keys: Vec::new(),
            total_time: None,
            compact: false,
            reverse: false,
            freeze: Vec::new(),
        }
    }

    /// An empty parameter-keyframe list (`"MD5":"…"` style).
    pub fn params() -> Self {
        Self { compact: true, ..Self::empty() }
    }

    /// Serializes in Filmora's own spacing style.
    pub fn to_text(&self) -> String {
        let mut keys: Vec<Kf> = self.keys.clone();
        // Hash what will actually be read back.
        for k in &mut keys {
            k.time = fmt_num(k.time).parse().unwrap_or(k.time);
            k.value = fmt_num(k.value).parse().unwrap_or(k.value);
        }
        let mut hasher = Md5::new();
        for k in &keys {
            hasher.update(k.time.to_le_bytes());
            hasher.update((k.interpolation as f64).to_le_bytes());
            hasher.update(k.value.to_le_bytes());
        }
        let md5: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        let sets: Vec<String> = keys
            .iter()
            .map(|k| {
                let mut s = format!(
                    "{{\"_time\": {},\"Interpolation\": {},\"_value\": {}",
                    fmt_num(k.time),
                    k.interpolation,
                    fmt_num(k.value)
                );
                // Filmora writes the left derivative first when a key has both.
                if let Some(d) = k.left_derivative.filter(|_| k.right_derivative.is_some()) {
                    s += &format!(",\"_leftDerivative\": {}", fmt_num(d));
                }
                if let Some(d) = k.right_derivative {
                    s += &format!(",\"_rightDerivative\": {}", fmt_num(d));
                }
                if let Some(d) = k.left_derivative.filter(|_| k.right_derivative.is_none()) {
                    s += &format!(",\"_leftDerivative\": {}", fmt_num(d));
                }
                if let Some(y) = k.left_y {
                    s += &format!(",\"_leftDerivativeY\": {}", fmt_num(y));
                }
                if let Some(y) = k.right_y {
                    s += &format!(",\"_rightDerivativeY\": {}", fmt_num(y));
                }
                s + "}"
            })
            .collect();
        let sep = if self.compact { "" } else { " " };
        let mut out = format!(
            "{{\"Version\": {},\"ParameterType\": {},\"keyframeSets\": [{}],\"MD5\":{sep}\"{}\"",
            self.version,
            self.parameter_type,
            sets.join(","),
            md5
        );
        if let Some(t) = self.total_time {
            out += &format!(",\"_totalTime\": {}", fmt_num(t));
        }
        if self.reverse {
            out += ",\"reverse\": 1";
        }
        if !self.freeze.is_empty() {
            let list: Vec<String> =
                self.freeze.iter().map(|(t, d)| format!("\"{} {}\"", fmt_fixed(*t), fmt_fixed(*d))).collect();
            out += &format!(",\"_freeze\": [{}]", list.join(","));
        }
        out + "}"
    }
}

/// Short form for numbers that fit in one decimal, eight decimals otherwise.
fn fmt_num(x: f64) -> String {
    let short = format!("{x:.1}");
    if short.parse::<f64>().is_ok_and(|s| (s - x).abs() <= 1e-12 * x.abs().max(1.0)) {
        if short == "-0.0" {
            "0.0".into()
        } else {
            short
        }
    } else {
        fmt_fixed(x)
    }
}

fn fmt_fixed(x: f64) -> String {
    format!("{x:.8}")
}

/// Parses a nested JSON document stored as a string field (`pipBuf`, `scriptBuf`).
pub fn nested(v: &Value, key: &str) -> Option<Value> {
    serde_json::from_str(v.get(key)?.as_str()?).ok()
}

/// Stores a nested JSON document and updates its `<key>Size` companion
/// (byte length plus the C string terminator), when present.
pub fn set_nested(v: &mut Value, key: &str, doc: &Value) {
    let text = serde_json::to_string(doc).unwrap_or_default();
    let size_key = format!("{key}Size");
    if v.get(&size_key).is_some() || key == "pipBuf" || key == "scriptBuf" {
        put(v, &size_key, json!(text.len() + 1));
    }
    put(v, key, Value::String(text));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyframe_md5_matches_filmora() {
        let text = "{\"Version\": 3,\"ParameterType\": 0,\"keyframeSets\": [{\"_time\": 0.0,\"Interpolation\": 6,\"_value\": 1.0},{\"_time\": 105.83906670,\"Interpolation\": 6,\"_value\": 1.0}],\"MD5\": \"544f08eab48e61e5ed14e4f633cf6b94\",\"_totalTime\": 105.83906670}";
        let p = KfParam::parse(text).unwrap();
        assert_eq!(p.to_text(), text);
        assert_eq!(
            KfParam::empty().to_text(),
            "{\"Version\": 3,\"ParameterType\": 0,\"keyframeSets\": [],\"MD5\": \"d41d8cd98f00b204e9800998ecf8427e\"}"
        );
    }

    #[test]
    fn derivatives_round_trip() {
        let text = "{\"Version\": 3,\"ParameterType\": 0,\"keyframeSets\": [{\"_time\": 0.0,\"Interpolation\": 6,\"_value\": 3.0,\"_rightDerivative\": -0.13290479},{\"_time\": 15.04836670,\"Interpolation\": 6,\"_value\": 1.0,\"_leftDerivative\": -0.13290479}],\"MD5\": \"be1e3ba2e1fd816cc54e30c8be20bea6\",\"_totalTime\": 15.04836670}";
        assert_eq!(KfParam::parse(text).unwrap().to_text(), text);
    }

    #[test]
    fn guid_notation() {
        assert_eq!(to_guid("e8bdf2b3-1799-4f46-82d4-54cf4dae2de9"), "E8-BD-F2-B3-17-99-4F-46-82-D4-54-CF-4D-AE-2D-E9");
        let g = new_guid();
        assert_eq!(to_guid(&g), g);
    }

    #[test]
    fn user_data_round_trip() {
        let mut v = json!({});
        set_user_data(&mut v, 50, b"Layer 01");
        set_user_data(&mut v, 3, &guid_blob("AA-BB"));
        assert_eq!(user_data_str(&v, 50).unwrap(), "Layer 01");
        assert_eq!(user_data_str(&v, 3).unwrap(), "AA-BB");
        assert_eq!(v["userData"][1]["size"], 64);
    }
}

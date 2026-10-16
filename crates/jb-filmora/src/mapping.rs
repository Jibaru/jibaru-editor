//! Shared vocabulary of the format: clip types, built-in effect ids,
//! identity markers in `extra`, path and color conversions.

use jb_core::Extra;
use serde_json::{json, Value};

pub const CLIP_VIDEO: i64 = 1;
pub const CLIP_AUDIO: i64 = 2;
pub const CLIP_TEXT: i64 = 4;
pub const CLIP_TIMELINE: i64 = 7;
pub const CLIP_GRAPHIC: i64 = 26;

pub const TRACK_VIDEO: i64 = 1;
pub const TRACK_AUDIO: i64 = 2;

pub const FX_TRANSFORM: &str = "video/effect/transform";
pub const FX_CROP: &str = "video/effect/crop-pan-zoom";
pub const FX_VOLUME: &str = "audio/effect/volume";
pub const FX_FADE: &str = "audio/effect/fade";

/// Effects Filmora attaches to every clip of a kind. They are kept in the
/// file but only surfaced as effects when they carry parameters.
pub const STRUCTURAL_EFFECTS: &[&str] = &[
    FX_CROP,
    "audio/effect/clip_volume",
    "audio/effect/change_channel",
    "audio/effect/ducking",
    "audio/effect/equalizer",
];

/// userData keys (see docs/filmora-format.md).
pub mod ud {
    /// Clip: kind code (1 video, 2 audio, 5 text, 12 title/compound).
    pub const KIND: i64 = 1;
    /// Clip/effect/transition/timeline: GUID; on clips it is the link group.
    pub const GUID: i64 = 3;
    /// Clip: id of the owning timeline.
    pub const OWNER_TIMELINE: i64 = 6;
    /// Clip: media bin item id.
    pub const MEDIA_ITEM: i64 = 10;
    /// Clip/transition: JSON with the store resource (id, slug, res_type).
    pub const RESOURCE: i64 = 12;
    /// Clip: display name.
    pub const NAME: i64 = 50;
    /// Clip: JSON with the nested timeline template info.
    pub const TIMELINE_INFO: i64 = 55;
}

/// `{"filmora": {...}}` identity marker stored in `extra`.
pub fn marker(fields: Value) -> Extra {
    json!({ "filmora": fields })
}

pub fn marker_str<'a>(extra: &'a Extra, key: &str) -> Option<&'a str> {
    extra.get("filmora")?.get(key)?.as_str()
}

pub fn marker_i64(extra: &Extra, key: &str) -> Option<i64> {
    extra.get("filmora")?.get(key)?.as_i64()
}

/// `file://Users/me/a.mp4` (Filmora omits the third slash) → `/Users/me/a.mp4`.
pub fn url_to_path(url: &str) -> Option<String> {
    let rest = url.strip_prefix("file://")?;
    let rest = rest.trim_start_matches('/');
    if rest.is_empty() {
        return None;
    }
    let decoded = percent_decode(rest);
    // Windows drive paths stay as they are.
    if decoded.as_bytes().get(1) == Some(&b':') {
        Some(decoded)
    } else {
        Some(format!("/{decoded}"))
    }
}

pub fn path_to_url(path: &str) -> String {
    format!("file://{}", path.replace('\\', "/").trim_start_matches('/'))
}

fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Filmora colors are `0xRRGGBB` integers with a separate 0..255 alpha.
pub fn color_from_int(c: i64, alpha: i64) -> [f64; 4] {
    let c = c as u32;
    [
        ((c >> 16) & 0xFF) as f64 / 255.0,
        ((c >> 8) & 0xFF) as f64 / 255.0,
        (c & 0xFF) as f64 / 255.0,
        alpha.clamp(0, 255) as f64 / 255.0,
    ]
}

pub fn color_to_int(c: [f64; 4]) -> (i64, i64) {
    let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as i64;
    ((q(c[0]) << 16) | (q(c[1]) << 8) | q(c[2]), q(c[3]))
}

/// Rendered glyph height relative to Filmora's text box height
/// (calibrated against Filmora exports: the "Basic" title box is 6.67% of
/// the frame height; measured on Filmora 15.7 exports, Arial text set at
/// 0.752 of the box matches the rendered text width to 0.2%).
pub const TITLE_GLYPH_RATIO: f64 = 0.752;

pub fn file_stem(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(url_to_path("file://Users/a/b c.mp4").unwrap(), "/Users/a/b c.mp4");
        assert_eq!(url_to_path("file:///Users/a/b%20c.mp4").unwrap(), "/Users/a/b c.mp4");
        assert_eq!(url_to_path("file://C:/x/y.mp4").unwrap(), "C:/x/y.mp4");
        assert_eq!(path_to_url("/Users/a/b c.mp4"), "file://Users/a/b c.mp4");
        assert!(url_to_path("").is_none());
    }

    #[test]
    fn colors() {
        let c = color_from_int(0xFF8000, 255);
        assert_eq!(color_to_int(c), (0xFF8000, 255));
    }
}

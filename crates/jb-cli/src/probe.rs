//! Media probing for `import-media`.
//!
//! Interim implementation that shells out to the `ffprobe` CLI. When
//! `jb_media::probe` lands, `probe` becomes a thin wrapper around it.

use crate::error::{code, JbError, JbResult};
use jb_core::time::seconds_to_ticks;
use jb_core::{new_id, Extra, MediaAsset, MediaKind, Rational};
use serde_json::Value;
use std::path::Path;
use std::process::Command as Proc;

/// Formats ffprobe reports for still images.
const IMAGE_FORMATS: &[&str] =
    &["image2", "png_pipe", "jpeg_pipe", "webp_pipe", "bmp_pipe", "tiff_pipe", "gif", "svg_pipe"];

pub fn probe(path: &Path) -> JbResult<MediaAsset> {
    if !path.is_file() {
        return Err(JbError::new(code::FILE_NOT_FOUND, format!("media file `{}` does not exist", path.display()))
            .hint("pass an existing file path; relative paths are resolved against the current directory"));
    }
    let abs = std::fs::canonicalize(path)
        .map_err(|e| JbError::new(code::IO, format!("cannot resolve `{}`: {e}", path.display())))?;
    let out = Proc::new(ffprobe_bin())
        .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(&abs)
        .output()
        .map_err(|e| {
            JbError::new(code::PROBE_FAILED, format!("cannot run ffprobe: {e}"))
                .hint("install FFmpeg so `ffprobe` is on PATH, or set JB_FFPROBE to its location")
        })?;
    if !out.status.success() {
        return Err(JbError::new(
            code::PROBE_FAILED,
            format!("ffprobe could not read `{}`: {}", abs.display(), String::from_utf8_lossy(&out.stderr).trim()),
        )
        .hint("check that the file is a supported video, audio or image file"));
    }
    let info: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| JbError::new(code::PROBE_FAILED, format!("unexpected ffprobe output: {e}")))?;
    asset_from_ffprobe(&abs, &info)
}

fn ffprobe_bin() -> String {
    std::env::var("JB_FFPROBE").unwrap_or_else(|_| "ffprobe".into())
}

fn parse_rate(s: &str) -> Option<Rational> {
    let (n, d) = s.split_once('/')?;
    let r = Rational::new(n.parse().ok()?, d.parse().ok()?);
    (r.num > 0 && r.den > 0).then_some(r)
}

/// Maps ffprobe's JSON report to a `MediaAsset`.
pub fn asset_from_ffprobe(path: &Path, info: &Value) -> JbResult<MediaAsset> {
    let streams = info["streams"].as_array().cloned().unwrap_or_default();
    let of_type = |t: &str| streams.iter().find(|s| s["codec_type"] == t).cloned();
    let video = of_type("video").filter(|v| v["disposition"]["attached_pic"] != 1);
    let audio = of_type("audio");
    let format_name = info["format"]["format_name"].as_str().unwrap_or_default();
    let is_image = video.is_some() && format_name.split(',').any(|f| IMAGE_FORMATS.contains(&f));
    let kind = match (&video, &audio) {
        (Some(_), _) if is_image => MediaKind::Image,
        (Some(_), _) => MediaKind::Video,
        (None, Some(_)) => MediaKind::Audio,
        (None, None) => {
            return Err(JbError::new(code::PROBE_FAILED, format!("`{}` has no video or audio streams", path.display())))
        }
    };
    let seconds = info["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .or_else(|| video.as_ref().and_then(|v| v["duration"].as_str()?.parse().ok()))
        .unwrap_or(0.0);
    let dim = |k: &str| video.as_ref().and_then(|v| v[k].as_u64()).unwrap_or(0) as u32;
    let frame_rate = match kind {
        MediaKind::Video => video.as_ref().and_then(|v| {
            parse_rate(v["avg_frame_rate"].as_str()?).or_else(|| parse_rate(v["r_frame_rate"].as_str()?))
        }),
        _ => None,
    };
    Ok(MediaAsset {
        id: new_id(),
        name: path.file_name().and_then(|n| n.to_str()).unwrap_or("media").to_string(),
        path: path.to_string_lossy().into_owned(),
        kind,
        // Still images have no intrinsic duration.
        duration: if kind == MediaKind::Image { 0 } else { seconds_to_ticks(seconds) },
        width: dim("width"),
        height: dim("height"),
        frame_rate,
        has_audio: audio.is_some(),
        extra: Extra::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_video_with_audio() {
        let info = json!({
            "streams": [
                {"codec_type": "video", "width": 1920, "height": 1080, "avg_frame_rate": "30000/1001"},
                {"codec_type": "audio"}
            ],
            "format": {"format_name": "mov,mp4,m4a,3gp,3g2,mj2", "duration": "2.5"}
        });
        let a = asset_from_ffprobe(Path::new("/a/clip.mp4"), &info).unwrap();
        assert_eq!(a.kind, MediaKind::Video);
        assert_eq!(a.duration, 25_000_000);
        assert_eq!(a.frame_rate, Some(Rational::new(30000, 1001)));
        assert!(a.has_audio);
        assert_eq!(a.name, "clip.mp4");
    }

    #[test]
    fn maps_image_and_audio() {
        let img = json!({"streams": [{"codec_type": "video", "width": 10, "height": 10}], "format": {"format_name": "png_pipe"}});
        assert_eq!(asset_from_ffprobe(Path::new("/a.png"), &img).unwrap().kind, MediaKind::Image);
        let aud = json!({"streams": [{"codec_type": "audio"}], "format": {"format_name": "wav", "duration": "1"}});
        assert_eq!(asset_from_ffprobe(Path::new("/a.wav"), &aud).unwrap().kind, MediaKind::Audio);
    }
}

//! Seams to crates that are developed in parallel. Each function has its
//! final signature; until the backing crate lands it returns a
//! `not_implemented` error naming the missing crate, so wiring one up is a
//! change local to this file.

use crate::error::{JbError, JbResult};
use jb_core::{Project, Ticks};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Imports a Filmora `.wfp` project .
pub fn filmora_open(path: &Path) -> JbResult<Project> {
    jb_filmora::read(path)
        .map(|wfp| wfp.project)
        .map_err(|e| JbError::invalid(format!("cannot read {}: {e}", path.display())))
}

/// Reports which effects/transitions/titles of a `.wfp` project map to
/// registered Jibaru effects .
pub fn fx_coverage(path: &Path) -> JbResult<Value> {
    let wfp = jb_filmora::read(path).map_err(|e| JbError::invalid(format!("cannot read {}: {e}", path.display())))?;
    serde_json::to_value(jb_filmora::coverage(&wfp)).map_err(|e| JbError::invalid(e.to_string()))
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub output: PathBuf,
    /// Preview height in pixels (e.g. 540); `None` renders at project resolution.
    pub preview_height: Option<u32>,
    /// Timeline range `[start, end)`; `None` renders the whole sequence.
    pub range: Option<(Ticks, Ticks)>,
    /// Sequence to render; defaults to the main sequence.
    pub sequence_id: Option<String>,
}

/// Output size for a render: the project size, or scaled to `preview_height`.
fn output_size(project: &Project, preview_height: Option<u32>) -> (u32, u32) {
    let (w, h) = (project.settings.width, project.settings.height);
    match preview_height {
        Some(ph) if ph < h => (((w as u64 * ph as u64 / h as u64) as u32 + 1) & !1, ph & !1),
        _ => (w, h),
    }
}

/// Builds a media pool for `project`; previews read camera proxies (.LRV) when present.
fn media_pool(project: &Project, preview: bool) -> jb_media::MediaPool {
    let pool = jb_media::MediaPool::for_project(project, jb_media::PoolConfig::default());
    if preview {
        for m in &project.media {
            if let Some(proxy) = jb_media::cache::camera_proxy(Path::new(&m.path)) {
                pool.set_proxy(&m.id, Some(proxy));
            }
        }
    }
    pool
}

fn gpu_renderer(width: u32, height: u32, preview: bool) -> JbResult<jb_render::Renderer> {
    let mut r = jb_render::Renderer::new(width, height)
        .map_err(|e| JbError::invalid(format!("no usable GPU: {e}")).hint("rendering needs Metal, D3D12 or Vulkan"))?;
    if preview {
        r.options.source_max_size = Some((width, height));
    }
    Ok(r)
}

/// Encodes a sequence to a video file.
pub fn render(project: &Project, opts: &RenderOptions) -> JbResult<Value> {
    let seq_id = opts.sequence_id.clone().unwrap_or_else(|| project.main_sequence.clone());
    let preview = opts.preview_height.is_some();
    let (width, height) = output_size(project, opts.preview_height);
    let pool = media_pool(project, preview);
    let mut renderer = gpu_renderer(width, height, preview)?;
    let mut settings = jb_export::ExportSettings::new(&opts.output);
    settings.width = width;
    settings.height = height;
    settings.range = opts.range;
    if project.settings.width * project.settings.height > 1920 * 1080 {
        settings.codec = jb_export::VideoCodec::Hevc;
    }
    if preview {
        settings.prefetch_max_size = Some((width, height));
    }
    let mut unknown = std::collections::BTreeSet::new();
    let mut warnings = Vec::new();
    let mut last_report = std::time::Instant::now();
    let report = jb_export::export(
        project,
        &seq_id,
        &settings,
        &pool,
        &mut |plan| {
            let frame = renderer.render(project, plan, &pool);
            let rep = renderer.last_report();
            unknown.extend(rep.unknown_kinds.iter().cloned());
            if warnings.len() < 50 {
                warnings.extend(rep.errors.iter().cloned());
            }
            frame
        },
        &mut |p| {
            if last_report.elapsed().as_secs() >= 2 {
                last_report = std::time::Instant::now();
                eprintln!(
                    "render: {:.1}% ({}/{} frames, {:.1} fps)",
                    p.fraction() * 100.0,
                    p.frame,
                    p.total_frames,
                    p.fps
                );
            }
            true
        },
    )
    .map_err(|e| JbError::invalid(format!("render failed: {e:#}")))?;
    Ok(serde_json::json!({
        "output": report.path,
        "encoder": report.encoder,
        "hardware_encoder": report.hardware_encoder,
        "width": report.width,
        "height": report.height,
        "frames": report.frames,
        "elapsed_seconds": report.elapsed.as_secs_f64(),
        "fps": report.fps,
        "unknown_effects": unknown,
        "warnings": warnings,
    }))
}

/// Renders a single frame to an image file (PNG or JPEG by extension).
pub fn render_frame(project: &Project, at: Ticks, output: &Path, max_height: Option<u32>) -> JbResult<Value> {
    let (width, height) = output_size(project, max_height);
    let pool = media_pool(project, false);
    let mut renderer = gpu_renderer(width, height, max_height.is_some())?;
    let plan = jb_core::eval::evaluate(project, project.main(), at);
    let frame = renderer.render(project, &plan, &pool);
    let report = renderer.last_report().clone();
    let img = image::RgbaImage::from_raw(frame.width, frame.height, frame_rgba(&frame))
        .ok_or_else(|| JbError::invalid("renderer returned a malformed frame"))?;
    img.save(output).map_err(|e| JbError::invalid(format!("cannot write {}: {e}", output.display())))?;
    Ok(serde_json::json!({
        "output": output,
        "width": frame.width,
        "height": frame.height,
        "layers": report.layers,
        "unknown_effects": report.unknown_kinds,
        "warnings": report.errors,
    }))
}

/// Tightly packed RGBA bytes of an RGBA frame.
fn frame_rgba(frame: &jb_core::VideoFrame) -> Vec<u8> {
    let row = frame.width as usize * 4;
    if frame.stride == row {
        return frame.data[..row * frame.height as usize].to_vec();
    }
    (0..frame.height as usize)
        .flat_map(|y| frame.data[y * frame.stride..y * frame.stride + row].iter().copied())
        .collect()
}

/// Parses `540p` / `540` into a preview height.
pub fn parse_preview(s: &str) -> JbResult<u32> {
    s.trim_end_matches('p').parse::<u32>().ok().filter(|h| *h > 0).ok_or_else(|| {
        JbError::invalid(format!("invalid preview size `{s}`")).hint("use a height like `540p` or `720p`")
    })
}

/// Measures the loudness of a clip's audio and builds the
/// `set_normalization` command for `target` LUFS. On a camera video clip
/// the linked audio clip is measured. Returns the command and the measured
/// loudness.
pub fn normalize_command(project: &Project, clip_id: &str, target: f64) -> JbResult<(jb_core::Command, f64)> {
    let clip = project
        .clip(clip_id)
        .ok_or_else(|| JbError::new(crate::error::code::NOT_FOUND, format!("clip {clip_id} not found")))?;
    let measured_id = project
        .sequences
        .iter()
        .flat_map(|s| s.tracks.iter())
        .filter(|t| t.kind == jb_core::TrackKind::Audio)
        .flat_map(|t| t.clips.iter())
        .find(|c| c.id != clip.id && clip.link_group.is_some() && c.link_group == clip.link_group)
        .map_or(clip_id, |c| c.id.as_str());
    let pool = media_pool(project, false);
    let lufs = jb_media::mixer::clip_loudness(project, measured_id, &pool)
        .map_err(|e| JbError::invalid(format!("cannot measure loudness: {e:#}")))?;
    if !lufs.is_finite() {
        return Err(JbError::invalid(format!("clip {clip_id} is silent")).hint("normalization needs audible audio"));
    }
    let cmd = jb_core::Command::SetNormalization {
        clip_id: clip_id.to_string(),
        target_lufs: Some(target),
        measured_lufs: Some(lufs),
    };
    Ok((cmd, lufs))
}

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

/// Encodes a sequence to a video file (pending `jb-export`, `jb-render`, `jb-media`).
pub fn render(_project: &Project, _opts: &RenderOptions) -> JbResult<Value> {
    Err(JbError::not_implemented("rendering", "jb-export"))
}

/// Renders a single frame to an image file (pending `jb-render`, `jb-media`).
pub fn render_frame(_project: &Project, _at: Ticks, _output: &Path, _max_height: Option<u32>) -> JbResult<Value> {
    Err(JbError::not_implemented("frame preview", "jb-render"))
}

/// Parses `540p` / `540` into a preview height.
pub fn parse_preview(s: &str) -> JbResult<u32> {
    s.trim_end_matches('p').parse::<u32>().ok().filter(|h| *h > 0).ok_or_else(|| {
        JbError::invalid(format!("invalid preview size `{s}`")).hint("use a height like `540p` or `720p`")
    })
}

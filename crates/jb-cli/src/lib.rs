//! Front-end independent operations behind the `jb` CLI and the `jb-mcp`
//! server: loading/saving, summaries, command parsing and execution, media
//! import and frame evaluation, all with structured, actionable errors.

pub mod commands;
pub mod error;
pub mod integrations;
pub mod probe;
pub mod project_io;
pub mod summary;
pub mod timecode;

pub use error::{JbError, JbResult};

use jb_core::{Command, Project, Ticks};
use serde_json::{json, Value};
use std::path::Path;

/// JSON Schema of `Command` (the command bus input).
pub fn command_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(jb_core::Command)).expect("schema serializes")
}

/// JSON Schema of `Project` (the native project model).
pub fn project_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(jb_core::Project)).expect("schema serializes")
}

/// JSON Schema of `FramePlan` (output of frame evaluation).
pub fn frame_plan_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(jb_core::eval::FramePlan)).expect("schema serializes")
}

/// Probes `paths` and builds one `batch` command adding every new asset.
/// Files already in the project (same absolute path) are skipped and
/// reported with their existing id. Returns `None` when nothing is new.
pub fn import_media(project: &Project, paths: &[impl AsRef<Path>]) -> JbResult<(Option<Command>, Value)> {
    if paths.is_empty() {
        return Err(JbError::invalid("no media files given").hint("pass one or more file paths"));
    }
    let mut added = Vec::new();
    let mut skipped = Vec::new();
    let mut assets = Vec::new();
    for p in paths {
        let asset = probe::probe(p.as_ref())?;
        let dup = project.media.iter().chain(assets.iter()).find(|m| m.path == asset.path);
        if let Some(existing) = dup {
            skipped.push(json!({ "path": asset.path, "existing_id": existing.id, "reason": "already imported" }));
            continue;
        }
        added.push(summary::media_json(&asset));
        assets.push(asset);
    }
    let cmd = (!assets.is_empty())
        .then(|| Command::Batch { commands: assets.into_iter().map(|asset| Command::AddMedia { asset }).collect() });
    Ok((cmd, json!({ "added": added, "skipped": skipped })))
}

/// Evaluates the frame plan of a sequence (main by default) at `t`.
pub fn evaluate(project: &Project, sequence_id: Option<&str>, t: Ticks) -> JbResult<Value> {
    let seq = match sequence_id {
        Some(id) => project.sequence(id).ok_or_else(|| {
            JbError::new(error::code::NOT_FOUND, format!("sequence {id} not found")).hint(format!(
                "valid sequence ids: {}",
                project.sequences.iter().map(|s| s.id.as_str()).collect::<Vec<_>>().join(", ")
            ))
        })?,
        None => project.main(),
    };
    let plan = jb_core::eval::evaluate(project, seq, t);
    let audio = jb_core::eval::audio_voices(project, seq, t);
    Ok(json!({
        "sequence_id": seq.id,
        "time": timecode::time_json(t, project.settings.frame_rate),
        "plan": plan,
        "audio": audio,
    }))
}

/// Parses a range `a-b` (or `a..b`) of time values.
pub fn parse_range(s: &str, fps: jb_core::Rational) -> JbResult<(Ticks, Ticks)> {
    let (a, b) = s.split_once("..").or_else(|| s.split_once('-')).ok_or_else(|| {
        JbError::invalid(format!("invalid range `{s}`")).hint("use `start-end`, e.g. `00:00:05-00:00:10` or `5s-10s`")
    })?;
    let (a, b) = (timecode::parse_time(a, fps)?, timecode::parse_time(b, fps)?);
    if b <= a {
        return Err(JbError::invalid(format!("range `{s}` is empty")).hint("the end must be after the start"));
    }
    Ok((a, b))
}

//! A single error type shared by the CLI and the MCP server. Every error has
//! a stable machine-readable `code`, a human message and, when possible, a
//! hint telling the caller (often an agent) how to fix the problem.

use jb_core::CommandError;
use serde::Serialize;
use serde_json::{json, Value};

/// Stable error codes. They are part of the CLI/MCP contract.
pub mod code {
    pub const IO: &str = "io_error";
    pub const FILE_NOT_FOUND: &str = "file_not_found";
    pub const PARSE: &str = "parse_error";
    pub const INVALID_ARGUMENT: &str = "invalid_argument";
    pub const NOT_FOUND: &str = "not_found";
    pub const COMMAND_REJECTED: &str = "command_rejected";
    pub const TRACK_LOCKED: &str = "track_locked";
    pub const PROBE_FAILED: &str = "probe_failed";
    pub const NO_PROJECT: &str = "no_project_open";
    pub const NOTHING_TO_UNDO: &str = "nothing_to_undo";
    pub const NOTHING_TO_REDO: &str = "nothing_to_redo";
    pub const NOT_IMPLEMENTED: &str = "not_implemented";
    pub const UNSUPPORTED_FORMAT: &str = "unsupported_format";
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JbError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

pub type JbResult<T> = Result<T, JbError>;

impl JbError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), hint: None }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(code::INVALID_ARGUMENT, message)
    }

    pub fn not_implemented(feature: &str, pending_crate: &str) -> Self {
        Self::new(code::NOT_IMPLEMENTED, format!("{feature} is not available yet"))
            .hint(format!("{feature} needs the `{pending_crate}` crate, which has not landed in this build yet"))
    }

    /// Process exit code for this error. Stable contract:
    /// 1 = io/internal, 3 = file not found, 4 = invalid input or rejected
    /// command, 5 = not implemented. (2 is reserved for usage errors.)
    pub fn exit_code(&self) -> i32 {
        match self.code {
            code::FILE_NOT_FOUND => 3,
            code::PARSE
            | code::INVALID_ARGUMENT
            | code::NOT_FOUND
            | code::COMMAND_REJECTED
            | code::TRACK_LOCKED
            | code::NOTHING_TO_UNDO
            | code::NOTHING_TO_REDO
            | code::NO_PROJECT
            | code::UNSUPPORTED_FORMAT => 4,
            code::NOT_IMPLEMENTED => 5,
            _ => 1,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({ "ok": false, "error": self })
    }

    /// Adds context about which command of a sequence failed.
    pub fn at_command(mut self, index: usize) -> Self {
        self.message = format!("command #{index}: {}", self.message);
        self
    }
}

impl std::fmt::Display for JbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)?;
        if let Some(h) = &self.hint {
            write!(f, "\nhint: {h}")?;
        }
        Ok(())
    }
}

impl std::error::Error for JbError {}

impl From<CommandError> for JbError {
    fn from(e: CommandError) -> Self {
        match e {
            CommandError::NotFound(what) => JbError::new(code::NOT_FOUND, format!("not found: {what}"))
                .hint("list valid ids with `jb info <project> --json` or the get_project / find_clips tools"),
            CommandError::Invalid(msg) => JbError::new(code::COMMAND_REJECTED, msg).hint(
                "clips on one track may not overlap and must have end > start; times are ticks (10_000_000 per second)",
            ),
            CommandError::Locked(track) => JbError::new(code::TRACK_LOCKED, format!("track {track} is locked"))
                .hint(format!(r#"unlock it first: {{"op":"set_track_flags","track_id":"{track}","locked":false}}"#)),
        }
    }
}

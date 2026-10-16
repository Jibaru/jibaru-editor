//! Lossless reader and writer for Filmora `.wfp` project files.
//!
//! A `.wfp` is a ZIP archive. [`read`] maps it into a [`jb_core::Project`]
//! and keeps every original document and archive entry in [`Raw`].
//! [`write`] patches edits made to the project back into those original
//! JSON structures, so fields Jibaru does not understand survive untouched.
//! See `docs/filmora-format.md` for the reverse-engineered format.
//!
//! Every imported entity carries its Filmora identity in its `extra` field
//! (`{"filmora": {...}}`), which is how the writer finds the JSON object to
//! patch. Entities without one (created in Jibaru) get Filmora-shaped JSON.

#![recursion_limit = "256"]

mod archive;
mod coverage;
mod export;
mod import;
mod json;
mod mapping;
mod markers;
mod props;
mod title;

pub use archive::Entry;
pub use coverage::{coverage, Coverage, CoverageCategory, CoverageItem, ParamUsage, ResourceIndex, ResourceInfo};

use jb_core::Project;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("json error in {path}: {source}")]
    Json { path: String, source: serde_json::Error },
    #[error("invalid .wfp: {0}")]
    Format(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Path of the project description inside the archive.
pub const PROJECT_INFO: &str = "ProjectFolder/project_info.json";
/// Path of the media bin description inside the archive.
pub const MEDIAS_INFO: &str = "ProjectFolder/Medias/medias_info.json";

/// A Filmora project: the editable model plus everything needed to write
/// the original file back.
#[derive(Debug, Clone)]
pub struct WfpProject {
    pub project: Project,
    pub raw: Raw,
    /// Things that were preserved but could not be mapped into the model.
    pub warnings: Vec<String>,
}

/// The original documents of a `.wfp`.
#[derive(Debug, Clone)]
pub struct Raw {
    /// Every archive entry, in archive order, with its original bytes.
    pub entries: Vec<Entry>,
    /// `ProjectFolder/project_info.json`.
    pub project_info: Value,
    /// Archive path of the timeline document (`.../timeline.wesproj`).
    pub timeline_path: String,
    pub timeline: Value,
    /// `ProjectFolder/Medias/medias_info.json`, when present.
    pub medias_info: Option<Value>,
    /// Archive path and content of the timeline's `extra.json` (markers).
    pub extra_path: Option<String>,
    pub extra: Option<Value>,
    /// The project exactly as it was mapped at read time; the writer diffs
    /// against it so only fields that were edited get rewritten.
    pub baseline: Project,
}

impl Raw {
    pub fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

/// Reads a `.wfp` file.
pub fn read(path: impl AsRef<Path>) -> Result<WfpProject> {
    let file = std::fs::File::open(path.as_ref())?;
    let entries = archive::read_entries(std::io::BufReader::new(file))?;
    from_entries(entries)
}

/// Reads a `.wfp` from memory.
pub fn read_bytes(bytes: &[u8]) -> Result<WfpProject> {
    from_entries(archive::read_entries(std::io::Cursor::new(bytes))?)
}

/// Builds a project from already extracted archive entries.
pub fn from_entries(entries: Vec<Entry>) -> Result<WfpProject> {
    import::import(entries)
}

/// Report of a write.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WriteReport {
    /// Edits that could not be expressed in Filmora's format and were skipped.
    pub warnings: Vec<String>,
}

/// Writes a `.wfp` file, patching the edits made to `wfp.project` into the
/// original documents.
pub fn write(wfp: &WfpProject, path: impl AsRef<Path>) -> Result<WriteReport> {
    let (entries, report) = export::export(wfp)?;
    let file = std::fs::File::create(path.as_ref())?;
    archive::write_entries(std::io::BufWriter::new(file), &entries)?;
    Ok(report)
}

/// Writes a `.wfp` into memory.
pub fn write_bytes(wfp: &WfpProject) -> Result<(Vec<u8>, WriteReport)> {
    let (entries, report) = export::export(wfp)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    archive::write_entries(&mut cursor, &entries)?;
    Ok((cursor.into_inner(), report))
}

/// The archive entries `write` would produce, without zipping them.
pub fn to_entries(wfp: &WfpProject) -> Result<(Vec<Entry>, WriteReport)> {
    export::export(wfp)
}

fn parse_json(path: &str, bytes: &[u8]) -> Result<Value> {
    // Some documents carry a UTF-8 BOM or a trailing NUL.
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let end = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    serde_json::from_slice(&bytes[..end]).map_err(|source| Error::Json { path: path.to_string(), source })
}

/// Serializes a document in the same style as the original bytes
/// (pretty-printed with 4 spaces, or compact).
fn serialize_like(original: &[u8], doc: &Value) -> Vec<u8> {
    if original.contains(&b'\n') {
        let mut out = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
        serde::Serialize::serialize(doc, &mut ser).expect("serializing a Value cannot fail");
        out
    } else {
        serde_json::to_vec(doc).expect("serializing a Value cannot fail")
    }
}

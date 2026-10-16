//! Loading, saving and creating projects, independent of the front end.

use crate::error::{code, JbError, JbResult};
use crate::integrations;
use jb_core::{from_native, to_native, Project, ProjectSettings, Rational, NATIVE_EXTENSION};
use std::path::Path;

/// Project file formats the tooling knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Native,
    Filmora,
}

pub fn detect_format(path: &Path) -> JbResult<Format> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some(NATIVE_EXTENSION) => Ok(Format::Native),
        Some("wfp") => Ok(Format::Filmora),
        _ => Err(JbError::new(code::UNSUPPORTED_FORMAT, format!("unsupported project file `{}`", path.display()))
            .hint("use a native `.jbp` project (create one with `jb new`) or a Filmora `.wfp` file")),
    }
}

/// Loads a project from any supported format.
pub fn load_project(path: &Path) -> JbResult<Project> {
    let format = detect_format(path)?;
    if !path.exists() {
        return Err(JbError::new(code::FILE_NOT_FOUND, format!("project file `{}` does not exist", path.display()))
            .hint("check the path, or create a new project with `jb new <file.jbp>` / the new_project tool"));
    }
    match format {
        Format::Native => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| JbError::new(code::IO, format!("cannot read `{}`: {e}", path.display())))?;
            from_native(&text).map_err(|e| {
                JbError::new(code::PARSE, format!("`{}` is not a valid jibaru project: {e}", path.display()))
                    .hint("run `jb schema project` to see the expected shape")
            })
        }
        Format::Filmora => integrations::filmora_open(path),
    }
}

/// Saves a project in the native format, atomically (write + rename).
pub fn save_project(path: &Path, project: &Project) -> JbResult<()> {
    if detect_format(path)? != Format::Native {
        return Err(JbError::new(code::UNSUPPORTED_FORMAT, format!("can only save `.{NATIVE_EXTENSION}` files"))
            .hint(format!("choose an output path ending in `.{NATIVE_EXTENSION}`")));
    }
    let text = to_native(project).map_err(|e| JbError::new(code::IO, format!("serialize project: {e}")))?;
    let tmp = path.with_extension(format!("{NATIVE_EXTENSION}.tmp"));
    let io = |e: std::io::Error| JbError::new(code::IO, format!("cannot write `{}`: {e}", path.display()));
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    std::fs::write(&tmp, text).map_err(io)?;
    std::fs::rename(&tmp, path).map_err(io)
}

/// Parses `30`, `29.97`, `23.976`, `59.94` or an exact `30000/1001`.
pub fn parse_fps(s: &str) -> JbResult<Rational> {
    let bad = || {
        JbError::invalid(format!("invalid frame rate `{s}`"))
            .hint("use an integer (`30`), a decimal (`29.97`) or a fraction (`30000/1001`)")
    };
    let r = if let Some((n, d)) = s.split_once('/') {
        Rational::new(n.trim().parse().map_err(|_| bad())?, d.trim().parse().map_err(|_| bad())?)
    } else {
        let v: f64 = s.trim().parse().map_err(|_| bad())?;
        if (v - v.round()).abs() < 1e-9 {
            Rational::new(v.round() as i64, 1)
        } else {
            // NTSC style rates: 29.97 -> 30000/1001, 23.976 -> 24000/1001.
            let ntsc = (v * 1.001).round();
            if (ntsc / 1.001 - v).abs() < 0.01 {
                Rational::new(ntsc as i64 * 1000, 1001)
            } else {
                Rational::new((v * 1000.0).round() as i64, 1000)
            }
        }
    };
    if r.num <= 0 || r.den <= 0 {
        return Err(bad());
    }
    Ok(r)
}

pub fn new_project(name: &str, width: u32, height: u32, fps: Rational) -> JbResult<Project> {
    if width == 0 || height == 0 {
        return Err(JbError::invalid("width and height must be positive"));
    }
    Ok(Project::new(
        name,
        ProjectSettings { width, height, frame_rate: fps, sample_rate: 48_000, background: [0.0, 0.0, 0.0, 1.0] },
    ))
}

/// Default project name derived from a file path.
pub fn name_from_path(path: &Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_forms() {
        assert_eq!(parse_fps("30").unwrap(), Rational::new(30, 1));
        assert_eq!(parse_fps("29.97").unwrap(), Rational::new(30000, 1001));
        assert_eq!(parse_fps("23.976").unwrap(), Rational::new(24000, 1001));
        assert_eq!(parse_fps("60000/1001").unwrap(), Rational::new(60000, 1001));
        assert!(parse_fps("0").is_err());
        assert!(parse_fps("x").is_err());
    }
}

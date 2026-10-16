//! Media probing for `import-media`, backed by `jb_media::probe` (linked
//! FFmpeg libraries, so no external `ffprobe` binary is needed).

use crate::error::{code, JbError, JbResult};
use jb_core::MediaAsset;
use std::path::Path;

pub fn probe(path: &Path) -> JbResult<MediaAsset> {
    if !path.is_file() {
        return Err(JbError::new(code::FILE_NOT_FOUND, format!("media file `{}` does not exist", path.display()))
            .hint("pass an existing file path; relative paths are resolved against the current directory"));
    }
    let abs = std::fs::canonicalize(path)
        .map_err(|e| JbError::new(code::IO, format!("cannot resolve `{}`: {e}", path.display())))?;
    let mut asset = jb_media::probe(&abs).map_err(|e| {
        JbError::new(code::PROBE_FAILED, format!("could not read `{}`: {e:#}", abs.display()))
            .hint("check that the file is a supported video, audio or image file")
    })?;
    asset.path = abs.to_string_lossy().into_owned();
    Ok(asset)
}

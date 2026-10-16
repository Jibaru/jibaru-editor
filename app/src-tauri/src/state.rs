//! The app's editing session: one `jb_core::History` behind a mutex. Every
//! UI edit arrives as a serialized `Command`, exactly like CLI and agent
//! edits, so all clients share one API and one undo stack.

use jb_core::{Command, History, MediaAsset, Project, ProjectSettings, Rational};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Session {
    pub history: History,
    pub path: Option<PathBuf>,
    /// The original Filmora documents when the project came from a `.wfp`,
    /// so saving back patches them losslessly.
    pub wfp: Option<jb_filmora::WfpProject>,
    revision: u64,
    saved_revision: u64,
    autosaved_revision: u64,
    /// Files already backed up this session before being overwritten.
    backed_up: Vec<PathBuf>,
}

/// What the frontend receives after every change (see app/src/core/types.ts).
#[derive(Serialize, Clone)]
pub struct Snapshot {
    pub project: Project,
    pub path: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
    pub revision: u64,
    /// True when the project came from a `.wfp` and can be saved back to one.
    pub wfp: bool,
}

pub fn default_settings() -> ProjectSettings {
    ProjectSettings {
        width: 1920,
        height: 1080,
        frame_rate: Rational::new(30, 1),
        sample_rate: 48000,
        background: [0.0, 0.0, 0.0, 1.0],
    }
}

/// A new project with one video and one audio track, like Filmora's default.
pub fn empty_project(name: &str, settings: ProjectSettings) -> Project {
    let mut h = History::new(Project::new(name, settings));
    for kind in [jb_core::TrackKind::Video, jb_core::TrackKind::Audio] {
        h.execute(&Command::AddTrack { sequence_id: None, kind, index: None }).expect("add default track");
    }
    h.project
}

impl Session {
    pub fn new(project: Project, path: Option<PathBuf>) -> Self {
        Self {
            history: History::new(project),
            path,
            wfp: None,
            revision: 0,
            saved_revision: 0,
            autosaved_revision: 0,
            backed_up: Vec::new(),
        }
    }

    pub fn from_loaded(loaded: Loaded, path: PathBuf) -> Self {
        let mut s = Self::new(loaded.project, Some(path));
        s.wfp = loaded.wfp;
        s
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            project: self.history.project.clone(),
            path: self.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            dirty: self.dirty(),
            revision: self.revision,
            wfp: self.wfp.is_some(),
        }
    }

    pub fn dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn execute(&mut self, cmd: &Command) -> Result<(), String> {
        self.history.execute(cmd).map_err(|e| e.to_string())?;
        self.revision += 1;
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let done = self.history.undo();
        if done {
            self.revision += 1;
        }
        done
    }

    pub fn redo(&mut self) -> bool {
        let done = self.history.redo();
        if done {
            self.revision += 1;
        }
        done
    }

    /// The project to autosave, when it changed since the last save/autosave.
    pub fn autosave_due(&self) -> Option<(Project, u64)> {
        (self.dirty() && self.revision != self.autosaved_revision)
            .then(|| (self.history.project.clone(), self.revision))
    }

    pub fn mark_autosaved(&mut self, revision: u64) {
        self.autosaved_revision = revision;
    }

    /// Saves to `path` (or the current path). An existing file is copied to
    /// `backup_dir` the first time it is overwritten in this session, and
    /// writes go through a temporary file so a failure never truncates the
    /// original. Returns the writer's report (warnings: edits that could
    /// not be written; notes: edits kept only for Jibaru).
    pub fn save(&mut self, path: Option<&Path>, backup_dir: Option<&Path>) -> anyhow::Result<jb_filmora::WriteReport> {
        let target =
            path.map(Path::to_path_buf).or_else(|| self.path.clone()).ok_or_else(|| anyhow::anyhow!("no path"))?;
        let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let tmp = target.with_extension(format!("{ext}.saving"));
        let report = match ext.as_str() {
            "jbp" => {
                std::fs::write(&tmp, jb_core::to_native(&self.history.project)?)?;
                jb_filmora::WriteReport::default()
            }
            "wfp" => {
                let Some(wfp) = &self.wfp else {
                    anyhow::bail!("only projects opened from a .wfp can be saved as .wfp");
                };
                let mut out = wfp.clone();
                out.project = self.history.project.clone();
                jb_filmora::write(&out, &tmp)?
            }
            _ => anyhow::bail!("cannot save as {}; use .jbp or .wfp", target.display()),
        };
        if target.exists() && !self.backed_up.contains(&target) {
            if let Some(dir) = backup_dir {
                std::fs::create_dir_all(dir)?;
                let stamp =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
                let stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("project");
                std::fs::copy(&target, dir.join(format!("{stem}-{stamp}.{ext}")))?;
            }
            self.backed_up.push(target.clone());
        }
        if let Err(e) = std::fs::rename(&tmp, &target) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        self.path = Some(target);
        self.saved_revision = self.revision;
        Ok(report)
    }
}

pub struct Loaded {
    pub project: Project,
    pub wfp: Option<jb_filmora::WfpProject>,
}

/// Loads a native `.jbp` project or a Filmora `.wfp` (via `jb-filmora`).
pub fn load(path: &Path) -> anyhow::Result<Loaded> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "jbp" | "json" => Ok(Loaded { project: jb_core::from_native(&std::fs::read_to_string(path)?)?, wfp: None }),
        "wfp" => {
            let wfp = jb_filmora::read(path)?;
            Ok(Loaded { project: wfp.project.clone(), wfp: Some(wfp) })
        }
        _ => anyhow::bail!("unsupported project type: {}", path.display()),
    }
}

pub struct AppState(pub Mutex<Session>);

pub fn add_media_command(assets: Vec<MediaAsset>) -> Command {
    Command::Batch { commands: assets.into_iter().map(|asset| Command::AddMedia { asset }).collect() }
}

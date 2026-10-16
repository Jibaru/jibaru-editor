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
    revision: u64,
    saved_revision: u64,
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
        Self { history: History::new(project), path, revision: 0, saved_revision: 0 }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            project: self.history.project.clone(),
            path: self.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            dirty: self.revision != self.saved_revision,
            revision: self.revision,
        }
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

    pub fn save(&mut self, path: Option<&Path>) -> anyhow::Result<()> {
        let target = path.map(Path::to_path_buf).or_else(|| self.path.clone()).ok_or_else(|| anyhow::anyhow!("no path"))?;
        let is_native = target.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("jbp"));
        if !is_native {
            anyhow::bail!("saving as {} is not supported yet; use .jbp", target.display());
        }
        std::fs::write(&target, jb_core::to_native(&self.history.project)?)?;
        self.path = Some(target);
        self.saved_revision = self.revision;
        Ok(())
    }
}

/// Loads a project file. Native `.jbp` is always supported; Filmora `.wfp`
/// goes through `jb-filmora` once that crate provides a reader.
pub fn load(path: &Path) -> anyhow::Result<Project> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "jbp" | "json" => Ok(jb_core::from_native(&std::fs::read_to_string(path)?)?),
        "wfp" => anyhow::bail!("opening Filmora projects is not available in this build yet"),
        _ => anyhow::bail!("unsupported project type: {}", path.display()),
    }
}

pub struct AppState(pub Mutex<Session>);

pub fn add_media_command(assets: Vec<MediaAsset>) -> Command {
    Command::Batch { commands: assets.into_iter().map(|asset| Command::AddMedia { asset }).collect() }
}

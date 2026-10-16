//! Export jobs: `jb-export` on a background thread with its own media pool
//! (full-resolution sources, no proxies) and its own GPU renderer, so the
//! preview stays usable while exporting. Progress and the result are
//! reported through callbacks (`export-progress` / `export-done` events).

use jb_core::{Project, Rational, Ticks};
use jb_media::{MediaPool, PoolConfig};
use jb_render::Registry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    pub path: String,
    /// "h264" or "hevc".
    pub codec: String,
    /// Output size; 0 uses the project size.
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    pub frame_rate: Option<Rational>,
    /// "low", "medium" or "high": scales the default bit rate.
    #[serde(default)]
    pub quality: String,
    /// Explicit bit rate in bits per second (overrides `quality`).
    pub bit_rate: Option<u64>,
    /// Timeline range `[start, end)` in ticks.
    pub range: Option<(Ticks, Ticks)>,
    #[serde(default = "yes")]
    pub audio: bool,
    #[serde(default = "yes")]
    pub hardware: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub frame: u64,
    pub total: u64,
    pub fps: f64,
    pub elapsed: f64,
    /// Estimated seconds left.
    pub eta: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Done {
    pub ok: bool,
    pub cancelled: bool,
    pub error: Option<String>,
    pub path: String,
    pub encoder: String,
    pub hardware_encoder: bool,
    pub width: u32,
    pub height: u32,
    pub frames: u64,
    pub fps: f64,
    pub elapsed: f64,
    pub unknown_effects: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct Exporter {
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    last: Arc<Mutex<Option<Done>>>,
}

/// Effect and transition kinds used by `project` that the renderer does not
/// implement (they render as pass-through).
pub fn unknown_effects(project: &Project) -> Vec<String> {
    let reg = Registry::builtin();
    let mut out = BTreeSet::new();
    for seq in &project.sequences {
        for t in &seq.tracks {
            for c in &t.clips {
                for e in c.effects.iter().chain(&c.animations).filter(|e| e.enabled) {
                    if reg.get(&e.kind).is_none() {
                        out.insert(e.kind.clone());
                    }
                }
                if let Some(tr) = &c.transition_out {
                    if reg.get(&tr.kind).is_none() {
                        out.insert(tr.kind.clone());
                    }
                }
            }
        }
    }
    out.into_iter().collect()
}

impl Exporter {
    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn last(&self) -> Option<Done> {
        self.last.lock().unwrap().clone()
    }

    pub fn start(
        &self,
        project: Project,
        req: ExportRequest,
        on_progress: impl Fn(&Progress) + Send + 'static,
        on_done: impl Fn(&Done) + Send + 'static,
    ) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("an export is already running".into());
        }
        self.cancel.store(false, Ordering::SeqCst);
        let cancel = self.cancel.clone();
        let running = self.running.clone();
        let last = self.last.clone();
        std::thread::Builder::new()
            .name("jb-export".into())
            .spawn(move || {
                let done = run(&project, &req, &cancel, &on_progress);
                *last.lock().unwrap() = Some(done.clone());
                running.store(false, Ordering::SeqCst);
                on_done(&done);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn run(project: &Project, req: &ExportRequest, cancel: &AtomicBool, on_progress: &dyn Fn(&Progress)) -> Done {
    let mut done = Done { path: req.path.clone(), ..Default::default() };
    let s = &project.settings;
    let (width, height) = if req.width == 0 || req.height == 0 { (s.width, s.height) } else { (req.width, req.height) };
    let (width, height) = (width & !1, height & !1);
    let rate = req.frame_rate.unwrap_or(s.frame_rate);
    let mut settings = jb_export::ExportSettings::new(PathBuf::from(&req.path));
    settings.width = width;
    settings.height = height;
    settings.frame_rate = Some(rate);
    settings.codec = if req.codec.eq_ignore_ascii_case("hevc") || req.codec.eq_ignore_ascii_case("h265") {
        jb_export::VideoCodec::Hevc
    } else {
        jb_export::VideoCodec::H264
    };
    let base = jb_media::encode::default_bit_rate(width, height, rate.as_f64());
    settings.video_bit_rate = req.bit_rate.unwrap_or_else(|| match req.quality.as_str() {
        "low" => base / 2,
        "high" => base * 8 / 5,
        _ => base,
    });
    settings.range = req.range;
    settings.audio = req.audio;
    settings.hardware = req.hardware;

    let pool = MediaPool::for_project(project, PoolConfig::default());
    let mut renderer = match jb_render::Renderer::new(width, height) {
        Ok(r) => r,
        Err(e) => {
            done.error = Some(format!("no usable GPU: {e}"));
            return done;
        }
    };
    let mut unknown = BTreeSet::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut last = Instant::now() - std::time::Duration::from_secs(1);
    let result = jb_export::export(
        project,
        &project.main_sequence,
        &settings,
        &pool,
        &mut |plan| {
            let frame = renderer.render(project, plan, &pool);
            let rep = renderer.last_report();
            unknown.extend(rep.unknown_kinds.iter().cloned());
            for w in &rep.errors {
                if warnings.len() < 50 && !warnings.contains(w) {
                    warnings.push(w.clone());
                }
            }
            frame
        },
        &mut |p| {
            if last.elapsed().as_millis() >= 250 || p.frame == p.total_frames {
                last = Instant::now();
                let left = p.total_frames.saturating_sub(p.frame) as f64;
                on_progress(&Progress {
                    frame: p.frame,
                    total: p.total_frames,
                    fps: p.fps,
                    elapsed: p.elapsed.as_secs_f64(),
                    eta: if p.fps > 0.0 { left / p.fps } else { 0.0 },
                });
            }
            !cancel.load(Ordering::Relaxed)
        },
    );
    done.unknown_effects = unknown.into_iter().collect();
    done.warnings = warnings;
    match result {
        Ok(r) => {
            done.ok = true;
            done.path = r.path.to_string_lossy().into_owned();
            done.encoder = r.encoder;
            done.hardware_encoder = r.hardware_encoder;
            done.width = r.width;
            done.height = r.height;
            done.frames = r.frames;
            done.fps = r.fps;
            done.elapsed = r.elapsed.as_secs_f64();
        }
        Err(e) => {
            done.cancelled = cancel.load(Ordering::Relaxed);
            done.error = Some(if done.cancelled { "Export cancelled".into() } else { format!("{e:#}") });
        }
    }
    done
}

/// Shows `path` in Finder / Explorer.
pub fn reveal(path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let r = std::process::Command::new("xdg-open")
        .arg(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("/")))
        .spawn();
    r.map(|_| ())
}

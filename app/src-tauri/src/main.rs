// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_out;
mod export;
mod media;
#[cfg(target_os = "macos")]
mod menu;
mod playback;
mod proxies;
mod state;

use jb_core::{Command, MediaAsset, ProjectSettings, Ticks};
use media::Engine;
use playback::Playback;
use proxies::Proxies;
use serde::Serialize;
use state::{AppState, Session, Snapshot};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::http::{header, Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager, State};

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Engine services shared by commands, the `jb://` protocol and background threads.
struct Services {
    engine: Arc<Engine>,
    playback: Playback,
    proxies: Proxies,
    exporter: export::Exporter,
    /// Set once the user confirmed closing with unsaved changes.
    force_close: AtomicBool,
    backup_dir: PathBuf,
    autosave_dir: PathBuf,
}

fn with_session<T>(state: &State<AppState>, f: impl FnOnce(&mut Session) -> Res<T>) -> Res<T> {
    let mut s = state.0.lock().map_err(err)?;
    f(&mut s)
}

/// Hands the current project to the preview engine and proxy manager.
fn engine_sync(app: &AppHandle, s: &Session) {
    let svc = app.state::<Services>();
    svc.playback.set_project(s.history.project.clone());
    svc.proxies.register(&s.history.project);
}

/// Runs `f` on the session and then syncs the engine with the result.
fn edit<T>(app: &AppHandle, f: impl FnOnce(&mut Session) -> Res<T>) -> Res<T> {
    let state = app.state::<AppState>();
    let mut s = state.0.lock().map_err(err)?;
    let out = f(&mut s)?;
    engine_sync(app, &s);
    Ok(out)
}

/// Notifies every window (and future agent bridges) about a new state.
fn changed(app: &AppHandle, snap: &Snapshot) {
    let _ = app.emit("project-changed", snap);
}

#[tauri::command]
fn get_project(state: State<AppState>) -> Res<Snapshot> {
    with_session(&state, |s| Ok(s.snapshot()))
}

#[tauri::command]
fn new_project(app: AppHandle, name: String, settings: Option<serde_json::Value>) -> Res<Snapshot> {
    let mut base = serde_json::to_value(state::default_settings()).map_err(err)?;
    if let (Some(serde_json::Value::Object(over)), serde_json::Value::Object(b)) = (settings, &mut base) {
        b.extend(over);
    }
    let settings: ProjectSettings = serde_json::from_value(base).map_err(err)?;
    edit(&app, |s| {
        *s = Session::new(state::empty_project(&name, settings), None);
        Ok(s.snapshot())
    })
}

#[tauri::command]
async fn open_project(app: AppHandle, path: String) -> Res<Snapshot> {
    let p = path.clone();
    let loaded =
        tauri::async_runtime::spawn_blocking(move || state::load(Path::new(&p))).await.map_err(err)?.map_err(err)?;
    edit(&app, |s| {
        *s = Session::from_loaded(loaded, PathBuf::from(&path));
        Ok(s.snapshot())
    })
}

#[derive(Serialize)]
struct Saved {
    snapshot: Snapshot,
    warnings: Vec<String>,
    /// Edits saved for Jibaru only (a `.wfp` sidecar); Filmora won't show them.
    notes: Vec<String>,
}

#[tauri::command]
fn save_project(app: AppHandle, path: Option<String>) -> Res<Saved> {
    let svc = app.state::<Services>();
    let state = app.state::<AppState>();
    with_session(&state, |s| {
        let report = s.save(path.as_deref().map(Path::new), Some(&svc.backup_dir)).map_err(err)?;
        Ok(Saved { snapshot: s.snapshot(), warnings: report.warnings, notes: report.notes })
    })
}

#[tauri::command]
fn execute(app: AppHandle, command: Command) -> Res<Snapshot> {
    edit(&app, |s| {
        s.execute(&command)?;
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn undo(app: AppHandle) -> Res<Snapshot> {
    edit(&app, |s| {
        s.undo();
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn redo(app: AppHandle) -> Res<Snapshot> {
    edit(&app, |s| {
        s.redo();
        Ok(s.snapshot())
    })
}

#[tauri::command]
async fn probe_media(paths: Vec<String>) -> Res<Vec<MediaAsset>> {
    tauri::async_runtime::spawn_blocking(move || {
        paths.iter().map(|p| media::probe(Path::new(p)).map_err(err)).collect()
    })
    .await
    .map_err(err)?
}

#[tauri::command]
fn fx_catalog() -> Vec<media::FxEntry> {
    media::catalog()
}

/// Extensions of the files the media importer accepts.
const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "mkv", "webm", "avi", "mts", "m2ts", "mp3", "wav", "m4a", "aac", "flac", "ogg", "jpg", "jpeg",
    "png", "heic", "webp", "gif", "bmp", "tif", "tiff",
];

/// Files to import for `paths`: files as given, folders expanded to the
/// media files inside them (recursively, sorted, hidden entries skipped).
fn expand_media_paths(paths: Vec<String>) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut list: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.')))
            .collect();
        list.sort();
        for p in list {
            if p.is_dir() {
                walk(&p, out);
            } else if p
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| MEDIA_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
            {
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
    let mut out = Vec::new();
    for p in paths {
        let path = Path::new(&p);
        if path.is_dir() {
            walk(path, &mut out);
        } else {
            out.push(p);
        }
    }
    out
}

/// Executes `cmd` as one undo step and returns the new snapshot.
fn execute_one(app: &AppHandle, cmd: Command) -> Res<Snapshot> {
    edit(app, |s| {
        s.execute(&cmd)?;
        Ok(s.snapshot())
    })
}

fn project_clone(app: &AppHandle) -> Res<jb_core::Project> {
    let state = app.state::<AppState>();
    let s = state.0.lock().map_err(err)?;
    Ok(s.history.project.clone())
}

/// Auto Normalization: measures the clip's loudness and normalizes it to
/// `target_lufs`; `None` turns normalization off. On a camera clip the
/// linked audio clip is measured (that is what plays).
#[tauri::command]
async fn normalize_clip(app: AppHandle, clip_id: String, target_lufs: Option<f64>) -> Res<Snapshot> {
    let Some(target) = target_lufs else {
        return execute_one(&app, Command::SetNormalization { clip_id, target_lufs: None, measured_lufs: None });
    };
    let project = project_clone(&app)?;
    let clip = project.clip(&clip_id).ok_or_else(|| format!("clip {clip_id} not found"))?;
    let measured_id = project
        .sequences
        .iter()
        .flat_map(|s| s.tracks.iter())
        .filter(|t| t.kind == jb_core::TrackKind::Audio)
        .flat_map(|t| t.clips.iter())
        .find(|c| c.id != clip.id && clip.link_group.is_some() && c.link_group == clip.link_group)
        .map_or(clip_id.clone(), |c| c.id.clone());
    let lufs = tauri::async_runtime::spawn_blocking(move || {
        let pool = jb_media::MediaPool::for_project(&project, jb_media::PoolConfig::default());
        jb_media::mixer::clip_loudness(&project, &measured_id, &pool)
    })
    .await
    .map_err(err)?
    .map_err(|e| format!("cannot measure loudness: {e:#}"))?;
    if !lufs.is_finite() {
        return Err("the clip is silent; normalization needs audible audio".into());
    }
    execute_one(&app, Command::SetNormalization { clip_id, target_lufs: Some(target), measured_lufs: Some(lufs) })
}

/// Stabilization: analyzes the camera motion of the clip and stores the
/// corrections in the `data` parameter of its `jb.stabilize` effect
/// `effect_id`, using the effect's `smoothness` (0..100, default 50).
#[tauri::command]
async fn analyze_stabilization(app: AppHandle, clip_id: String, effect_id: String) -> Res<Snapshot> {
    let project = project_clone(&app)?;
    let clip = project.clip(&clip_id).ok_or_else(|| format!("clip {clip_id} not found"))?;
    let effect =
        clip.effects.iter().find(|e| e.id == effect_id).ok_or_else(|| format!("effect {effect_id} not found"))?;
    let smoothness = match effect.params.get("smoothness").map(|p| &p.value) {
        Some(jb_core::Value::Float(v)) => *v,
        _ => 50.0,
    };
    let id = clip_id.clone();
    let data =
        tauri::async_runtime::spawn_blocking(move || jb_media::stabilize::analyze_clip(&project, &id, smoothness))
            .await
            .map_err(err)?
            .map_err(|e| format!("stabilization failed: {e:#}"))?;
    let param = jb_core::Param { value: jb_core::Value::Text(data), keyframes: Vec::new() };
    let params = std::collections::BTreeMap::from([("data".to_string(), param)]);
    execute_one(&app, Command::SetEffectParams { clip_id, effect_id, params })
}

/// Decodes standard or URL-safe base64 (padding, whitespace and a
/// `data:` URL prefix tolerated).
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.split_once(";base64,").map_or(s, |(_, b)| b);
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err("invalid base64".into()),
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// Player snapshot: writes the PNG next to the project (`Snapshots/`), or to
/// `~/Pictures/Jibaru Snapshots/` for unsaved projects, and adds it to the
/// media library like Filmora does.
#[tauri::command]
async fn save_snapshot(app: AppHandle, png: String, name: String) -> Res<Snapshot> {
    let bytes = base64_decode(&png)?;
    if !bytes.starts_with(b"\x89PNG") {
        return Err("snapshot is not a PNG image".into());
    }
    let project_dir = {
        let state = app.state::<AppState>();
        let s = state.0.lock().map_err(err)?;
        s.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf))
    };
    let dir = snapshot_dir(project_dir);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let file = unique_file(&dir, &sanitize(&name), "png");
    std::fs::write(&file, &bytes).map_err(err)?;
    import_media(app, vec![file.to_string_lossy().into_owned()]).await
}

fn snapshot_dir(project_dir: Option<PathBuf>) -> PathBuf {
    match project_dir {
        Some(d) => d.join("Snapshots"),
        None => std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Pictures")
            .join("Jibaru Snapshots"),
    }
}

/// `dir/base.ext`, or `dir/base (2).ext`... when taken.
fn unique_file(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut file = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while file.exists() {
        file = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    file
}

/// Opens an http(s) URL in the default browser.
#[tauri::command]
fn open_url(url: String) -> Res<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.chars().any(|c| c.is_whitespace()) {
        return Err("only http(s) URLs can be opened".into());
    }
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(err)
}

#[tauri::command]
async fn import_media(app: AppHandle, paths: Vec<String>) -> Res<Snapshot> {
    let paths = expand_media_paths(paths);
    if paths.is_empty() {
        return Err("No media files found".into());
    }

    let known: Vec<String> = {
        let state = app.state::<AppState>();
        let s = state.0.lock().map_err(err)?;
        s.history.project.media.iter().map(|m| m.path.clone()).collect()
    };
    let fresh: Vec<String> = paths.into_iter().filter(|p| !known.contains(p)).collect();
    // Probe what we can; one unreadable file should not block the rest.
    let probed = tauri::async_runtime::spawn_blocking(move || {
        fresh.iter().map(|p| (p.clone(), media::probe(Path::new(p)))).collect::<Vec<_>>()
    })
    .await
    .map_err(err)?;
    let mut failed = Vec::new();
    let mut assets = Vec::new();
    for (p, r) in probed {
        match r {
            Ok(a) => assets.push(a),
            Err(e) => failed
                .push(format!("{}: {e}", Path::new(&p).file_name().map_or(p.clone(), |n| n.to_string_lossy().into()))),
        }
    }
    let snap = edit(&app, |s| {
        let new: Vec<MediaAsset> = assets.into_iter().filter(|a| s.history.project.media(&a.id).is_none()).collect();
        if !new.is_empty() {
            s.execute(&state::add_media_command(new))?;
        }
        Ok(s.snapshot())
    })?;
    changed(&app, &snap);
    if !failed.is_empty() {
        return Err(format!("Could not import {}", failed.join(", ")));
    }
    Ok(snap)
}

// ---------- Playback ----------

#[tauri::command]
fn playback_play(app: AppHandle, time: Ticks, rate: Option<f64>) {
    app.state::<Services>().playback.play(time, rate.unwrap_or(1.0));
}

#[tauri::command]
fn playback_pause(app: AppHandle) -> Ticks {
    app.state::<Services>().playback.pause()
}

#[tauri::command]
fn playback_seek(app: AppHandle, time: Ticks, scrub: Option<bool>) {
    app.state::<Services>().playback.seek(time, scrub.unwrap_or(false));
}

#[tauri::command]
fn playback_size(app: AppHandle, width: u32, height: u32) {
    app.state::<Services>().playback.set_size(width, height);
}

#[tauri::command]
fn playback_volume(app: AppHandle, volume: f32) {
    app.state::<Services>().playback.set_volume(volume);
}

#[tauri::command]
fn playback_stats(app: AppHandle) -> playback::Stats {
    app.state::<Services>().playback.stats()
}

/// Display-side statistics from the webview, logged with `JB_PLAYBACK_LOG=1`.
#[tauri::command]
fn playback_report(app: AppHandle, displayed_fps: f64, frames: u64) {
    if std::env::var_os("JB_PLAYBACK_LOG").is_some() {
        let s = app.state::<Services>().playback.stats();
        eprintln!(
            "playback: displayed {displayed_fps:.1} fps ({frames} frames), rendered {:.1} fps, dropped {}, render {:.1} ms, {}x{}, audio clock {}, underruns {}, audio rms {:.3}",
            s.fps, s.dropped, s.render_ms, s.width, s.height, s.audio_clock, s.audio_underruns, s.audio_rms
        );
    }
}

// ---------- Proxies ----------

#[tauri::command]
fn proxy_statuses(app: AppHandle) -> Vec<proxies::ProxyStatus> {
    app.state::<Services>().proxies.statuses()
}

// ---------- Export ----------

#[tauri::command]
fn export_check(state: State<AppState>) -> Res<Vec<String>> {
    with_session(&state, |s| Ok(export::unknown_effects(&s.history.project)))
}

#[tauri::command]
fn export_start(app: AppHandle, request: export::ExportRequest) -> Res<()> {
    let project = {
        let state = app.state::<AppState>();
        let s = state.0.lock().map_err(err)?;
        s.history.project.clone()
    };
    if let Some(dir) = Path::new(&request.path).parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let (a, b) = (app.clone(), app.clone());
    app.state::<Services>().exporter.start(
        project,
        request,
        move |p| {
            let _ = a.emit("export-progress", p);
        },
        move |d| {
            let _ = b.emit("export-done", d);
        },
    )
}

#[tauri::command]
fn export_cancel(app: AppHandle) {
    app.state::<Services>().exporter.cancel();
}

#[tauri::command]
fn export_status(app: AppHandle) -> serde_json::Value {
    let ex = &app.state::<Services>().exporter;
    serde_json::json!({ "running": ex.running(), "last": ex.last() })
}

#[tauri::command]
fn reveal_path(path: String) -> Res<()> {
    export::reveal(&path).map_err(err)
}

#[tauri::command]
fn default_export_dir() -> String {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    home.map(|h| h.join(if cfg!(target_os = "windows") { "Videos" } else { "Movies" }))
        .unwrap_or_else(std::env::temp_dir)
        .to_string_lossy()
        .into_owned()
}

// ---------- App lifecycle ----------

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.state::<Services>().force_close.store(true, Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
fn autosave_dir(app: AppHandle) -> String {
    app.state::<Services>().autosave_dir.to_string_lossy().into_owned()
}

fn sanitize(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' }).collect();
    let s = s.trim().to_string();
    if s.is_empty() {
        "Untitled".into()
    } else {
        s
    }
}

/// Every 30 s, writes unsaved changes to `<app data>/autosave/<name>.autosave.jbp`
/// (never to the user's project file).
fn autosave_loop(app: AppHandle) {
    loop {
        std::thread::sleep(Duration::from_secs(30));
        let due = {
            let state = app.state::<AppState>();
            let s = match state.0.lock() {
                Ok(s) => s,
                Err(_) => continue,
            };
            s.autosave_due()
        };
        let Some((project, revision)) = due else { continue };
        let dir = app.state::<Services>().autosave_dir.clone();
        let file = dir.join(format!("{}.autosave.jbp", sanitize(&project.name)));
        let tmp = file.with_extension("jbp.saving");
        let result = std::fs::create_dir_all(&dir)
            .map_err(anyhow::Error::from)
            .and_then(|_| Ok(jb_core::to_native(&project)?))
            .and_then(|json| Ok(std::fs::write(&tmp, json)?))
            .and_then(|_| Ok(std::fs::rename(&tmp, &file)?));
        match result {
            Ok(()) => {
                if let Ok(mut s) = app.state::<AppState>().0.lock() {
                    s.mark_autosaved(revision);
                }
                let _ = app.emit("autosaved", file.to_string_lossy());
            }
            Err(e) => eprintln!("autosave failed: {e:#}"),
        }
    }
}

/// Test automation (development only): with `JB_AUTOMATION_DIR` set, every
/// `*.js` file that appears in that directory is evaluated in the main
/// webview and then deleted. The frontend exposes its store as `window.__jb`;
/// scripts report back with `__jb.log(...)`, which prints to stderr.
fn automation_loop(app: AppHandle, dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> =
            entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "js")).collect();
        files.sort();
        for f in files {
            let Ok(js) = std::fs::read_to_string(&f) else { continue };
            let _ = std::fs::remove_file(&f);
            if let Some(w) = app.get_webview_window("main") {
                let _ =
                    w.eval(format!("(async () => {{ try {{ {js}\n }} catch (e) {{ __jb.log('error: ' + e) }} }})()"));
            }
        }
    }
}

#[tauri::command]
fn automation_log(text: String) {
    eprintln!("automation: {text}");
}

fn session_dirty(app: &AppHandle) -> bool {
    app.state::<AppState>().0.lock().map(|s| s.dirty()).unwrap_or(false)
}

// ---------- jb:// protocol ----------

fn query(uri: &tauri::http::Uri) -> std::collections::HashMap<String, String> {
    uri.query()
        .unwrap_or("")
        .split('&')
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((decode(k), decode(v)))
        })
        .collect()
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |j: usize| bytes.get(j).and_then(|c| (*c as char).to_digit(16));
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if hex(i + 1).is_some() && hex(i + 2).is_some() => {
                out.push((hex(i + 1).unwrap() * 16 + hex(i + 2).unwrap()) as u8);
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `JBLF` header (32 bytes, little endian) followed by RGBA pixels.
fn live_body(f: &playback::LiveFrame) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + f.rgba.len());
    out.extend_from_slice(b"JBLF");
    out.extend_from_slice(&f.width.to_le_bytes());
    out.extend_from_slice(&f.height.to_le_bytes());
    out.extend_from_slice(&(f.playing as u32).to_le_bytes());
    out.extend_from_slice(&f.seq.to_le_bytes());
    out.extend_from_slice(&f.time.to_le_bytes());
    out.extend_from_slice(&f.rgba[..(f.width * f.height * 4) as usize]);
    out
}

fn media_response(app: &AppHandle, svc: &Services, request: &tauri::http::Request<Vec<u8>>) -> Response<Vec<u8>> {
    let q = query(request.uri());
    let num = |k: &str, d: f64| q.get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
    let path = q.get("path").map(PathBuf::from);
    let engine = &svc.engine;
    // (body, mime, cacheable)
    let result: anyhow::Result<Option<(Vec<u8>, &str, bool)>> = match request.uri().path().trim_start_matches('/') {
        "thumb" => path
            .ok_or_else(|| anyhow::anyhow!("missing path"))
            .and_then(|p| engine.thumb(&p, num("t", 0.0), num("h", 90.0) as u32))
            .map(|b| Some((b, "image/jpeg", true))),
        "wave" => path
            .ok_or_else(|| anyhow::anyhow!("missing path"))
            .and_then(|p| engine.wave(&p, num("pps", 20.0) as u32))
            .map(|b| Some((b, "application/json", true))),
        "frame" => {
            let state = app.state::<AppState>();
            let project = state.0.lock().map(|s| s.history.project.clone());
            match project {
                Ok(p) => engine.frame(&p, num("t", 0.0), num("h", 360.0) as u32).map(|b| Some((b, "image/jpeg", true))),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        }
        "live" => {
            let after = num("after", 0.0) as u64;
            Ok(svc
                .playback
                .wait_frame(after, Duration::from_millis(250))
                .map(|f| (live_body(&f), "application/octet-stream", false)))
        }
        other => Err(anyhow::anyhow!("unknown route {other}")),
    };
    let builder = Response::builder().header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    match result {
        Ok(Some((body, mime, cacheable))) => builder
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, if cacheable { "max-age=31536000" } else { "no-store" })
            .body(body),
        Ok(None) => builder.status(StatusCode::NO_CONTENT).header(header::CACHE_CONTROL, "no-store").body(Vec::new()),
        Err(e) => builder.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into_bytes()),
    }
    .expect("valid response")
}

fn main() {
    if std::env::args().any(|a| a == "--fx-catalog") {
        println!("{}", serde_json::to_string_pretty(&media::catalog()).expect("catalog json"));
        return;
    }
    let session = Session::new(state::empty_project("Untitled", state::default_settings()), None);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState(Mutex::new(session)))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                let handle = app.handle().clone();
                app.set_menu(menu::build(&handle)?)?;
                app.on_menu_event(move |app, e| menu::forward(app, e.id().as_ref()));
            }
            let audio = audio_out::open();
            let config = jb_media::PoolConfig {
                sample_rate: audio.as_ref().map_or(48_000, |a| a.sample_rate),
                ..Default::default()
            };
            let pool = Arc::new(jb_media::MediaPool::new(config));
            let cache = Arc::new(jb_media::MediaCache::new(jb_media::MediaCache::default_dir())?);
            let engine = Arc::new(Engine::new(cache.clone(), pool.clone()));
            // Optional project to open at startup: `jibaru-editor path/to/project.jbp`.
            if let Some(p) = std::env::args().nth(1).filter(|a| !a.starts_with('-')) {
                match state::load(Path::new(&p)) {
                    Ok(loaded) => {
                        let state = app.state::<AppState>();
                        *state.0.lock().expect("session lock") = Session::from_loaded(loaded, PathBuf::from(p));
                    }
                    Err(e) => eprintln!("cannot open {p}: {e:#}"),
                }
            }
            let project = app.state::<AppState>().0.lock().expect("session lock").history.project.clone();
            let h = app.handle().clone();
            let playback = Playback::new(pool.clone(), audio, project, move |t| {
                let _ = h.emit("playback-ended", t);
            });
            let busy = playback.clone();
            let h = app.handle().clone();
            let proxies = Proxies::new(
                cache,
                pool,
                move || busy.is_playing(),
                move |s| {
                    let _ = h.emit("proxy-status", s);
                },
            );
            let data = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir().join("jibaru-editor"));
            app.manage(Services {
                engine,
                playback,
                proxies,
                exporter: export::Exporter::default(),
                force_close: AtomicBool::new(false),
                backup_dir: data.join("backups"),
                autosave_dir: data.join("autosave"),
            });
            {
                let state = app.state::<AppState>();
                let s = state.0.lock().expect("session lock");
                engine_sync(app.handle(), &s);
            }
            let h = app.handle().clone();
            std::thread::Builder::new().name("jb-autosave".into()).spawn(move || autosave_loop(h))?;
            if let Some(dir) = std::env::var_os("JB_AUTOMATION_DIR") {
                let h = app.handle().clone();
                std::thread::Builder::new()
                    .name("jb-automation".into())
                    .spawn(move || automation_loop(h, dir.into()))?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if session_dirty(app) && !app.state::<Services>().force_close.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = app.emit("close-requested", ());
                }
            }
        })
        .register_asynchronous_uri_scheme_protocol("jb", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || {
                let svc = app.state::<Services>();
                responder.respond(media_response(&app, &svc, &request));
            });
        })
        .invoke_handler(tauri::generate_handler![
            get_project,
            new_project,
            open_project,
            save_project,
            execute,
            undo,
            redo,
            probe_media,
            import_media,
            fx_catalog,
            playback_play,
            playback_pause,
            playback_seek,
            playback_size,
            playback_volume,
            playback_stats,
            playback_report,
            proxy_statuses,
            export_check,
            export_start,
            export_cancel,
            export_status,
            reveal_path,
            default_export_dir,
            normalize_clip,
            analyze_stabilization,
            save_snapshot,
            open_url,
            quit_app,
            autosave_dir,
            automation_log
        ])
        .build(tauri::generate_context!())
        .expect("error while building Jibaru Editor");
    app.run(|app, event| {
        // Quitting from the Dock or the system asks the frontend first when
        // there are unsaved changes; `quit_app` exits for real.
        if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
            if session_dirty(app) && !app.state::<Services>().force_close.load(Ordering::SeqCst) {
                api.prevent_exit();
                let _ = app.emit("close-requested", ());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64() {
        assert_eq!(base64_decode("aGVsbG8gd29ybGQ=").unwrap(), b"hello world");
        assert_eq!(base64_decode("data:image/png;base64,iVBORw==").unwrap(), b"\x89PNG");
        assert!(base64_decode("a$b").is_err());
    }

    #[test]
    fn expands_folders_to_media_files() {
        let dir = std::env::temp_dir().join(format!("jb-import-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for f in ["b.mp4", "a.PNG", "notes.txt", ".hidden.mp4", "sub/c.wav"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let got = expand_media_paths(vec![dir.to_string_lossy().into_owned(), "/x/file.mov".into()]);
        let names: Vec<String> =
            got.iter().map(|p| Path::new(p).file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["a.PNG", "b.mp4", "c.wav", "file.mov"]);
        assert_eq!(unique_file(&dir, "b", "mp4"), dir.join("b (2).mp4"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

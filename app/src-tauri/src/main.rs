// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod media;
mod state;

use jb_core::{Command, MediaAsset, ProjectSettings};
use media::Cache;
use state::{AppState, Session, Snapshot};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::http::{header, Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager, State};

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn with_session<T>(state: &State<AppState>, f: impl FnOnce(&mut Session) -> Res<T>) -> Res<T> {
    let mut s = state.0.lock().map_err(err)?;
    f(&mut s)
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
fn new_project(state: State<AppState>, name: String, settings: Option<serde_json::Value>) -> Res<Snapshot> {
    let mut base = serde_json::to_value(state::default_settings()).map_err(err)?;
    if let (Some(serde_json::Value::Object(over)), serde_json::Value::Object(b)) = (settings, &mut base) {
        b.extend(over);
    }
    let settings: ProjectSettings = serde_json::from_value(base).map_err(err)?;
    with_session(&state, |s| {
        *s = Session::new(state::empty_project(&name, settings), None);
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn open_project(state: State<AppState>, path: String) -> Res<Snapshot> {
    let project = state::load(Path::new(&path)).map_err(err)?;
    with_session(&state, |s| {
        *s = Session::new(project, Some(PathBuf::from(&path)));
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn save_project(state: State<AppState>, path: Option<String>) -> Res<Snapshot> {
    with_session(&state, |s| {
        s.save(path.as_deref().map(Path::new)).map_err(err)?;
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn execute(state: State<AppState>, command: Command) -> Res<Snapshot> {
    with_session(&state, |s| {
        s.execute(&command)?;
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn undo(state: State<AppState>) -> Res<Snapshot> {
    with_session(&state, |s| {
        s.undo();
        Ok(s.snapshot())
    })
}

#[tauri::command]
fn redo(state: State<AppState>) -> Res<Snapshot> {
    with_session(&state, |s| {
        s.redo();
        Ok(s.snapshot())
    })
}

#[tauri::command]
async fn probe_media(paths: Vec<String>) -> Res<Vec<MediaAsset>> {
    tauri::async_runtime::spawn_blocking(move || paths.iter().map(|p| media::probe(Path::new(p)).map_err(err)).collect())
        .await
        .map_err(err)?
}

#[tauri::command]
async fn import_media(app: AppHandle, paths: Vec<String>) -> Res<Snapshot> {
    let known: Vec<String> = {
        let state = app.state::<AppState>();
        let s = state.0.lock().map_err(err)?;
        s.history.project.media.iter().map(|m| m.path.clone()).collect()
    };
    let fresh: Vec<String> = paths.into_iter().filter(|p| !known.contains(p)).collect();
    let assets = probe_media(fresh).await?;
    let state = app.state::<AppState>();
    let snap = with_session(&state, |s| {
        let new: Vec<MediaAsset> =
            assets.into_iter().filter(|a| s.history.project.media(&a.id).is_none()).collect();
        if !new.is_empty() {
            s.execute(&state::add_media_command(new))?;
        }
        Ok(s.snapshot())
    })?;
    changed(&app, &snap);
    Ok(snap)
}

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

fn media_response(app: &AppHandle, cache: &Cache, request: &tauri::http::Request<Vec<u8>>) -> Response<Vec<u8>> {
    let q = query(request.uri());
    let num = |k: &str, d: f64| q.get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
    let path = q.get("path").map(PathBuf::from);
    let result: anyhow::Result<Option<(Vec<u8>, &str)>> = match request.uri().path().trim_start_matches('/') {
        "thumb" => path
            .ok_or_else(|| anyhow::anyhow!("missing path"))
            .and_then(|p| cache.thumb(&p, num("t", 0.0), num("h", 90.0) as u32))
            .map(|b| Some((b, "image/jpeg"))),
        "wave" => path
            .ok_or_else(|| anyhow::anyhow!("missing path"))
            .and_then(|p| cache.wave(&p, num("pps", 20.0) as u32))
            .map(|b| Some((b, "application/json"))),
        "frame" => {
            let state = app.state::<AppState>();
            let project = state.0.lock().map(|s| s.history.project.clone());
            match project {
                Ok(p) => media::render_frame(cache, &p, num("t", 0.0), num("h", 360.0) as u32)
                    .map(|o| o.map(|b| (b, "image/jpeg"))),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        }
        other => Err(anyhow::anyhow!("unknown route {other}")),
    };
    let builder = Response::builder().header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    match result {
        Ok(Some((body, mime))) => builder
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, "max-age=31536000")
            .body(body),
        Ok(None) => builder.status(StatusCode::NO_CONTENT).body(Vec::new()),
        Err(e) => builder.status(StatusCode::INTERNAL_SERVER_ERROR).body(e.to_string().into_bytes()),
    }
    .expect("valid response")
}

fn main() {
    let session = Session::new(state::empty_project("Untitled", state::default_settings()), None);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState(Mutex::new(session)))
        .setup(|app| {
            let dir = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir().join("jibaru-editor"));
            app.manage(Arc::new(Cache::new(dir.join("media"))));
            // Optional project to open at startup: `jibaru-editor path/to/project.jbp`.
            if let Some(p) = std::env::args().nth(1).filter(|a| !a.starts_with('-')) {
                if let Ok(project) = state::load(Path::new(&p)) {
                    let state = app.state::<AppState>();
                    *state.0.lock().expect("session lock") = Session::new(project, Some(PathBuf::from(p)));
                }
            }
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("jb", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || {
                let cache = app.state::<Arc<Cache>>().inner().clone();
                responder.respond(media_response(&app, &cache, &request));
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
            import_media
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jibaru Editor");
}

//! Native macOS menu bar. Items emit a `menu` event with their id; the
//! frontend runs the matching action (see app/src/actions.ts). Cut, Copy,
//! Paste and Select All are the system items so text fields keep working;
//! the webview handles them for timeline clips through DOM clipboard events.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, label: &str, accel: Option<&str>| MenuItem::with_id(app, id, label, true, accel);
    let app_menu = Submenu::with_items(
        app,
        "Jibaru Editor",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("About Jibaru Editor"), Some(AboutMetadata::default()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &item("app.quit", "Quit Jibaru Editor", Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item("file.new", "New Project", Some("CmdOrCtrl+N"))?,
            &item("file.open", "Open Project...", Some("CmdOrCtrl+O"))?,
            &PredefinedMenuItem::separator(app)?,
            &item("file.save", "Save Project", Some("CmdOrCtrl+S"))?,
            &item("file.save_as", "Save Project As...", Some("CmdOrCtrl+Shift+S"))?,
            &PredefinedMenuItem::separator(app)?,
            &item("file.import", "Import Media Files...", Some("CmdOrCtrl+I"))?,
            &PredefinedMenuItem::separator(app)?,
            &item("file.export", "Export...", Some("CmdOrCtrl+E"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &item("edit.undo", "Undo", Some("CmdOrCtrl+Z"))?,
            &item("edit.redo", "Redo", Some("CmdOrCtrl+Shift+Z"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &item("edit.duplicate", "Duplicate", Some("CmdOrCtrl+D"))?,
            &item("edit.delete", "Delete", None)?,
            &item("edit.ripple_delete", "Ripple Delete", None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &item("edit.split", "Split", Some("CmdOrCtrl+B"))?,
        ],
    )?;
    let tools = Submenu::with_items(
        app,
        "Tools",
        true,
        &[
            &item("tools.add_video_track", "Add Video Track", None)?,
            &item("tools.add_audio_track", "Add Audio Track", None)?,
            &PredefinedMenuItem::separator(app)?,
            &item("tools.snapping", "Snapping", None)?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &item("view.zoom_in", "Zoom In", Some("CmdOrCtrl+="))?,
            &item("view.zoom_out", "Zoom Out", Some("CmdOrCtrl+-"))?,
            &item("view.zoom_fit", "Zoom to Fit Timeline", Some("Shift+Z"))?,
            &PredefinedMenuItem::separator(app)?,
            &item("view.play", "Play/Pause", None)?,
            &item("view.stop", "Stop", Some("CmdOrCtrl+."))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;
    let help = Submenu::with_items(app, "Help", true, &[&item("help.shortcuts", "Keyboard Shortcuts", None)?])?;
    Menu::with_items(app, &[&app_menu, &file, &edit, &tools, &view, &help])
}

pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let _ = app.emit("menu", id.to_string());
}

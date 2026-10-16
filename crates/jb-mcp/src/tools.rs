//! Tool definitions and the transport-independent session that executes
//! them. The MCP layer (`server.rs`) only translates to and from rmcp types.

use jb_cli::error::code;
use jb_cli::integrations::{self, RenderOptions};
use jb_cli::summary::{self, ClipQuery, Detail};
use jb_cli::{commands, project_io, timecode, JbError, JbResult};
use jb_core::{History, Project, Ticks};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

/// An open project plus its undo history.
pub struct Session {
    pub path: Option<PathBuf>,
    pub history: History,
    /// True when there are changes not yet written by `save_project`.
    pub dirty: bool,
    /// Original Filmora documents when the project came from a `.wfp`,
    /// so saving back to `.wfp` is lossless.
    pub wfp: Option<jb_filmora::WfpProject>,
}

#[derive(Default)]
pub struct State {
    pub session: Option<Session>,
}

pub struct ToolDef {
    pub name: &'static str,
    pub title: &'static str,
    pub description: String,
    pub input_schema: Value,
    pub read_only: bool,
}

const TIME_HELP: &str = "Time: a number of seconds (e.g. 1.5) or a string: \"5\", \"1.5s\", \"00:01:02.500\", \"00:00:05:12\" (HH:MM:SS:FF), \"120f\" (frame), \"50000000t\" (ticks).";

fn time_schema(desc: &str) -> Value {
    json!({ "type": ["string", "number"], "description": format!("{desc} {TIME_HELP}") })
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false })
}

/// Input schema for `execute_command`, embedding the full Command schema.
fn execute_command_schema() -> Value {
    let mut cmd = jb_cli::command_schema();
    let defs = cmd.as_object_mut().and_then(|o| o.remove("$defs")).unwrap_or_else(|| json!({}));
    if let Some(o) = cmd.as_object_mut() {
        o.remove("$schema");
        o.remove("title");
        o.insert(
            "description".into(),
            json!("One Command object, tagged by `op`. Use op `batch` to apply several commands as one undo step."),
        );
    }
    let mut s = obj(
        json!({
            "command": cmd,
            "dry_run": { "type": "boolean", "default": false, "description": "Validate and report the effect without changing the session." },
        }),
        &["command"],
    );
    s["$defs"] = defs;
    s
}

pub fn tool_defs() -> Vec<ToolDef> {
    let d = |name, title, description: &str, input_schema, read_only| ToolDef {
        name,
        title,
        description: description.to_string(),
        input_schema,
        read_only,
    };
    vec![
        d("new_project", "New project",
          "Create a new empty native project (.jbp), save it to `path` and open it as the current session. Fails if the file exists unless `overwrite` is true. Next steps: import_media, then execute_command with add_track / add_clip.",
          obj(json!({
              "path": {"type": "string", "description": "Destination file, must end in .jbp."},
              "width": {"type": "integer", "minimum": 1, "default": 1920},
              "height": {"type": "integer", "minimum": 1, "default": 1080},
              "fps": {"type": ["string", "number"], "default": "30", "description": "Frame rate: 30, 29.97, \"30000/1001\"."},
              "name": {"type": "string", "description": "Project name; defaults to the file stem."},
              "overwrite": {"type": "boolean", "default": false},
              "discard_changes": {"type": "boolean", "default": false, "description": "Allow replacing a session that has unsaved changes."},
          }), &["path"]), false),
        d("open_project", "Open project",
          "Open a project file as the current session (replaces any open project). Supports native .jbp files and Filmora .wfp projects (the original Filmora documents are kept, so save_project can write the .wfp back losslessly; `import_warnings` lists anything that could not be mapped). Returns a summary with sequence and track ids; use get_project for clips. Refuses to drop unsaved changes unless `discard_changes` is true.",
          obj(json!({
              "path": {"type": "string", "description": "Path to a .jbp (or .wfp) file."},
              "discard_changes": {"type": "boolean", "default": false},
          }), &["path"]), false),
        d("save_project", "Save project",
          "Write the current session to disk: .jbp (native) or .wfp (only for projects opened from a .wfp; edits are patched into the original Filmora documents and `warnings` lists any edit Filmora cannot store). Without `path` saves to the file it was opened from; with `path` saves there and makes it the session's file (save as). Edits made with execute_command/import_media/undo/redo are in memory until saved.",
          obj(json!({ "path": {"type": "string", "description": "Optional new .jbp or .wfp path (save as)."} }), &[]), false),
        d("get_project", "Get project",
          "Describe the open project. detail=\"summary\": settings, duration, media, sequences and tracks (ids, kind, flags, clip counts). detail=\"clips\" (default): also every clip with id, name, source, start/end/duration (ticks, seconds, timecode, frame), effects and transition. detail=\"full\": also the raw project model (`project`, same shape as the Project schema at jb://schema/project). Times are ticks of 100 ns (10_000_000 per second).",
          obj(json!({ "detail": {"type": "string", "enum": ["summary", "clips", "full"], "default": "clips"} }), &[]), true),
        d("list_media", "List media",
          "List the media assets of the open project (id, name, path, kind video|audio|image, duration, size, frame rate, has_audio). Use a media `id` as `source.media_id` of an add_clip command.",
          obj(json!({}), &[]), true),
        d("import_media", "Import media",
          "Probe media files and add them to the project's media bin as one undo step. Returns each new asset with its `id`, kind and duration (ticks). Files already imported (same absolute path) are skipped and reported with their existing id. Does not place clips on the timeline: use execute_command add_clip with source {\"type\":\"media\",\"media_id\":<id>}.",
          obj(json!({ "paths": {"type": "array", "items": {"type": "string"}, "minItems": 1, "description": "Media file paths (absolute, or relative to the server's working directory)."} }), &["paths"]), false),
        d("execute_command", "Execute command",
          "Apply one edit Command to the open project; each call is one undo step and is atomic (on error nothing changes). The `command` schema is the exact Command type (also at resource jb://schema/command). Times are integer ticks (10_000_000 per second). Ids may be omitted for new objects (clip.id, clip.name, effect.id, transition.id, asset.id) and are generated. The result lists `created` / `removed` ids of tracks, clips, media and effects, so you learn the id of a new track or the right half of a split. Typical flow: add_track {kind:\"video\"} -> add_clip {track_id, clip:{source:{type:\"media\",media_id}, start, end, source_in}} -> split_clip / trim_clip / move_clip. Clips on one track may not overlap. Find ids with get_project or find_clips.",
          execute_command_schema(), false),
        d("undo", "Undo", "Undo the last edit (execute_command, import_media or redo) of the session. Returns the session status, including can_undo/can_redo.", obj(json!({}), &[]), false),
        d("redo", "Redo", "Redo the last undone edit. Returns the session status, including can_undo/can_redo.", obj(json!({}), &[]), false),
        d("evaluate_frame", "Evaluate frame",
          "Resolve what is visible and audible at one instant: returns `plan.layers` bottom to top (clip_id, content media/title/color/nested with source_time, resolved transform, opacity, blend mode, effects, active transition) and `audio` voices (clip_id, media_id, source_time, gain, pan). Use it to verify edits without rendering.",
          obj(json!({
              "at": time_schema("Timeline time to evaluate."),
              "sequence_id": {"type": "string", "description": "Sequence to evaluate; defaults to the main sequence."},
          }), &["at"]), true),
        d("find_clips", "Find clips",
          "Find clips matching all given criteria: `at` (clip contains this time), `from`/`to` (clip overlaps the range), `track_id` or `track_index` (0 = bottom track), `name` (case-insensitive substring), `media_id`, `sequence_id`. With no criteria returns every clip. Each result includes the clip id, track id/index and timing.",
          obj(json!({
              "at": time_schema("Clips under this time."),
              "from": time_schema("Range start (use with `to`)."),
              "to": time_schema("Range end (exclusive)."),
              "track_id": {"type": "string"},
              "track_index": {"type": "integer", "minimum": 0},
              "name": {"type": "string"},
              "media_id": {"type": "string"},
              "sequence_id": {"type": "string"},
          }), &[]), true),
        d("get_schema", "Get schema",
          "Return a JSON Schema: `command` (input of execute_command), `project` (the project model returned by get_project detail=full) or `frame_plan` (output of evaluate_frame). Same content as resources jb://schema/{command,project,frame_plan}.",
          obj(json!({ "which": {"type": "string", "enum": ["command", "project", "frame_plan"]} }), &["which"]), true),
        d("list_presets", "List presets",
          "Names and values accepted by the editing commands: speed ramp presets (set_speed_ramp), equalizer presets and bands (set_equalizer), normalization targets, blend modes, fit modes, animation kinds and placements (add_animation), keyframe parameter paths and interpolations (add_keyframe) and paste_attributes attributes.",
          obj(json!({}), &[]), true),
        d("normalize_audio", "Normalize audio",
          "Filmora's Auto Normalization: measures the integrated loudness of a clip's audio (on a camera video clip, its linked audio clip) and sets `normalize` so it plays at `target_lufs` (Filmora offers -23, -16, -14 and -12). One undo step; returns the measured loudness.",
          obj(json!({
              "clip_id": {"type": "string"},
              "target_lufs": {"type": "number", "default": -14.0, "maximum": 0, "minimum": -70},
          }), &["clip_id"]), false),
        d("render", "Render",
          "Encode the open project (main sequence by default) to a video file with GPU rendering and hardware encoding when available. Returns the encoder used, size, frame count, speed and any effects that have no renderer (`unknown_effects`). Use `preview` (e.g. \"540p\") for fast drafts.",
          obj(json!({
              "output": {"type": "string", "description": "Output video path, e.g. out.mp4."},
              "preview": {"type": "string", "description": "Preview height such as \"540p\"; omit for full resolution."},
              "from": time_schema("Optional range start."),
              "to": time_schema("Optional range end."),
              "sequence_id": {"type": "string"},
          }), &["output"]), false),
        d("preview_frame", "Preview frame",
          "Render a single frame at `at` of the main sequence to a PNG or JPEG file (by extension), e.g. to look at the result of an edit. `max_height` renders a smaller preview.",
          obj(json!({
              "at": time_schema("Timeline time."),
              "output": {"type": "string", "description": "Output image path (.png)."},
              "max_height": {"type": "integer", "minimum": 1},
          }), &["at", "output"]), false),
    ]
}

fn args<T: DeserializeOwned>(tool: &str, v: Value) -> JbResult<T> {
    serde_json::from_value(v).map_err(|e| {
        JbError::invalid(format!("invalid arguments for {tool}: {e}"))
            .hint(format!("see the inputSchema of `{tool}` in tools/list for the accepted fields"))
    })
}

/// A time argument: seconds as a number or any string accepted by `parse_time`.
#[derive(Deserialize)]
#[serde(untagged)]
enum TimeArg {
    Seconds(f64),
    Text(String),
}

impl TimeArg {
    fn ticks(&self, p: &Project) -> JbResult<Ticks> {
        match self {
            TimeArg::Seconds(s) if s.is_finite() && *s >= 0.0 => Ok(jb_core::time::seconds_to_ticks(*s)),
            TimeArg::Seconds(s) => Err(JbError::invalid(format!("invalid time {s}")).hint(TIME_HELP)),
            TimeArg::Text(t) => timecode::parse_time(t, p.settings.frame_rate),
        }
    }
}

fn opt_range(p: &Project, from: Option<TimeArg>, to: Option<TimeArg>) -> JbResult<Option<(Ticks, Ticks)>> {
    match (from, to) {
        (None, None) => Ok(None),
        (Some(a), Some(b)) => {
            let (a, b) = (a.ticks(p)?, b.ticks(p)?);
            if b <= a {
                return Err(JbError::invalid("`to` must be after `from`"));
            }
            Ok(Some((a, b)))
        }
        _ => Err(JbError::invalid("`from` and `to` must be given together")),
    }
}

impl State {
    fn session(&self) -> JbResult<&Session> {
        self.session.as_ref().ok_or_else(no_project)
    }

    fn session_mut(&mut self) -> JbResult<&mut Session> {
        self.session.as_mut().ok_or_else(no_project)
    }

    fn status(&self) -> Value {
        match &self.session {
            None => json!({ "open": false }),
            Some(s) => json!({
                "open": true,
                "path": s.path,
                "dirty": s.dirty,
                "can_undo": s.history.can_undo(),
                "can_redo": s.history.can_redo(),
                "name": s.history.project.name,
                "duration": timecode::time_json(s.history.project.main().duration(), s.history.project.settings.frame_rate),
            }),
        }
    }

    fn ensure_replaceable(&self, discard: bool) -> JbResult<()> {
        match &self.session {
            Some(s) if s.dirty && !discard => Err(JbError::invalid("the open project has unsaved changes")
                .hint("call save_project first, or pass discard_changes: true to drop them")),
            _ => Ok(()),
        }
    }

    /// Executes a tool. Errors are returned to the agent as tool results.
    pub fn call(&mut self, name: &str, a: Value) -> JbResult<Value> {
        let a = if a.is_null() { json!({}) } else { a };
        match name {
            "new_project" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    path: PathBuf,
                    width: Option<u32>,
                    height: Option<u32>,
                    fps: Option<Value>,
                    name: Option<String>,
                    #[serde(default)]
                    overwrite: bool,
                    #[serde(default)]
                    discard_changes: bool,
                }
                let a: A = args(name, a)?;
                self.ensure_replaceable(a.discard_changes)?;
                if a.path.exists() && !a.overwrite {
                    return Err(JbError::invalid(format!("`{}` already exists", a.path.display()))
                        .hint("pass overwrite: true, choose another path, or open it with open_project"));
                }
                let fps = match a.fps {
                    None => jb_core::Rational::new(30, 1),
                    Some(Value::String(s)) => project_io::parse_fps(&s)?,
                    Some(v) => project_io::parse_fps(&v.to_string())?,
                };
                let pname = a.name.unwrap_or_else(|| project_io::name_from_path(&a.path));
                let p = project_io::new_project(&pname, a.width.unwrap_or(1920), a.height.unwrap_or(1080), fps)?;
                project_io::save_project(&a.path, &p)?;
                self.session = Some(Session { path: Some(a.path), history: History::new(p), dirty: false, wfp: None });
                Ok(self.with_status(json!({ "project": self.summary(Detail::Summary)? })))
            }
            "open_project" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    path: PathBuf,
                    #[serde(default)]
                    discard_changes: bool,
                }
                let a: A = args(name, a)?;
                self.ensure_replaceable(a.discard_changes)?;
                let loaded = project_io::load_any(&a.path)?;
                let warnings = loaded.wfp.as_ref().map(|w| w.warnings.clone()).unwrap_or_default();
                self.session = Some(Session {
                    path: Some(a.path),
                    history: History::new(loaded.project),
                    dirty: false,
                    wfp: loaded.wfp,
                });
                let mut out = json!({ "project": self.summary(Detail::Summary)? });
                if !warnings.is_empty() {
                    out["import_warnings"] = json!(warnings);
                }
                Ok(self.with_status(out))
            }
            "save_project" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    path: Option<PathBuf>,
                }
                let a: A = args(name, a)?;
                let s = self.session_mut()?;
                let path = a.path.or_else(|| s.path.clone()).ok_or_else(|| {
                    JbError::invalid("this session has no file yet").hint("pass `path` ending in .jbp (or .wfp)")
                })?;
                let warnings = project_io::save_any(&path, &s.history.project, s.wfp.as_ref())?;
                s.path = Some(path.clone());
                s.dirty = false;
                Ok(self.with_status(json!({ "saved_to": path, "warnings": warnings })))
            }
            "get_project" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    detail: Option<String>,
                }
                let a: A = args(name, a)?;
                let detail = match a.detail.as_deref() {
                    None => Detail::Clips,
                    Some(d) => d.parse().map_err(JbError::invalid)?,
                };
                Ok(self.with_status(self.summary(detail)?))
            }
            "list_media" => {
                let _: Map<String, Value> = args(name, a)?;
                let p = &self.session()?.history.project;
                Ok(
                    json!({ "count": p.media.len(), "media": p.media.iter().map(summary::media_json).collect::<Vec<_>>() }),
                )
            }
            "import_media" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    paths: Vec<PathBuf>,
                }
                let a: A = args(name, a)?;
                let s = self.session_mut()?;
                let (cmd, report) = jb_cli::import_media(&s.history.project, &a.paths)?;
                if let Some(cmd) = cmd {
                    commands::execute_all(&mut s.history, &[cmd])?;
                    s.dirty = true;
                }
                Ok(self.with_status(report))
            }
            "execute_command" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    command: Value,
                    #[serde(default)]
                    dry_run: bool,
                }
                let a: A = args(name, a)?;
                // Accept a JSON-encoded string too; some clients stringify nested objects.
                let raw = match a.command {
                    Value::String(s) => serde_json::from_str(&s).map_err(|e| {
                        JbError::new(code::PARSE, format!("`command` is a string but not valid JSON: {e}"))
                            .hint("pass the Command as a JSON object, e.g. {\"op\":\"add_track\",\"kind\":\"video\"}")
                    })?,
                    v => v,
                };
                let cmd = commands::command_from_value(raw)?;
                let s = self.session_mut()?;
                // execute_all is atomic, so on error the session is untouched.
                let result = if a.dry_run {
                    commands::execute_all(&mut s.history.clone(), std::slice::from_ref(&cmd))
                } else {
                    commands::execute_all(&mut s.history, std::slice::from_ref(&cmd))
                };
                let mut report = result.map_err(|e| enrich(e, &s.history.project))?;
                if !a.dry_run {
                    s.dirty = true;
                }
                report["dry_run"] = json!(a.dry_run);
                Ok(self.with_status(report))
            }
            "undo" | "redo" => {
                let _: Map<String, Value> = args(name, a)?;
                let s = self.session_mut()?;
                let before = s.history.project.clone();
                let done = if name == "undo" { s.history.undo() } else { s.history.redo() };
                if !done {
                    let (c, m) = if name == "undo" {
                        (code::NOTHING_TO_UNDO, "nothing to undo")
                    } else {
                        (code::NOTHING_TO_REDO, "nothing to redo")
                    };
                    return Err(JbError::new(c, m).hint("check can_undo / can_redo in any session status"));
                }
                s.dirty = true;
                let report = commands::diff(&before, &s.history.project);
                Ok(self.with_status(report))
            }
            "evaluate_frame" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    at: TimeArg,
                    sequence_id: Option<String>,
                }
                let a: A = args(name, a)?;
                let p = &self.session()?.history.project;
                jb_cli::evaluate(p, a.sequence_id.as_deref(), a.at.ticks(p)?)
            }
            "find_clips" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    at: Option<TimeArg>,
                    from: Option<TimeArg>,
                    to: Option<TimeArg>,
                    track_id: Option<String>,
                    track_index: Option<usize>,
                    name: Option<String>,
                    media_id: Option<String>,
                    sequence_id: Option<String>,
                }
                let a: A = args(name, a)?;
                let p = &self.session()?.history.project;
                let q = ClipQuery {
                    at: a.at.map(|t| t.ticks(p)).transpose()?,
                    range: opt_range(p, a.from, a.to)?,
                    track_id: a.track_id,
                    track_index: a.track_index,
                    name: a.name,
                    media_id: a.media_id,
                    sequence_id: a.sequence_id,
                };
                let clips = summary::find_clips(p, &q);
                Ok(json!({ "count": clips.len(), "clips": clips }))
            }
            "get_schema" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    which: String,
                }
                let a: A = args(name, a)?;
                schema(&a.which)
            }
            "list_presets" => {
                let _: Map<String, Value> = args(name, a)?;
                Ok(jb_cli::presets())
            }
            "normalize_audio" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    clip_id: String,
                    target_lufs: Option<f64>,
                }
                let a: A = args(name, a)?;
                let s = self.session_mut()?;
                let target = a.target_lufs.unwrap_or(-14.0);
                let (cmd, measured) = integrations::normalize_command(&s.history.project, &a.clip_id, target)
                    .map_err(|e| enrich(e, &s.history.project))?;
                let mut report = commands::execute_all(&mut s.history, &[cmd])?;
                s.dirty = true;
                report["measured_lufs"] = json!(measured);
                report["target_lufs"] = json!(target);
                report["gain_db"] = json!(target - measured);
                Ok(self.with_status(report))
            }
            "render" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    output: PathBuf,
                    preview: Option<String>,
                    from: Option<TimeArg>,
                    to: Option<TimeArg>,
                    sequence_id: Option<String>,
                }
                let a: A = args(name, a)?;
                let p = &self.session()?.history.project;
                let opts = RenderOptions {
                    output: a.output,
                    preview_height: a.preview.map(|s| integrations::parse_preview(&s)).transpose()?,
                    range: opt_range(p, a.from, a.to)?,
                    sequence_id: a.sequence_id,
                };
                integrations::render(p, &opts)
            }
            "preview_frame" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct A {
                    at: TimeArg,
                    output: PathBuf,
                    max_height: Option<u32>,
                }
                let a: A = args(name, a)?;
                let p = &self.session()?.history.project;
                integrations::render_frame(p, a.at.ticks(p)?, &a.output, a.max_height)
            }
            other => Err(JbError::invalid(format!("unknown tool `{other}`")).hint(format!(
                "available tools: {}",
                tool_defs().iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
            ))),
        }
    }

    fn summary(&self, detail: Detail) -> JbResult<Value> {
        Ok(summary::project_summary(&self.session()?.history.project, detail))
    }

    fn with_status(&self, mut v: Value) -> Value {
        if let Some(o) = v.as_object_mut() {
            o.insert("session".into(), self.status());
        }
        v
    }

    /// Text of a `jb://` resource.
    pub fn read_resource(&self, uri: &str) -> JbResult<Value> {
        match uri {
            "jb://schema/command" => schema("command"),
            "jb://schema/project" => schema("project"),
            "jb://schema/frame_plan" => schema("frame_plan"),
            "jb://session" => Ok(self.status()),
            "jb://project" => self.summary(Detail::Clips),
            "jb://project/full" => Ok(serde_json::to_value(&self.session()?.history.project).unwrap_or_default()),
            _ => Err(JbError::new(code::NOT_FOUND, format!("unknown resource `{uri}`")).hint(format!(
                "available resources: {}",
                RESOURCES.iter().map(|r| r.0).collect::<Vec<_>>().join(", ")
            ))),
        }
    }
}

/// (uri, name, description)
pub const RESOURCES: &[(&str, &str, &str)] = &[
    ("jb://schema/command", "Command schema", "JSON Schema of the Command accepted by execute_command."),
    ("jb://schema/project", "Project schema", "JSON Schema of the native project model."),
    ("jb://schema/frame_plan", "FramePlan schema", "JSON Schema of evaluate_frame's `plan`."),
    ("jb://session", "Session status", "Open file, dirty flag, can_undo/can_redo and duration."),
    ("jb://project", "Project summary", "The open project with tracks and clips (same as get_project detail=clips)."),
    ("jb://project/full", "Project model", "The raw open project model, matching jb://schema/project."),
];

fn schema(which: &str) -> JbResult<Value> {
    match which {
        "command" => Ok(jb_cli::command_schema()),
        "project" => Ok(jb_cli::project_schema()),
        "frame_plan" => Ok(jb_cli::frame_plan_schema()),
        other => Err(JbError::invalid(format!("unknown schema `{other}`")).hint("use command, project or frame_plan")),
    }
}

fn no_project() -> JbError {
    JbError::new(code::NO_PROJECT, "no project is open")
        .hint("call open_project {\"path\": \"file.jbp\"} or new_project {\"path\": \"file.jbp\"} first")
}

/// Adds the valid ids to not-found errors so the agent can retry directly.
fn enrich(mut e: JbError, p: &Project) -> JbError {
    if e.code != code::NOT_FOUND {
        return e;
    }
    let list = |ids: Vec<String>| {
        let n = ids.len();
        let mut s = ids.into_iter().take(20).collect::<Vec<_>>().join(", ");
        if n > 20 {
            s += &format!(" … ({n} total, use find_clips)");
        }
        if s.is_empty() {
            "none".into()
        } else {
            s
        }
    };
    let tracks = p
        .sequences
        .iter()
        .flat_map(|s| s.tracks.iter().map(|t| format!("{} ({:?} \"{}\")", t.id, t.kind, t.name)))
        .collect();
    let clips =
        p.sequences.iter().flat_map(|s| s.tracks.iter().flat_map(|t| t.clips.iter().map(|c| c.id.clone()))).collect();
    let media = p.media.iter().map(|m| format!("{} ({})", m.id, m.name)).collect();
    let what = e.message.to_lowercase();
    let hint = if what.contains("track") {
        format!("valid track ids: {}", list(tracks))
    } else if what.contains("clip") {
        format!("valid clip ids: {}", list(clips))
    } else if what.contains("media") {
        format!("valid media ids: {}", list(media))
    } else if what.contains("sequence") {
        format!("valid sequence ids: {}", list(p.sequences.iter().map(|s| s.id.clone()).collect()))
    } else {
        "use get_project or find_clips to list valid ids".into()
    };
    e.hint = Some(hint);
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_flow() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jbp");
        let mut st = State::default();
        assert_eq!(st.call("get_project", json!({})).unwrap_err().code, code::NO_PROJECT);
        st.call("new_project", json!({"path": path, "fps": 25})).unwrap();
        let r = st.call("execute_command", json!({"command": {"op": "add_track", "kind": "video"}})).unwrap();
        let track = r["created"]["tracks"][0].as_str().unwrap().to_string();
        let r = st
            .call(
                "execute_command",
                json!({"command": {"op": "add_clip", "track_id": track, "clip": {"source": {"type": "color", "color": [0, 1, 0, 1]}, "start": 0, "end": 20_000_000}}}),
            )
            .unwrap();
        assert_eq!(r["session"]["dirty"], true);
        let found = st.call("find_clips", json!({"at": 1.0})).unwrap();
        assert_eq!(found["count"], 1);
        let plan = st.call("evaluate_frame", json!({"at": "00:00:01"})).unwrap();
        assert_eq!(plan["plan"]["layers"].as_array().unwrap().len(), 1);
        let err =
            st.call("execute_command", json!({"command": {"op": "remove_track", "track_id": "zzz"}})).unwrap_err();
        assert!(err.hint.unwrap().contains(&track));
        st.call("undo", json!({})).unwrap();
        assert_eq!(st.call("find_clips", json!({})).unwrap()["count"], 0);
        st.call("redo", json!({})).unwrap();
        st.call("save_project", json!({})).unwrap();
        let reopened = project_io::load_project(&path).unwrap();
        assert_eq!(reopened.main().tracks[0].clips.len(), 1);
        assert!(st.call("open_project", json!({"path": path})).is_ok());
        assert_eq!(st.call("redo", json!({})).unwrap_err().code, code::NOTHING_TO_REDO);
        assert_eq!(st.call("get_project", json!({"bogus": 1})).unwrap_err().code, code::INVALID_ARGUMENT);
    }

    #[test]
    fn wfp_sessions_save_back_to_wfp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.wfp");
        let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../jb-filmora/tests/fixtures/synthetic.wfp");
        std::fs::copy(fixture, &path).unwrap();
        let mut st = State::default();
        st.call("open_project", json!({"path": path})).unwrap();
        let clip =
            st.call("find_clips", json!({"name": "logo"})).unwrap()["clips"][0]["id"].as_str().unwrap().to_string();
        st.call(
            "execute_command",
            json!({"command": {"op": "set_blend_mode", "clip_id": clip, "blend_mode": "multiply"}}),
        )
        .unwrap();
        st.call(
            "execute_command",
            json!({"command": {"op": "add_marker", "marker": {"time": 10_000_000, "name": "x"}}}),
        )
        .unwrap();
        let saved = st.call("save_project", json!({})).unwrap();
        assert_eq!(saved["warnings"], json!([]));
        let again = jb_filmora::read(&path).unwrap();
        assert_eq!(again.project.clip(&clip).unwrap().blend_mode, jb_core::BlendMode::Multiply);
        assert_eq!(again.project.main().markers[0].name, "x");
        assert!(st.call("list_presets", json!({})).unwrap()["speed_ramps"].as_array().unwrap().len() >= 6);
    }

    #[test]
    fn dry_run_does_not_change_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = State::default();
        st.call("new_project", json!({"path": dir.path().join("b.jbp")})).unwrap();
        let r = st
            .call("execute_command", json!({"command": {"op": "add_track", "kind": "audio"}, "dry_run": true}))
            .unwrap();
        assert_eq!(r["created"]["tracks"].as_array().unwrap().len(), 1);
        assert_eq!(r["session"]["can_undo"], false);
        assert_eq!(r["session"]["dirty"], false);
    }
}

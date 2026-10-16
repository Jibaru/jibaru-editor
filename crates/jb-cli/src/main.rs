//! `jb`: the Jibaru Editor command-line tool. Every subcommand accepts
//! `--json`, which prints `{"ok":true,"data":...}` on success or
//! `{"ok":false,"error":{"code","message","hint"}}` on failure, and exits
//! non-zero on error (see `JbError::exit_code`).

use clap::{Parser, Subcommand};
use jb_cli::error::code;
use jb_cli::integrations::{self, RenderOptions};
use jb_cli::summary::{self, ClipQuery, Detail};
use jb_cli::{commands, project_io, timecode, JbError, JbResult};
use jb_core::History;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "jb",
    version,
    about = "Jibaru Editor command-line tool: create, inspect and edit .jbp video projects"
)]
struct Cli {
    /// Print machine-readable JSON (`{"ok":true,"data":...}` or `{"ok":false,"error":...}`).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new empty project.
    New {
        file: PathBuf,
        #[arg(long, default_value_t = 1920)]
        width: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
        /// Frame rate: `30`, `29.97` or `30000/1001`.
        #[arg(long, default_value = "30")]
        fps: String,
        /// Project name (defaults to the file stem).
        #[arg(long)]
        name: Option<String>,
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
        /// Validate and print the project without writing it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show tracks, clips, duration and media of a project.
    Info {
        project: PathBuf,
        /// summary (no clips), clips (default) or full (adds the raw model).
        #[arg(long, default_value = "clips")]
        detail: Detail,
    },
    /// Apply Command JSON to a project and save it.
    Exec {
        project: PathBuf,
        /// A Command JSON object or array. Omit when using --file.
        command: Option<String>,
        /// Read commands (JSON, array or JSON Lines) from a file, `-` for stdin.
        #[arg(long, short)]
        file: Option<PathBuf>,
        /// Save the result to this path instead of overwriting the project.
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Apply in memory and report, without saving.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print JSON Schemas of the Command, Project and FramePlan types.
    Schema {
        /// Which schema: command, project, frame-plan or all.
        #[arg(default_value = "all")]
        which: String,
    },
    /// Print the evaluated frame plan (layers and audio voices) at a time.
    Eval {
        project: PathBuf,
        /// Seconds, HH:MM:SS.mmm, HH:MM:SS:FF, <n>f or <n>t.
        #[arg(long)]
        at: String,
        /// Sequence id (defaults to the main sequence).
        #[arg(long)]
        sequence: Option<String>,
    },
    /// Find clips by time, track, name or media.
    FindClips {
        project: PathBuf,
        #[arg(long)]
        at: Option<String>,
        /// Overlapping range `start-end`.
        #[arg(long)]
        range: Option<String>,
        /// Track id or zero-based track index.
        #[arg(long)]
        track: Option<String>,
        /// Case-insensitive substring of the clip name.
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        media: Option<String>,
        #[arg(long)]
        sequence: Option<String>,
    },
    /// Probe media files and add them to the project's media bin.
    ImportMedia {
        project: PathBuf,
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Open a Filmora .wfp project and summarize it.
    Open {
        file: PathBuf,
        #[arg(long, default_value = "clips")]
        detail: Detail,
    },
    /// Convert a Filmora .wfp project into a native .jbp project.
    Convert {
        input: PathBuf,
        output: PathBuf,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Effect tools.
    Fx {
        #[command(subcommand)]
        cmd: FxCmd,
    },
    /// Render a single frame to a PNG or JPEG image.
    Frame {
        project: PathBuf,
        /// Timeline time, e.g. `00:01:02:15` or `62.5`.
        #[arg(long)]
        at: String,
        #[arg(long, short)]
        output: PathBuf,
        /// Preview resolution by height, e.g. `540p`.
        #[arg(long)]
        preview: Option<String>,
    },
    /// Render a project to a video file.
    Render {
        project: PathBuf,
        #[arg(long, short)]
        output: PathBuf,
        /// Preview resolution by height, e.g. `540p`.
        #[arg(long)]
        preview: Option<String>,
        /// Timeline range `start-end`, e.g. `5s-10s`.
        #[arg(long)]
        range: Option<String>,
        #[arg(long)]
        sequence: Option<String>,
    },
}

#[derive(Subcommand)]
enum FxCmd {
    /// Report which effects of a .wfp project are supported.
    Coverage { file: PathBuf },
}

/// Successful output: JSON data plus a human rendering.
struct Out {
    data: Value,
    text: String,
}

impl Out {
    fn new(data: Value, text: impl Into<String>) -> Self {
        Self { data, text: text.into() }
    }
    fn json(data: Value) -> Self {
        let text = serde_json::to_string_pretty(&data).unwrap_or_default();
        Self { data, text }
    }
}

fn main() {
    let wants_json = std::env::args().any(|a| a == "--json");
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            if wants_json
                && !matches!(e.kind(), clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion)
            {
                let err = JbError::new("usage_error", e.to_string().trim().to_string())
                    .hint("run `jb --help` or `jb <command> --help`");
                println!("{}", err.to_json());
                std::process::exit(2);
            }
            e.exit();
        }
    };
    match run(cli.cmd) {
        Ok(out) => {
            if cli.json {
                println!("{}", json!({ "ok": true, "data": out.data }));
            } else if !out.text.is_empty() {
                println!("{}", out.text.trim_end());
            }
        }
        Err(e) => {
            if cli.json {
                println!("{}", e.to_json());
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(e.exit_code());
        }
    }
}

fn ensure_writable(path: &Path, force: bool) -> JbResult<()> {
    if path.exists() && !force {
        return Err(
            JbError::invalid(format!("`{}` already exists", path.display())).hint("pass --force to overwrite it")
        );
    }
    project_io::detect_format(path).and_then(|f| {
        if f == project_io::Format::Native {
            Ok(())
        } else {
            Err(JbError::new(code::UNSUPPORTED_FORMAT, "output must be a .jbp file")
                .hint("choose a path ending in .jbp"))
        }
    })
}

fn read_input(file: &Path) -> JbResult<String> {
    if file == Path::new("-") {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).map_err(|e| JbError::new(code::IO, format!("read stdin: {e}")))?;
        return Ok(s);
    }
    std::fs::read_to_string(file).map_err(|e| {
        let c = if e.kind() == std::io::ErrorKind::NotFound { code::FILE_NOT_FOUND } else { code::IO };
        JbError::new(c, format!("cannot read `{}`: {e}", file.display()))
    })
}

fn run(cmd: Cmd) -> JbResult<Out> {
    match cmd {
        Cmd::New { file, width, height, fps, name, force, dry_run } => {
            ensure_writable(&file, force)?;
            let fps = project_io::parse_fps(&fps)?;
            let name = name.unwrap_or_else(|| project_io::name_from_path(&file));
            let p = project_io::new_project(&name, width, height, fps)?;
            if !dry_run {
                project_io::save_project(&file, &p)?;
            }
            let data =
                json!({ "path": file, "dry_run": dry_run, "project": summary::project_summary(&p, Detail::Summary) });
            let verb = if dry_run { "would create" } else { "created" };
            Ok(Out::new(data, format!("{verb} {} ({width}x{height} @ {:.3} fps)", file.display(), fps.as_f64())))
        }
        Cmd::Info { project, detail } => {
            let p = project_io::load_project(&project)?;
            Ok(Out::new(summary::project_summary(&p, detail), summary::project_text(&p)))
        }
        Cmd::Open { file, detail } => {
            let p = project_io::load_project(&file)?;
            Ok(Out::new(summary::project_summary(&p, detail), summary::project_text(&p)))
        }
        Cmd::Exec { project, command, file, output, dry_run } => {
            let text = match (command, file) {
                (Some(c), None) => c,
                (None, Some(f)) => read_input(&f)?,
                (Some(_), Some(_)) => {
                    return Err(JbError::invalid("pass either a command argument or --file, not both"))
                }
                (None, None) => {
                    return Err(JbError::invalid("no commands given")
                        .hint("pass Command JSON as an argument or use --file cmds.jsonl (`-` for stdin)"))
                }
            };
            let cmds = commands::parse_commands(&text)?;
            let mut h = History::new(project_io::load_project(&project)?);
            let mut report = commands::execute_all(&mut h, &cmds)?;
            let target = output.unwrap_or(project);
            if !dry_run {
                project_io::save_project(&target, &h.project)?;
            }
            report["dry_run"] = json!(dry_run);
            report["saved_to"] = if dry_run { Value::Null } else { json!(target) };
            report["duration"] = timecode::time_json(h.project.main().duration(), h.project.settings.frame_rate);
            let created = &report["created"];
            let mut text = format!(
                "applied {} command(s){}",
                cmds.len(),
                if dry_run { " (dry run, not saved)".to_string() } else { format!(", saved {}", target.display()) }
            );
            for kind in ["tracks", "clips", "media", "effects"] {
                if let Some(ids) = created[kind].as_array().filter(|a| !a.is_empty()) {
                    let ids: Vec<_> = ids.iter().filter_map(Value::as_str).collect();
                    text += &format!("\ncreated {kind}: {}", ids.join(", "));
                }
            }
            Ok(Out::new(report, text))
        }
        Cmd::Schema { which } => {
            let data = match which.as_str() {
                "command" => jb_cli::command_schema(),
                "project" => jb_cli::project_schema(),
                "frame-plan" | "frame_plan" => jb_cli::frame_plan_schema(),
                "all" => json!({
                    "command": jb_cli::command_schema(),
                    "project": jb_cli::project_schema(),
                    "frame_plan": jb_cli::frame_plan_schema(),
                }),
                other => {
                    return Err(JbError::invalid(format!("unknown schema `{other}`"))
                        .hint("use one of: command, project, frame-plan, all"))
                }
            };
            Ok(Out::json(data))
        }
        Cmd::Eval { project, at, sequence } => {
            let p = project_io::load_project(&project)?;
            let t = timecode::parse_time(&at, p.settings.frame_rate)?;
            Ok(Out::json(jb_cli::evaluate(&p, sequence.as_deref(), t)?))
        }
        Cmd::FindClips { project, at, range, track, name, media, sequence } => {
            let p = project_io::load_project(&project)?;
            let fps = p.settings.frame_rate;
            let mut q = ClipQuery { name, media_id: media, sequence_id: sequence, ..Default::default() };
            q.at = at.map(|a| timecode::parse_time(&a, fps)).transpose()?;
            q.range = range.map(|r| jb_cli::parse_range(&r, fps)).transpose()?;
            match track.map(|t| t.parse::<usize>().map_err(|_| t)) {
                Some(Ok(i)) => q.track_index = Some(i),
                Some(Err(id)) => q.track_id = Some(id),
                None => {}
            }
            let clips = summary::find_clips(&p, &q);
            let text = clips
                .iter()
                .map(|c| {
                    format!(
                        "{} track#{} {}..{} \"{}\"",
                        c["id"].as_str().unwrap_or_default(),
                        c["track_index"],
                        c["start"]["timecode"].as_str().unwrap_or_default(),
                        c["end"]["timecode"].as_str().unwrap_or_default(),
                        c["name"].as_str().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let text = if text.is_empty() { "no matching clips".to_string() } else { text };
            Ok(Out::new(json!({ "count": clips.len(), "clips": clips }), text))
        }
        Cmd::ImportMedia { project, files, dry_run } => {
            let mut h = History::new(project_io::load_project(&project)?);
            let (cmd, mut report) = jb_cli::import_media(&h.project, &files)?;
            if let Some(cmd) = cmd {
                commands::execute_all(&mut h, &[cmd])?;
                if !dry_run {
                    project_io::save_project(&project, &h.project)?;
                }
            }
            report["dry_run"] = json!(dry_run);
            let mut text = String::new();
            for m in report["added"].as_array().into_iter().flatten() {
                text += &format!(
                    "added {} {} {}\n",
                    m["id"].as_str().unwrap_or_default(),
                    m["kind"].as_str().unwrap_or_default(),
                    m["path"].as_str().unwrap_or_default()
                );
            }
            for m in report["skipped"].as_array().into_iter().flatten() {
                text += &format!(
                    "skipped {} (already imported as {})\n",
                    m["path"].as_str().unwrap_or_default(),
                    m["existing_id"].as_str().unwrap_or_default()
                );
            }
            if dry_run {
                text += "dry run: project not saved\n";
            }
            Ok(Out::new(report, text))
        }
        Cmd::Convert { input, output, force, dry_run } => {
            ensure_writable(&output, force)?;
            let p = project_io::load_project(&input)?;
            if !dry_run {
                project_io::save_project(&output, &p)?;
            }
            let data = json!({ "input": input, "output": output, "dry_run": dry_run, "project": summary::project_summary(&p, Detail::Summary) });
            Ok(Out::new(data, format!("converted {} -> {}", input.display(), output.display())))
        }
        Cmd::Fx { cmd: FxCmd::Coverage { file } } => Ok(Out::json(integrations::fx_coverage(&file)?)),
        Cmd::Frame { project, at, output, preview } => {
            let p = project_io::load_project(&project)?;
            let t = timecode::parse_time(&at, p.settings.frame_rate)?;
            let h = preview.map(|s| integrations::parse_preview(&s)).transpose()?;
            Ok(Out::json(integrations::render_frame(&p, t, &output, h)?))
        }
        Cmd::Render { project, output, preview, range, sequence } => {
            let p = project_io::load_project(&project)?;
            let opts = RenderOptions {
                output,
                preview_height: preview.map(|s| integrations::parse_preview(&s)).transpose()?,
                range: range.map(|r| jb_cli::parse_range(&r, p.settings.frame_rate)).transpose()?,
                sequence_id: sequence,
            };
            Ok(Out::json(integrations::render(&p, &opts)?))
        }
    }
}

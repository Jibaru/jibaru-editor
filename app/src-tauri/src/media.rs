//! Media services for the UI until `jb-media` lands: probing, thumbnails,
//! waveforms and preview frames, implemented by running the FFmpeg command
//! line tools. Served to the webview through the `jb://` protocol:
//!
//! - `jb://localhost/thumb?path=&t=&h=`  JPEG of a media file at `t` seconds
//! - `jb://localhost/wave?path=&pps=`    JSON `{ pps, peaks }`
//! - `jb://localhost/frame?t=&h=`        JPEG of the main sequence at `t` seconds
//!
//! The `frame` route is the seam for the native renderer: it evaluates the
//! timeline with `jb_core::eval` and currently extracts the top layer's frame
//! with FFmpeg; swapping in `jb-render` only changes `render_frame`.

use jb_core::eval::{evaluate, LayerContent};
use jb_core::{MediaAsset, MediaKind, Project, Rational, TICKS_PER_SECOND};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn find_tool(name: &str) -> PathBuf {
    let env_key = format!("JB_{}", name.to_uppercase());
    if let Ok(p) = std::env::var(&env_key) {
        return PathBuf::from(p);
    }
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    // GUI apps on macOS do not inherit the shell PATH, so also try the usual places.
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    if let Ok(cur) = std::env::current_exe() {
        if let Some(d) = cur.parent() {
            dirs.insert(0, d.to_path_buf());
        }
    }
    dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file()).unwrap_or_else(|| PathBuf::from(exe))
}

fn ffmpeg() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| find_tool("ffmpeg"))
}

fn ffprobe() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| find_tool("ffprobe"))
}

fn run(cmd: &mut Command) -> anyhow::Result<Vec<u8>> {
    let out = cmd.stdin(Stdio::null()).stderr(Stdio::piped()).output()?;
    if !out.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout)
}

fn is_image(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "heic" | "webp" | "bmp" | "gif" | "tif" | "tiff")
}

fn hash_of(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Stable media id derived from the path, so re-importing a file is a no-op.
pub fn media_id(path: &Path) -> String {
    hash_of(&path.to_string_lossy())
}

pub fn probe(path: &Path) -> anyhow::Result<MediaAsset> {
    let out = run(Command::new(ffprobe()).args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"]).arg(path))?;
    let info: serde_json::Value = serde_json::from_slice(&out)?;
    let streams = info["streams"].as_array().cloned().unwrap_or_default();
    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"].as_i64().unwrap_or(0) == 0);
    let audio = streams.iter().any(|s| s["codec_type"] == "audio");
    let image = is_image(path);
    let seconds: f64 = info["format"]["duration"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let frame_rate = video.and_then(|v| v["avg_frame_rate"].as_str()).and_then(|r| {
        let (n, d) = r.split_once('/')?;
        let (n, d): (i64, i64) = (n.parse().ok()?, d.parse().ok()?);
        (n > 0 && d > 0).then(|| Rational::new(n, d))
    });
    let kind = if image {
        MediaKind::Image
    } else if video.is_some() {
        MediaKind::Video
    } else if audio {
        MediaKind::Audio
    } else {
        anyhow::bail!("{} has no audio or video", path.display());
    };
    Ok(MediaAsset {
        id: media_id(path),
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        path: path.to_string_lossy().into_owned(),
        kind,
        duration: if image { 5 * TICKS_PER_SECOND } else { (seconds * TICKS_PER_SECOND as f64).round() as i64 },
        width: video.and_then(|v| v["width"].as_u64()).unwrap_or(0) as u32,
        height: video.and_then(|v| v["height"].as_u64()).unwrap_or(0) as u32,
        frame_rate: if image { None } else { frame_rate },
        has_audio: audio,
        extra: serde_json::Value::Null,
    })
}

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Self { dir }
    }

    fn get_or(&self, name: String, make: impl FnOnce() -> anyhow::Result<Vec<u8>>) -> anyhow::Result<Vec<u8>> {
        let file = self.dir.join(name);
        if let Ok(b) = std::fs::read(&file) {
            return Ok(b);
        }
        let bytes = make()?;
        let tmp = file.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        let _ = std::fs::rename(&tmp, &file);
        Ok(bytes)
    }

    fn stamp(path: &Path) -> u128 {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis())
    }

    pub fn thumb(&self, path: &Path, t: f64, h: u32) -> anyhow::Result<Vec<u8>> {
        let t = if is_image(path) { 0.0 } else { t.max(0.0) };
        let name = format!("{}.jpg", hash_of(&format!("thumb|{}|{}|{}|{h}", path.display(), Self::stamp(path), (t * 100.0) as i64)));
        self.get_or(name, || {
            let mut cmd = Command::new(ffmpeg());
            cmd.args(["-v", "error"]);
            if t > 0.0 {
                cmd.args(["-ss", &format!("{t:.3}")]);
            }
            cmd.arg("-i").arg(path).args([
                "-frames:v",
                "1",
                "-vf",
                &format!("scale=-2:{h}"),
                "-q:v",
                "5",
                "-f",
                "image2",
                "-c:v",
                "mjpeg",
                "pipe:1",
            ]);
            run(&mut cmd)
        })
    }

    pub fn wave(&self, path: &Path, pps: u32) -> anyhow::Result<Vec<u8>> {
        let name = format!("{}.json", hash_of(&format!("wave|{}|{}|{pps}", path.display(), Self::stamp(path))));
        self.get_or(name, || {
            const RATE: u32 = 8000;
            let pcm = run(Command::new(ffmpeg()).args(["-v", "error", "-i"]).arg(path).args([
                "-vn",
                "-ac",
                "1",
                "-ar",
                &RATE.to_string(),
                "-f",
                "s16le",
                "pipe:1",
            ]))?;
            let per = (RATE / pps.max(1)).max(1) as usize;
            let samples: Vec<i16> = pcm.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
            let peaks: Vec<f64> = samples
                .chunks(per)
                .map(|c| {
                    let m = c.iter().map(|s| (*s as i32).unsigned_abs()).max().unwrap_or(0);
                    (m as f64 / 32768.0 * 1000.0).round() / 1000.0
                })
                .collect();
            Ok(serde_json::to_vec(&serde_json::json!({ "pps": pps, "peaks": peaks }))?)
        })
    }
}

/// Renders the main sequence at `t` seconds. Until `jb-render` is wired in,
/// this shows the top-most visible media layer, unscaled and without effects.
pub fn render_frame(cache: &Cache, project: &Project, t: f64, h: u32) -> anyhow::Result<Option<Vec<u8>>> {
    let ticks = (t * TICKS_PER_SECOND as f64).round() as i64;
    let plan = evaluate(project, project.main(), ticks);
    for layer in plan.layers.iter().rev() {
        if let LayerContent::Media { media_id, source_time } = &layer.content {
            if let Some(m) = project.media(media_id) {
                if m.kind != MediaKind::Audio {
                    let st = *source_time as f64 / TICKS_PER_SECOND as f64;
                    return cache.thumb(Path::new(&m.path), st, h).map(Some);
                }
            }
        }
    }
    Ok(None)
}

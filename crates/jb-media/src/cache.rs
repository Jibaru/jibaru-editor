//! On-disk cache of thumbnails, waveform peaks and proxies for the UI.
//! Entries are keyed by the source path, size and modification time, so an
//! edited or replaced file never serves stale data.

use crate::convert::{convert_frame, Scaler};
use crate::probe::is_image_path;
use crate::util::fnv1a;
use crate::video::{DecodeMode, VideoDecoder};
use crate::waveform::Waveform;
use anyhow::{Context as _, Result};
use jb_core::frame::{ColorSpace, PixelFormat, VideoFrame};
use jb_core::time::{Ticks, TICKS_PER_SECOND};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub struct Thumbnail {
    /// JPEG file on disk.
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Source time the thumbnail shows.
    pub time: Ticks,
}

pub struct MediaCache {
    dir: PathBuf,
    hardware: bool,
    waveforms: Mutex<HashMap<String, Arc<Waveform>>>,
}

impl MediaCache {
    /// A cache rooted at `dir` (created if needed).
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        for sub in ["thumbs", "waveforms", "proxies"] {
            std::fs::create_dir_all(dir.join(sub)).with_context(|| format!("cannot create {}", dir.display()))?;
        }
        crate::init();
        Ok(Self { dir, hardware: true, waveforms: Mutex::new(HashMap::new()) })
    }

    /// The per-user cache directory: `~/Library/Caches/jibaru-editor` on
    /// macOS, `%LOCALAPPDATA%\jibaru-editor\cache` on Windows,
    /// `$XDG_CACHE_HOME/jibaru-editor` elsewhere.
    pub fn default_dir() -> PathBuf {
        let home = || std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        if cfg!(target_os = "macos") {
            home().join("Library/Caches/jibaru-editor")
        } else if cfg!(target_os = "windows") {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("jibaru-editor")
                .join("cache")
        } else {
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home().join(".cache"))
                .join("jibaru-editor")
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Disables hardware decoding for cache generation.
    pub fn set_hardware(&mut self, on: bool) {
        self.hardware = on;
    }

    /// Cache key of a source file: hash of path, size and mtime.
    pub fn key(path: &Path) -> Result<String> {
        let meta = std::fs::metadata(path).with_context(|| format!("cannot read {}", path.display()))?;
        let mtime =
            meta.modified().ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
        let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let mut bytes = canon.to_string_lossy().as_bytes().to_vec();
        bytes.extend_from_slice(&meta.len().to_le_bytes());
        bytes.extend_from_slice(&mtime.to_le_bytes());
        Ok(format!("{:016x}", fnv1a(&bytes)))
    }

    fn thumb_path(&self, key: &str, time: Ticks, height: u32) -> PathBuf {
        self.dir.join("thumbs").join(format!("{key}_{}_{height}.jpg", time / (TICKS_PER_SECOND / 1000)))
    }

    /// A thumbnail of `path` near source time `time` (keyframe-accurate, for
    /// speed), `height` pixels tall.
    pub fn thumbnail(&self, path: &Path, time: Ticks, height: u32) -> Result<Thumbnail> {
        Ok(self.filmstrip(path, &[time], height)?.remove(0))
    }

    /// Thumbnails at each of `times` (sorted for sequential decoding).
    pub fn filmstrip(&self, path: &Path, times: &[Ticks], height: u32) -> Result<Vec<Thumbnail>> {
        let key = Self::key(path)?;
        let height = height.max(2) & !1;
        let mut out: Vec<Option<Thumbnail>> = vec![None; times.len()];
        let mut todo: Vec<usize> = Vec::new();
        for (i, &t) in times.iter().enumerate() {
            let p = self.thumb_path(&key, t, height);
            match image::image_dimensions(&p) {
                Ok((w, h)) => out[i] = Some(Thumbnail { path: p, width: w, height: h, time: t }),
                Err(_) => todo.push(i),
            }
        }
        if !todo.is_empty() {
            todo.sort_by_key(|&i| times[i]);
            let mut scaler = Scaler::bilinear();
            let mut dec = if is_image_path(path) { None } else { Some(VideoDecoder::open(path, self.hardware)?) };
            let image = if dec.is_none() { Some(crate::image::load_image(path, None)?) } else { None };
            for i in todo {
                let t = times[i];
                let frame = match (&mut dec, &image) {
                    (Some(d), _) => d.frame_at(t, Some((height * 4, height)), DecodeMode::Keyframe)?,
                    (None, Some(img)) => Some(img.clone()),
                    _ => None,
                };
                let Some(frame) = frame else { continue };
                let thumb = self.write_thumb(&mut scaler, &frame, &self.thumb_path(&key, t, height), height, t)?;
                out[i] = Some(thumb);
            }
        }
        out.into_iter().map(|t| t.context("no frame for thumbnail")).collect()
    }

    fn write_thumb(&self, scaler: &mut Scaler, f: &VideoFrame, dst: &Path, height: u32, t: Ticks) -> Result<Thumbnail> {
        let width = (((f.width as f64 * height as f64 / f.height.max(1) as f64).round() as u32).max(2)) & !1;
        let rgba = convert_frame(scaler, f, width, height, PixelFormat::Rgba8, ColorSpace::Bt709, true)?;
        let img = image::RgbaImage::from_raw(width, height, rgba.data.to_vec()).context("bad thumbnail buffer")?;
        let rgb = image::DynamicImage::ImageRgba8(img).into_rgb8();
        let tmp = dst.with_extension("tmp.jpg");
        rgb.save_with_format(&tmp, image::ImageFormat::Jpeg)?;
        std::fs::rename(&tmp, dst)?;
        Ok(Thumbnail { path: dst.to_path_buf(), width, height, time: t })
    }

    /// Waveform peaks of `path`, computed once and cached in memory and on disk.
    pub fn waveform(&self, path: &Path) -> Result<Arc<Waveform>> {
        let key = Self::key(path)?;
        if let Some(w) = self.waveforms.lock().unwrap().get(&key) {
            return Ok(w.clone());
        }
        let file = self.dir.join("waveforms").join(format!("{key}.peaks"));
        let w = match Waveform::load(&file) {
            Ok(w) => w,
            Err(_) => {
                let w = Waveform::compute(path)?;
                w.save(&file)?;
                w
            }
        };
        let w = Arc::new(w);
        self.waveforms.lock().unwrap().insert(key, w.clone());
        Ok(w)
    }

    /// Where the proxy of `path` lives in this cache (may not exist yet).
    pub fn proxy_path(&self, path: &Path) -> Result<PathBuf> {
        Ok(self.dir.join("proxies").join(format!("{}.mp4", Self::key(path)?)))
    }

    /// The cached proxy of `path`, if it was generated already.
    pub fn existing_proxy(&self, path: &Path) -> Option<PathBuf> {
        self.proxy_path(path).ok().filter(|p| p.exists())
    }

    /// Generates the proxy of `path` if missing and returns its location.
    pub fn ensure_proxy(
        &self,
        path: &Path,
        opts: &crate::proxy::ProxyOptions,
        progress: &mut dyn FnMut(f32) -> bool,
    ) -> Result<PathBuf> {
        let dst = self.proxy_path(path)?;
        if !dst.exists() {
            crate::proxy::generate_proxy(path, &dst, opts, progress)?;
        }
        Ok(dst)
    }
}

/// Camera-made low-resolution proxy next to a source file, if any (GoPro
/// writes `GL*.LRV` beside `GX*.MP4`; DJI writes `.LRF`).
pub fn camera_proxy(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let dir = path.parent()?;
    let mut names = Vec::new();
    if let Some(rest) = stem.strip_prefix("GX").or_else(|| stem.strip_prefix("GH")) {
        names.push(format!("GL{rest}.LRV"));
    }
    names.push(format!("{stem}.LRV"));
    names.push(format!("{stem}.LRF"));
    names.into_iter().map(|n| dir.join(n)).find(|p| p.exists())
}

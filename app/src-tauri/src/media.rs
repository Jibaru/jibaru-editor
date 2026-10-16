//! Media services for the webview, served through the `jb://` protocol:
//!
//! - `jb://localhost/thumb?path=&t=&h=`  JPEG of a media file at `t` seconds (jb-media cache)
//! - `jb://localhost/wave?path=&pps=`    JSON `{ pps, peaks }` (jb-media waveform)
//! - `jb://localhost/frame?t=&h=`        JPEG of the main sequence at `t` seconds,
//!   composited by `jb-render` from frames decoded by `jb-media`.
//! - `jb://localhost/live?after=`        the latest preview frame (see `playback.rs`):
//!   a 32-byte header (`JBLF`, width, height, flags, seq, time) then raw RGBA.

use jb_core::eval::evaluate;
use jb_core::{MediaAsset, Project, TICKS_PER_SECOND};
use jb_media::{MediaCache, MediaPool};
use jb_render::{FxKind, Registry, Renderer};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

fn is_image_path(p: &Path) -> bool {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "heic" | "webp" | "gif" | "bmp" | "tif" | "tiff")
}

/// Stable media id derived from the path, so re-importing a file is a no-op.
pub fn media_id(path: &Path) -> String {
    let mut h = DefaultHasher::new();
    path.to_string_lossy().hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn probe(path: &Path) -> anyhow::Result<MediaAsset> {
    Ok(jb_media::probe_info(path)?.to_asset(media_id(path)))
}

pub struct Engine {
    pub cache: Arc<MediaCache>,
    pub pool: Arc<MediaPool>,
    /// Created lazily: GPU initialization is slow and may fail (no adapter).
    renderer: Mutex<Option<Renderer>>,
    /// Bounds concurrent thumbnail decodes (a filmstrip requests dozens at
    /// once and each 4K decoder holds hundreds of megabytes).
    thumb_slots: (Mutex<usize>, Condvar),
}

const THUMB_SLOTS: usize = 3;

fn secs_to_ticks(s: f64) -> i64 {
    (s * TICKS_PER_SECOND as f64).round() as i64
}

impl Engine {
    pub fn new(cache: Arc<MediaCache>, pool: Arc<MediaPool>) -> Self {
        Self { cache, pool, renderer: Mutex::new(None), thumb_slots: (Mutex::new(0), Condvar::new()) }
    }

    pub fn thumb(&self, path: &Path, t: f64, h: u32) -> anyhow::Result<Vec<u8>> {
        // Thumbnails come from the preview proxy when there is one: same
        // picture, a fraction of the decode cost.
        let src = if is_image_path(path) {
            None
        } else {
            jb_media::cache::camera_proxy(path).or_else(|| self.cache.existing_proxy(path))
        };
        let src = src.as_deref().unwrap_or(path);
        let (lock, cv) = &self.thumb_slots;
        {
            let mut n = lock.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            while *n >= THUMB_SLOTS {
                n = cv.wait(n).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            *n += 1;
        }
        let thumb = self.cache.thumbnail(src, secs_to_ticks(t.max(0.0)), h);
        *lock.lock().map_err(|e| anyhow::anyhow!("{e}"))? -= 1;
        cv.notify_one();
        Ok(std::fs::read(thumb?.path)?)
    }

    /// Peak envelope as `pps` absolute values per second, normalized 0..1.
    pub fn wave(&self, path: &Path, pps: u32) -> anyhow::Result<Vec<u8>> {
        let w = self.cache.waveform(path)?;
        let dur = w.duration();
        let count = ((dur as f64 / TICKS_PER_SECOND as f64) * pps as f64).ceil() as usize;
        let peaks: Vec<f32> = w
            .buckets(0, dur, count)
            .iter()
            .map(|[lo, hi]| ((lo.abs().max(hi.abs()) * 1000.0).round() / 1000.0).min(1.0))
            .collect();
        Ok(serde_json::to_vec(&serde_json::json!({ "pps": pps, "peaks": peaks }))?)
    }

    /// Renders the main sequence at `t` seconds, `h` pixels tall, as JPEG.
    pub fn frame(&self, project: &Project, t: f64, h: u32) -> anyhow::Result<Vec<u8>> {
        let s = &project.settings;
        let h = h.clamp(90, s.height.max(90)) & !1;
        let w = ((s.width as f64 * h as f64 / s.height.max(1) as f64).round() as u32).max(2) & !1;
        let mut guard = self.renderer.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        if guard.is_none() {
            *guard = Some(Renderer::new(w, h)?);
        }
        let r = guard.as_mut().expect("renderer");
        if r.size() != (w, h) {
            r.resize(w, h);
        }
        r.options.source_max_size = Some((w, h));
        self.pool.sync(project);
        let plan = evaluate(project, project.main(), secs_to_ticks(t));
        let frame = r.render(project, &plan, self.pool.as_ref());
        let row = frame.width as usize * 4;
        let data: Vec<u8> = if frame.stride == row {
            frame.data[..row * frame.height as usize].to_vec()
        } else {
            frame.data.chunks(frame.stride).take(frame.height as usize).flat_map(|r| r[..row].iter().copied()).collect()
        };
        let img = image::RgbaImage::from_raw(frame.width, frame.height, data)
            .ok_or_else(|| anyhow::anyhow!("bad frame buffer"))?;
        let rgb = image::DynamicImage::ImageRgba8(img).into_rgb8();
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&rgb)?;
        Ok(out)
    }
}

/// An effect, filter, transition or animation the renderer implements.
#[derive(Serialize)]
pub struct FxEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub kind: &'static str,
    pub description: &'static str,
    pub params: Vec<FxParam>,
    /// Library sidebar groups listing the item (Filmora's categories), with
    /// its position in each; empty for items not shown in the library.
    pub groups: Vec<FxGroup>,
}

#[derive(Serialize)]
pub struct FxGroup {
    pub group: &'static str,
    /// Position of the group in its tab's sidebar.
    pub index: usize,
    /// Position of the item in the group.
    pub order: usize,
}

#[derive(Serialize)]
pub struct FxParam {
    pub name: &'static str,
    pub label: &'static str,
    pub ty: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: [f32; 4],
    pub unit: &'static str,
}

pub fn catalog() -> Vec<FxEntry> {
    let reg = Registry::builtin();
    reg.iter()
        .map(|i| FxEntry {
            id: i.id,
            name: i.name,
            category: i.category,
            kind: match i.kind {
                FxKind::Transition => "transition",
                FxKind::Effect => "effect",
                FxKind::Filter => "filter",
                FxKind::Animation => "animation",
            },
            description: i.description,
            params: i
                .params
                .iter()
                .map(|p| FxParam {
                    name: p.name,
                    label: p.label,
                    ty: match p.ty {
                        jb_render::ParamType::Float => "float",
                        jb_render::ParamType::Int => "int",
                        jb_render::ParamType::Bool => "bool",
                        jb_render::ParamType::Color => "color",
                        jb_render::ParamType::Vec2 => "vec2",
                        jb_render::ParamType::Text => "text",
                    },
                    min: p.min,
                    max: p.max,
                    default: p.default,
                    unit: p.unit,
                })
                .collect(),
            groups: jb_render::library::places(i.id)
                .filter(|(g, _)| g.kind == i.kind)
                .map(|(g, order)| FxGroup {
                    group: g.name,
                    index: jb_render::library::group_names(g.kind).position(|n| n == g.name).unwrap_or(0),
                    order,
                })
                .collect(),
        })
        .collect()
}

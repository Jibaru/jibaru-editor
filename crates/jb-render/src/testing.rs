//! Test helpers: synthetic frames, an in-memory frame source and builders for
//! single-effect / single-transition frame plans. Used by the tests, the
//! `fx-sheet` example and the perf benchmark; handy for downstream crates too.

use jb_core::eval::{ActiveTransition, FramePlan, Layer, LayerContent, ResolvedEffect, ResolvedTransform};
use jb_core::frame::{ColorSpace, FrameSource, PixelFormat, VideoFrame};
use jb_core::param::Value;
use jb_core::{BlendMode, FitMode, MediaAsset, MediaKind, Project, ProjectSettings, Rational, Ticks};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i as i32 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

fn rgba_frame(w: u32, h: u32, f: impl Fn(f32, f32) -> [f32; 3]) -> VideoFrame {
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let c = f((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            let i = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                data[i + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            data[i + 3] = 255;
        }
    }
    VideoFrame {
        width: w,
        height: h,
        format: PixelFormat::Rgba8,
        color_space: ColorSpace::Bt709,
        full_range: true,
        stride: (w * 4) as usize,
        data: Arc::new(data),
        pts: 0,
    }
}

/// Clip "A": a cool-toned test card (hue sweep, grey ramp, grid, white disc).
pub fn pattern_a(w: u32, h: u32) -> VideoFrame {
    let aspect = w as f32 / h as f32;
    rgba_frame(w, h, |x, y| {
        let mut c = hsv(0.45 + 0.35 * x, 0.65, 0.35 + 0.55 * (1.0 - y));
        if y > 0.85 {
            let g = (x * 8.0).floor() / 7.0;
            c = [g, g, g];
        }
        let (gx, gy) = ((x * 16.0).fract(), (y * 9.0).fract());
        if gx < 0.04 || gy < 0.04 {
            c = c.map(|v| v * 0.5);
        }
        let d = (((x - 0.3) * aspect).powi(2) + (y - 0.42).powi(2)).sqrt();
        if d < 0.18 {
            c = [0.95, 0.95, 0.92];
        }
        if d < 0.1 {
            c = [0.1, 0.12, 0.2];
        }
        c
    })
}

/// Clip "B": a warm-toned card (diagonal stripes, red/yellow, dark square).
pub fn pattern_b(w: u32, h: u32) -> VideoFrame {
    let aspect = w as f32 / h as f32;
    rgba_frame(w, h, |x, y| {
        let s = ((x * aspect + y) * 6.0).fract() < 0.5;
        let mut c = if s { [0.92, 0.45, 0.15] } else { [0.98, 0.82, 0.3] };
        let dx = (x - 0.7) * aspect;
        let dy = y - 0.5;
        if dx.abs() < 0.2 && dy.abs() < 0.2 {
            c = [0.25, 0.08, 0.12];
        }
        if (dx * dx + dy * dy).sqrt() < 0.08 {
            c = [0.2, 0.75, 0.95];
        }
        c
    })
}

/// Converts an RGBA8 frame to NV12 (BT.709, limited or full range).
pub fn to_nv12(f: &VideoFrame, full_range: bool) -> VideoFrame {
    let (w, h) = (f.width as usize, f.height as usize);
    let (kr, kb) = (0.2126f32, 0.0722f32);
    let kg = 1.0 - kr - kb;
    let px = |x: usize, y: usize| {
        let i = y * f.stride + x * 4;
        [f.data[i] as f32 / 255.0, f.data[i + 1] as f32 / 255.0, f.data[i + 2] as f32 / 255.0]
    };
    let enc = |v: f32, lo: f32, span: f32| (lo + v * span).round().clamp(0.0, 255.0) as u8;
    let (ylo, yspan, clo, cspan) = if full_range { (0.0, 255.0, 128.0, 255.0) } else { (16.0, 219.0, 128.0, 224.0) };
    let cw = w.div_ceil(2);
    let chh = h.div_ceil(2);
    let stride = cw * 2;
    let stride = stride.max(w);
    let mut data = vec![0u8; stride * h + stride * chh];
    for y in 0..h {
        for x in 0..w {
            let c = px(x, y);
            let yy = kr * c[0] + kg * c[1] + kb * c[2];
            data[y * stride + x] = enc(yy, ylo, yspan);
        }
    }
    for cy in 0..chh {
        for cx in 0..cw {
            let mut acc = [0f32; 3];
            let mut n = 0.0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (x, y) = ((cx * 2 + dx).min(w - 1), (cy * 2 + dy).min(h - 1));
                let c = px(x, y);
                for k in 0..3 {
                    acc[k] += c[k];
                }
                n += 1.0;
            }
            let c = acc.map(|v| v / n);
            let yy = kr * c[0] + kg * c[1] + kb * c[2];
            let cb = (c[2] - yy) / (2.0 * (1.0 - kb));
            let cr = (c[0] - yy) / (2.0 * (1.0 - kr));
            let base = stride * h + cy * stride + cx * 2;
            data[base] = enc(cb, clo, cspan);
            data[base + 1] = enc(cr, clo, cspan);
        }
    }
    VideoFrame {
        width: f.width,
        height: f.height,
        format: PixelFormat::Nv12,
        color_space: ColorSpace::Bt709,
        full_range,
        stride,
        data: Arc::new(data),
        pts: f.pts,
    }
}

/// Frame source serving fixed frames by media id.
#[derive(Default, Clone)]
pub struct MemorySource {
    pub frames: HashMap<String, VideoFrame>,
}

impl MemorySource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, id: &str, f: VideoFrame) -> Self {
        self.frames.insert(id.to_string(), f);
        self
    }

    /// Source with `pattern_a` as "a" and `pattern_b` as "b".
    pub fn ab(w: u32, h: u32) -> Self {
        Self::new().with("a", pattern_a(w, h)).with("b", pattern_b(w, h))
    }
}

impl FrameSource for MemorySource {
    fn frame(&self, media_id: &str, _t: Ticks, _max: Option<(u32, u32)>) -> anyhow::Result<Option<VideoFrame>> {
        Ok(self.frames.get(media_id).cloned())
    }
}

/// A project of the given size with video media "a" and "b".
pub fn project(w: u32, h: u32) -> Project {
    let mut p = Project::new(
        "test",
        ProjectSettings { width: w, height: h, frame_rate: Rational::new(30, 1), sample_rate: 48000, background: [0.0, 0.0, 0.0, 1.0] },
    );
    for id in ["a", "b"] {
        p.media.push(MediaAsset {
            id: id.into(),
            name: id.into(),
            path: String::new(),
            kind: MediaKind::Video,
            duration: 10 * jb_core::TICKS_PER_SECOND,
            width: w,
            height: h,
            frame_rate: None,
            has_audio: false,
            extra: jb_core::Extra::Null,
        });
    }
    p
}

pub fn identity_transform() -> ResolvedTransform {
    ResolvedTransform {
        position: [0.0, 0.0],
        scale: [1.0, 1.0],
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        crop: [0.0; 4],
        fit: FitMode::Fit,
    }
}

/// A full-frame layer of media `id`.
pub fn media_layer(id: &str) -> Layer {
    Layer {
        clip_id: format!("clip-{id}"),
        content: LayerContent::Media { media_id: id.into(), source_time: 0 },
        local_time: 0,
        transform: identity_transform(),
        opacity: 1.0,
        blend_mode: BlendMode::Normal,
        effects: Vec::new(),
        transition: None,
    }
}

pub fn effect(kind: &str, progress: f64, params: &[(&str, Value)]) -> ResolvedEffect {
    ResolvedEffect {
        kind: kind.into(),
        params: params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        progress,
    }
}

/// Plan with media "a" and one effect applied (local time follows progress over 2 s).
pub fn effect_plan(kind: &str, progress: f64, params: &[(&str, Value)]) -> FramePlan {
    let mut l = media_layer("a");
    l.local_time = (progress * 2.0 * jb_core::TICKS_PER_SECOND as f64) as Ticks;
    l.effects.push(effect(kind, progress, params));
    FramePlan { time: l.local_time, layers: vec![l] }
}

/// Plan with a transition from "a" to "b".
pub fn transition_plan(kind: &str, progress: f64, params: &[(&str, Value)]) -> FramePlan {
    let mut l = media_layer("a");
    l.transition = Some(Box::new(ActiveTransition {
        kind: kind.into(),
        progress,
        params: params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect::<BTreeMap<_, _>>(),
        incoming: media_layer("b"),
    }));
    FramePlan { time: 0, layers: vec![l] }
}

/// Mean absolute difference per channel (0..255) between two RGBA8 frames.
pub fn mean_abs_diff(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).unsigned_abs() as u64).sum::<u64>() as f64 / a.len() as f64
}

/// RGBA of pixel (x, y) of an RGBA8 frame.
pub fn pixel(f: &VideoFrame, x: u32, y: u32) -> [u8; 4] {
    let i = y as usize * f.stride + x as usize * 4;
    [f.data[i], f.data[i + 1], f.data[i + 2], f.data[i + 3]]
}

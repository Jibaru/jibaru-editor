//! Library previews: procedural sample footage, a small preview project per
//! library item, and the card thumbnails / hover animations rendered from it.
//!
//! Like Filmora, every Transitions/Effects/Filters/Animation/Titles card
//! shows a representative still and plays a short looping preview on hover.
//! The footage is our own: two animated procedural scenes ("A", a cool
//! mountain lake, and "B", a warm sunset sea) drawn on the CPU by
//! [`SampleSource`], so nothing has to be downloaded or shipped.
//!
//! A card preview is one JPEG still plus one JPEG sprite sheet holding
//! [`CARD_LAYOUT`]`.frames` frames in a grid, which the UI steps through on
//! hover (instant start, no video decoder, any webview). [`PreviewCache`]
//! keeps them on disk under a directory named after [`PREVIEW_VERSION`],
//! which changes with every change to the renderer sources.

use crate::registry::{FxKind, Registry};
use crate::Renderer;
use jb_core::frame::{ColorSpace, FrameSource, PixelFormat, VideoFrame};
use jb_core::model::{AnimationPlacement, Clip, ClipSource, EffectInstance, Sequence, Track, TrackKind, Transition};
use jb_core::param::Param;
use jb_core::{MediaAsset, MediaKind, Project, ProjectSettings, Rational, Ticks, TitleData, TICKS_PER_SECOND};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Changes whenever the renderer (or this generator) changes: cached
/// previews from other versions are stale.
pub const PREVIEW_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("JB_RENDER_FINGERPRINT"));

/// Media id of sample scene "A" (cool mountain lake, pans right).
pub const SAMPLE_A: &str = "jb-sample-a";
/// Media id of sample scene "B" (warm sunset sea, pans left).
pub const SAMPLE_B: &str = "jb-sample-b";

/// Length of a sample-footage preview (Filmora's transition previews are 2 s).
pub const PREVIEW_DURATION: Ticks = 2 * TICKS_PER_SECOND;
/// Length of an effect/filter preview on the user's footage (Filmora: 5 s).
pub const FOOTAGE_PREVIEW_DURATION: Ticks = 5 * TICKS_PER_SECOND;

pub fn is_sample(media_id: &str) -> bool {
    media_id == SAMPLE_A || media_id == SAMPLE_B
}

/// Layout of a sprite sheet: `frames` frames of `frame_w`x`frame_h`,
/// `cols` per row, played at `fps`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct SpriteLayout {
    pub frame_w: u32,
    pub frame_h: u32,
    pub frames: u32,
    pub cols: u32,
    pub fps: f32,
}

impl SpriteLayout {
    pub fn rows(&self) -> u32 {
        self.frames.div_ceil(self.cols)
    }
    pub fn sheet_size(&self) -> (u32, u32) {
        (self.frame_w * self.cols, self.frame_h * self.rows())
    }
}

/// Library cards: 2 s at 15 fps, sized for a 128-point card on a 2x display.
pub const CARD_LAYOUT: SpriteLayout = SpriteLayout { frame_w: 256, frame_h: 144, frames: 30, cols: 6, fps: 15.0 };

const JPEG_QUALITY: u8 = 70;

// ---------- Procedural sample footage ----------

/// Panorama width in frame widths: the camera pans across it.
const PANO_W: f32 = 1.45;
/// Sample footage is frozen after this many seconds (the pans end).
const MAX_SECONDS: f32 = 10.0;

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn scale(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// Smooth 1D noise in about -1..1 (sum of detuned sines).
fn fbm(x: f32, seed: f32) -> f32 {
    let (mut v, mut a, mut f) = (0.0, 0.55, 1.0);
    for i in 0..4 {
        let fi = i as f32;
        v += a * (0.6 * (x * f + seed * 1.7 + fi * 3.1).sin() + 0.4 * (x * f * 2.31 + seed * 0.9 + fi).sin());
        a *= 0.5;
        f *= 2.03;
    }
    v
}

/// A precomputed still panorama: RGB plus a mask byte (scene specific).
struct Pano {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

impl Pano {
    fn build(w: usize, h: usize, f: impl Fn(f32, f32) -> ([f32; 3], f32) + Sync) -> Self {
        let mut px = vec![[0u8; 4]; w * h];
        par_rows(&mut px, w, |y, row| {
            let v = (y as f32 + 0.5) / h as f32;
            for (x, p) in row.iter_mut().enumerate() {
                let u = (x as f32 + 0.5) / w as f32 * PANO_W;
                let (c, m) = f(u, v);
                *p = [to8(c[0]), to8(c[1]), to8(c[2]), to8(m)];
            }
        });
        Self { w, h, px }
    }

    /// Bilinear-in-x sample at panorama pixel column `x` (fractional), row `y`.
    fn at(&self, x: f32, y: usize) -> ([f32; 3], f32) {
        let x = x.clamp(0.0, (self.w - 1) as f32);
        let x0 = x.floor() as usize;
        let x1 = (x0 + 1).min(self.w - 1);
        let k = x - x0 as f32;
        let y = y.min(self.h - 1);
        let (a, b) = (self.px[y * self.w + x0], self.px[y * self.w + x1]);
        let ch = |i: usize| (a[i] as f32 * (1.0 - k) + b[i] as f32 * k) / 255.0;
        ([ch(0), ch(1), ch(2)], ch(3))
    }
}

fn to8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Runs `f(y, row)` over the rows of `buf` on all cores.
fn par_rows<T: Send>(buf: &mut [T], w: usize, f: impl Fn(usize, &mut [T]) + Sync) {
    let rows = buf.len() / w.max(1);
    let n = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16).min(rows.max(1));
    let per = rows.div_ceil(n);
    std::thread::scope(|s| {
        for (i, chunk) in buf.chunks_mut(per * w).enumerate() {
            let f = &f;
            s.spawn(move || {
                for (j, row) in chunk.chunks_mut(w).enumerate() {
                    f(i * per + j, row);
                }
            });
        }
    });
}

const A_HORIZON: f32 = 0.64;

/// Scene A above the horizon: sky, sun and three mountain ridges. The mask
/// is 1 for sky.
fn scene_a_land(u: f32, v: f32, aspect: f32) -> ([f32; 3], f32) {
    let hz = A_HORIZON;
    let mut c = mix([0.17, 0.37, 0.68], [0.8, 0.9, 0.96], (v / hz).clamp(0.0, 1.0).powf(1.4));
    let d = (((u - 0.98) * aspect).powi(2) + (v - 0.17).powi(2)).sqrt();
    c = mix(c, [1.0, 0.96, 0.84], (0.5 * (-d * 7.0).exp()).min(1.0));
    if d < 0.045 {
        c = [1.0, 0.98, 0.92];
    }
    let mut sky = 1.0;
    // Snowy peaks: the skyline is the highest of a few jagged cones.
    let peaks = |u: f32| {
        let mut m = 0.0f32;
        for (c, w, k) in [(0.3, 0.32, 1.0), (0.78, 0.26, 0.8), (1.2, 0.34, 0.95), (1.55, 0.3, 0.7)] {
            m = m.max(k * (1.0 - ((u - c) / w).abs()).max(0.0).powf(1.3));
        }
        m + 0.025 * fbm(u * 14.0, 1.0) + 0.012 * fbm(u * 37.0, 7.0)
    };
    let p = peaks(u);
    let h1 = 0.5 - 0.33 * p;
    if v > h1 {
        sky = 0.0;
        // Light from the right (the sun): faces rising to the right are in shade.
        let lit = peaks(u + 0.004) > p;
        let base = if lit { [0.6, 0.68, 0.82] } else { [0.36, 0.45, 0.62] };
        let depth = ((v - h1) / 0.25).min(1.0);
        c = mix(base, [0.42, 0.52, 0.68], depth);
        let snow_line = 0.33 + 0.02 * fbm(u * 23.0, 4.0);
        if v < snow_line && v < h1 + 0.09 {
            c = if lit { [0.96, 0.97, 0.99] } else { [0.72, 0.78, 0.88] };
        }
    }
    let h2 = 0.5 + 0.04 * fbm(u * 4.0, 2.5);
    if v > h2 {
        sky = 0.0;
        c = mix([0.26, 0.4, 0.5], [0.18, 0.3, 0.36], ((v - h2) / 0.1).min(1.0));
    }
    // Forest along the shore, with soft tree tips.
    let tri = ((u * 55.0).fract() - 0.5).abs() * 2.0;
    let h3 = 0.575 + 0.02 * fbm(u * 6.5, 5.0) + 0.02 * tri * tri;
    if v > h3 {
        sky = 0.0;
        c = mix([0.12, 0.25, 0.18], [0.06, 0.14, 0.1], ((v - h3) / 0.06).min(1.0));
    }
    (c, sky)
}

/// Scene A panorama: land above the horizon, its reflection in the lake below.
/// Mask: 1 = sky, 0.5 = reflected sky, 0 = anything else.
fn scene_a(u: f32, v: f32, aspect: f32) -> ([f32; 3], f32) {
    let hz = A_HORIZON;
    if v < hz {
        return scene_a_land(u, v, aspect);
    }
    let rv = hz - (v - hz) * 1.15;
    let (c, sky) = scene_a_land(u, rv.max(0.0), aspect);
    let depth = (v - hz) / (1.0 - hz);
    let c = mix(scale(c, 0.68), [0.05, 0.16, 0.24], 0.25 + depth * 0.45);
    (c, sky * 0.5)
}

const B_HORIZON: f32 = 0.6;

fn palm(u: f32, v: f32, aspect: f32, base_u: f32, base_v: f32, height: f32) -> bool {
    // Leaning trunk.
    let top_v = base_v - height;
    if v > top_v && v < base_v {
        let s = (base_v - v) / height;
        let xc = base_u + 0.05 * height * 2.5 * s * s;
        if ((u - xc) * aspect).abs() < 0.012 * height * 2.5 * (1.2 - 0.5 * s) {
            return true;
        }
    }
    // Drooping fronds from the top.
    let top_u = base_u + 0.05 * height * 2.5;
    let len = height * 0.55;
    if ((u - top_u) * aspect).abs() > len * 1.2 || (v - top_v).abs() > len {
        return false;
    }
    for i in 0..7 {
        let th = -2.9 + i as f32 * 0.95;
        let (dx, dy) = (th.cos(), th.sin() * 0.55);
        for k in 1..=28 {
            let s = k as f32 / 28.0;
            let px = top_u * aspect + dx * len * s;
            let py = top_v + dy * len * s + 0.55 * len * s * s;
            let r = 0.016 * height * 2.5 * (1.0 - 0.75 * s) + 0.003;
            if (u * aspect - px).powi(2) + (v - py).powi(2) < r * r {
                return true;
            }
        }
    }
    false
}

/// Scene B panorama: sunset sky, sun on the horizon, sea, island
/// silhouettes and palms. Mask: strength of the sun glint on the water.
fn scene_b(u: f32, v: f32, aspect: f32) -> ([f32; 3], f32) {
    let hz = B_HORIZON;
    let sun_u = 0.78;
    let mut glint = 0.0;
    let mut c = if v < hz {
        let t = v / hz;
        let c = if t < 0.55 {
            mix([0.22, 0.1, 0.38], [0.86, 0.32, 0.45], t / 0.55)
        } else {
            mix([0.86, 0.32, 0.45], [1.0, 0.74, 0.38], (t - 0.55) / 0.45)
        };
        let d = (((u - sun_u) * aspect).powi(2) + (v - (hz - 0.03)).powi(2)).sqrt();
        let c = mix(c, [1.0, 0.86, 0.55], (0.6 * (-d * 5.0).exp()).min(1.0));
        if d < 0.11 {
            mix([1.0, 0.95, 0.7], [1.0, 0.8, 0.45], (v - (hz - 0.14)) / 0.14)
        } else {
            c
        }
    } else {
        let t = ((v - hz) / (1.0 - hz)).powf(0.7);
        let w = 0.04 + (v - hz) * 0.5;
        glint = smoothstep(w, 0.0, (u - sun_u).abs());
        mix([0.6, 0.28, 0.4], [0.12, 0.06, 0.2], t)
    };
    // Islands.
    let hill = 0.83 - 0.16 * (-((u - 0.2) / 0.2).powi(2)).exp() - 0.1 * (-((u - 1.32) / 0.14).powi(2)).exp();
    let far = hz - 0.035 * (-((u - 0.58) / 0.12).powi(2)).exp();
    if v > far && v < hz + 0.004 && u > 0.46 && u < 0.7 {
        c = [0.36, 0.14, 0.3];
        glint = 0.0;
    }
    if v > hill + 0.01 * fbm(u * 20.0, 3.0) || palm(u, v, aspect, 0.3, 0.76, 0.42) || palm(u, v, aspect, 1.27, 0.8, 0.3)
    {
        c = [0.09, 0.03, 0.1];
        glint = 0.0;
    }
    (c, glint)
}

/// Draws one frame of a scene from its panorama.
fn scene_frame(scene: u8, pano: &Pano, w: u32, h: u32, secs: f32) -> VideoFrame {
    let t = secs.clamp(0.0, MAX_SECONDS);
    let (w, h) = (w as usize, h as usize);
    let aspect = w as f32 / h as f32;
    let px_per_u = pano.w as f32 / PANO_W;
    // Pan offset in frame widths.
    let pan = if scene == 0 { 0.02 + 0.035 * t } else { 0.4 - 0.032 * t };
    let mut data = vec![0u8; w * h * 4];
    par_rows(&mut data, w * 4, |y, row| {
        let v = (y as f32 + 0.5) / h as f32;
        let py = ((v * pano.h as f32) as usize).min(pano.h - 1);
        for x in 0..w {
            let fu = (x as f32 + 0.5) / w as f32;
            let u = fu + pan;
            let c = if scene == 0 {
                let mut sx = u * px_per_u;
                let lake = v > A_HORIZON;
                if lake {
                    sx += (v * 260.0 - t * 4.0 + u * 9.0).sin() * (v - A_HORIZON) * 0.012 * px_per_u;
                }
                let (mut c, m) = pano.at(sx, py);
                if m > 0.2 {
                    // Clouds drift faster than the pan (parallax).
                    let cv = if lake { A_HORIZON - (v - A_HORIZON) * 1.15 } else { v };
                    let cu = u + 0.03 * t;
                    let mut a = 0.0f32;
                    for (k, (x0, y0, rx, ry)) in [
                        (0.12, 0.11, 0.13, 0.07),
                        (0.5, 0.2, 0.1, 0.05),
                        (0.88, 0.09, 0.17, 0.08),
                        (1.3, 0.15, 0.12, 0.06),
                    ]
                    .iter()
                    .enumerate()
                    {
                        // Three puffs over a flat base.
                        let dx = (cu - x0) / rx;
                        let dy = (cv - y0) / ry;
                        if dx.abs() > 1.6 || dy > 0.45 {
                            continue;
                        }
                        let s = 0.15 * (k as f32);
                        for (ox, oy, r) in [(-0.55, 0.15, 0.62), (0.0, -0.1, 0.9), (0.6 + s, 0.2, 0.6)] {
                            let (px, py) = ((dx - ox) / r, (dy - oy) / r);
                            a = a.max(1.0 - (px * px + py * py));
                        }
                    }
                    let a = smoothstep(0.0, 0.7, a) * if lake { 0.45 } else { 0.85 };
                    c = mix(c, if lake { [0.55, 0.62, 0.68] } else { [0.97, 0.97, 0.99] }, a);
                }
                if lake {
                    c = scale(c, 1.0 + 0.07 * (v * 420.0 + t * 6.0 + (u * 30.0).sin() * 2.0).sin());
                }
                c
            } else {
                let (mut c, g) = pano.at(u * px_per_u, py);
                if g > 0.0 {
                    let s = 0.5 + 0.5 * (v * 520.0 + t * 7.0 + (u * 40.0 + t * 3.0).sin() * 2.0).sin();
                    c = mix(c, [1.0, 0.86, 0.56], g * s * 0.75);
                }
                // Three birds gliding left.
                if v < 0.45 {
                    for (k, (y0, sp)) in [(0.2, 0.05), (0.26, 0.045), (0.16, 0.055)].iter().enumerate() {
                        let bx = 0.95 - sp * t - k as f32 * 0.07;
                        let flap = 0.5 + 0.5 * (t * 9.0 + k as f32 * 2.0).sin();
                        let dx = (fu - bx) * aspect;
                        let dy = v - y0;
                        let wing = 0.022;
                        if dx.abs() < wing {
                            let yy = -dx.abs() * (0.25 + 0.6 * flap);
                            if (dy - yy).abs() < 0.0035 {
                                c = [0.15, 0.05, 0.12];
                            }
                        }
                    }
                }
                c
            };
            let i = x * 4;
            row[i] = to8(c[0]);
            row[i + 1] = to8(c[1]);
            row[i + 2] = to8(c[2]);
            row[i + 3] = 255;
        }
    });
    VideoFrame {
        width: w as u32,
        height: h as u32,
        format: PixelFormat::Rgba8,
        color_space: ColorSpace::Bt709,
        full_range: true,
        stride: w * 4,
        data: Arc::new(data),
        pts: 0,
    }
}

type FrameKey = (u8, i64, u32, u32);

/// Frame source for the procedural sample scenes ([`SAMPLE_A`], [`SAMPLE_B`]).
/// Frames are drawn at 30 fps steps at the size the renderer asks for
/// (16:9, 960x540 by default) and a few are kept for reuse.
pub struct SampleSource {
    panos: Mutex<HashMap<(u8, u32, u32), Arc<Pano>>>,
    frames: Mutex<VecDeque<(FrameKey, VideoFrame)>>,
}

impl Default for SampleSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleSource {
    pub fn new() -> Self {
        Self { panos: Mutex::new(HashMap::new()), frames: Mutex::new(VecDeque::new()) }
    }

    fn size_for(max: Option<(u32, u32)>) -> (u32, u32) {
        let (mw, mh) = max.unwrap_or((960, 540));
        let h = mh.min(mw * 9 / 16).clamp(36, 1080) & !1;
        ((h * 16 / 9) & !1, h)
    }

    /// Frame of scene `id` at `t` (None for other ids).
    pub fn frame_at(&self, id: &str, t: Ticks, max: Option<(u32, u32)>) -> Option<VideoFrame> {
        let scene = match id {
            SAMPLE_A => 0u8,
            SAMPLE_B => 1u8,
            _ => return None,
        };
        let (w, h) = Self::size_for(max);
        let q = (t.max(0) as f64 * 30.0 / TICKS_PER_SECOND as f64).round() as i64;
        let key = (scene, q, w, h);
        let lock = |m: &Mutex<VecDeque<(FrameKey, VideoFrame)>>| {
            m.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| *k == key).map(|(_, f)| f.clone())
        };
        if let Some(f) = lock(&self.frames) {
            return Some(f);
        }
        let pano = {
            let mut panos = self.panos.lock().unwrap_or_else(|e| e.into_inner());
            if panos.len() > 8 {
                panos.clear();
            }
            panos
                .entry((scene, w, h))
                .or_insert_with(|| {
                    let aspect = w as f32 / h as f32;
                    let pw = (w as f32 * PANO_W).ceil() as usize;
                    Arc::new(if scene == 0 {
                        Pano::build(pw, h as usize, |u, v| scene_a(u, v, aspect))
                    } else {
                        Pano::build(pw, h as usize, |u, v| scene_b(u, v, aspect))
                    })
                })
                .clone()
        };
        let f = scene_frame(scene, &pano, w, h, q as f32 / 30.0);
        let mut frames = self.frames.lock().unwrap_or_else(|e| e.into_inner());
        frames.push_back((key, f.clone()));
        while frames.len() > 12 {
            frames.pop_front();
        }
        Some(f)
    }
}

impl FrameSource for SampleSource {
    fn frame(&self, media_id: &str, t: Ticks, max: Option<(u32, u32)>) -> anyhow::Result<Option<VideoFrame>> {
        Ok(self.frame_at(media_id, t, max))
    }
}

/// Serves the sample scenes and forwards every other media id to `base`.
pub struct WithSamples<'a> {
    pub base: &'a dyn FrameSource,
    pub samples: &'a SampleSource,
}

impl FrameSource for WithSamples<'_> {
    fn frame(&self, media_id: &str, t: Ticks, max: Option<(u32, u32)>) -> anyhow::Result<Option<VideoFrame>> {
        if is_sample(media_id) {
            return Ok(self.samples.frame_at(media_id, t, max));
        }
        self.base.frame(media_id, t, max)
    }
}

// ---------- Preview projects ----------

/// What a library card shows.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum PreviewItem {
    /// A registry effect, filter, transition or animation by id.
    Fx(String),
    /// A title template with its text and style.
    Title(TitleData),
}

impl PreviewItem {
    /// File-name-safe cache key.
    pub fn key(&self) -> String {
        match self {
            PreviewItem::Fx(id) => {
                id.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect()
            }
            PreviewItem::Title(t) => {
                format!("title-{:016x}", fnv(serde_json::to_string(t).unwrap_or_default().as_bytes()))
            }
        }
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// A project that plays one library item, and the time of its most
/// representative frame (the card thumbnail).
#[derive(Debug, Clone)]
pub struct PreviewProject {
    pub project: Project,
    pub duration: Ticks,
    pub still: Ticks,
    /// Filters: the still is split, original left and filtered right
    /// (as Filmora's filter cards).
    pub split_still: bool,
}

const SECOND: Ticks = TICKS_PER_SECOND;

fn clip(id: &str, source: ClipSource, start: Ticks, end: Ticks) -> Clip {
    let mut c: Clip = serde_json::from_value(serde_json::json!({
        "id": id, "name": id, "source": { "type": "adjustment" }, "start": start, "end": end
    }))
    .expect("minimal clip");
    c.source = source;
    c
}

fn media(id: &str) -> ClipSource {
    ClipSource::Media { media_id: id.into() }
}

fn track(id: &str, clips: Vec<Clip>) -> Track {
    let mut t: Track =
        serde_json::from_value(serde_json::json!({ "id": id, "kind": "video", "name": id, "clips": [] }))
            .expect("minimal track");
    t.kind = TrackKind::Video;
    t.clips = clips;
    t
}

fn instance(kind: &str) -> EffectInstance {
    serde_json::from_value(serde_json::json!({ "id": format!("preview-{kind}"), "kind": kind })).expect("effect")
}

fn sample_project(w: u32, h: u32, tracks: Vec<Track>) -> Project {
    let mut p = Project::new(
        "Preview",
        ProjectSettings {
            width: w,
            height: h,
            frame_rate: Rational::new(30, 1),
            sample_rate: 48000,
            background: [0.0, 0.0, 0.0, 1.0],
        },
    );
    for id in [SAMPLE_A, SAMPLE_B] {
        p.media.push(MediaAsset {
            id: id.into(),
            name: id.into(),
            path: String::new(),
            kind: MediaKind::Video,
            duration: 3600 * SECOND,
            width: w,
            height: h,
            frame_rate: Some(Rational::new(30, 1)),
            has_audio: false,
            extra: jb_core::Extra::Null,
        });
    }
    let main = p.main_sequence.clone();
    p.sequence_mut(&main).expect("main").tracks = tracks;
    p
}

/// Entry (in) vs exit (out) animation, as the app's Animation tab decides.
pub fn is_out_animation(id: &str, description: &str) -> bool {
    description.starts_with("Out:") || id.ends_with("_out")
}

/// The sample-footage preview of `item` at `w`x`h`; None for unknown ids.
pub fn sample_preview(registry: &Registry, item: &PreviewItem, w: u32, h: u32) -> Option<PreviewProject> {
    let d = PREVIEW_DURATION;
    let mut split_still = false;
    let (tracks, still) = match item {
        PreviewItem::Title(t) => {
            let title = clip("title", ClipSource::Title(t.clone()), 0, d);
            (vec![track("v1", vec![clip("a", media(SAMPLE_A), 0, d)]), track("v2", vec![title])], d / 2)
        }
        PreviewItem::Fx(id) => {
            let canonical = registry.canonical(id)?;
            let info = registry.iter().find(|i| i.id == canonical)?;
            match info.kind {
                FxKind::Transition => {
                    let mut a = clip("a", media(SAMPLE_A), 0, d / 2);
                    let mut b = clip("b", media(SAMPLE_B), d / 2, d);
                    // B continues where it would have been had it played all along.
                    b.source_in = d / 2;
                    a.transition_out = Some(Transition {
                        id: "preview-transition".into(),
                        kind: id.clone(),
                        duration: d * 6 / 10,
                        alignment: Default::default(),
                        params: Default::default(),
                        extra: jb_core::Extra::Null,
                    });
                    (vec![track("v1", vec![a, b])], d / 2)
                }
                FxKind::Effect | FxKind::Filter => {
                    split_still = info.kind == FxKind::Filter;
                    let mut a = clip("a", media(SAMPLE_A), 0, d);
                    a.effects.push(instance(id));
                    (vec![track("v1", vec![a])], d / 2)
                }
                FxKind::Animation => {
                    let out = is_out_animation(info.id, info.description);
                    let mut fg = clip("a", media(SAMPLE_A), 0, d);
                    fg.transform.scale_x = Param::float(0.62);
                    fg.transform.scale_y = Param::float(0.62);
                    let mut anim = instance(id);
                    let len = d / 2;
                    anim.range = Some(if out { (d - len, d) } else { (0, len) });
                    anim.placement = Some(if out { AnimationPlacement::Out } else { AnimationPlacement::In });
                    fg.animations.push(anim);
                    let still = if out { d - len + len * 4 / 10 } else { len * 6 / 10 };
                    (vec![track("v1", vec![clip("b", media(SAMPLE_B), 0, d)]), track("v2", vec![fg])], still)
                }
            }
        }
    };
    Some(PreviewProject { project: sample_project(w, h, tracks), duration: d, still, split_still })
}

/// Filmora previews effects and filters on the user's own footage: the
/// main sequence from `at` for 5 s with the effect applied to all of it.
/// None when there is nothing to show there (empty timeline).
pub fn footage_preview(registry: &Registry, base: &Project, at: Ticks, kind: &str) -> Option<PreviewProject> {
    let canonical = registry.canonical(kind)?;
    let info = registry.iter().find(|i| i.id == canonical)?;
    if !matches!(info.kind, FxKind::Effect | FxKind::Filter) {
        return None;
    }
    let main = base.main();
    let total = main.duration();
    if total <= 0 {
        return None;
    }
    let mut from = at.clamp(0, total - 1);
    if total - from < SECOND {
        from = (total - FOOTAGE_PREVIEW_DURATION).max(0);
    }
    let len = FOOTAGE_PREVIEW_DURATION.min(total - from);
    if jb_core::eval::evaluate(base, main, from).layers.is_empty() {
        return None;
    }
    let mut p = base.clone();
    let mut nested = clip("footage", ClipSource::Sequence { sequence_id: main.id.clone() }, 0, len);
    nested.source_in = from;
    nested.effects.push(instance(kind));
    let seq = Sequence {
        id: "jb-preview".into(),
        name: "Preview".into(),
        tracks: vec![track("v1", vec![nested])],
        markers: Vec::new(),
        extra: jb_core::Extra::Null,
    };
    p.sequences.retain(|s| s.id != seq.id);
    p.main_sequence = seq.id.clone();
    p.sequences.push(seq);
    Some(PreviewProject { project: p, duration: len, still: len / 2, split_still: false })
}

// ---------- Rendering card previews ----------

/// Encoded card preview: still and sprite sheet (JPEG).
pub struct CardPreview {
    pub still: Vec<u8>,
    pub sprite: Vec<u8>,
}

fn rows_of(f: &VideoFrame) -> impl Iterator<Item = &[u8]> {
    let row = f.width as usize * 4;
    f.data.chunks(f.stride).take(f.height as usize).map(move |r| &r[..row])
}

/// Below this luma standard deviation (8-bit) a frame counts as flat.
const FLAT_SPREAD: f64 = 14.0;

/// Standard deviation of the luma of an RGBA8 buffer.
pub fn luma_spread(rgba: &[u8]) -> f64 {
    let n = (rgba.len() / 4).max(1) as f64;
    let (mut s, mut s2) = (0.0f64, 0.0f64);
    for p in rgba.chunks_exact(4) {
        let l = 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
        s += l;
        s2 += l * l;
    }
    (s2 / n - (s / n).powi(2)).max(0.0).sqrt()
}

fn jpeg(w: u32, h: u32, rgba: &[u8]) -> anyhow::Result<Vec<u8>> {
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).encode(
        &rgb,
        w,
        h,
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(out)
}

/// Renders `item`'s card still and hover sprite with `r` (resized to the
/// layout's frame size).
pub fn render_card(
    r: &mut Renderer,
    samples: &SampleSource,
    item: &PreviewItem,
    layout: SpriteLayout,
) -> anyhow::Result<CardPreview> {
    let (w, h) = (layout.frame_w, layout.frame_h);
    let pp =
        sample_preview(r.registry(), item, w, h).ok_or_else(|| anyhow::anyhow!("unknown preview item {item:?}"))?;
    r.resize(w, h);
    r.options.source_max_size = Some((w, h));
    let project = &pp.project;
    let render = |r: &mut Renderer, p: &Project, t: Ticks| {
        let plan = jb_core::eval::evaluate(p, p.main(), t);
        r.render(p, &plan, samples)
    };
    // Sprite: frames spread evenly over the preview.
    let (sw, sh) = layout.sheet_size();
    let mut sheet = vec![0u8; (sw * sh * 4) as usize];
    let mut frames: Vec<(Ticks, Vec<u8>)> = Vec::with_capacity(layout.frames as usize);
    for i in 0..layout.frames {
        let t = (pp.duration as f64 * i as f64 / layout.frames as f64).round() as Ticks;
        let f = render(r, project, t);
        let px: Vec<u8> = rows_of(&f).flatten().copied().collect();
        let (cx, cy) = ((i % layout.cols) * w, (i / layout.cols) * h);
        let row = (w * 4) as usize;
        for (y, line) in px.chunks(row).enumerate() {
            let o = (((cy + y as u32) * sw + cx) * 4) as usize;
            sheet[o..o + row].copy_from_slice(line);
        }
        frames.push((t, px));
    }
    // Still: the representative moment, or the nearest frame with something
    // to see when that one is flat (a dip to black at its midpoint).
    let mut still_px: Vec<u8> = rows_of(&render(r, project, pp.still)).flatten().copied().collect();
    if luma_spread(&still_px) < FLAT_SPREAD {
        frames.sort_by_key(|(t, _)| (t - pp.still).abs());
        if let Some((_, px)) = frames.iter().find(|(_, px)| luma_spread(px) >= FLAT_SPREAD) {
            still_px = px.clone();
        }
    }
    if pp.split_still {
        let mut plain = project.clone();
        let main = plain.main_sequence.clone();
        for t in &mut plain.sequence_mut(&main).expect("main").tracks {
            for c in &mut t.clips {
                c.effects.clear();
            }
        }
        let orig: Vec<u8> = rows_of(&render(r, &plain, pp.still)).flatten().copied().collect();
        let mid = w as usize / 2;
        for y in 0..h as usize {
            for x in 0..=mid {
                let i = (y * w as usize + x) * 4;
                if x < mid {
                    still_px[i..i + 4].copy_from_slice(&orig[i..i + 4]);
                } else {
                    still_px[i..i + 3].copy_from_slice(&[245, 245, 245]);
                }
            }
        }
    }
    Ok(CardPreview { still: jpeg(w, h, &still_px)?, sprite: jpeg(sw, sh, &sheet)? })
}

/// Card previews on disk: `<root>/<PREVIEW_VERSION>/<key>.jpg` (still) and
/// `<key>.sprite.jpg`.
#[derive(Debug, Clone)]
pub struct PreviewCache {
    pub dir: PathBuf,
}

impl PreviewCache {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { dir: root.as_ref().join(PREVIEW_VERSION) }
    }

    pub fn paths(&self, item: &PreviewItem) -> (PathBuf, PathBuf) {
        let k = item.key();
        (self.dir.join(format!("{k}.jpg")), self.dir.join(format!("{k}.sprite.jpg")))
    }

    pub fn has(&self, item: &PreviewItem) -> bool {
        let (a, b) = self.paths(item);
        a.is_file() && b.is_file()
    }

    /// Renders and stores `item` unless cached; returns (still, sprite) paths.
    pub fn ensure(
        &self,
        r: &mut Renderer,
        samples: &SampleSource,
        item: &PreviewItem,
    ) -> anyhow::Result<(PathBuf, PathBuf)> {
        let (still, sprite) = self.paths(item);
        if still.is_file() && sprite.is_file() {
            return Ok((still, sprite));
        }
        let card = render_card(r, samples, item, CARD_LAYOUT)?;
        std::fs::create_dir_all(&self.dir)?;
        let info = self.dir.join("info.json");
        if !info.is_file() {
            let json = serde_json::json!({ "version": PREVIEW_VERSION, "layout": CARD_LAYOUT });
            std::fs::write(&info, json.to_string())?;
        }
        for (path, bytes) in [(&sprite, &card.sprite), (&still, &card.still)] {
            let tmp = path.with_extension(format!("tmp{}", std::process::id()));
            std::fs::write(&tmp, bytes)?;
            std::fs::rename(&tmp, path)?;
        }
        Ok((still, sprite))
    }

    /// Deletes previews rendered by other renderer versions.
    pub fn prune_other_versions(&self) {
        let Some(root) = self.dir.parent() else { return };
        let Ok(rd) = std::fs::read_dir(root) else { return };
        for e in rd.flatten() {
            if e.path() != self.dir && e.path().is_dir() {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }

    /// Total bytes stored for this version.
    pub fn size(&self) -> u64 {
        std::fs::read_dir(&self.dir)
            .map(|rd| rd.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum())
            .unwrap_or(0)
    }
}

/// Title templates of the library (kept in sync with `app/src/library.ts`),
/// used by `jb fx previews`.
pub fn sample_titles() -> Vec<(&'static str, TitleData)> {
    let base = TitleData {
        text: "Title".into(),
        font_family: "Helvetica Neue".into(),
        font_size: 0.09,
        color: [1.0, 1.0, 1.0, 1.0],
        position: [0.5, 0.5],
        bold: true,
        ..Default::default()
    };
    vec![
        ("default", base.clone()),
        ("basic", TitleData { text: "Basic Title".into(), ..base.clone() }),
        (
            "lower_third",
            TitleData { text: "Lower Third".into(), position: [0.3, 0.82], font_size: 0.06, ..base.clone() },
        ),
        ("subtitle", TitleData { text: "Subtitle".into(), position: [0.5, 0.9], font_size: 0.05, bold: false, ..base }),
    ]
}

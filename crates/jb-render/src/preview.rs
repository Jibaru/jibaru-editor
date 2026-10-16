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
//
// Both scenes are functions of normalized coordinates (`u` across, in frame
// widths; `v` down, in frame heights), so they are drawn at whatever size
// the renderer asks for. Detail comes from 2D value noise whose octave count
// grows with the resolution (down to ~2 px features), and edges are
// antialiased analytically, so a 1080p player frame is as crisp as a card.
// The still parts are drawn once per size into panoramas the camera pans
// across; per frame only the moving parts (clouds, water, birds) and a
// light film grain are added.

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

/// Integer hash to 0..1.
fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    let mut h =
        (ix as u32).wrapping_mul(0x8da6_b343) ^ (iy as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xff_ffff) as f32 / 16_777_215.0
}

/// Smooth 2D value noise in -1..1.
fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let (xf, yf) = (x.floor(), y.floor());
    let (fx, fy) = (x - xf, y - yf);
    let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let uy = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);
    let (i, j) = (xf as i32, yf as i32);
    let a = hash2(i, j, seed);
    let b = hash2(i + 1, j, seed);
    let c = hash2(i, j + 1, seed);
    let d = hash2(i + 1, j + 1, seed);
    (a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy) * 2.0 - 1.0
}

/// Fractal noise (about -1..1): `oct` octaves, each twice the frequency of
/// the previous one (rotated, so the lattice does not show).
fn fbm2(x: f32, y: f32, seed: u32, oct: u32) -> f32 {
    let (mut v, mut a, mut x, mut y) = (0.0, 0.5, x, y);
    for i in 0..oct {
        v += a * vnoise(x, y, seed.wrapping_add(i * 7919));
        let (nx, ny) = (x * 1.6 - y * 1.2, x * 1.2 + y * 1.6);
        x = nx + 17.3;
        y = ny + 9.1;
        a *= 0.5;
    }
    v / (1.0 - 0.5f32.powi(oct as i32))
}

/// Ridged fractal noise in 0..1: sharp crests, like eroded rock.
fn ridged(x: f32, y: f32, seed: u32, oct: u32) -> f32 {
    let (mut v, mut a, mut norm, mut x, mut y) = (0.0, 0.5, 0.0, x, y);
    for i in 0..oct {
        let n = 1.0 - vnoise(x, y, seed.wrapping_add(i * 7919)).abs();
        v += a * n * n;
        norm += a;
        let (nx, ny) = (x * 1.6 - y * 1.2, x * 1.2 + y * 1.6);
        x = nx + 17.3;
        y = ny + 9.1;
        a *= 0.5;
    }
    v / norm
}

/// The output resolution, as the scene functions need it.
#[derive(Clone, Copy)]
struct Res {
    /// Width / height.
    aspect: f32,
    /// One pixel in `v` units (1 / height).
    px: f32,
}

impl Res {
    fn new(w: usize, h: usize) -> Self {
        Self { aspect: w as f32 / h as f32, px: 1.0 / h as f32 }
    }
    /// Octaves of noise starting at `f` cycles per frame height, until
    /// features are about 2 px.
    fn oct(&self, f: f32) -> u32 {
        ((1.0 / (self.px * 2.5 * f)).log2().ceil().max(1.0) as u32).min(9)
    }
    /// Antialiased coverage of `d > 0`, `d` a signed distance in `v` units.
    fn cover(&self, d: f32) -> f32 {
        smoothstep(-self.px * 0.8, self.px * 0.8, d)
    }
}

/// A precomputed still panorama: RGB plus a scene-specific mask byte.
struct Pano {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

impl Pano {
    /// Draws `f(u, v)` for the rows whose `v` is in `rows` (others stay 0).
    fn build(w: usize, h: usize, rows: std::ops::Range<f32>, f: impl Fn(f32, f32) -> ([f32; 3], f32) + Sync) -> Self {
        let mut px = vec![[0u8; 4]; w * h];
        par_rows(&mut px, w, |y, row| {
            let v = (y as f32 + 0.5) / h as f32;
            if !rows.contains(&v) {
                return;
            }
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

    fn row_of(&self, v: f32) -> usize {
        ((v.max(0.0) * self.h as f32) as usize).min(self.h - 1)
    }
}

fn to8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Runs `f(y, row)` over the rows of `buf` on all cores (in small bands
/// handed out on demand, since some rows cost more than others).
fn par_rows<T: Send>(buf: &mut [T], w: usize, f: impl Fn(usize, &mut [T]) + Sync) {
    let rows = buf.len() / w.max(1);
    let n = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16).min(rows.max(1));
    let band = rows.div_ceil(n * 6).max(1);
    let bands: Vec<Mutex<(usize, &mut [T])>> =
        buf.chunks_mut(band * w).enumerate().map(|(i, c)| Mutex::new((i * band, c))).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..n {
            let (f, next, bands) = (&f, &next, &bands);
            s.spawn(move || {
                while let Some(b) = bands.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
                    let mut b = b.lock().unwrap_or_else(|e| e.into_inner());
                    let y0 = b.0;
                    for (j, row) in b.1.chunks_mut(w).enumerate() {
                        f(y0 + j, row);
                    }
                }
            });
        }
    });
}

// ----- Scene A: a mountain lake on a clear morning -----

const A_HORIZON: f32 = 0.64;
const A_SUN: (f32, f32) = (0.98, 0.16);

fn a_sky(u: f32, v: f32, r: &Res) -> [f32; 3] {
    let x = u * r.aspect;
    let t = (v / A_HORIZON).clamp(0.0, 1.0);
    let mut c = mix([0.09, 0.25, 0.55], [0.68, 0.81, 0.93], t.powf(1.3));
    c = scale(c, 1.0 + 0.025 * fbm2(x * 1.3, v * 2.5, 11, 3));
    let d = (((u - A_SUN.0) * r.aspect).powi(2) + (v - A_SUN.1).powi(2)).sqrt();
    c = mix(c, [1.0, 0.95, 0.84], (0.5 * (-d * 5.5).exp() + 0.35 * (-d * 22.0).exp()).min(1.0));
    mix(c, [1.0, 0.99, 0.95], r.cover(0.036 - d))
}

/// Skyline of the snowy range (height above its base, 0..~1).
fn a_peaks(u: f32, r: &Res) -> f32 {
    let mut m = 0.0f32;
    for (c, w, k) in [(0.3, 0.32, 1.0), (0.78, 0.26, 0.8), (1.2, 0.34, 0.95), (1.55, 0.3, 0.7)] {
        m = m.max(k * (1.0 - ((u - c) / w).abs()).max(0.0).powf(1.3));
    }
    let x = u * r.aspect;
    0.8 * m + 0.38 * (ridged(x * 1.7, 0.37, 3, r.oct(1.7)) - 0.45)
}

/// Rock relief of the range: gullies running down the faces.
fn a_rock(x: f32, v: f32, r: &Res) -> f32 {
    ridged(x * 6.0, v * 2.6, 5, r.oct(6.0))
}

/// One row of conifers at `(x, v)`: coverage, which side of the tree
/// (-1 shaded .. 1 lit) and a per-tree random value.
fn a_trees(x: f32, v: f32, r: &Res, spacing: f32, base: f32, height: f32, seed: u32) -> (f32, f32, f32) {
    let k0 = (x / spacing).floor() as i32;
    let mut best = (0.0f32, 0.0f32, 0.0f32);
    for k in k0 - 2..=k0 + 2 {
        let rnd = hash2(k, 0, seed);
        let cx = (k as f32 + 0.5 + (hash2(k, 1, seed) - 0.5) * 0.6) * spacing;
        let hk = height * (0.6 + 0.6 * rnd);
        let foot = base + 0.012 * vnoise(cx * 3.0, 0.5, seed + 9);
        let tip = foot - hk;
        let half = spacing * (0.55 + 0.35 * hash2(k, 2, seed));
        let dx = (x - cx).abs();
        // Layered branches: the outline widens in steps toward the foot.
        let s = ((v - tip) / hk).clamp(0.0, 1.0);
        let tiers = 1.0 - 0.18 * (s * 5.0).fract();
        let edge = half * s * tiers;
        let slope = hk / half;
        let d = (edge - dx) * slope / (1.0 + slope * slope).sqrt();
        let cov = r.cover(d.min(v - tip)) * r.cover(foot + 0.05 - v);
        if cov > best.0 {
            best = (cov, ((x - cx) / half).clamp(-1.0, 1.0), rnd);
        }
    }
    best
}

/// Scene A above the horizon. Mask: sky coverage.
fn a_land(u: f32, v: f32, r: &Res) -> ([f32; 3], f32) {
    let x = u * r.aspect;
    let mut c = a_sky(u, v, r);
    let mut sky = 1.0;
    let haze = [0.66, 0.76, 0.88];
    // Distant hazy range.
    let hf = 0.475 - 0.08 * (0.5 + 0.5 * fbm2(x * 1.7, 0.9, 8, r.oct(1.7)));
    let k = r.cover(v - hf);
    if k > 0.0 {
        let far = mix([0.47, 0.57, 0.72], haze, 0.35 + 0.1 * fbm2(x * 9.0, v * 9.0, 9, r.oct(9.0)));
        c = mix(c, far, k);
        sky *= 1.0 - k;
    }
    // The snowy range, lit from the right (the sun).
    let p = a_peaks(u, r);
    let h1 = 0.5 - 0.33 * p;
    let k = r.cover(v - h1);
    if k > 0.0 {
        let e = 0.003;
        // Faces descending to the right face the sun.
        let face = -(a_peaks(u + e, r) - a_peaks(u - e, r)) / (2.0 * e * r.aspect) * 0.33;
        let ex = 1.5 * r.px;
        let s0 = a_rock(x, v, r);
        let gx = (a_rock(x + ex, v, r) - a_rock(x - ex, v, r)) / (2.0 * ex);
        let light = smoothstep(-1.0, 1.0, face * 1.8 + gx * 0.12);
        let grain = 1.0 + 0.16 * fbm2(x * 40.0, v * 40.0, 6, r.oct(40.0));
        let rock = scale(mix([0.15, 0.17, 0.25], [0.5, 0.48, 0.52], light), grain * (0.8 + 0.4 * s0));
        let below = v - h1;
        let snow = smoothstep(0.17, 0.07, below + 0.12 * (s0 - 0.5) + 0.04 * fbm2(x * 14.0, v * 14.0, 7, r.oct(14.0)))
            * smoothstep(0.46, 0.4, v);
        let snowc = mix([0.5, 0.6, 0.78], [0.98, 0.98, 1.0], smoothstep(0.25, 0.75, light));
        let mut m = mix(rock, snowc, snow);
        m = mix(m, haze, 0.1 + 0.55 * smoothstep(0.36, 0.53, v));
        c = mix(c, m, k);
        sky *= 1.0 - k;
    }
    // Wooded hills.
    let h2 = 0.515 + 0.03 * fbm2(x * 3.5, 2.5, 2, r.oct(3.5));
    let k = r.cover(v - h2);
    if k > 0.0 {
        // Treetops seen from afar: a bumpy canopy texture.
        let n = fbm2(x * 150.0, v * 110.0, 12, r.oct(150.0));
        let m = fbm2(x * 9.0, v * 9.0, 13, 3);
        let hills = mix([0.22, 0.36, 0.38], [0.1, 0.21, 0.2], smoothstep(h2, h2 + 0.07, v));
        c = mix(c, scale(hills, 1.0 + 0.22 * n + 0.1 * m), k);
        sky *= 1.0 - k;
    }
    // Two rows of conifers along the shore; the back row is smaller and hazier.
    for (sp, base, ht, seed, dark, hz) in [(0.011, 0.6, 0.035, 31, 1.0, 0.25), (0.019, 0.643, 0.065, 37, 0.7, 0.0)] {
        let (cov, side, rnd) = a_trees(x, v, r, sp, base, ht, seed);
        if cov > 0.0 {
            let g = mix([0.07, 0.18, 0.13], [0.13, 0.27, 0.18], rnd);
            let lit = 0.75 + 0.45 * smoothstep(-0.2, 0.9, side);
            let n = 1.0 + 0.2 * fbm2(x * 140.0, v * 140.0, seed + 1, r.oct(140.0));
            let t = scale(g, lit * n * dark);
            c = mix(c, mix(t, haze, hz), cov);
            sky *= 1.0 - cov;
        }
    }
    // Shoreline.
    let k = r.cover(v - (A_HORIZON - 0.005));
    if k > 0.0 {
        c = mix(c, [0.08, 0.1, 0.09], k);
        sky *= 1.0 - k;
    }
    (c, sky)
}

/// Scene A panorama: land above the horizon, its (slightly blurred)
/// reflection in the lake below. Mask: sky coverage (reflected, in the lake).
fn scene_a(u: f32, v: f32, r: &Res) -> ([f32; 3], f32) {
    let hz = A_HORIZON;
    if v < hz {
        return a_land(u, v, r);
    }
    let depth = (v - hz) / (1.0 - hz);
    let rv = hz - (v - hz) * 1.05;
    let blur = 0.002 + depth * 0.008;
    let (c1, s1) = a_land(u, (rv - blur).max(0.0), r);
    let (c2, s2) = a_land(u, (rv + blur).max(0.0), r);
    let refl = mix(c1, c2, 0.5);
    let c = mix(scale(refl, 0.74), [0.03, 0.12, 0.19], 0.15 + depth * 0.55);
    (c, (s1 + s2) * 0.5)
}

/// Scene A clouds (they drift over the panorama): color and alpha.
fn a_clouds(u: f32, v: f32, r: &Res) -> ([f32; 3], f32) {
    let x = u * r.aspect;
    let sun = (((u - A_SUN.0) * r.aspect).powi(2) + (v - A_SUN.1).powi(2)).sqrt();
    let band = smoothstep(0.01, 0.08, v) * smoothstep(0.34, 0.2, v) * smoothstep(0.06, 0.32, sun);
    if band <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let oct = r.oct(2.4);
    let dens = |x: f32, v: f32| fbm2(x * 2.0, v * 4.5, 21, oct) + 0.6 * band - 0.55;
    let d = dens(x, v);
    let a = smoothstep(0.0, 0.14, d) * band;
    if a <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    // Brighter where the cloud thins toward the sun (up and to the right).
    let toward = dens(x + 0.02, v - 0.025);
    let light = smoothstep(-0.2, 0.15, d - toward + 0.04);
    let c = mix([0.55, 0.62, 0.75], [1.0, 1.0, 1.0], light);
    (c, a * 0.97)
}

// ----- Scene B: a tropical sunset over the sea -----

const B_HORIZON: f32 = 0.6;
/// Sun center (u, v) and radius.
const B_SUN: (f32, f32, f32) = (0.78, B_HORIZON - 0.03, 0.1);

fn b_sky(u: f32, v: f32, r: &Res) -> [f32; 3] {
    let t = (v / B_HORIZON).clamp(0.0, 1.0);
    let stops: [(f32, [f32; 3]); 4] =
        [(0.0, [0.13, 0.07, 0.3]), (0.42, [0.58, 0.2, 0.46]), (0.72, [0.96, 0.43, 0.38]), (1.0, [1.0, 0.76, 0.43])];
    let mut c = stops[0].1;
    for w in stops.windows(2) {
        if t >= w[0].0 {
            c = mix(w[0].1, w[1].1, smoothstep(0.0, 1.0, (t - w[0].0) / (w[1].0 - w[0].0)));
        }
    }
    let d = (((u - B_SUN.0) * r.aspect).powi(2) + (v - B_SUN.1).powi(2)).sqrt();
    c = mix(c, [1.0, 0.85, 0.55], (0.55 * (-d * 4.0).exp() + 0.3 * (-d * 14.0).exp()).min(1.0));
    let disk = mix([1.0, 0.97, 0.78], [1.0, 0.72, 0.38], smoothstep(B_SUN.1 - B_SUN.2, B_HORIZON, v));
    mix(c, disk, r.cover(B_SUN.2 - d))
}

/// Coverage of a palm tree standing at (`base_u`, `base_v`).
fn palm(u: f32, v: f32, r: &Res, base_u: f32, base_v: f32, height: f32) -> f32 {
    let a = r.aspect;
    let top_v = base_v - height;
    let top_u = base_u + 0.125 * height;
    let len = height * 0.55;
    let mut cov = 0.0f32;
    // Leaning, tapering trunk.
    if v > top_v - r.px && v < base_v {
        let s = (base_v - v) / height;
        let xc = base_u + 0.125 * height * s * s;
        let half = 0.03 * height * (1.2 - 0.5 * s);
        cov = cov.max(r.cover(half - ((u - xc) * a).abs()));
    }
    if ((u - top_u) * a).abs() > len * 1.3 || (v - top_v).abs() > len * 1.1 {
        return cov;
    }
    // Drooping fronds: a tapering rib with leaflets hanging from it.
    let seg = |px: f32, py: f32, (ax, ay): (f32, f32), (bx, by): (f32, f32)| {
        let (vx, vy) = (bx - ax, by - ay);
        let t = (((px - ax) * vx + (py - ay) * vy) / (vx * vx + vy * vy).max(1e-12)).clamp(0.0, 1.0);
        ((px - ax - vx * t).powi(2) + (py - ay - vy * t).powi(2)).sqrt()
    };
    let (qx, qy) = (u * a, v);
    let mut d = f32::MAX;
    for i in 0..7 {
        let th = -2.9 + i as f32 * 0.95;
        let (dx, dy) = (th.cos(), th.sin() * 0.55);
        let at = |s: f32| (top_u * a + dx * len * s, top_v + dy * len * s + 0.55 * len * s * s);
        let mut prev = at(0.0);
        for k in 1..=32 {
            let s = k as f32 / 32.0;
            let p = at(s);
            let w = 0.012 * height * (1.0 - 0.8 * s) + 0.0015;
            d = d.min(seg(qx, qy, prev, p) - w);
            // Leaflets on both sides, drooping, shorter toward the tip.
            let ll = 0.12 * height * (1.0 - 0.75 * s) * (0.8 + 0.4 * hash2(i, k, 77));
            let (tx, ty) = (p.0 - prev.0, p.1 - prev.1);
            let tl = (tx * tx + ty * ty).sqrt().max(1e-6);
            let (nx, ny) = (-ty / tl, tx / tl);
            for side in [-1.0f32, 1.0] {
                let end = (p.0 + (nx * side * 0.8 + tx / tl * 0.3) * ll, p.1 + (ny * side * 0.8 + 0.6) * ll);
                d = d.min(seg(qx, qy, p, end) - 0.009 * height * (1.0 - 0.5 * s) - 0.0008);
            }
            prev = p;
        }
    }
    cov.max(r.cover(-d))
}

/// Scene B panorama. Mask: sky coverage above the horizon; below it the
/// wave value (0.05..1) of open water, 0 on land.
fn scene_b(u: f32, v: f32, r: &Res) -> ([f32; 3], f32) {
    let hz = B_HORIZON;
    let x = u * r.aspect;
    let (mut c, mut mask) = if v < hz {
        (b_sky(u, v, r), 1.0)
    } else {
        let t = ((v - hz) / (1.0 - hz)).powf(0.6);
        let base = mix([0.74, 0.37, 0.42], [0.08, 0.04, 0.16], t);
        // Perspective waves: finer and flatter toward the horizon.
        let z = 1.0 / (v - hz + 0.012);
        let n = fbm2(x * z * 0.9, z * 1.8, 41, 5);
        let wave = (0.5 + 0.5 * n).clamp(0.05, 1.0);
        let c = scale(base, 0.82 + 0.36 * wave);
        // The sky's warm glow reflected near the horizon.
        let c = mix(c, [1.0, 0.62, 0.45], 0.35 * (-(v - hz) * 25.0).exp());
        (c, wave)
    };
    // Far islands.
    let far = hz - 0.04 * (-((u - 0.58) / 0.1).powi(2)).exp() * (1.0 + 0.2 * fbm2(x * 12.0, 0.3, 4, r.oct(12.0)));
    let k = r.cover(v - far) * r.cover(hz - v);
    if k > 0.0 {
        c = mix(c, [0.4, 0.15, 0.32], k);
        mask *= 1.0 - k;
    }
    // Headlands with palms, in silhouette.
    let hill = 0.83 - 0.16 * (-((u - 0.2) / 0.2).powi(2)).exp() - 0.1 * (-((u - 1.32) / 0.14).powi(2)).exp()
        + 0.008 * fbm2(x * 18.0, 0.7, 3, r.oct(18.0));
    let k = r.cover(v - hill).max(palm(u, v, r, 0.3, 0.76, 0.42)).max(palm(u, v, r, 1.27, 0.8, 0.3));
    if k > 0.0 {
        let sil = mix([0.13, 0.05, 0.14], [0.05, 0.02, 0.07], smoothstep(0.5, 0.9, v));
        c = mix(c, sil, k);
        mask *= 1.0 - k;
    }
    (c, mask)
}

/// Scene B clouds: long streaks lit from below by the setting sun.
fn b_clouds(u: f32, v: f32, r: &Res) -> ([f32; 3], f32) {
    let x = u * r.aspect;
    let band = smoothstep(0.04, 0.14, v) * smoothstep(0.52, 0.36, v);
    if band <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let n = fbm2(x * 1.1, v * 10.0, 31, r.oct(10.0));
    let a = smoothstep(0.0, 0.4, n + 0.3 * band - 0.18) * band * 0.9;
    if a <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let d = (((u - B_SUN.0) * r.aspect).powi(2) + (v - B_SUN.1).powi(2)).sqrt();
    let under = smoothstep(0.1, 0.5, v) * (0.35 + 0.65 * (-d * 2.5).exp());
    let c = mix([0.34, 0.12, 0.32], [1.0, 0.6, 0.42], under * (0.7 + 0.3 * n));
    (c, a)
}

/// The still layers of a scene at one size: panorama and clouds.
struct Scene {
    land: Pano,
    clouds: Pano,
}

impl Scene {
    fn build(scene: u8, w: u32, h: u32) -> Self {
        let r = Res::new(w as usize, h as usize);
        let pw = (w as f32 * PANO_W).ceil() as usize;
        let h = h as usize;
        if scene == 0 {
            Scene {
                land: Pano::build(pw, h, 0.0..2.0, |u, v| scene_a(u, v, &r)),
                clouds: Pano::build(pw, h, 0.0..A_HORIZON, |u, v| a_clouds(u, v, &r)),
            }
        } else {
            Scene {
                land: Pano::build(pw, h, 0.0..2.0, |u, v| scene_b(u, v, &r)),
                clouds: Pano::build(pw, h, 0.0..B_HORIZON, |u, v| b_clouds(u, v, &r)),
            }
        }
    }
}

/// Draws one frame of a scene from its still layers.
fn scene_frame(scene: u8, layers: &Scene, w: u32, h: u32, secs: f32) -> VideoFrame {
    let t = secs.clamp(0.0, MAX_SECONDS);
    let (w, h) = (w as usize, h as usize);
    let r = Res::new(w, h);
    let (pano, clouds) = (&layers.land, &layers.clouds);
    let px_per_u = pano.w as f32 / PANO_W;
    // Pan offset in frame widths.
    let pan = if scene == 0 { 0.02 + 0.035 * t } else { 0.4 - 0.032 * t };
    let grain_seed = (secs * 30.0).round() as u32;
    // Film grain is a full-size detail; small frames (cards) get little.
    let grain = 0.028 * (h as f32 / 1080.0).clamp(0.15, 1.0);
    let mut data = vec![0u8; w * h * 4];
    par_rows(&mut data, w * 4, |y, row| {
        let v = (y as f32 + 0.5) / h as f32;
        let py = pano.row_of(v);
        for x in 0..w {
            let fu = (x as f32 + 0.5) / w as f32;
            let u = fu + pan;
            let mut c = if scene == 0 {
                let lake = v > A_HORIZON;
                let mut sx = u * px_per_u;
                let z = 1.0 / (v - A_HORIZON + 0.03);
                if lake {
                    let wob = (z * 38.0 - t * 2.6 + u * 7.0).sin() * 0.6 + (z * 91.0 + t * 4.1 + u * 23.0).sin() * 0.4;
                    sx += wob * (v - A_HORIZON) * 0.018 * px_per_u;
                }
                let (mut c, sky) = pano.at(sx, py);
                if sky > 0.0 {
                    // Clouds drift faster than the pan (parallax), mirrored in the lake.
                    let cv = if lake { A_HORIZON - (v - A_HORIZON) * 1.05 } else { v };
                    let (cc, a) = clouds.at(sx + 0.03 * t * px_per_u, clouds.row_of(cv));
                    let cc = if lake { mix(scale(cc, 0.7), [0.03, 0.12, 0.19], 0.3) } else { cc };
                    c = mix(c, cc, a * sky * if lake { 0.75 } else { 1.0 });
                }
                if lake {
                    c = scale(c, 1.0 + 0.06 * (z * 60.0 + t * 5.0 + (u * 30.0).sin() * 2.0).sin());
                }
                c
            } else {
                let sea = v > B_HORIZON;
                let mut sx = u * px_per_u;
                if sea {
                    sx += (v * 300.0 + t * 2.0).sin() * (v - B_HORIZON) * 0.01 * px_per_u;
                }
                let (mut c, m) = pano.at(sx, py);
                if !sea && m > 0.0 {
                    let (cc, a) = clouds.at(sx + 0.015 * t * px_per_u, clouds.row_of(v));
                    c = mix(c, cc, a * m);
                } else if sea && m > 0.0 {
                    // The sun's glitter path: wave crests catch the light.
                    let wpath = 0.03 + (v - B_HORIZON) * 0.45;
                    let g = smoothstep(wpath, 0.0, (u - B_SUN.0).abs());
                    let s = smoothstep(0.55, 0.85, m + 0.18 * (t * 5.0 + u * 90.0 + v * 260.0).sin());
                    c = mix(c, [1.0, 0.86, 0.56], g * s * 0.85);
                }
                // Three birds gliding left.
                if v < 0.45 {
                    for (k, (y0, sp)) in [(0.2, 0.05), (0.26, 0.045), (0.16, 0.055)].iter().enumerate() {
                        let bx = 0.95 - sp * t - k as f32 * 0.07;
                        let flap = 0.5 + 0.5 * (t * 9.0 + k as f32 * 2.0).sin();
                        let dx = ((fu - bx) * r.aspect).abs();
                        let wing = 0.022;
                        if dx < wing + r.px {
                            let yy = -dx * (0.25 + 0.6 * flap);
                            let thick = 0.0035 * (1.0 - 0.6 * dx / wing);
                            let k = r.cover(thick - (v - y0 - yy).abs()) * r.cover(wing - dx);
                            c = mix(c, [0.15, 0.05, 0.12], k);
                        }
                    }
                }
                c
            };
            // Lens vignette and a light film grain.
            let (dx, dy) = (fu - 0.5, v - 0.5);
            let vig = 1.0 - 0.44 * (dx * dx + dy * dy);
            let g = (hash2(x as i32, y as i32, grain_seed) - 0.5) * grain;
            c = [c[0] * vig + g, c[1] * vig + g, c[2] * vig + g];
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
    panos: Mutex<HashMap<(u8, u32, u32), Arc<Scene>>>,
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
            panos.entry((scene, w, h)).or_insert_with(|| Arc::new(Scene::build(scene, w, h))).clone()
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

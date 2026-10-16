//! Video stabilization analysis: global camera motion estimated from the
//! frames a clip plays, smoothed, and turned into per-frame corrections for
//! the `jb.stabilize` effect of jb-render.
//!
//! Frames are decoded at a low resolution, converted to luma and matched
//! coarse to fine (block matching on a 3-level pyramid with a sub-pixel
//! parabolic refinement) to get the translation between consecutive frames.
//! The accumulated trajectory is smoothed with a gaussian whose width
//! follows `smoothness`; the correction of a frame is `smoothed - raw`, the
//! shift that moves the image back onto the smooth path.
//!
//! The result is serialized as `"<fps>|dx,dy;dx,dy;..."`: one entry per
//! timeline frame from clip-local time 0 at the project frame rate, `dx`
//! and `dy` in fractions of the frame width and height (positive moves the
//! image right / down), 4 decimals.

use crate::video::{DecodeMode, VideoDecoder};
use anyhow::{Context as _, Result};
use jb_core::frame::{PixelFormat, VideoFrame};
use jb_core::time::{Ticks, TICKS_PER_SECOND};
use jb_core::{ClipSource, Project};
use std::path::Path;

/// Analysis width in pixels.
const WIDTH: usize = 192;

/// Grayscale image, row-major.
#[derive(Clone)]
struct Gray {
    w: usize,
    h: usize,
    px: Vec<f32>,
}

impl Gray {
    fn at(&self, x: usize, y: usize) -> f32 {
        self.px[y * self.w + x]
    }

    fn half(&self) -> Gray {
        let (w, h) = (self.w / 2, self.h / 2);
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let s = self.at(2 * x, 2 * y)
                    + self.at(2 * x + 1, 2 * y)
                    + self.at(2 * x, 2 * y + 1)
                    + self.at(2 * x + 1, 2 * y + 1);
                px.push(s * 0.25);
            }
        }
        Gray { w, h, px }
    }
}

/// Luma of a decoded frame, box-resampled to `WIDTH` columns.
fn luma(f: &VideoFrame) -> Gray {
    let (sw, sh) = (f.width as usize, f.height as usize);
    let src = |x: usize, y: usize| -> f32 {
        match f.format {
            PixelFormat::Rgba8 => {
                let i = y * f.stride + x * 4;
                let d = &f.data;
                0.2126 * d[i] as f32 + 0.7152 * d[i + 1] as f32 + 0.0722 * d[i + 2] as f32
            }
            PixelFormat::Nv12 => f.data[y * f.stride + x] as f32,
            PixelFormat::P010 => {
                let i = y * f.stride + x * 2;
                u16::from_le_bytes([f.data[i], f.data[i + 1]]) as f32 / 256.0
            }
        }
    };
    let w = WIDTH.min(sw).max(8);
    let h = ((sh * w) as f64 / sw as f64).round().max(8.0) as usize;
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        let (y0, y1) = (y * sh / h, ((y + 1) * sh / h).max(y * sh / h + 1));
        for x in 0..w {
            let (x0, x1) = (x * sw / w, ((x + 1) * sw / w).max(x * sw / w + 1));
            let mut s = 0.0;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    s += src(xx, yy);
                }
            }
            px.push(s / ((y1 - y0) * (x1 - x0)) as f32);
        }
    }
    Gray { w, h, px }
}

/// Mean absolute difference between `a` and `b` shifted by (dx, dy) over the
/// central region (`margin` pixels in from every edge).
fn sad(a: &Gray, b: &Gray, dx: i32, dy: i32, margin: usize) -> f32 {
    let mut s = 0.0;
    let mut n = 0usize;
    for y in margin..a.h - margin {
        let by = y as i32 + dy;
        for x in margin..a.w - margin {
            let bx = x as i32 + dx;
            s += (a.at(x, y) - b.at(bx as usize, by as usize)).abs();
            n += 1;
        }
    }
    s / n.max(1) as f32
}

/// Best integer shift of `b` against `a` within `range` around `(cx, cy)`.
fn search(a: &Gray, b: &Gray, cx: i32, cy: i32, range: i32, margin: usize) -> (i32, i32, [f32; 4], f32) {
    let lim = margin as i32 - 1;
    let mut best = (cx, cy, f32::MAX);
    for dy in (cy - range).max(-lim)..=(cy + range).min(lim) {
        for dx in (cx - range).max(-lim)..=(cx + range).min(lim) {
            let e = sad(a, b, dx, dy, margin);
            if e < best.2 {
                best = (dx, dy, e);
            }
        }
    }
    let (bx, by, e) = best;
    let at = |dx: i32, dy: i32| {
        if dx.abs() <= lim && dy.abs() <= lim {
            sad(a, b, dx, dy, margin)
        } else {
            e
        }
    };
    (bx, by, [at(bx - 1, by), at(bx + 1, by), at(bx, by - 1), at(bx, by + 1)], e)
}

/// Sub-pixel offset of a parabola through (-1, l), (0, c), (1, r).
fn parabola(l: f32, c: f32, r: f32) -> f32 {
    let d = l - 2.0 * c + r;
    if d <= 1e-6 {
        0.0
    } else {
        (0.5 * (l - r) / d).clamp(-0.5, 0.5)
    }
}

/// Translation (in pixels of `a`) that maps the content of `a` onto `b`:
/// content at (x, y) in `a` appears at (x + dx, y + dy) in `b`.
fn motion(a: &Gray, b: &Gray) -> (f32, f32) {
    let pa = [a.clone(), a.half(), a.half().half()];
    let pb = [b.clone(), b.half(), b.half().half()];
    let (mut cx, mut cy) = (0i32, 0i32);
    let mut result = (0.0, 0.0);
    for level in (0..3).rev() {
        let (la, lb) = (&pa[level], &pb[level]);
        let margin = (la.w.min(la.h) / 5).max(3);
        let range = if level == 2 { (margin as i32 - 1).min(8) } else { 2 };
        let (bx, by, n, e) = search(la, lb, cx, cy, range, margin);
        if level == 0 {
            result = (bx as f32 + parabola(n[0], e, n[1]), by as f32 + parabola(n[2], e, n[3]));
        } else {
            cx = bx * 2;
            cy = by * 2;
        }
    }
    result
}

fn gaussian_smooth(x: &[f64], sigma: f64) -> Vec<f64> {
    if sigma < 0.5 || x.len() < 2 {
        return x.to_vec();
    }
    let r = (sigma * 3.0).ceil() as i64;
    let n = x.len() as i64;
    (0..n)
        .map(|i| {
            let (mut s, mut w) = (0.0, 0.0);
            for k in -r..=r {
                // Mirror at the ends so the path does not drift there.
                let mut j = i + k;
                if j < 0 {
                    j = -j;
                }
                if j >= n {
                    j = 2 * (n - 1) - j;
                }
                let j = j.clamp(0, n - 1);
                let g = (-0.5 * (k as f64 / sigma).powi(2)).exp();
                s += g * x[j as usize];
                w += g;
            }
            s / w
        })
        .collect()
}

/// Per-frame corrections (fractions of width / height) for the frames shown
/// at `times` (source times of `path`).
pub fn analyze(path: &Path, times: &[Ticks], smoothness: f64) -> Result<Vec<[f64; 2]>> {
    let mut dec = VideoDecoder::open(path, false)?;
    let mut frames: Vec<Gray> = Vec::with_capacity(times.len());
    for &t in times {
        let f = dec
            .frame_at(t, Some((WIDTH as u32 * 2, WIDTH as u32 * 2)), DecodeMode::Accurate)?
            .with_context(|| format!("no frame at {t}"))?;
        frames.push(luma(&f));
    }
    if frames.is_empty() {
        return Ok(Vec::new());
    }
    let (w, h) = (frames[0].w as f64, frames[0].h as f64);
    let mut path_x = vec![0.0f64; frames.len()];
    let mut path_y = vec![0.0f64; frames.len()];
    for i in 1..frames.len() {
        let (dx, dy) = if frames[i].w == frames[i - 1].w && frames[i].h == frames[i - 1].h {
            motion(&frames[i - 1], &frames[i])
        } else {
            (0.0, 0.0)
        };
        path_x[i] = path_x[i - 1] + dx as f64;
        path_y[i] = path_y[i - 1] + dy as f64;
    }
    // Smoothness 0..100 -> gaussian sigma of 0..~1.5 s at 30 fps.
    let sigma = smoothness.clamp(0.0, 100.0) / 100.0 * 45.0;
    let sx = gaussian_smooth(&path_x, sigma);
    let sy = gaussian_smooth(&path_y, sigma);
    Ok((0..frames.len()).map(|i| [(sx[i] - path_x[i]) / w, (sy[i] - path_y[i]) / h]).collect())
}

/// Serializes corrections as the `data` parameter of `jb.stabilize`.
pub fn encode(fps: f64, corrections: &[[f64; 2]]) -> String {
    let list: Vec<String> = corrections.iter().map(|c| format!("{:.4},{:.4}", c[0], c[1])).collect();
    format!("{}|{}", (fps * 1000.0).round() / 1000.0, list.join(";"))
}

/// Analyzes the clip `clip_id` of `project` at the project frame rate and
/// returns the `jb.stabilize` `data` string.
pub fn analyze_clip(project: &Project, clip_id: &str, smoothness: f64) -> Result<String> {
    let clip = project.clip(clip_id).with_context(|| format!("clip {clip_id} not found"))?;
    let ClipSource::Media { media_id } = &clip.source else { anyhow::bail!("clip {clip_id} does not play a video") };
    let media = project.media(media_id).with_context(|| format!("media {media_id} not found"))?;
    if media.kind != jb_core::MediaKind::Video {
        anyhow::bail!("only video clips can be stabilized");
    }
    let rate = project.settings.frame_rate;
    let fps = rate.num as f64 / rate.den as f64;
    let n = ((clip.duration() as f64 * fps / TICKS_PER_SECOND as f64).ceil() as usize).max(1);
    let times: Vec<Ticks> = (0..n)
        .map(|k| {
            let local = (k as f64 * TICKS_PER_SECOND as f64 / fps).round() as Ticks;
            jb_core::eval::source_time(clip, local.min(clip.duration() - 1))
        })
        .collect();
    let corrections = analyze(Path::new(&media.path), &times, smoothness)?;
    Ok(encode(fps, &corrections))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smooth value noise (8 px cells), shifted by (ox, oy).
    fn pattern(w: usize, h: usize, ox: f32, oy: f32) -> Gray {
        let hash = |x: i64, y: i64| -> f32 {
            let mut v = (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) as u64;
            v ^= v >> 13;
            v = v.wrapping_mul(0x5bd1_e995);
            v ^= v >> 15;
            (v % 1000) as f32 / 1000.0
        };
        let noise = |x: f32, y: f32| -> f32 {
            let (gx, gy) = (x / 8.0, y / 8.0);
            let (ix, iy) = (gx.floor() as i64, gy.floor() as i64);
            let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
            let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
            let a = hash(ix, iy) + (hash(ix + 1, iy) - hash(ix, iy)) * sx;
            let b = hash(ix, iy + 1) + (hash(ix + 1, iy + 1) - hash(ix, iy + 1)) * sx;
            a + (b - a) * sy
        };
        let px = (0..h)
            .flat_map(|y| (0..w).map(move |x| 255.0 * noise(x as f32 - ox + 100.0, y as f32 - oy + 100.0)))
            .collect();
        Gray { w, h, px }
    }

    #[test]
    fn finds_translation() {
        let a = pattern(192, 108, 0.0, 0.0);
        for (dx, dy) in [(3.0, -2.0), (-7.0, 5.0), (12.0, 0.0), (0.5, -1.5)] {
            let b = pattern(192, 108, dx, dy);
            let (x, y) = motion(&a, &b);
            assert!((x - dx).abs() < 0.35 && (y - dy).abs() < 0.35, "{dx},{dy} -> {x},{y}");
        }
    }

    #[test]
    fn smoothing_removes_jitter() {
        let raw: Vec<f64> = (0..100).map(|i| if i % 2 == 0 { 2.0 } else { -2.0 }).collect();
        let s = gaussian_smooth(&raw, 10.0);
        assert!(s.iter().all(|v| v.abs() < 0.3));
        assert_eq!(encode(29.97, &[[0.01, -0.02]]), "29.97|0.0100,-0.0200");
    }
}

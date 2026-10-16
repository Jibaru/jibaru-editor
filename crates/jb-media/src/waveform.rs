//! Waveform peaks for timeline display.

use crate::audio::{AudioDecoder, CHANNELS};
use anyhow::{Context as _, Result};
use jb_core::time::{Ticks, TICKS_PER_SECOND};
use std::io::{Read, Write};
use std::path::Path;

/// Peaks per second stored for every asset. The UI downsamples on the fly
/// with [`Waveform::buckets`].
pub const PEAKS_PER_SECOND: u32 = 200;
/// Rate used to decode audio for peak extraction (fast and plenty for display).
const ANALYSIS_RATE: u32 = 24_000;

const MAGIC: &[u8; 8] = b"JBWAVE01";

/// Min/max envelope of the mono downmix, `rate` pairs per second.
#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    pub rate: u32,
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

impl Waveform {
    pub fn duration(&self) -> Ticks {
        self.min.len() as i64 * TICKS_PER_SECOND / self.rate as i64
    }

    /// Reduces the source time range `[start, end)` to `count` (min, max)
    /// buckets for drawing.
    pub fn buckets(&self, start: Ticks, end: Ticks, count: usize) -> Vec<[f32; 2]> {
        let mut out = vec![[0.0f32, 0.0f32]; count];
        if count == 0 || end <= start || self.min.is_empty() {
            return out;
        }
        let to_idx = |t: Ticks| (t as f64 * self.rate as f64 / TICKS_PER_SECOND as f64).max(0.0);
        let (a, b) = (to_idx(start), to_idx(end));
        let step = (b - a) / count as f64;
        for (k, o) in out.iter_mut().enumerate() {
            let i0 = (a + step * k as f64).floor() as usize;
            let i1 = ((a + step * (k + 1) as f64).ceil() as usize).max(i0 + 1).min(self.min.len());
            if i0 >= self.min.len() {
                break;
            }
            let (mut lo, mut hi) = (0.0f32, 0.0f32);
            for i in i0..i1 {
                lo = lo.min(self.min[i]);
                hi = hi.max(self.max[i]);
            }
            *o = [lo, hi];
        }
        out
    }

    /// Decodes the audio of `path` and computes its peaks.
    pub fn compute(path: &Path) -> Result<Self> {
        let mut dec = AudioDecoder::open(path, ANALYSIS_RATE)?;
        let per_peak = (ANALYSIS_RATE / PEAKS_PER_SECOND) as usize;
        let chunk = per_peak * 256;
        let (mut min, mut max) = (Vec::new(), Vec::new());
        let mut s = 0i64;
        let duration = crate::probe_info(path)?.duration;
        let total = crate::audio::ticks_to_samples(duration, ANALYSIS_RATE);
        while s < total {
            let buf = dec.read_samples(s, chunk)?;
            for frame in buf.chunks(per_peak * CHANNELS) {
                let (mut lo, mut hi) = (0.0f32, 0.0f32);
                for p in frame.chunks(CHANNELS) {
                    let m = 0.5 * (p[0] + p[1]);
                    lo = lo.min(m);
                    hi = hi.max(m);
                }
                min.push(lo);
                max.push(hi);
            }
            s += chunk as i64;
        }
        let keep = crate::audio::ticks_to_samples(duration, PEAKS_PER_SECOND).max(0) as usize;
        min.truncate(keep.max(1));
        max.truncate(keep.max(1));
        Ok(Self { rate: PEAKS_PER_SECOND, min, max })
    }

    /// Binary format: magic, u32 rate, u64 count, then i16 (min, max) pairs.
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut bytes = Vec::with_capacity(20 + self.min.len() * 4);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.rate.to_le_bytes());
        bytes.extend_from_slice(&(self.min.len() as u64).to_le_bytes());
        let q = |v: f32| ((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes();
        for (a, b) in self.min.iter().zip(&self.max) {
            bytes.extend_from_slice(&q(*a));
            bytes.extend_from_slice(&q(*b));
        }
        let tmp = path.with_extension("tmp");
        std::fs::File::create(&tmp)?.write_all(&bytes)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.read_to_end(&mut bytes)?;
        anyhow::ensure!(bytes.len() >= 20 && &bytes[..8] == MAGIC, "not a waveform file");
        let rate = u32::from_le_bytes(bytes[8..12].try_into()?);
        let n = u64::from_le_bytes(bytes[12..20].try_into()?) as usize;
        let body = bytes.get(20..20 + n * 4).context("truncated waveform file")?;
        let (mut min, mut max) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for c in body.as_chunks::<4>().0 {
            min.push(i16::from_le_bytes([c[0], c[1]]) as f32 / 32767.0);
            max.push(i16::from_le_bytes([c[2], c[3]]) as f32 / 32767.0);
        }
        Ok(Self { rate, min, max })
    }
}

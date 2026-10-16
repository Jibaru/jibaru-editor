//! Pitch-preserving time stretch and pitch shift (WSOLA, our own
//! implementation).
//!
//! Output is a sum of Hann-windowed grains at a fixed hop on the absolute
//! output timeline, so any output range can be rendered independently (the
//! mixer works in small blocks). Grain `k` starts at output sample `k * HOP`
//! and reads the source from where the timeline maps that instant, played
//! at the pitch ratio, shifted by up to `SEARCH` samples to line its
//! waveform up with the natural continuation of grain `k - 1` (waveform
//! similarity overlap-add). Rendered grains are cached per thread, so
//! consecutive blocks reuse them.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub const CHANNELS: usize = 2;
/// Grain hop in output samples; grains are twice as long (50% overlap).
pub const HOP: i64 = 1024;
const GRAIN: usize = 2 * HOP as usize;
/// Maximum alignment shift, in source samples.
const SEARCH: i64 = 384;
/// Length of the compared overlap, in samples.
const COMPARE: usize = 512;

/// Reads interleaved stereo source samples `[start, start + len)` (sample
/// indices at the output rate; negative or past-the-end ranges read silence).
pub trait SampleReader {
    fn read(&self, start: i64, len: usize) -> Result<Vec<f32>>;
    /// Identity of the source for the grain cache.
    fn key(&self) -> &str;
}

/// How a voice maps output samples to source samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapping {
    /// Source sample position (fractional) at output sample `origin`.
    pub source_at_origin: f64,
    pub origin: i64,
    /// Source samples consumed per output sample (> 0).
    pub rate: f64,
    /// Pitch ratio of the grain content (1 = original pitch).
    pub pitch: f64,
    pub reverse: bool,
}

impl Mapping {
    fn source_at(&self, out: i64) -> f64 {
        let d = (out - self.origin) as f64 * self.rate;
        if self.reverse {
            self.source_at_origin - d
        } else {
            self.source_at_origin + d
        }
    }

    fn dir(&self) -> f64 {
        if self.reverse {
            -1.0
        } else {
            1.0
        }
    }
}

fn hann(i: usize) -> f32 {
    (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / GRAIN as f64).cos()) as f32
}

/// Source samples around `from` read at `step` per index: returns a
/// closure-friendly buffer and its first sample index.
struct Span {
    first: i64,
    data: Vec<f32>,
}

impl Span {
    fn read(r: &dyn SampleReader, lo: f64, hi: f64) -> Result<Span> {
        let first = lo.floor() as i64 - 1;
        let len = (hi.ceil() as i64 - first + 3).max(1) as usize;
        Ok(Span { first, data: r.read(first, len)? })
    }

    /// Linearly interpolated frame at fractional source position `p`.
    fn at(&self, p: f64, out: &mut [f32; 2]) {
        let x = p - self.first as f64;
        let i0 = x.floor();
        let frac = (x - i0) as f32;
        let n = self.data.len() / CHANNELS;
        let i0 = (i0.max(0.0) as usize).min(n.saturating_sub(1));
        let i1 = (i0 + 1).min(n.saturating_sub(1));
        for c in 0..CHANNELS {
            let a = self.data[i0 * CHANNELS + c];
            let b = self.data[i1 * CHANNELS + c];
            out[c] = a + (b - a) * frac;
        }
    }

    fn mono(&self, p: f64) -> f32 {
        let mut f = [0.0; 2];
        self.at(p, &mut f);
        0.5 * (f[0] + f[1])
    }
}

/// Grains followed back when an alignment chain has to be rebuilt.
const CHAIN: i64 = 48;

fn key_of(r: &dyn SampleReader, m: &Mapping, k: i64) -> GrainKey {
    let anchor = m.source_at(k * HOP);
    (r.key().to_string(), k, anchor.to_bits(), m.rate.to_bits(), m.pitch.to_bits(), m.reverse)
}

/// Shift of grain `k`: aligned against the continuation of grain `k - 1`
/// with that grain's own shift, so the chain matches what was rendered.
/// Shifts are cached; a missing chain restarts `CHAIN` grains back.
fn alignment(r: &dyn SampleReader, m: &Mapping, k: i64) -> Result<f64> {
    let cached = |j: i64| SHIFTS.with(|c| c.borrow().get(&key_of(r, m, j)).copied());
    if let Some(d) = cached(k) {
        return Ok(d);
    }
    let mut j = k - 1;
    let mut prev = 0.0;
    while j > k - CHAIN {
        if let Some(d) = cached(j) {
            prev = d;
            break;
        }
        j -= 1;
    }
    for i in j + 1..=k {
        prev = align_one(r, m, i, prev)?;
        SHIFTS.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() > 1 << 14 {
                c.clear();
            }
            c.insert(key_of(r, m, i), prev);
        });
    }
    Ok(prev)
}

fn align_one(r: &dyn SampleReader, m: &Mapping, k: i64, prev: f64) -> Result<f64> {
    let p = m.pitch * m.dir();
    let prev_start = m.source_at((k - 1) * HOP) + prev;
    let start = m.source_at(k * HOP);
    let reference: Vec<f32> = {
        let a = prev_start + p * HOP as f64;
        let b = a + p * COMPARE as f64;
        let span = Span::read(r, a.min(b), a.max(b))?;
        (0..COMPARE).step_by(2).map(|i| span.mono(a + p * i as f64)).collect()
    };
    let energy: f32 = reference.iter().map(|x| x * x).sum();
    if energy < 1e-9 {
        return Ok(0.0);
    }
    let lo = start - SEARCH as f64;
    let hi = start + SEARCH as f64;
    let ext = p * COMPARE as f64;
    let span = Span::read(r, lo.min(lo + ext), hi.max(hi + ext))?;
    let mut best = (f32::MIN, 0.0);
    let mut d = -SEARCH;
    while d <= SEARCH {
        let s = start + d as f64;
        let (mut dot, mut e) = (0.0f32, 0.0f32);
        for (j, rv) in reference.iter().enumerate() {
            let c = span.mono(s + p * (2 * j) as f64);
            dot += c * rv;
            e += c * c;
        }
        let score = dot / (e.sqrt() + 1e-6);
        if score > best.0 {
            best = (score, d as f64);
        }
        d += 2;
    }
    Ok(best.1)
}

type GrainKey = (String, i64, u64, u64, u64, bool);

thread_local! {
    static GRAINS: std::cell::RefCell<HashMap<GrainKey, Arc<Vec<f32>>>> = std::cell::RefCell::new(HashMap::new());
    static SHIFTS: std::cell::RefCell<HashMap<GrainKey, f64>> = std::cell::RefCell::new(HashMap::new());
}

fn grain(r: &dyn SampleReader, m: &Mapping, k: i64) -> Result<Arc<Vec<f32>>> {
    let anchor = m.source_at(k * HOP);
    let key = key_of(r, m, k);
    if let Some(g) = GRAINS.with(|c| c.borrow().get(&key).cloned()) {
        return Ok(g);
    }
    let start = anchor + alignment(r, m, k)?;
    let p = m.pitch * m.dir();
    let end = start + p * GRAIN as f64;
    let span = Span::read(r, start.min(end), start.max(end))?;
    let mut out = vec![0f32; GRAIN * CHANNELS];
    let mut f = [0.0; 2];
    for i in 0..GRAIN {
        span.at(start + p * i as f64, &mut f);
        let w = hann(i);
        out[i * CHANNELS] = f[0] * w;
        out[i * CHANNELS + 1] = f[1] * w;
    }
    let g = Arc::new(out);
    GRAINS.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 512 {
            c.clear();
        }
        c.insert(key, g.clone());
    });
    Ok(g)
}

/// Renders output samples `[start, start + n)` of a stretched voice.
pub fn render(r: &dyn SampleReader, m: &Mapping, start: i64, n: usize) -> Result<Vec<f32>> {
    let mut out = vec![0f32; n * CHANNELS];
    let end = start + n as i64;
    let first = (start - GRAIN as i64 + 1).div_euclid(HOP);
    let last = (end - 1).div_euclid(HOP);
    for k in first..=last {
        let g0 = k * HOP;
        let a = start.max(g0);
        let b = end.min(g0 + GRAIN as i64);
        if a >= b {
            continue;
        }
        let g = grain(r, m, k)?;
        for j in a..b {
            let gi = (j - g0) as usize * CHANNELS;
            let oi = (j - start) as usize * CHANNELS;
            out[oi] += g[gi];
            out[oi + 1] += g[gi + 1];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sine source at `freq` cycles per sample.
    struct Sine(f64);
    impl SampleReader for Sine {
        fn read(&self, start: i64, len: usize) -> Result<Vec<f32>> {
            Ok((0..len)
                .flat_map(|i| {
                    let v = ((start + i as i64) as f64 * self.0 * std::f64::consts::TAU).sin() as f32;
                    [v, v]
                })
                .collect())
        }
        fn key(&self) -> &str {
            "sine"
        }
    }

    /// Dominant frequency (cycles/sample) by zero crossings.
    fn freq(x: &[f32]) -> f64 {
        let mono: Vec<f32> = x.chunks(2).map(|c| c[0]).collect();
        let crossings = mono.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        crossings as f64 / mono.len() as f64
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    #[test]
    fn stretch_keeps_pitch_and_level() {
        let src = Sine(440.0 / 48000.0);
        for rate in [0.5, 1.5, 2.0] {
            let m = Mapping { source_at_origin: 0.0, origin: 0, rate, pitch: 1.0, reverse: false };
            let out = render(&src, &m, 10_000, 9600).unwrap();
            let f = freq(&out) * 48000.0;
            assert!((f - 440.0).abs() < 8.0, "rate {rate}: {f} Hz");
            assert!((rms(&out) - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.1, "rate {rate}: rms {}", rms(&out));
        }
    }

    #[test]
    fn pitch_shift_changes_frequency_not_speed() {
        let src = Sine(440.0 / 48000.0);
        let semis: f64 = 12.0;
        let m = Mapping { source_at_origin: 0.0, origin: 0, rate: 1.0, pitch: 2f64.powf(semis / 12.0), reverse: false };
        let out = render(&src, &m, 4800, 9600).unwrap();
        let f = freq(&out) * 48000.0;
        assert!((f - 880.0).abs() < 15.0, "{f} Hz");
    }

    #[test]
    fn blocks_concatenate_seamlessly() {
        let src = Sine(300.0 / 48000.0);
        let m = Mapping { source_at_origin: 123.0, origin: 0, rate: 1.3, pitch: 1.0, reverse: false };
        let whole = render(&src, &m, 5000, 1024).unwrap();
        let mut parts = Vec::new();
        for i in 0..8 {
            parts.extend(render(&src, &m, 5000 + i * 128, 128).unwrap());
        }
        assert_eq!(whole, parts);
    }
}

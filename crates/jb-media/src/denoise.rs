//! Spectral noise reduction for voices with `AudioSettings::denoise_db`
//! (Filmora's AI Denoise, whose only parameter is an attenuation limit).
//!
//! A clean-room approximation: a stationary noise spectrum is estimated per
//! clip (a low percentile of short-time power per frequency bin around the
//! playhead) and every STFT frame is attenuated per bin by a power
//! subtraction gain floored at the attenuation limit. Output is computed in
//! chunks on a fixed timeline frame grid, so chunks join seamlessly and the
//! result does not depend on block boundaries.

use crate::audio::CHANNELS;
use anyhow::Result;
use realfft::num_complex::Complex32;
use realfft::{RealFftPlanner, RealToComplex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// FFT size and hop (75% overlap, sqrt-Hann analysis and synthesis).
const N: usize = 1024;
const HOP: usize = N / 4;
const BINS: usize = N / 2 + 1;
/// Output samples computed per cached chunk.
const CHUNK: usize = 8192;
/// Source span analyzed for the noise spectrum (samples on each side).
const PROFILE_SPAN: usize = 8 * 44_100;

/// Over-subtraction factor and noise percentile, chosen on Filmora exports of
/// denoised camera audio (docs/fidelity.md): Filmora removes most of the room
/// sound, so the "noise" is the median power, subtracted three times over.
/// `JB_DENOISE_BETA` / `JB_DENOISE_PCT` override them for calibration.
fn beta() -> f32 {
    std::env::var("JB_DENOISE_BETA").ok().and_then(|v| v.parse().ok()).unwrap_or(3.0)
}
/// Output gain (dB) of denoised voices: Filmora's denoised speech is a few
/// dB quieter than the input even where speech dominates (−2.5 dB on
/// average over 20 windows of Proyecto 2026-08-01, pre-gain levels −40 to
/// −20 dBFS).
/// `JB_DENOISE_GAIN` overrides it for calibration.
fn makeup_db() -> f32 {
    std::env::var("JB_DENOISE_GAIN").ok().and_then(|v| v.parse().ok()).unwrap_or(-2.5)
}
fn percentile() -> f32 {
    std::env::var("JB_DENOISE_PCT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.5)
}

struct Fft {
    fwd: Arc<dyn RealToComplex<f32>>,
    inv: Arc<dyn realfft::ComplexToReal<f32>>,
    window: Vec<f32>,
}

impl Fft {
    fn new() -> Self {
        let mut p = RealFftPlanner::<f32>::new();
        let window = (0..N).map(|i| (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos()).sqrt()).collect();
        Fft { fwd: p.plan_fft_forward(N), inv: p.plan_fft_inverse(N), window }
    }

    fn spectrum(&self, x: &[f32]) -> Vec<Complex32> {
        let mut buf: Vec<f32> = x.iter().zip(&self.window).map(|(a, w)| a * w).collect();
        let mut out = self.fwd.make_output_vec();
        self.fwd.process(&mut buf, &mut out).expect("fft");
        out
    }
}

/// Per-clip cache: noise spectrum (and where it was measured) and the last
/// processed chunk.
struct ClipState {
    profile_at: i64,
    noise: Vec<f32>,
    chunk_at: i64,
    chunk: Vec<f32>,
}

thread_local! {
    static FFT: Fft = Fft::new();
    static STATE: RefCell<HashMap<String, ClipState>> = Default::default();
}

/// Reads `len` stereo frames of the voice from timeline sample `at`.
pub(crate) type Reader<'a> = dyn Fn(i64, usize) -> Result<Vec<f32>> + 'a;

fn mid(stereo: &[f32]) -> Vec<f32> {
    stereo.chunks_exact(CHANNELS).map(|f| 0.5 * (f[0] + f[1])).collect()
}

fn noise_profile(fft: &Fft, read: &Reader<'_>, at: i64) -> Result<Vec<f32>> {
    let start = (at - PROFILE_SPAN as i64).max(0);
    let x = mid(&read(start, 2 * PROFILE_SPAN)?);
    let mut per_bin: Vec<Vec<f32>> = vec![Vec::new(); BINS];
    let mut s = 0;
    while s + N <= x.len() {
        let spec = fft.spectrum(&x[s..s + N]);
        for (b, c) in spec.iter().enumerate() {
            per_bin[b].push(c.norm_sqr());
        }
        s += N / 2;
    }
    let pct = percentile().clamp(0.0, 1.0);
    Ok(per_bin
        .into_iter()
        .map(|mut v| {
            if v.is_empty() {
                return 0.0;
            }
            v.sort_by(f32::total_cmp);
            v[((v.len() - 1) as f32 * pct) as usize]
        })
        .collect())
}

/// Denoised output for timeline samples `[c0, c0 + CHUNK)`.
fn process_chunk(fft: &Fft, read: &Reader<'_>, noise: &[f32], c0: i64, floor: f32) -> Result<Vec<f32>> {
    // Frames start on the global grid k * HOP; those overlapping the chunk,
    // plus one on each side for temporal smoothing of the power estimate.
    let first = (c0 - N as i64).div_euclid(HOP as i64) * HOP as i64 - HOP as i64;
    let last = (c0 + CHUNK as i64).div_euclid(HOP as i64) * HOP as i64 + HOP as i64;
    let span = (last - first) as usize + N;
    let raw = read(first, span)?;
    let frames = ((last - first) as usize) / HOP + 1;
    let mut specs: Vec<[Vec<Complex32>; 2]> = Vec::with_capacity(frames);
    let mut power: Vec<Vec<f32>> = Vec::with_capacity(frames);
    let mut ch = vec![0f32; N];
    for f in 0..frames {
        let off = f * HOP;
        let mut pair: [Vec<Complex32>; 2] = [Vec::new(), Vec::new()];
        for (c, slot) in pair.iter_mut().enumerate() {
            for i in 0..N {
                ch[i] = raw.get((off + i) * CHANNELS + c).copied().unwrap_or(0.0);
            }
            *slot = fft.spectrum(&ch);
        }
        power.push(pair[0].iter().zip(&pair[1]).map(|(l, r)| 0.25 * (l + r).norm_sqr()).collect());
        specs.push(pair);
    }
    let b = beta();
    let mut out = vec![0f32; CHUNK * CHANNELS];
    let mut time = vec![0f32; N];
    for f in 1..frames - 1 {
        let start = first + (f * HOP) as i64;
        if start + N as i64 <= c0 || start >= c0 + CHUNK as i64 {
            continue;
        }
        // Power smoothed over 3 frames and 5 bins.
        let gains: Vec<f32> = (0..BINS)
            .map(|k| {
                let (lo, hi) = (k.saturating_sub(2), (k + 2).min(BINS - 1));
                let mut p = 0.0;
                let mut nz = 0.0;
                for ff in f - 1..=f + 1 {
                    for kk in lo..=hi {
                        p += power[ff][kk];
                        nz += noise[kk];
                    }
                }
                if p <= 0.0 {
                    return floor;
                }
                (1.0 - b * nz / p).max(0.0).sqrt().max(floor)
            })
            .collect();
        for c in 0..CHANNELS {
            let mut spec: Vec<Complex32> = specs[f][c].iter().zip(&gains).map(|(s, g)| s * *g).collect();
            spec[0].im = 0.0;
            spec[BINS - 1].im = 0.0;
            fft.inv.process(&mut spec, &mut time).expect("ifft");
            for i in 0..N {
                let t = start + i as i64 - c0;
                if t >= 0 && (t as usize) < CHUNK {
                    // sqrt-Hann twice sums to 2 at 75% overlap; ifft is unnormalized.
                    out[t as usize * CHANNELS + c] += time[i] * fft.window[i] / (2.0 * N as f32);
                }
            }
        }
    }
    Ok(out)
}

/// Applies noise reduction to `n` frames of a voice starting at timeline
/// sample `at`. `read(at, len)` returns the voice's raw stereo samples.
pub(crate) fn denoise(key: &str, read: &Reader<'_>, at: i64, n: usize, limit_db: f64) -> Result<Vec<f32>> {
    let floor = 10f32.powf(-(limit_db as f32) / 20.0);
    let gain = 10f32.powf(makeup_db() / 20.0);
    FFT.with(|fft| {
        STATE.with(|st| {
            let mut st = st.borrow_mut();
            if st.len() > 64 {
                st.clear();
            }
            let stale = st.get(key).is_none_or(|s| (s.profile_at - at).abs() > 30 * 44_100);
            if stale {
                let noise = noise_profile(fft, read, at)?;
                st.insert(key.to_string(), ClipState { profile_at: at, noise, chunk_at: i64::MIN, chunk: Vec::new() });
            }
            let s = st.get_mut(key).unwrap();
            let mut out = Vec::with_capacity(n * CHANNELS);
            let mut pos = at;
            while pos < at + n as i64 {
                let c0 = pos.div_euclid(CHUNK as i64) * CHUNK as i64;
                if s.chunk_at != c0 {
                    s.chunk = process_chunk(fft, read, &s.noise, c0, floor)?;
                    s.chunk_at = c0;
                }
                let from = (pos - c0) as usize;
                let take = (CHUNK - from).min((at + n as i64 - pos) as usize);
                out.extend(s.chunk[from * CHANNELS..(from + take) * CHANNELS].iter().map(|v| v * gain));
                pos += take as i64;
            }
            Ok(out)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    /// Deterministic white noise plus, optionally, a tone one third of the time.
    fn signal(t: i64, tone: bool) -> f32 {
        let mut h = (t as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
        let noise = ((h % 20_000) as f32 / 10_000.0 - 1.0) * 0.05;
        let s = if tone && t >= 0 && (t / 14_700) % 3 == 1 { 0.5 * (t as f32 * 0.05).sin() } else { 0.0 };
        noise + s
    }

    fn run(tone: bool, key: &str) -> Vec<f32> {
        let read = move |t: i64, len: usize| -> Result<Vec<f32>> {
            Ok((0..len as i64).flat_map(|i| [signal(t + i, tone); 2]).collect())
        };
        denoise(key, &read, 44_100, 44_100, 15.0).unwrap()
    }

    #[test]
    fn stationary_noise_is_attenuated_to_the_limit() {
        let out = run(false, "noise");
        let input: Vec<f32> = (44_100..88_200).map(|t| signal(t, false)).collect();
        let db = 20.0 * (rms(&out) / rms(&input)).log10() - makeup_db();
        assert!(db < -10.0 && db > -16.0, "attenuation {db} dB");
    }

    #[test]
    fn a_loud_tone_survives() {
        let out = run(true, "tone");
        // Timeline samples 58800..73500 hold the tone (away from its edges).
        let seg = &out[(14_700 + 2048) * 2..(29_400 - 2048) * 2];
        let input: Vec<f32> = (58_800 + 2048..73_500 - 2048).flat_map(|t| [signal(t, true); 2]).collect();
        let db = 20.0 * (rms(seg) / rms(&input)).log10() - makeup_db();
        assert!(db > -1.5, "tone level change {db} dB");
    }
}

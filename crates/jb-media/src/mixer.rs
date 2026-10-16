//! Timeline audio mixer: renders the audible voices of a sequence (from
//! [`jb_core::eval::audio_voices`]) into interleaved f32 stereo.
//!
//! Gain (with fades) and pan are applied with per-block linear ramps so
//! keyframed volume does not zipper. Speed changes are rendered by
//! resampling (pitch follows speed); pitch-preserving time stretch is a
//! TODO.

use crate::audio::{samples_to_ticks, CHANNELS};
use crate::pool::MediaPool;
use anyhow::Result;
use jb_core::eval::{audio_voices, AudioVoice};
use jb_core::model::{Project, Sequence};
use jb_core::time::{Ticks, TICKS_PER_SECOND};

/// Anything that can supply decoded source audio at the mix rate.
pub trait AudioSource: Sync {
    /// `len` interleaved stereo frames of `media_id` from source time `start`.
    fn audio_samples(&self, media_id: &str, start: Ticks, len: usize) -> Result<Vec<f32>>;
    fn sample_rate(&self) -> u32;
}

impl AudioSource for MediaPool {
    fn audio_samples(&self, media_id: &str, start: Ticks, len: usize) -> Result<Vec<f32>> {
        MediaPool::audio_samples(self, media_id, start, len)
    }

    fn sample_rate(&self) -> u32 {
        self.config().sample_rate
    }
}

/// Samples per evaluation block (about 2.7 ms at 48 kHz): bounds the timing
/// error at clip edges and the step of parameter ramps.
const BLOCK: usize = 128;

fn tick_at(sample: i64, rate: u32) -> Ticks {
    let v = sample as i128 * TICKS_PER_SECOND as i128;
    let d = rate as i128;
    (if v >= 0 { (v + d / 2) / d } else { (v - d / 2) / d }) as Ticks
}

/// Balance pan law: center is unity on both sides.
fn pan_gains(pan: f64) -> (f32, f32) {
    let p = pan.clamp(-1.0, 1.0);
    ((1.0 - p).min(1.0) as f32, (1.0 + p).min(1.0) as f32)
}

/// Mixes `frames` stereo frames of `seq`, starting at timeline sample
/// `start_sample` (at the source's sample rate).
pub fn mix(project: &Project, seq: &Sequence, start_sample: i64, frames: usize, source: &dyn AudioSource) -> Result<Vec<f32>> {
    let rate = source.sample_rate();
    let mut out = vec![0f32; frames * CHANNELS];
    let mut next_voices: Option<Vec<AudioVoice>> = None;
    let mut off = 0usize;
    while off < frames {
        let n = BLOCK.min(frames - off);
        let s = start_sample + off as i64;
        let voices = next_voices.take().unwrap_or_else(|| audio_voices(project, seq, tick_at(s, rate)));
        let end_voices = audio_voices(project, seq, tick_at(s + n as i64, rate));
        for v in &voices {
            let end = end_voices.iter().find(|e| e.clip_id == v.clip_id && e.media_id == v.media_id);
            let (g0, g1) = (v.gain as f32, end.map_or(v.gain, |e| e.gain) as f32);
            if g0 == 0.0 && g1 == 0.0 {
                continue;
            }
            let (l0, r0) = pan_gains(v.pan);
            let (l1, r1) = pan_gains(end.map_or(v.pan, |e| e.pan));
            let reverse = project.clip(&v.clip_id).is_some_and(|c| c.speed.reverse);
            let src = voice_samples(source, v, n, rate, reverse)?;
            let dst = &mut out[off * CHANNELS..(off + n) * CHANNELS];
            for i in 0..n {
                let k = i as f32 / n as f32;
                let g = g0 + (g1 - g0) * k;
                let (pl, pr) = (l0 + (l1 - l0) * k, r0 + (r1 - r0) * k);
                dst[i * 2] += src[i * 2] * g * pl;
                dst[i * 2 + 1] += src[i * 2 + 1] * g * pr;
            }
        }
        next_voices = Some(end_voices);
        off += n;
    }
    for x in &mut out {
        *x = x.clamp(-1.0, 1.0);
    }
    Ok(out)
}

/// `n` output frames of one voice, resampled when its rate is not 1.
fn voice_samples(source: &dyn AudioSource, v: &AudioVoice, n: usize, rate: u32, reverse: bool) -> Result<Vec<f32>> {
    let r = v.rate.abs().max(1e-3);
    if (r - 1.0).abs() < 1e-9 && !reverse {
        return source.audio_samples(&v.media_id, v.source_time, n);
    }
    let p0 = v.source_time as f64 * rate as f64 / TICKS_PER_SECOND as f64;
    let span = r * n as f64;
    let (first, dir) = if reverse { ((p0 - span).floor() as i64 - 1, -1.0) } else { (p0.floor() as i64, 1.0) };
    let count = span.ceil() as usize + 3;
    let raw = source.audio_samples(&v.media_id, samples_to_ticks(first, rate), count)?;
    let mut out = vec![0f32; n * CHANNELS];
    for i in 0..n {
        let p = p0 + dir * r * i as f64 - first as f64;
        let i0 = p.floor().max(0.0) as usize;
        let frac = (p - p.floor()) as f32;
        let i1 = (i0 + 1).min(count - 1);
        let i0 = i0.min(count - 1);
        for c in 0..CHANNELS {
            let a = raw[i0 * CHANNELS + c];
            let b = raw[i1 * CHANNELS + c];
            out[i * CHANNELS + c] = a + (b - a) * frac;
        }
    }
    Ok(out)
}

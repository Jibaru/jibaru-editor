//! Timeline audio mixer: renders the audible voices of a sequence (from
//! [`jb_core::eval::audio_voices`]) into interleaved f32 stereo.
//!
//! Gain (with fades) and pan are applied with per-block linear ramps so
//! keyframed volume does not zipper. Speed changes keep the pitch
//! (WSOLA time stretch, `crate::stretch`) unless the clip opts out, in
//! which case they resample; pitch shift and the equalizer apply per voice.

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
pub fn mix(
    project: &Project,
    seq: &Sequence,
    start_sample: i64,
    frames: usize,
    source: &dyn AudioSource,
) -> Result<Vec<f32>> {
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
            let src = voice_block(source, v, s, n, rate)?;
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

struct VoiceReader<'a> {
    source: &'a dyn AudioSource,
    media_id: &'a str,
    key: String,
    rate: u32,
}

impl crate::stretch::SampleReader for VoiceReader<'_> {
    fn read(&self, start: i64, len: usize) -> Result<Vec<f32>> {
        if start >= 0 {
            return self.source.audio_samples(self.media_id, samples_to_ticks(start, self.rate), len);
        }
        let pad = ((-start) as usize).min(len);
        let mut out = vec![0f32; pad * CHANNELS];
        if len > pad {
            out.extend(self.source.audio_samples(self.media_id, 0, len - pad)?);
        }
        Ok(out)
    }
    fn key(&self) -> &str {
        &self.key
    }
}

/// `n` frames of a voice from timeline sample `at`, before EQ.
fn voice_raw(source: &dyn AudioSource, v: &AudioVoice, at: i64, n: usize, rate: u32) -> Result<Vec<f32>> {
    let r = v.rate.abs().max(1e-3);
    let shift = 2f64.powf(v.pitch / 12.0);
    let grain_pitch = if v.maintain_pitch { shift } else { r * shift };
    if (grain_pitch - r).abs() < 1e-9 {
        return voice_samples(source, v, n, rate, v.reverse);
    }
    // Canonical line through output sample 0 so consecutive blocks share grains.
    let src0 = v.source_time as f64 * rate as f64 / TICKS_PER_SECOND as f64;
    let d = r * at as f64;
    let origin = if v.reverse { src0 + d } else { src0 - d };
    let m = crate::stretch::Mapping {
        source_at_origin: (origin * 16.0).round() / 16.0,
        origin: 0,
        rate: r,
        pitch: grain_pitch,
        reverse: v.reverse,
    };
    let reader = VoiceReader { source, media_id: &v.media_id, key: format!("{}:{}", v.clip_id, v.media_id), rate };
    crate::stretch::render(&reader, &m, at, n)
}

/// Samples of equalizer warm-up when a voice's blocks are not contiguous.
const EQ_PREROLL: usize = 4096;

thread_local! {
    /// Equalizer state per clip: (next timeline sample, band gains, filter).
    static EQ_STATE: std::cell::RefCell<std::collections::HashMap<String, (i64, [f64; 10], crate::dsp::GraphicEq)>> =
        Default::default();
}

/// `n` processed frames of one voice starting at timeline sample `at`.
fn voice_block(source: &dyn AudioSource, v: &AudioVoice, at: i64, n: usize, rate: u32) -> Result<Vec<f32>> {
    let mut out = voice_raw(source, v, at, n, rate)?;
    let Some(bands) = v.equalizer else { return Ok(out) };
    let cached = EQ_STATE.with(|s| s.borrow_mut().remove(&v.clip_id)).filter(|(next, b, _)| *next == at && *b == bands);
    let mut eq = match cached {
        Some((_, _, eq)) => eq,
        None => {
            let mut eq = crate::dsp::GraphicEq::new(rate, &bands);
            let pre = EQ_PREROLL.min(at.max(0) as usize);
            if pre > 0 {
                let back = (v.rate.abs() * pre as f64 * TICKS_PER_SECOND as f64 / rate as f64) as Ticks;
                let earlier = AudioVoice {
                    source_time: if v.reverse { v.source_time + back } else { v.source_time - back },
                    ..v.clone()
                };
                let mut warm = voice_raw(source, &earlier, at - pre as i64, pre, rate)?;
                eq.process(&mut warm);
            }
            eq
        }
    };
    eq.process(&mut out);
    EQ_STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.len() > 256 {
            s.clear();
        }
        s.insert(v.clip_id.clone(), (at + n as i64, bands, eq));
    });
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

/// Integrated loudness (LUFS) of the source range a clip plays, before its
/// own gain. Used to fill in `Normalization::measured_lufs`.
pub fn clip_loudness(project: &Project, clip_id: &str, source: &dyn AudioSource) -> Result<f64> {
    use jb_core::model::ClipSource;
    let clip = project.clip(clip_id).ok_or_else(|| anyhow::anyhow!("clip {clip_id} not found"))?;
    let ClipSource::Media { media_id } = &clip.source else { anyhow::bail!("clip {clip_id} does not play media") };
    let rate = source.sample_rate();
    let start = crate::audio::ticks_to_samples(clip.source_in, rate);
    let span = crate::audio::ticks_to_samples(jb_core::speed::source_span(clip), rate).max(0) as usize;
    let mut meter = crate::dsp::LoudnessMeter::new(rate);
    let chunk = rate as usize * 5;
    let mut off = 0;
    while off < span {
        let n = chunk.min(span - off);
        meter.push(&source.audio_samples(media_id, samples_to_ticks(start + off as i64, rate), n)?);
        off += n;
    }
    Ok(meter.integrated())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jb_core::*;

    /// A 440 Hz tone at `amp` for every media id.
    struct Tone(f32);
    impl AudioSource for Tone {
        fn audio_samples(&self, _: &str, start: Ticks, len: usize) -> Result<Vec<f32>> {
            let s0 = crate::audio::ticks_to_samples(start, 48000);
            Ok((0..len)
                .flat_map(|i| {
                    let v = self.0 * ((s0 + i as i64) as f64 * 440.0 / 48000.0 * std::f64::consts::TAU).sin() as f32;
                    [v, v]
                })
                .collect())
        }
        fn sample_rate(&self) -> u32 {
            48000
        }
    }

    fn project(rate: f64, maintain: bool, pitch: f64) -> Project {
        let mut p = Project::new(
            "t",
            ProjectSettings {
                width: 16,
                height: 16,
                frame_rate: Rational::new(30, 1),
                sample_rate: 48000,
                background: [0.0; 4],
            },
        );
        p.media.push(MediaAsset {
            id: "m".into(),
            name: "m".into(),
            path: "/m.wav".into(),
            kind: MediaKind::Audio,
            duration: 100 * TICKS_PER_SECOND,
            width: 0,
            height: 0,
            frame_rate: None,
            has_audio: true,
            extra: Extra::Null,
        });
        let mut h = History::new(p);
        h.execute(&Command::AddTrack { sequence_id: None, kind: TrackKind::Audio, index: None }).unwrap();
        let t = h.project.main().tracks[0].id.clone();
        let mut c: Clip = serde_json::from_value(serde_json::json!({
            "id": "c", "name": "c", "source": {"type": "media", "media_id": "m"}, "start": 0, "end": 10 * TICKS_PER_SECOND
        }))
        .unwrap();
        c.speed.rate = rate;
        c.speed.maintain_pitch = maintain;
        c.audio.pitch = pitch;
        h.execute(&Command::AddClip { track_id: t, clip: c }).unwrap();
        h.project
    }

    fn crossings(x: &[f32]) -> f64 {
        let frames: Vec<&[f32]> = x.chunks(2).collect();
        let n = frames.windows(2).filter(|w| w[0][0] <= 0.0 && w[1][0] > 0.0).count();
        n as f64 / frames.len() as f64 * 48000.0
    }

    #[test]
    fn speed_keeps_pitch_unless_disabled() {
        for (rate, maintain, pitch, hz) in [(2.0, true, 0.0, 440.0), (2.0, false, 0.0, 880.0), (1.0, true, 12.0, 880.0)]
        {
            let p = project(rate, maintain, pitch);
            let out = mix(&p, p.main(), 48000, 24000, &Tone(0.5)).unwrap();
            let f = crossings(&out);
            assert!((f - hz).abs() < hz * 0.04, "rate {rate} maintain {maintain} pitch {pitch}: {f} Hz");
        }
    }

    #[test]
    fn normalization_and_equalizer_apply() {
        let mut p = project(1.0, true, 0.0);
        let lufs = clip_loudness(&p, "c", &Tone(0.5)).unwrap();
        assert!((-7.5..-5.5).contains(&lufs), "{lufs}");
        p.clip_mut("c").unwrap().audio.normalize =
            Some(Normalization { target_lufs: -14.0, measured_lufs: Some(lufs) });
        let out = mix(&p, p.main(), 48000, 48000, &Tone(0.5)).unwrap();
        let got = crate::dsp::integrated_loudness(&out, 48000);
        assert!((got + 14.0).abs() < 0.5, "{got}");
        p.clip_mut("c").unwrap().audio.equalizer = Equalizer::preset("low_bass");
        assert!(mix(&p, p.main(), 48000, 4800, &Tone(0.5)).is_ok());
    }
}

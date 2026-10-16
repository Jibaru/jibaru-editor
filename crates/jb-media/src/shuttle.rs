//! Audio for timeline playback at any speed or direction (J/K/L shuttle).
//!
//! The sequence is mixed at its normal speed by [`crate::mixer::mix`] and
//! that mix is then played at the shuttle rate: forward 1x is the mix
//! itself, reverse 1x reads it backwards sample by sample, and every other
//! speed is time-stretched with WSOLA ([`crate::stretch`]), so shuttling
//! keeps the original pitch (fast forward sounds like faster speech, not a
//! chipmunk; reverse grains play backwards). Output sample `k` corresponds to
//! timeline sample `origin + rate * k` (within the WSOLA search window of
//! ±8 ms), so the number of output samples played is a valid master clock.

use crate::audio::CHANNELS;
use crate::mixer::{mix, AudioSource};
use anyhow::Result;
use jb_core::model::{Project, Sequence};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Timeline samples mixed per cached chunk. Large chunks keep reverse
/// playback from seeking the decoders for every small read.
const CHUNK: i64 = 16384;
/// Chunks kept in the cache.
const MAX_CHUNKS: usize = 24;

/// Plays one sequence from timeline sample `origin` at `rate` (timeline
/// samples per output sample; negative plays backwards). Keep one per
/// play/seek: it caches the mix of the timeline around the playhead.
pub struct Shuttle {
    origin: i64,
    rate: f64,
    /// Identity of the timeline content; [`Shuttle::invalidate`] changes it.
    epoch: u64,
    chunks: RefCell<HashMap<i64, Arc<Vec<f32>>>>,
}

impl Shuttle {
    pub fn new(origin: i64, rate: f64) -> Self {
        Self { origin, rate: if rate == 0.0 { 1.0 } else { rate }, epoch: next_epoch(), chunks: Default::default() }
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Timeline sample heard at output sample `k`.
    pub fn timeline_sample(&self, k: i64) -> f64 {
        self.origin as f64 + self.rate * k as f64
    }

    /// Drops cached audio after the project changed; the time mapping stays.
    pub fn invalidate(&mut self) {
        self.chunks.get_mut().clear();
        self.epoch = next_epoch();
    }

    /// Output samples `[start, start + n)` as interleaved stereo. Meant to be
    /// called with consecutive ranges from output sample 0 (any range works,
    /// but grain alignment is chained from 0).
    pub fn render(
        &self,
        project: &Project,
        seq: &Sequence,
        source: &dyn AudioSource,
        start: i64,
        n: usize,
    ) -> Result<Vec<f32>> {
        let r = self.rate.abs();
        if (r - 1.0).abs() < 1e-9 {
            if self.rate > 0.0 {
                return mix(project, seq, self.origin + start, n, source);
            }
            // Timeline samples origin - start - (n - 1) ..= origin - start, reversed.
            let reader = Reader { shuttle: self, project, seq, source, key: String::new() };
            let fwd = reader.timeline(self.origin - start - n as i64 + 1, n)?;
            return Ok(fwd.chunks(CHANNELS).rev().flatten().copied().collect());
        }
        let reader = Reader { shuttle: self, project, seq, source, key: format!("shuttle:{}:{}", self.epoch, seq.id) };
        let m = crate::stretch::Mapping {
            source_at_origin: self.origin as f64,
            origin: 0,
            rate: r,
            pitch: 1.0,
            reverse: self.rate < 0.0,
            // Playback renders contiguously from output sample 0.
            chain_from: Some(0),
        };
        let mut out = crate::stretch::render(&reader, &m, start, n)?;
        for x in &mut out {
            *x = x.clamp(-1.0, 1.0);
        }
        Ok(out)
    }
}

/// Grain cache identity of a shuttle's timeline content (unique per process).
fn next_epoch() -> u64 {
    static EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    EPOCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

struct Reader<'a> {
    shuttle: &'a Shuttle,
    project: &'a Project,
    seq: &'a Sequence,
    source: &'a dyn AudioSource,
    key: String,
}

impl Reader<'_> {
    fn chunk(&self, index: i64) -> Result<Arc<Vec<f32>>> {
        if let Some(c) = self.shuttle.chunks.borrow().get(&index) {
            return Ok(c.clone());
        }
        let data = if index < 0 {
            vec![0.0; CHUNK as usize * CHANNELS]
        } else {
            mix(self.project, self.seq, index * CHUNK, CHUNK as usize, self.source)?
        };
        let data = Arc::new(data);
        let mut chunks = self.shuttle.chunks.borrow_mut();
        if chunks.len() >= MAX_CHUNKS {
            // Evict the chunk farthest from this one.
            if let Some(far) = chunks.keys().copied().max_by_key(|k| (k - index).abs()) {
                chunks.remove(&far);
            }
        }
        chunks.insert(index, data.clone());
        Ok(data)
    }

    /// Timeline samples `[start, start + len)` of the normal-speed mix.
    fn timeline(&self, start: i64, len: usize) -> Result<Vec<f32>> {
        let mut out = Vec::with_capacity(len * CHANNELS);
        let end = start + len as i64;
        let mut s = start;
        while s < end {
            let index = s.div_euclid(CHUNK);
            let from = s - index * CHUNK;
            let to = (end - index * CHUNK).min(CHUNK);
            let c = self.chunk(index)?;
            out.extend_from_slice(&c[from as usize * CHANNELS..to as usize * CHANNELS]);
            s = index * CHUNK + to;
        }
        Ok(out)
    }
}

impl crate::stretch::SampleReader for Reader<'_> {
    fn read(&self, start: i64, len: usize) -> Result<Vec<f32>> {
        self.timeline(start, len)
    }

    fn key(&self) -> &str {
        &self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jb_core::time::{Ticks, TICKS_PER_SECOND};
    use jb_core::*;

    /// A 440 Hz tone whose level rises with source time, so the direction
    /// of playback can be told from the output.
    struct Ramp;
    impl AudioSource for Ramp {
        fn audio_samples(&self, _: &str, start: Ticks, len: usize) -> Result<Vec<f32>> {
            let s0 = crate::audio::ticks_to_samples(start, 48000);
            Ok((0..len)
                .flat_map(|i| {
                    let s = (s0 + i as i64) as f64;
                    let v = (0.05 + 0.9 * s / (10.0 * 48000.0)) * (s * 440.0 / 48000.0 * std::f64::consts::TAU).sin();
                    [v as f32, v as f32]
                })
                .collect())
        }
        fn sample_rate(&self) -> u32 {
            48000
        }
    }

    fn project() -> Project {
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
            duration: 10 * TICKS_PER_SECOND,
            width: 0,
            height: 0,
            frame_rate: None,
            has_audio: true,
            extra: Extra::Null,
        });
        let mut h = History::new(p);
        h.execute(&Command::AddTrack { sequence_id: None, kind: TrackKind::Audio, index: None }).unwrap();
        let t = h.project.main().tracks[0].id.clone();
        let c: Clip = serde_json::from_value(serde_json::json!({
            "id": "c", "name": "c", "source": {"type": "media", "media_id": "m"}, "start": 0, "end": 10 * TICKS_PER_SECOND
        }))
        .unwrap();
        h.execute(&Command::AddClip { track_id: t, clip: c }).unwrap();
        h.project
    }

    fn freq(x: &[f32]) -> f64 {
        let mono: Vec<f32> = x.chunks(2).map(|c| c[0]).collect();
        mono.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f64 / mono.len() as f64 * 48000.0
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
    }

    /// Renders `secs` of output in 1024-frame blocks, like the playback engine.
    fn play(s: &Shuttle, p: &Project, secs: f64) -> Vec<f32> {
        let n = (secs * 48000.0) as usize;
        let mut out = Vec::new();
        let mut k = 0;
        while (k as usize) < n {
            out.extend(s.render(p, p.main(), &Ramp, k, 1024).unwrap());
            k += 1024;
        }
        out
    }

    #[test]
    fn every_shuttle_speed_is_audible_with_its_pitch_and_direction() {
        let p = project();
        let origin = 5 * 48000;
        for rate in [1.0, 2.0, 4.0, 8.0, 0.5, -1.0, -2.0, -4.0, -8.0] {
            let s = Shuttle::new(origin, rate);
            let out = play(&s, &p, 0.5);
            let f = freq(&out[4096..]);
            assert!((f - 440.0).abs() < 440.0 * 0.04, "rate {rate}: {f} Hz");
            // The ramp gets louder forward in time: the second half of the
            // output must be louder going forward, quieter going backward.
            let half = out.len() / 2;
            let (a, b) = (rms(&out[2048..half]), rms(&out[half..]));
            assert!(a > 0.1 && b > 0.1, "rate {rate}: silent ({a}, {b})");
            if rate.abs() >= 1.0 {
                assert_eq!(b > a, rate > 0.0, "rate {rate}: direction ({a} then {b})");
            }
        }
    }

    #[test]
    fn output_follows_the_timeline_at_the_shuttle_rate() {
        let p = project();
        let s = Shuttle::new(48000, -1.0);
        let out = s.render(&p, p.main(), &Ramp, 0, 4800).unwrap();
        let fwd = mix(&p, p.main(), 48000 - 4799, 4800, &Ramp).unwrap();
        let rev: Vec<f32> = fwd.chunks(2).rev().flatten().copied().collect();
        assert_eq!(out, rev);
        assert_eq!(Shuttle::new(48000, 4.0).timeline_sample(100), 48400.0);
        // Before the start of the timeline is silence, not an error.
        let s = Shuttle::new(1000, -8.0);
        let out = play(&s, &p, 0.2);
        assert!(rms(&out[out.len() / 2..]) < 1e-6);
    }
}

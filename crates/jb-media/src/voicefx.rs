//! Per-voice audio cleanup and voice filters (`AudioSettings::hum_removal`,
//! `hiss_removal` and `voice_filter`): plain local DSP, no AI.
//!
//! - Hum removal notches the mains frequency and its first three harmonics.
//! - Hiss removal lowers a high shelf above 6 kHz by up to 18 dB.
//! - Voice filters: band-limited `telephone`, `radio` (with a compressor) and
//!   `megaphone` (driven), a feedback `echo`, Freeverb-style `small_room` and
//!   `big_room`, a ring-modulated `robot`, a crushed `lofi` and a muffled,
//!   chorused `underwater`.
//!
//! A [`VoiceFx`] keeps its state (filters, delay lines) so consecutive blocks
//! of a voice join seamlessly; the mixer keeps one per clip.

use crate::dsp::Biquad;
use std::f64::consts::TAU;

/// Interleaved stereo processor for one voice.
pub struct VoiceFx {
    /// Cleanup filters applied to both channels in order.
    cleanup: Vec<Biquad>,
    filter: Option<Filter>,
    /// Samples processed so far (drives the modulators).
    n: u64,
    sr: f64,
}

enum Filter {
    /// Band limiting plus shaping: (filters, drive, compressor, makeup).
    Band {
        bq: Vec<Biquad>,
        drive: f64,
        comp: Option<Comp>,
        makeup: f64,
    },
    Echo {
        lines: [Delay; 2],
        fb: f64,
        wet: f64,
    },
    Reverb {
        rooms: [Freeverb; 2],
        wet: f64,
    },
    Robot {
        comb: [Delay; 2],
    },
    Lofi {
        hold: f64,
        phase: f64,
        held: [f64; 2],
        bq: Vec<Biquad>,
    },
    Underwater {
        bq: Vec<Biquad>,
        lines: [Delay; 2],
    },
}

struct Comp {
    env: f64,
    attack: f64,
    release: f64,
}

/// Circular delay line.
#[derive(Clone)]
struct Delay {
    buf: Vec<f64>,
    pos: usize,
}

impl Delay {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len.max(2)], pos: 0 }
    }
    /// Sample written `d` samples ago (fractional).
    fn tap(&self, d: f64) -> f64 {
        let len = self.buf.len();
        let d = d.clamp(1.0, (len - 1) as f64);
        let i = d.floor() as usize;
        let k = d - i as f64;
        let a = self.buf[(self.pos + len - i) % len];
        let b = self.buf[(self.pos + len - i - 1) % len];
        a + (b - a) * k
    }
    fn push(&mut self, x: f64) {
        self.pos = (self.pos + 1) % self.buf.len();
        self.buf[self.pos] = x;
    }
}

/// Freeverb (Jezar's public domain design): 8 damped combs, 4 allpasses.
struct Freeverb {
    combs: Vec<(Delay, f64)>,
    allpasses: Vec<Delay>,
    feedback: f64,
    damp: f64,
}

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];

impl Freeverb {
    fn new(sr: f64, room: f64, spread: usize) -> Self {
        let k = sr / 44100.0;
        let len = |n: usize| ((n + spread) as f64 * k).round() as usize;
        Self {
            combs: COMBS.iter().map(|n| (Delay::new(len(*n)), 0.0)).collect(),
            allpasses: ALLPASSES.iter().map(|n| Delay::new(len(*n))).collect(),
            feedback: 0.7 + 0.28 * room,
            damp: 0.4,
        }
    }
    fn process(&mut self, x: f64) -> f64 {
        let input = x * 0.015;
        let mut out = 0.0;
        for (d, store) in &mut self.combs {
            let y = d.buf[(d.pos + 1) % d.buf.len()];
            *store = y * (1.0 - self.damp) + *store * self.damp;
            d.push(input + *store * self.feedback);
            out += y;
        }
        for d in &mut self.allpasses {
            let b = d.buf[(d.pos + 1) % d.buf.len()];
            d.push(out + b * 0.5);
            out = b - out;
        }
        out
    }
}

fn soft(x: f64, drive: f64) -> f64 {
    if drive <= 1.0 {
        return x;
    }
    (x * drive).tanh() / drive.tanh()
}

impl VoiceFx {
    /// `None` when nothing is enabled. Unknown filter names are ignored.
    pub fn new(sr: u32, filter: Option<&str>, hum_hz: Option<f64>, hiss: Option<f64>) -> Option<Self> {
        let sr = sr as f64;
        let mut cleanup = Vec::new();
        if let Some(f) = hum_hz.filter(|f| *f > 0.0) {
            for h in 1..=4 {
                let hz = f * h as f64;
                if hz < sr * 0.45 {
                    cleanup.push(Biquad::notch(sr, hz, 30.0));
                }
            }
        }
        if let Some(s) = hiss.filter(|s| *s > 0.0) {
            cleanup.push(Biquad::high_shelf(sr, 6000.0, std::f64::consts::FRAC_1_SQRT_2, -0.18 * s.min(100.0)));
        }
        let q = std::f64::consts::FRAC_1_SQRT_2;
        let band = |lo: f64, hi: f64| {
            vec![
                Biquad::high_pass(sr, lo, q),
                Biquad::high_pass(sr, lo, q),
                Biquad::low_pass(sr, hi, q),
                Biquad::low_pass(sr, hi, q),
            ]
        };
        let filter = match filter.unwrap_or("") {
            "telephone" => Some(Filter::Band { bq: band(300.0, 3400.0), drive: 1.6, comp: None, makeup: 1.3 }),
            "radio" => Some(Filter::Band {
                bq: band(500.0, 5000.0),
                drive: 1.2,
                comp: Some(Comp { env: 0.0, attack: (-1.0 / (0.005 * sr)).exp(), release: (-1.0 / (0.12 * sr)).exp() }),
                makeup: 1.6,
            }),
            "megaphone" => {
                let mut bq = band(700.0, 3000.0);
                bq.push(Biquad::peaking(sr, 1500.0, 1.0, 6.0));
                Some(Filter::Band { bq, drive: 4.0, comp: None, makeup: 0.8 })
            }
            "echo" => {
                let n = (0.35 * sr) as usize + 2;
                Some(Filter::Echo { lines: [Delay::new(n), Delay::new(n)], fb: 0.35, wet: 0.35 })
            }
            "small_room" => {
                Some(Filter::Reverb { rooms: [Freeverb::new(sr, 0.45, 0), Freeverb::new(sr, 0.45, 23)], wet: 0.25 })
            }
            "big_room" => {
                Some(Filter::Reverb { rooms: [Freeverb::new(sr, 0.9, 0), Freeverb::new(sr, 0.9, 23)], wet: 0.45 })
            }
            "robot" => {
                let n = (0.012 * sr) as usize + 2;
                Some(Filter::Robot { comb: [Delay::new(n), Delay::new(n)] })
            }
            "lofi" => Some(Filter::Lofi {
                hold: (sr / 8000.0).max(1.0),
                phase: 0.0,
                held: [0.0; 2],
                bq: vec![Biquad::low_pass(sr, 4000.0, q), Biquad::low_pass(sr, 4000.0, q)],
            }),
            "underwater" => {
                let n = (0.03 * sr) as usize + 2;
                Some(Filter::Underwater {
                    bq: vec![Biquad::low_pass(sr, 600.0, q), Biquad::low_pass(sr, 600.0, q)],
                    lines: [Delay::new(n), Delay::new(n)],
                })
            }
            _ => None,
        };
        if cleanup.is_empty() && filter.is_none() {
            return None;
        }
        Some(Self { cleanup, filter, n: 0, sr })
    }

    /// Processes interleaved stereo samples in place.
    pub fn process(&mut self, x: &mut [f32]) {
        let sr = self.sr;
        for frame in x.chunks_exact_mut(2) {
            let t = self.n as f64 / sr;
            let mut s = [frame[0] as f64, frame[1] as f64];
            for (c, v) in s.iter_mut().enumerate() {
                for b in &mut self.cleanup {
                    *v = b.process(c, *v);
                }
            }
            match &mut self.filter {
                None => {}
                Some(Filter::Band { bq, drive, comp, makeup }) => {
                    for (c, v) in s.iter_mut().enumerate() {
                        for b in bq.iter_mut() {
                            *v = b.process(c, *v);
                        }
                    }
                    let mut g = 1.0;
                    if let Some(cp) = comp {
                        let lvl = s[0].abs().max(s[1].abs());
                        let k = if lvl > cp.env { cp.attack } else { cp.release };
                        cp.env = lvl + (cp.env - lvl) * k;
                        let thr = 0.1;
                        if cp.env > thr {
                            g = (thr / cp.env).powf(0.75);
                        }
                    }
                    for v in &mut s {
                        *v = soft(*v * g, *drive) * *makeup;
                    }
                }
                Some(Filter::Echo { lines, fb, wet }) => {
                    let d = 0.35 * sr;
                    for (c, v) in s.iter_mut().enumerate() {
                        let e = lines[c].tap(d);
                        lines[c].push(*v + e * *fb);
                        *v += e * *wet;
                    }
                }
                Some(Filter::Reverb { rooms, wet }) => {
                    let mono = (s[0] + s[1]) * 0.5;
                    for (c, v) in s.iter_mut().enumerate() {
                        let r = rooms[c].process(mono);
                        *v = *v * (1.0 - *wet * 0.5) + r * *wet * 3.0;
                    }
                }
                Some(Filter::Robot { comb }) => {
                    let m = (TAU * 30.0 * t).sin();
                    let d = 0.008 * sr;
                    for (c, v) in s.iter_mut().enumerate() {
                        let ring = *v * (0.3 + 0.7 * m);
                        let y = ring + comb[c].tap(d) * 0.5;
                        comb[c].push(y);
                        *v = y * 0.8;
                    }
                }
                Some(Filter::Lofi { hold, phase, held, bq }) => {
                    *phase += 1.0;
                    if *phase >= *hold {
                        *phase -= *hold;
                        *held = s;
                    }
                    for (c, v) in s.iter_mut().enumerate() {
                        let q = (held[c] * 127.0).round() / 127.0;
                        let mut y = q;
                        for b in bq.iter_mut() {
                            y = b.process(c, y);
                        }
                        *v = y;
                    }
                }
                Some(Filter::Underwater { bq, lines }) => {
                    for (c, v) in s.iter_mut().enumerate() {
                        let mut y = *v;
                        for b in bq.iter_mut() {
                            y = b.process(c, y);
                        }
                        let d = (0.015 + 0.005 * (TAU * 0.3 * t + c as f64 * 1.6).sin()) * sr;
                        let wet = lines[c].tap(d);
                        lines[c].push(y);
                        *v = (y + wet) * 0.7;
                    }
                }
            }
            frame[0] = s[0].clamp(-4.0, 4.0) as f32;
            frame[1] = s[1].clamp(-4.0, 4.0) as f32;
            self.n += 1;
        }
    }

    /// Warm-up length (samples) after a seek so filters and tails settle.
    pub fn preroll(filter: Option<&str>, sr: u32) -> usize {
        match filter {
            Some("echo" | "small_room" | "big_room") => sr as usize / 2,
            _ => 4096,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48000;

    fn tone(hz: f64, secs: f64) -> Vec<f32> {
        (0..(secs * SR as f64) as usize)
            .flat_map(|i| {
                let v = (0.3 * (TAU * hz * i as f64 / SR as f64).sin()) as f32;
                [v, v]
            })
            .collect()
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    #[test]
    fn every_filter_changes_the_signal() {
        let mut noise = Vec::new();
        let mut seed = 1u32;
        for _ in 0..SR as usize {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = ((seed >> 8) as f32 / (1 << 24) as f32 - 0.5) * 0.4;
            noise.extend([v, v]);
        }
        let src: Vec<f32> = tone(220.0, 1.0).iter().zip(&noise).map(|(a, b)| a + b).collect();
        for f in jb_core::VOICE_FILTERS {
            let mut fx = VoiceFx::new(SR, Some(f), None, None).unwrap_or_else(|| panic!("{f}"));
            let mut out = src.clone();
            // Two blocks: state carries over.
            let (a, b) = out.split_at_mut(src.len() / 2);
            fx.process(a);
            fx.process(b);
            assert!(out.iter().all(|v| v.is_finite()), "{f}");
            let diff: Vec<f32> = out.iter().zip(&src).map(|(a, b)| a - b).collect();
            assert!(rms(&diff) > 0.05 * rms(&src), "{f}: {} vs {}", rms(&diff), rms(&src));
            assert!(rms(&out) > 0.01, "{f} is silent");
        }
        assert!(VoiceFx::new(SR, Some("nope"), None, None).is_none());
    }

    #[test]
    fn hum_removal_notches_mains_only() {
        let run = |hz: f64| {
            let mut fx = VoiceFx::new(SR, None, Some(50.0), None).unwrap();
            let mut x = tone(hz, 2.0);
            fx.process(&mut x);
            let tail = &x[x.len() / 2..];
            20.0 * (rms(tail) / rms(&tone(hz, 1.0))).log10()
        };
        assert!(run(50.0) < -20.0, "50 Hz: {} dB", run(50.0));
        assert!(run(1000.0).abs() < 1.0, "1 kHz: {} dB", run(1000.0));
    }

    #[test]
    fn hiss_removal_lowers_highs() {
        let mut fx = VoiceFx::new(SR, None, None, Some(100.0)).unwrap();
        let mut hi = tone(12000.0, 1.0);
        fx.process(&mut hi);
        let db = 20.0 * (rms(&hi[hi.len() / 2..]) / rms(&tone(12000.0, 1.0))).log10();
        assert!(db < -12.0, "{db}");
        let mut fx = VoiceFx::new(SR, None, None, Some(100.0)).unwrap();
        let mut lo = tone(300.0, 1.0);
        fx.process(&mut lo);
        let db = 20.0 * (rms(&lo[lo.len() / 2..]) / rms(&tone(300.0, 1.0))).log10();
        assert!(db.abs() < 1.0, "{db}");
    }
}

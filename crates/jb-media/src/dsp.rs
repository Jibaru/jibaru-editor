//! Audio DSP for the mixer and analysis: biquad filters, the 10-band
//! graphic equalizer and ITU-R BS.1770 integrated loudness (LUFS).

/// A biquad in transposed direct form II, one state per channel.
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [[f64; 2]; 2],
}

impl Biquad {
    fn new(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Self {
        Self { b: [b0 / a0, b1 / a0, b2 / a0], a: [a1 / a0, a2 / a0], z: [[0.0; 2]; 2] }
    }

    /// Peaking EQ (RBJ cookbook).
    pub fn peaking(sr: f64, f0: f64, q: f64, gain_db: f64) -> Self {
        let a = 10f64.powf(gain_db / 40.0);
        let w = std::f64::consts::TAU * (f0 / sr).min(0.49);
        let alpha = w.sin() / (2.0 * q);
        Self::new(1.0 + alpha * a, -2.0 * w.cos(), 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * w.cos(), 1.0 - alpha / a)
    }

    /// High shelf (RBJ cookbook, shelf slope 1/sqrt(2) Q).
    pub fn high_shelf(sr: f64, f0: f64, q: f64, gain_db: f64) -> Self {
        let a = 10f64.powf(gain_db / 40.0);
        let w = std::f64::consts::TAU * f0 / sr;
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        let s = 2.0 * a.sqrt() * alpha;
        Self::new(
            a * ((a + 1.0) + (a - 1.0) * c + s),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
            a * ((a + 1.0) + (a - 1.0) * c - s),
            (a + 1.0) - (a - 1.0) * c + s,
            2.0 * ((a - 1.0) - (a + 1.0) * c),
            (a + 1.0) - (a - 1.0) * c - s,
        )
    }

    pub fn low_pass(sr: f64, f0: f64, q: f64) -> Self {
        let w = std::f64::consts::TAU * (f0 / sr).min(0.49);
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        Self::new((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }

    /// Notch (band-reject) at `f0` with quality `q`.
    pub fn notch(sr: f64, f0: f64, q: f64) -> Self {
        let w = std::f64::consts::TAU * (f0 / sr).min(0.49);
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        Self::new(1.0, -2.0 * c, 1.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }

    pub fn high_pass(sr: f64, f0: f64, q: f64) -> Self {
        let w = std::f64::consts::TAU * f0 / sr;
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        Self::new((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }

    #[inline]
    pub fn process(&mut self, ch: usize, x: f64) -> f64 {
        let z = &mut self.z[ch];
        let y = self.b[0] * x + z[0];
        z[0] = self.b[1] * x - self.a[0] * y + z[1];
        z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// The 10-band equalizer of `jb_core::Equalizer` (octave bands, peaking).
#[derive(Debug, Clone)]
pub struct GraphicEq {
    bands: Vec<Biquad>,
}

impl GraphicEq {
    pub fn new(sr: u32, gains_db: &[f64; 10]) -> Self {
        let bands = jb_core::Equalizer::FREQUENCIES
            .iter()
            .zip(gains_db)
            .filter(|(f, g)| **g != 0.0 && **f < sr as f64 * 0.45)
            .map(|(f, g)| Biquad::peaking(sr as f64, *f, std::f64::consts::SQRT_2, *g))
            .collect();
        Self { bands }
    }

    /// Filters interleaved stereo samples in place.
    pub fn process(&mut self, x: &mut [f32]) {
        for frame in x.chunks_mut(2) {
            for (c, s) in frame.iter_mut().enumerate() {
                let mut v = *s as f64;
                for b in &mut self.bands {
                    v = b.process(c, v);
                }
                *s = v as f32;
            }
        }
    }
}

/// Integrated loudness of interleaved stereo samples (BS.1770-4: K-weighting,
/// 400 ms blocks with 75% overlap, absolute gate -70 LUFS, relative gate
/// -10 LU). Returns `-inf` for silence.
pub fn integrated_loudness(x: &[f32], sr: u32) -> f64 {
    let mut m = LoudnessMeter::new(sr);
    m.push(x);
    m.integrated()
}

/// Streaming BS.1770 integrated loudness meter over interleaved stereo.
#[derive(Debug, Clone)]
pub struct LoudnessMeter {
    shelf: Biquad,
    hp: Biquad,
    /// Samples per 100 ms step (a gating block is four steps).
    step: usize,
    acc: f64,
    filled: usize,
    steps: Vec<f64>,
    total: f64,
    frames: usize,
}

impl LoudnessMeter {
    pub fn new(sr: u32) -> Self {
        let sr_f = sr as f64;
        Self {
            // K-weighting: high shelf (+4 dB above ~1.7 kHz) and RLB high-pass.
            shelf: Biquad::high_shelf(sr_f, 1681.974450955533, 0.7071752369554196, 3.999843853973347),
            hp: Biquad::high_pass(sr_f, 38.13547087602444, 0.5003270373238773),
            step: (sr as usize / 10).max(1),
            acc: 0.0,
            filled: 0,
            steps: Vec::new(),
            total: 0.0,
            frames: 0,
        }
    }

    pub fn push(&mut self, x: &[f32]) {
        for f in x.chunks(2) {
            let mut e = 0.0;
            for (c, s) in f.iter().enumerate() {
                let y = self.hp.process(c, self.shelf.process(c, *s as f64));
                e += y * y;
            }
            self.acc += e;
            self.total += e;
            self.frames += 1;
            self.filled += 1;
            if self.filled == self.step {
                self.steps.push(self.acc);
                self.acc = 0.0;
                self.filled = 0;
            }
        }
    }

    /// Integrated loudness in LUFS (`-inf` for silence).
    pub fn integrated(&self) -> f64 {
        let lufs = |z: f64| -0.691 + 10.0 * z.log10();
        if self.steps.len() < 4 {
            return lufs(self.total / self.frames.max(1) as f64);
        }
        let n = (4 * self.step) as f64;
        let blocks: Vec<f64> = self.steps.windows(4).map(|w| w.iter().sum::<f64>() / n).collect();
        let gated: Vec<f64> = blocks.iter().copied().filter(|z| lufs(*z) > -70.0).collect();
        if gated.is_empty() {
            return f64::NEG_INFINITY;
        }
        let rel = lufs(gated.iter().sum::<f64>() / gated.len() as f64) - 10.0;
        let kept: Vec<f64> = gated.into_iter().filter(|z| lufs(*z) > rel).collect();
        lufs(kept.iter().sum::<f64>() / kept.len().max(1) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f64, amp: f32, secs: f64, sr: u32) -> Vec<f32> {
        (0..(secs * sr as f64) as usize)
            .flat_map(|i| {
                let v = amp * (std::f64::consts::TAU * freq * i as f64 / sr as f64).sin() as f32;
                [v, v]
            })
            .collect()
    }

    #[test]
    fn loudness_of_reference_tone() {
        // BS.1770: a 997 Hz full-scale sine in both channels reads about -0.0 LUFS
        // per channel pair (-3.01 dB each + 3 dB for two channels).
        let x = sine(997.0, 1.0, 3.0, 48000);
        let l = integrated_loudness(&x, 48000);
        assert!((l - 0.0).abs() < 0.3, "{l}");
        let quiet = sine(997.0, 0.1, 3.0, 48000);
        assert!((integrated_loudness(&quiet, 48000) - (l - 20.0)).abs() < 0.1);
        assert_eq!(integrated_loudness(&vec![0.0; 96000], 48000), f64::NEG_INFINITY);
    }

    #[test]
    fn equalizer_boosts_its_band() {
        let mut gains = [0.0; 10];
        gains[5] = 6.0; // 1 kHz
        let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        let mut a = sine(1000.0, 0.25, 1.0, 48000);
        let base = rms(&a);
        GraphicEq::new(48000, &gains).process(&mut a);
        let db = 20.0 * (rms(&a[48000..]) / base).log10();
        assert!((db - 6.0).abs() < 0.5, "{db}");
        let mut b = sine(100.0, 0.25, 1.0, 48000);
        GraphicEq::new(48000, &gains).process(&mut b);
        assert!((20.0 * (rms(&b[48000..]) / base).log10()).abs() < 0.5);
    }
}

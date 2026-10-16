//! Time primitives. All timeline positions are expressed in ticks of 100 ns,
//! which matches the resolution used by Filmora project files and is fine
//! enough to represent any common frame rate without drift.

use serde::{Deserialize, Serialize};

/// Number of ticks in one second.
pub const TICKS_PER_SECOND: i64 = 10_000_000;

/// A point or duration on a timeline, in 100 ns ticks.
pub type Ticks = i64;

/// An exact rational number, used for frame rates (e.g. 30000/1001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub const fn new(num: i64, den: i64) -> Self {
        Self { num, den }
    }

    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Duration of one frame at this rate, in ticks (rounded to nearest).
    pub fn frame_duration(self) -> Ticks {
        (TICKS_PER_SECOND * self.den + self.num / 2) / self.num
    }

    /// Index of the frame that contains `t`.
    pub fn frame_at(self, t: Ticks) -> i64 {
        // floor(t * num / (den * TPS)) computed in i128 to avoid overflow.
        ((t as i128 * self.num as i128).div_euclid(self.den as i128 * TICKS_PER_SECOND as i128)) as i64
    }

    /// Start time of frame `index`, in ticks.
    pub fn frame_start(self, index: i64) -> Ticks {
        ((index as i128 * self.den as i128 * TICKS_PER_SECOND as i128 + self.num as i128 - 1) / self.num as i128)
            as Ticks
    }
}

pub fn seconds_to_ticks(s: f64) -> Ticks {
    (s * TICKS_PER_SECOND as f64).round() as Ticks
}

pub fn ticks_to_seconds(t: Ticks) -> f64 {
    t as f64 / TICKS_PER_SECOND as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntsc_frames_round_trip() {
        let r = Rational::new(30000, 1001);
        for i in [0, 1, 29, 30, 1799, 107_892] {
            assert_eq!(r.frame_at(r.frame_start(i)), i);
        }
    }
}

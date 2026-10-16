//! Animatable parameters used by effects, transforms and audio settings.

use crate::time::Ticks;
use serde::{Deserialize, Serialize};

/// A parameter value. Vectors and colors are stored as fixed arrays so they
/// can be interpolated component-wise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Value {
    Float(f64),
    Vec2([f64; 2]),
    /// Linear RGBA in 0..=1.
    Color([f64; 4]),
    Bool(bool),
    Text(String),
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(v) => Some(*v),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn lerp(&self, other: &Value, k: f64) -> Value {
        let mix = |a: f64, b: f64| a + (b - a) * k;
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => Value::Float(mix(*a, *b)),
            (Value::Vec2(a), Value::Vec2(b)) => Value::Vec2([mix(a[0], b[0]), mix(a[1], b[1])]),
            (Value::Color(a), Value::Color(b)) => {
                Value::Color([mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2]), mix(a[3], b[3])])
            }
            // Discrete values hold until the next keyframe.
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    #[default]
    Linear,
    Hold,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Interpolation {
    fn shape(self, k: f64) -> f64 {
        match self {
            Interpolation::Linear => k,
            Interpolation::Hold => 0.0,
            Interpolation::EaseIn => k * k,
            Interpolation::EaseOut => 1.0 - (1.0 - k) * (1.0 - k),
            Interpolation::EaseInOut => k * k * (3.0 - 2.0 * k),
        }
    }
}

/// A keyframe; `time` is relative to the start of the owning clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Keyframe {
    pub time: Ticks,
    pub value: Value,
    #[serde(default)]
    pub interpolation: Interpolation,
}

/// A parameter that is either constant or animated by keyframes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Param {
    pub value: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<Keyframe>,
}

impl Param {
    pub fn constant(value: Value) -> Self {
        Self { value, keyframes: Vec::new() }
    }

    pub fn float(v: f64) -> Self {
        Self::constant(Value::Float(v))
    }

    /// Value at clip-local time `t`. Keyframes must be sorted by time.
    pub fn sample(&self, t: Ticks) -> Value {
        let kfs = &self.keyframes;
        let Some(first) = kfs.first() else { return self.value.clone() };
        if t <= first.time {
            return first.value.clone();
        }
        for pair in kfs.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if t < b.time {
                let k = (t - a.time) as f64 / (b.time - a.time).max(1) as f64;
                return a.value.lerp(&b.value, a.interpolation.shape(k));
            }
        }
        kfs.last().unwrap().value.clone()
    }

    pub fn sample_f64(&self, t: Ticks) -> f64 {
        self.sample(t).as_f64().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_keyframes_interpolate() {
        let p = Param {
            value: Value::Float(0.0),
            keyframes: vec![
                Keyframe { time: 0, value: Value::Float(0.0), interpolation: Interpolation::Linear },
                Keyframe { time: 100, value: Value::Float(10.0), interpolation: Interpolation::Linear },
            ],
        };
        assert_eq!(p.sample_f64(50), 5.0);
        assert_eq!(p.sample_f64(500), 10.0);
    }
}

//! Speed ramping. A ramp is defined like Filmora's speed ramping panel: a
//! rate curve over the normalized *source* range of the clip
//! ([`SpeedCurve`]). Playback needs the rate over clip-local *timeline* time
//! instead (`Speed::ramp`), so a curve is converted by integrating
//! `dt = ds / rate(s)`; that also gives the clip's new duration.

use crate::model::*;
use crate::param::{Interpolation, Keyframe, Param, Value};
use crate::time::Ticks;

/// Filmora's built-in ramp presets: (name, [(source position 0..1, rate)]).
pub const PRESETS: &[(&str, &[(f64, f64)])] = &[
    ("montage", &[(0.0, 0.9), (0.1, 0.9), (0.5, 6.8), (0.65, 0.3), (0.8, 1.0), (1.0, 1.0)]),
    (
        "hero_moment",
        &[(0.0, 1.0), (0.05, 1.0), (0.35, 5.5), (0.45, 0.5), (0.55, 0.5), (0.65, 5.5), (0.95, 1.0), (1.0, 1.0)],
    ),
    ("bullet_time", &[(0.0, 5.2), (0.4, 5.2), (0.46, 0.5), (0.54, 0.5), (0.6, 5.2), (1.0, 5.2)]),
    ("jumper", &[(0.0, 0.6), (0.43, 0.6), (0.5, 6.0), (0.57, 0.6), (1.0, 0.6)]),
    ("flash_in", &[(0.0, 5.2), (0.4, 5.2), (0.6, 1.0), (1.0, 1.0)]),
    ("flash_out", &[(0.0, 1.0), (0.4, 1.0), (0.6, 5.2), (1.0, 5.2)]),
];

/// Normalizes a preset name ("Hero moment", "hero-moment" → "hero_moment").
pub fn preset_key(name: &str) -> String {
    let k = name.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match k.as_str() {
        "hero" => "hero_moment".into(),
        "bullet" => "bullet_time".into(),
        "flash" => "flash_in".into(),
        _ => k,
    }
}

pub fn preset_curve(name: &str) -> Option<SpeedCurve> {
    let key = preset_key(name);
    PRESETS.iter().find(|(n, _)| *n == key).map(|(n, pts)| SpeedCurve {
        preset: n.to_string(),
        points: pts.iter().map(|&(position, speed)| SpeedPoint { position, speed }).collect(),
    })
}

/// Validates and sorts custom points; the curve always covers 0..=1.
pub fn custom_curve(points: &[SpeedPoint]) -> Result<SpeedCurve, String> {
    if points.is_empty() {
        return Err("a custom speed curve needs at least one point".into());
    }
    let mut pts = points.to_vec();
    for p in &pts {
        if !(0.0..=1.0).contains(&p.position) || !p.position.is_finite() {
            return Err(format!("speed point position {} is outside 0..1", p.position));
        }
        if !(p.speed > 0.0 && p.speed <= 100.0) {
            return Err(format!("speed point rate {} must be in (0, 100]", p.speed));
        }
    }
    pts.sort_by(|a, b| a.position.total_cmp(&b.position));
    if pts[0].position > 0.0 {
        pts.insert(0, SpeedPoint { position: 0.0, speed: pts[0].speed });
    }
    if pts[pts.len() - 1].position < 1.0 {
        let s = pts[pts.len() - 1].speed;
        pts.push(SpeedPoint { position: 1.0, speed: s });
    }
    Ok(SpeedCurve { preset: "custom".into(), points: pts })
}

/// Rate of a curve at normalized source position `x`.
pub fn rate_at(curve: &SpeedCurve, x: f64) -> f64 {
    let pts = &curve.points;
    let Some(first) = pts.first() else { return 1.0 };
    if x <= first.position {
        return first.speed;
    }
    for w in pts.windows(2) {
        if x <= w[1].position {
            let span = (w[1].position - w[0].position).max(1e-12);
            return w[0].speed + (w[1].speed - w[0].speed) * (x - w[0].position) / span;
        }
    }
    pts[pts.len() - 1].speed
}

/// Sub-steps per curve segment when converting to timeline keyframes.
const STEPS: usize = 12;

/// Converts a curve over a source span (ticks) into a timeline rate ramp.
/// Returns the ramp and the timeline duration the clip needs.
pub fn ramp_for(curve: &SpeedCurve, source_span: Ticks) -> (Param, Ticks) {
    let span = source_span.max(0) as f64;
    let mut xs: Vec<f64> = Vec::new();
    for w in curve.points.windows(2) {
        for i in 0..STEPS {
            xs.push(w[0].position + (w[1].position - w[0].position) * i as f64 / STEPS as f64);
        }
    }
    xs.push(1.0);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let mut t = 0.0;
    let mut keys = Vec::with_capacity(xs.len());
    let mut prev: Option<(f64, f64)> = None;
    for &x in &xs {
        let r = rate_at(curve, x);
        if let Some((px, pr)) = prev {
            // dt = ds / r with r linear in s: exact log integral.
            let ds = (x - px) * span;
            t += if (r - pr).abs() < 1e-9 { ds / r } else { ds * (r / pr).ln() / (r - pr) };
        }
        keys.push(Keyframe { time: t.round() as Ticks, value: Value::Float(r), interpolation: Interpolation::Linear });
        prev = Some((x, r));
    }
    keys.dedup_by_key(|k| k.time);
    let mut ramp = Param { value: Value::Float(1.0), keyframes: keys };
    // Playback interpolates linearly in timeline time; rescale so the ramp
    // consumes exactly the source span.
    let t = t.round().max(1.0) as Ticks;
    let consumed = crate::eval::integrate_rate(&ramp, t) as f64;
    let k = if consumed > 0.0 { span / consumed } else { 1.0 };
    for kf in &mut ramp.keyframes {
        kf.time = (kf.time as f64 * k).round() as Ticks;
    }
    ramp.keyframes.dedup_by_key(|k| k.time);
    let dur = ((t as f64 * k).round() as Ticks).max(1);
    ramp.value = Value::Float(if dur > 0 { span / dur as f64 } else { 1.0 });
    (ramp, dur)
}

/// Source time consumed by a clip over its whole duration.
pub fn source_span(clip: &Clip) -> Ticks {
    if clip.speed.freeze {
        return 0;
    }
    let end = crate::eval::source_time(
        &Clip { speed: Speed { reverse: false, ..clip.speed.clone() }, ..clip.clone() },
        clip.duration(),
    );
    end - clip.source_in
}

/// The part of a curve between normalized positions `a` and `b`,
/// renormalized to 0..=1.
pub fn sub_curve(curve: &SpeedCurve, a: f64, b: f64) -> SpeedCurve {
    let w = (b - a).max(1e-12);
    let mut pts = vec![SpeedPoint { position: 0.0, speed: rate_at(curve, a) }];
    for p in &curve.points {
        if p.position > a && p.position < b {
            pts.push(SpeedPoint { position: (p.position - a) / w, speed: p.speed });
        }
    }
    pts.push(SpeedPoint { position: 1.0, speed: rate_at(curve, b) });
    SpeedCurve { preset: curve.preset.clone(), points: pts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TICKS_PER_SECOND as S;

    #[test]
    fn constant_curve_matches_rate() {
        let c = custom_curve(&[SpeedPoint { position: 0.0, speed: 2.0 }]).unwrap();
        let (ramp, dur) = ramp_for(&c, 10 * S);
        assert_eq!(dur, 5 * S);
        assert_eq!(ramp.sample_f64(S), 2.0);
    }

    #[test]
    fn presets_integrate_to_the_source_span() {
        for (name, _) in PRESETS {
            let c = preset_curve(name).unwrap();
            let (ramp, dur) = ramp_for(&c, 10 * S);
            let consumed = crate::eval::integrate_rate(&ramp, dur);
            assert!((consumed - 10 * S).abs() < S / 100, "{name}: {consumed}");
        }
    }

    #[test]
    fn sub_curve_renormalizes() {
        let c = preset_curve("flash_in").unwrap();
        let s = sub_curve(&c, 0.5, 1.0);
        assert_eq!(s.points[0].position, 0.0);
        assert!((s.points[0].speed - rate_at(&c, 0.5)).abs() < 1e-9);
        assert!((rate_at(&s, 0.2) - rate_at(&c, 0.6)).abs() < 1e-9);
    }
}

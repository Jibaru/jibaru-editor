//! Parsing and formatting of human/agent friendly time values.
//!
//! Accepted inputs (`fps` is the project frame rate):
//! - `12345t`        raw ticks (10_000_000 per second)
//! - `48f`           frame index
//! - `1.5` / `1.5s`  seconds
//! - `MM:SS(.fff)` or `HH:MM:SS(.fff)`
//! - `HH:MM:SS:FF`   SMPTE-style with a frame field (`;` also accepted)

use crate::error::{JbError, JbResult};
use jb_core::time::{seconds_to_ticks, ticks_to_seconds};
use jb_core::{Rational, Ticks, TICKS_PER_SECOND};
use serde_json::{json, Value};

const HELP: &str = "use seconds (`5`, `1.5s`), `HH:MM:SS.mmm`, `HH:MM:SS:FF`, frames (`120f`) or ticks (`50000000t`)";

pub fn parse_time(input: &str, fps: Rational) -> JbResult<Ticks> {
    let s = input.trim();
    let bad = || JbError::invalid(format!("invalid time `{input}`")).hint(HELP);
    if s.is_empty() {
        return Err(bad());
    }
    if let Some(n) = s.strip_suffix('t') {
        return n.trim().parse::<i64>().map_err(|_| bad()).and_then(non_negative(input));
    }
    if let Some(n) = s.strip_suffix('f') {
        let i = n.trim().parse::<i64>().map_err(|_| bad())?;
        return non_negative(input)(fps.frame_start(i));
    }
    if !s.contains(':') {
        let n = s.strip_suffix('s').unwrap_or(s).trim();
        let secs = n.parse::<f64>().map_err(|_| bad())?;
        if !secs.is_finite() {
            return Err(bad());
        }
        return non_negative(input)(seconds_to_ticks(secs));
    }
    let parts: Vec<&str> = s.split([':', ';']).collect();
    let num = |p: &str| p.parse::<f64>().map_err(|_| bad());
    let ticks = match parts.as_slice() {
        [m, sec] => seconds_to_ticks(num(m)? * 60.0 + num(sec)?),
        [h, m, sec] => seconds_to_ticks(num(h)? * 3600.0 + num(m)? * 60.0 + num(sec)?),
        [h, m, sec, f] => {
            let whole = num(h)? * 3600.0 + num(m)? * 60.0 + num(sec)?;
            let frame = f.parse::<i64>().map_err(|_| bad())?;
            let base = fps.frame_at(seconds_to_ticks(whole));
            fps.frame_start(base + frame)
        }
        _ => return Err(bad()),
    };
    non_negative(input)(ticks)
}

fn non_negative(input: &str) -> impl Fn(Ticks) -> JbResult<Ticks> + '_ {
    move |t| {
        if t < 0 {
            Err(JbError::invalid(format!("time `{input}` is negative")).hint(HELP))
        } else {
            Ok(t)
        }
    }
}

/// `HH:MM:SS.mmm`.
pub fn format_clock(t: Ticks) -> String {
    let ms_total = (t as i128 * 1000 / TICKS_PER_SECOND as i128) as i64;
    let (h, rem) = (ms_total / 3_600_000, ms_total % 3_600_000);
    let (m, rem) = (rem / 60_000, rem % 60_000);
    format!("{h:02}:{m:02}:{:02}.{:03}", rem / 1000, rem % 1000)
}

/// A time value in every representation agents find useful.
pub fn time_json(t: Ticks, fps: Rational) -> Value {
    json!({
        "ticks": t,
        "seconds": ticks_to_seconds(t),
        "timecode": format_clock(t),
        "frame": fps.frame_at(t),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FPS: Rational = Rational::new(30, 1);

    #[test]
    fn parses_all_forms() {
        assert_eq!(parse_time("5", FPS).unwrap(), 5 * TICKS_PER_SECOND);
        assert_eq!(parse_time("1.5s", FPS).unwrap(), 15_000_000);
        assert_eq!(parse_time("123t", FPS).unwrap(), 123);
        assert_eq!(parse_time("30f", FPS).unwrap(), TICKS_PER_SECOND);
        assert_eq!(parse_time("01:02", FPS).unwrap(), 62 * TICKS_PER_SECOND);
        assert_eq!(parse_time("00:01:02.5", FPS).unwrap(), 625_000_000);
        assert_eq!(parse_time("00:00:01:15", FPS).unwrap(), 15_000_000);
        assert!(parse_time("abc", FPS).is_err());
        assert!(parse_time("-1", FPS).is_err());
    }

    #[test]
    fn formats_clock() {
        assert_eq!(format_clock(625_000_000), "00:01:02.500");
    }
}

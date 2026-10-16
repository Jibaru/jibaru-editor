//! Stable, agent-friendly JSON views of a project. These shapes are part of
//! the CLI/MCP contract: fields may be added but never renamed or removed.

use crate::timecode::time_json;
use jb_core::time::ticks_to_seconds;
use jb_core::{Clip, MediaAsset, Project, Sequence, Track};
use serde_json::{json, Value};

/// How much of the project to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Detail {
    /// Settings, media, sequences and tracks with clip counts.
    Summary,
    /// Like `Summary` plus every clip on every track.
    #[default]
    Clips,
    /// The complete native project model.
    Full,
}

impl std::str::FromStr for Detail {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "summary" => Ok(Detail::Summary),
            "clips" => Ok(Detail::Clips),
            "full" => Ok(Detail::Full),
            _ => Err(format!("unknown detail `{s}`; expected summary, clips or full")),
        }
    }
}

pub fn fps_json(p: &Project) -> Value {
    let r = p.settings.frame_rate;
    json!({ "num": r.num, "den": r.den, "value": r.as_f64() })
}

pub fn media_json(m: &MediaAsset) -> Value {
    json!({
        "id": m.id,
        "name": m.name,
        "path": m.path,
        "kind": m.kind,
        "duration_ticks": m.duration,
        "duration_seconds": ticks_to_seconds(m.duration),
        "width": m.width,
        "height": m.height,
        "frame_rate": m.frame_rate.map(|r| json!({"num": r.num, "den": r.den})),
        "has_audio": m.has_audio,
    })
}

pub fn clip_json(p: &Project, seq: &Sequence, track_index: usize, track: &Track, c: &Clip) -> Value {
    let fps = p.settings.frame_rate;
    json!({
        "id": c.id,
        "name": c.name,
        "sequence_id": seq.id,
        "track_id": track.id,
        "track_index": track_index,
        "track_kind": track.kind,
        "source": c.source,
        "start": time_json(c.start, fps),
        "end": time_json(c.end, fps),
        "duration": time_json(c.duration(), fps),
        "source_in_ticks": c.source_in,
        "speed": c.speed.rate,
        "reverse": c.speed.reverse,
        "effects": c.effects.iter().map(|e| json!({"id": e.id, "kind": e.kind, "enabled": e.enabled})).collect::<Vec<_>>(),
        "animations": c.animations.iter().map(|e| json!({"id": e.id, "kind": e.kind})).collect::<Vec<_>>(),
        "transition_out": c.transition_out.as_ref().map(|t| json!({"id": t.id, "kind": t.kind, "duration_ticks": t.duration})),
        "link_group": c.link_group,
        "disabled": c.disabled,
    })
}

fn track_json(p: &Project, seq: &Sequence, i: usize, t: &Track, with_clips: bool) -> Value {
    let mut v = json!({
        "id": t.id,
        "index": i,
        "kind": t.kind,
        "name": t.name,
        "locked": t.locked,
        "muted": t.muted,
        "hidden": t.hidden,
        "clip_count": t.clips.len(),
    });
    if with_clips {
        v["clips"] = t.clips.iter().map(|c| clip_json(p, seq, i, t, c)).collect();
    }
    v
}

pub fn project_summary(p: &Project, detail: Detail) -> Value {
    let fps = p.settings.frame_rate;
    let with_clips = detail != Detail::Summary;
    let mut v = json!({
        "id": p.id,
        "name": p.name,
        "settings": {
            "width": p.settings.width,
            "height": p.settings.height,
            "frame_rate": fps_json(p),
            "sample_rate": p.settings.sample_rate,
        },
        "main_sequence": p.main_sequence,
        "duration": time_json(p.main().duration(), fps),
        "media": p.media.iter().map(media_json).collect::<Vec<_>>(),
        "sequences": p.sequences.iter().map(|s| json!({
            "id": s.id,
            "name": s.name,
            "is_main": s.id == p.main_sequence,
            "duration": time_json(s.duration(), fps),
            "tracks": s.tracks.iter().enumerate().map(|(i, t)| track_json(p, s, i, t, with_clips)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    if detail == Detail::Full {
        v["project"] = serde_json::to_value(p).unwrap_or(Value::Null);
    }
    v
}

/// Criteria for `find_clips`. All set criteria must match.
#[derive(Debug, Clone, Default)]
pub struct ClipQuery {
    /// Clips whose `[start, end)` contains this time.
    pub at: Option<jb_core::Ticks>,
    /// Clips overlapping `[from, to)`.
    pub range: Option<(jb_core::Ticks, jb_core::Ticks)>,
    pub track_id: Option<String>,
    /// Zero-based track index within the sequence.
    pub track_index: Option<usize>,
    /// Case-insensitive substring of the clip name.
    pub name: Option<String>,
    pub media_id: Option<String>,
    /// Defaults to all sequences.
    pub sequence_id: Option<String>,
}

pub fn find_clips(p: &Project, q: &ClipQuery) -> Vec<Value> {
    let needle = q.name.as_ref().map(|n| n.to_lowercase());
    let mut out = Vec::new();
    for s in p.sequences.iter().filter(|s| q.sequence_id.as_ref().is_none_or(|id| &s.id == id)) {
        for (i, t) in s.tracks.iter().enumerate() {
            if q.track_id.as_ref().is_some_and(|id| &t.id != id) || q.track_index.is_some_and(|ix| ix != i) {
                continue;
            }
            for c in &t.clips {
                let ok = q.at.is_none_or(|at| c.contains(at))
                    && q.range.is_none_or(|(a, b)| c.start < b && c.end > a)
                    && needle.as_ref().is_none_or(|n| c.name.to_lowercase().contains(n))
                    && q.media_id
                        .as_ref()
                        .is_none_or(|m| matches!(&c.source, jb_core::ClipSource::Media { media_id } if media_id == m));
                if ok {
                    out.push(clip_json(p, s, i, t, c));
                }
            }
        }
    }
    out
}

/// Human readable multi-line description, for non-JSON CLI output.
pub fn project_text(p: &Project) -> String {
    use crate::timecode::format_clock;
    let mut s = String::new();
    let r = p.settings.frame_rate;
    s += &format!(
        "{} ({}x{} @ {:.3} fps), duration {}\n",
        p.name,
        p.settings.width,
        p.settings.height,
        r.as_f64(),
        format_clock(p.main().duration())
    );
    s += &format!("media: {}\n", p.media.len());
    for m in &p.media {
        s += &format!("  {} {:?} {} [{}] {}\n", m.id, m.kind, format_clock(m.duration), m.name, m.path);
    }
    for seq in &p.sequences {
        let main = if seq.id == p.main_sequence { " (main)" } else { "" };
        s += &format!("sequence {} \"{}\"{main}: {} tracks\n", seq.id, seq.name, seq.tracks.len());
        for (i, t) in seq.tracks.iter().enumerate() {
            let mut flags = Vec::new();
            for (on, f) in [(t.locked, "locked"), (t.muted, "muted"), (t.hidden, "hidden")] {
                if on {
                    flags.push(f);
                }
            }
            let flags = if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(",")) };
            s += &format!("  #{i} {:?} {} \"{}\"{flags}: {} clips\n", t.kind, t.id, t.name, t.clips.len());
            for c in &t.clips {
                s += &format!("    {} {}..{} \"{}\"\n", c.id, format_clock(c.start), format_clock(c.end), c.name);
            }
        }
    }
    s
}

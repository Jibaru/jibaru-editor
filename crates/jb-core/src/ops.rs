//! Implementations of the editing commands (see `command.rs` for their
//! documentation). Every function may leave the project partially modified
//! on error; `History::execute` applies commands to a copy.

use crate::command::{ClipAttribute, CommandError, Edge, Result};
use crate::edit::{self, ParamPath};
use crate::model::*;
use crate::param::{Interpolation, Keyframe, Param, Value};
use crate::speed;
use crate::time::{Ticks, TICKS_PER_SECOND};
use std::collections::{BTreeMap, BTreeSet};

fn not_found(what: impl std::fmt::Display) -> CommandError {
    CommandError::NotFound(what.to_string())
}

fn invalid(what: impl Into<String>) -> CommandError {
    CommandError::Invalid(what.into())
}

// -- lookups -------------------------------------------------------------------

pub fn track_mut<'a>(p: &'a mut Project, track_id: &str) -> Result<&'a mut Track> {
    p.sequences
        .iter_mut()
        .flat_map(|s| s.tracks.iter_mut())
        .find(|t| t.id == track_id)
        .ok_or_else(|| not_found(format!("track {track_id}")))
}

fn locate_track(p: &Project, track_id: &str) -> Result<(usize, usize)> {
    for (si, s) in p.sequences.iter().enumerate() {
        if let Some(ti) = s.tracks.iter().position(|t| t.id == track_id) {
            return Ok((si, ti));
        }
    }
    Err(not_found(format!("track {track_id}")))
}

fn locate(p: &Project, clip_id: &str) -> Result<(usize, usize, usize)> {
    p.locate_clip(clip_id).ok_or_else(|| not_found(format!("clip {clip_id}")))
}

fn clip<'a>(p: &'a Project, clip_id: &str) -> Result<&'a Clip> {
    p.clip(clip_id).ok_or_else(|| not_found(format!("clip {clip_id}")))
}

fn all_clips(p: &Project) -> impl Iterator<Item = &Clip> {
    p.sequences.iter().flat_map(|s| s.tracks.iter().flat_map(|t| t.clips.iter()))
}

pub fn sort_and_check(track: &Track) -> Result<()> {
    for w in track.clips.windows(2) {
        if w[0].end > w[1].start {
            return Err(invalid(format!("clips {} and {} overlap", w[0].id, w[1].id)));
        }
    }
    Ok(())
}

pub fn insert_clip(track: &mut Track, clip: Clip) -> Result<()> {
    if clip.end <= clip.start {
        return Err(invalid("clip has non-positive duration"));
    }
    if clip.start < 0 {
        return Err(invalid("clip would start before zero"));
    }
    let pos = track.clips.partition_point(|c| c.start < clip.start);
    track.clips.insert(pos, clip);
    sort_and_check(track)
}

/// Ids of the clip plus every clip in its link group.
pub fn linked_ids(p: &Project, clip_id: &str) -> Result<Vec<Id>> {
    let c = clip(p, clip_id)?;
    Ok(match &c.link_group {
        None => vec![c.id.clone()],
        Some(g) => all_clips(p).filter(|x| x.link_group.as_ref() == Some(g)).map(|x| x.id.clone()).collect(),
    })
}

/// Clips that move together with `clip_id`: its link group and its group,
/// transitively.
fn moving_ids(p: &Project, clip_id: &str) -> Result<Vec<Id>> {
    clip(p, clip_id)?;
    let mut out: Vec<Id> = vec![clip_id.to_string()];
    let mut i = 0;
    while i < out.len() {
        let c = clip(p, &out[i])?.clone();
        for x in all_clips(p) {
            let same =
                (c.link_group.is_some() && x.link_group == c.link_group) || (c.group.is_some() && x.group == c.group);
            if same && !out.contains(&x.id) {
                out.push(x.id.clone());
            }
        }
        i += 1;
    }
    Ok(out)
}

fn ensure_unlocked(p: &Project, clip_id: &str) -> Result<()> {
    let (s, t, _) = locate(p, clip_id)?;
    let track = &p.sequences[s].tracks[t];
    if track.locked {
        return Err(CommandError::Locked(track.id.clone()));
    }
    Ok(())
}

fn take_clip(p: &mut Project, clip_id: &str) -> Result<(usize, usize, Clip)> {
    let (s, t, c) = locate(p, clip_id)?;
    Ok((s, t, p.sequences[s].tracks[t].clips.remove(c)))
}

pub fn with_clip<F: FnOnce(&mut Clip) -> Result<()>>(p: &mut Project, clip_id: &str, f: F) -> Result<()> {
    ensure_unlocked(p, clip_id)?;
    let c = p.clip_mut(clip_id).ok_or_else(|| not_found(format!("clip {clip_id}")))?;
    f(c)
}

/// Clips whose audio settings an audio command changes: on a video-track
/// clip whose sound lives on linked audio-track clips, those; else the clip.
fn audio_targets(p: &Project, clip_id: &str) -> Result<Vec<Id>> {
    let (s, t, _) = locate(p, clip_id)?;
    let c = clip(p, clip_id)?;
    if p.sequences[s].tracks[t].kind == TrackKind::Video && c.link_group.is_some() {
        let audio: Vec<Id> = p.sequences[s]
            .tracks
            .iter()
            .filter(|tr| tr.kind == TrackKind::Audio)
            .flat_map(|tr| tr.clips.iter())
            .filter(|x| x.link_group == c.link_group)
            .map(|x| x.id.clone())
            .collect();
        if !audio.is_empty() {
            return Ok(audio);
        }
    }
    Ok(vec![clip_id.to_string()])
}

pub fn with_audio(p: &mut Project, clip_id: &str, f: impl Fn(&mut AudioSettings)) -> Result<()> {
    for id in audio_targets(p, clip_id)? {
        with_clip(p, &id, |c| {
            f(&mut c.audio);
            Ok(())
        })?;
    }
    Ok(())
}

/// Runs `f` on the title of a title clip, or of the single title inside a
/// title template instance (compound clip).
pub fn with_title(p: &mut Project, clip_id: &str, f: impl FnOnce(&mut TitleData) -> Result<()>) -> Result<()> {
    ensure_unlocked(p, clip_id)?;
    let c = clip(p, clip_id)?;
    let target = match &c.source {
        ClipSource::Title(_) => clip_id.to_string(),
        ClipSource::Sequence { sequence_id } => {
            let seq = p.sequence(sequence_id).ok_or_else(|| not_found(format!("sequence {sequence_id}")))?;
            let titles: Vec<&Clip> = seq
                .tracks
                .iter()
                .flat_map(|t| t.clips.iter())
                .filter(|c| matches!(c.source, ClipSource::Title(_)))
                .collect();
            match titles.as_slice() {
                [one] => one.id.clone(),
                [] => return Err(invalid("clip is not a title")),
                _ => return Err(invalid("the template holds several titles; pass the id of one of them")),
            }
        }
        _ => return Err(invalid("clip is not a title")),
    };
    let c = p.clip_mut(&target).expect("located");
    match &mut c.source {
        ClipSource::Title(t) => f(t),
        _ => unreachable!(),
    }
}

/// Media duration bound for a clip's source range (None = unbounded).
fn source_limit(p: &Project, c: &Clip) -> Option<Ticks> {
    match &c.source {
        ClipSource::Media { media_id } => {
            p.media(media_id).filter(|m| m.kind != MediaKind::Image && m.duration > 0).map(|m| m.duration)
        }
        _ => None,
    }
}

/// Fails when a clip reads outside its media, allowing one frame of slack.
fn check_source(p: &Project, c: &Clip) -> Result<()> {
    if c.source_in < 0 {
        return Err(invalid(format!("clip {} would start before the beginning of its source", c.id)));
    }
    if let Some(limit) = source_limit(p, c) {
        let end = c.source_in + speed::source_span(c);
        let slack = p.settings.frame_rate.frame_duration();
        if end > limit + slack {
            return Err(invalid(format!(
                "clip {} would read past the end of its media ({:.3}s > {:.3}s)",
                c.id,
                end as f64 / TICKS_PER_SECOND as f64,
                limit as f64 / TICKS_PER_SECOND as f64
            )));
        }
    }
    Ok(())
}

// -- tracks --------------------------------------------------------------------

pub fn add_track(p: &mut Project, sequence_id: Option<&str>, kind: TrackKind, index: Option<usize>) -> Result<()> {
    let sid = sequence_id.map(str::to_string).unwrap_or_else(|| p.main_sequence.clone());
    let seq = p.sequence_mut(&sid).ok_or_else(|| not_found(format!("sequence {sid}")))?;
    let n = seq.tracks.iter().filter(|t| t.kind == kind).count() + 1;
    let name = match kind {
        TrackKind::Video => format!("Video {n}"),
        TrackKind::Audio => format!("Audio {n}"),
    };
    let track = Track {
        id: new_id(),
        kind,
        name,
        locked: false,
        muted: false,
        hidden: false,
        clips: Vec::new(),
        height: None,
        extra: Extra::Null,
    };
    let i = index.unwrap_or(seq.tracks.len()).min(seq.tracks.len());
    seq.tracks.insert(i, track);
    Ok(())
}

pub fn move_track(p: &mut Project, track_id: &str, index: usize) -> Result<()> {
    let (s, t) = locate_track(p, track_id)?;
    let tracks = &mut p.sequences[s].tracks;
    if index >= tracks.len() {
        return Err(invalid(format!("track index {index} out of range (0..{})", tracks.len())));
    }
    let tr = tracks.remove(t);
    tracks.insert(index, tr);
    Ok(())
}

// -- clips ---------------------------------------------------------------------

/// Signed source time elapsed after clip-local `local` (extrapolated before 0).
fn elapsed(c: &Clip, local: Ticks) -> Ticks {
    if c.speed.freeze {
        return 0;
    }
    match &c.speed.ramp {
        Some(r) if local >= 0 => crate::eval::integrate_rate(r, local),
        Some(r) => (local as f64 * r.sample_f64(0)).round() as Ticks,
        None => (local as f64 * c.speed.rate).round() as Ticks,
    }
}

/// The clip re-cut to its old clip-local window `[d, e)`: the timeline range
/// becomes `[start + d, start + e)` and the source range, speed ramp,
/// keyframes, ranged effects, animations and markers follow the content.
/// With `bound_params` keyframe curves are cut at `e` too (split); trims
/// leave keyframes past the end in place.
pub fn recut(c: &Clip, d: Ticks, e: Ticks, bound_params: bool) -> Clip {
    let len = c.duration();
    let mut n = c.clone();
    n.start = c.start + d;
    n.end = c.start + e;
    if !c.speed.freeze {
        n.source_in =
            if c.speed.reverse { c.source_in + elapsed(c, len) - elapsed(c, e) } else { c.source_in + elapsed(c, d) };
    }
    if let Some(ramp) = &c.speed.ramp {
        let (ed, ee, total) = (elapsed(c, d), elapsed(c, e), elapsed(c, len).max(1));
        let mut r = edit::window_param(ramp, d, Some(e));
        let avg = (ee - ed) as f64 / (e - d).max(1) as f64;
        r.value = Value::Float(avg);
        n.speed.ramp = Some(r);
        n.speed.rate = avg;
        if let Some(curve) = &c.speed.curve {
            let (fa, fb) = (ed as f64 / total as f64, ee as f64 / total as f64);
            n.speed.curve = if (0.0..=1.0).contains(&fa) && (0.0..=1.0 + 1e-9).contains(&fb) {
                let (a, b) = if c.speed.reverse { (1.0 - fb, 1.0 - fa) } else { (fa, fb) };
                Some(speed::sub_curve(curve, a.max(0.0), b.min(1.0)))
            } else {
                None
            };
        }
    }
    if d != 0 || bound_params {
        edit::for_each_param(&mut n, |p| *p = edit::window_param(p, d, if bound_params { Some(e) } else { None }));
    }
    edit::rebase_local(&mut n, d, e - d);
    let len = n.duration();
    n.audio.fade_in = n.audio.fade_in.min(len);
    n.audio.fade_out = n.audio.fade_out.min(len);
    n
}

/// Fresh ids for everything owned by a copied clip.
fn renew_ids(c: &mut Clip) {
    c.id = new_id();
    for e in c.effects.iter_mut().chain(c.animations.iter_mut()) {
        e.id = new_id();
    }
    if let Some(t) = &mut c.transition_out {
        t.id = new_id();
    }
    for m in &mut c.markers {
        m.id = new_id();
    }
}

/// Splits one clip (not its partners) at timeline `at`; returns the right half's id.
fn split_one(p: &mut Project, clip_id: &str, at: Ticks) -> Result<Id> {
    ensure_unlocked(p, clip_id)?;
    let (s, t, c) = take_clip(p, clip_id)?;
    if !c.contains(at) || at == c.start {
        return Err(invalid("split point outside clip"));
    }
    let cut = at - c.start;
    let mut left = recut(&c, 0, cut, true);
    let mut right = recut(&c, cut, c.duration(), true);
    left.id = c.id.clone();
    for e in right.effects.iter_mut().chain(right.animations.iter_mut()) {
        e.id = new_id();
    }
    right.id = new_id();
    left.transition_out = None;
    left.audio.fade_out = 0;
    right.audio.fade_in = 0;
    left.animations.retain(|a| a.placement != Some(AnimationPlacement::Out));
    right.animations.retain(|a| a.placement != Some(AnimationPlacement::In));
    right.markers.iter_mut().for_each(|m| m.id = new_id());
    let rid = right.id.clone();
    let track = &mut p.sequences[s].tracks[t];
    insert_clip(track, left)?;
    insert_clip(track, right)?;
    Ok(rid)
}

/// Splits a clip and its link group at `at`. The right halves share a new
/// link group. Returns the ids of the right halves.
pub fn split_clip(p: &mut Project, clip_id: &str, at: Ticks) -> Result<Vec<Id>> {
    let group = clip(p, clip_id)?.link_group.clone();
    let mut rights = Vec::new();
    for id in linked_ids(p, clip_id)? {
        rights.push(split_one(p, &id, at)?);
    }
    if group.is_some() {
        let g = new_id();
        for id in &rights {
            p.clip_mut(id).expect("just inserted").link_group = Some(g.clone());
        }
    }
    Ok(rights)
}

pub fn remove_clip(p: &mut Project, clip_id: &str, ripple: bool) -> Result<()> {
    for id in linked_ids(p, clip_id)? {
        ensure_unlocked(p, &id)?;
        let (s, t, c) = take_clip(p, &id)?;
        if ripple {
            let gap = c.duration();
            for x in p.sequences[s].tracks[t].clips.iter_mut().filter(|x| x.start >= c.end) {
                x.start -= gap;
                x.end -= gap;
            }
        }
    }
    Ok(())
}

pub fn move_clip(p: &mut Project, clip_id: &str, start: Ticks, track_id: Option<&str>) -> Result<()> {
    let anchor = clip(p, clip_id)?.start;
    let delta = start - anchor;
    let ids = moving_ids(p, clip_id)?;
    let mut taken = Vec::new();
    for id in &ids {
        ensure_unlocked(p, id)?;
        taken.push(take_clip(p, id)?);
    }
    for (s, t, mut c) in taken {
        c.start += delta;
        c.end += delta;
        if c.start < 0 {
            return Err(invalid("clip would start before zero"));
        }
        match track_id.filter(|_| c.id == clip_id) {
            Some(tid) => {
                let tr = track_mut(p, tid)?;
                if tr.locked {
                    return Err(CommandError::Locked(tid.to_string()));
                }
                insert_clip(tr, c)?
            }
            None => insert_clip(&mut p.sequences[s].tracks[t], c)?,
        }
    }
    Ok(())
}

pub fn trim_clip(p: &mut Project, clip_id: &str, start: Option<Ticks>, end: Option<Ticks>) -> Result<()> {
    for id in linked_ids(p, clip_id)? {
        ensure_unlocked(p, &id)?;
        let (s, t, c) = take_clip(p, &id)?;
        let d = start.map_or(0, |ns| ns - c.start);
        let e = end.map_or(c.duration(), |ne| ne - c.start);
        if e <= d {
            return Err(invalid("trim leaves a non-positive duration"));
        }
        let n = recut(&c, d, e, false);
        if d < 0 || e > c.duration() {
            check_source(p, &n)?;
        } else if n.source_in < 0 {
            return Err(invalid("trim before source start"));
        }
        insert_clip(&mut p.sequences[s].tracks[t], n)?;
    }
    Ok(())
}

/// Shifts every clip that starts at or after `from` on the given tracks.
fn shift_tracks(
    p: &mut Project,
    tracks: &BTreeSet<(usize, usize)>,
    from: Ticks,
    delta: Ticks,
    skip: &[Id],
) -> Result<()> {
    for &(s, t) in tracks {
        let tr = &mut p.sequences[s].tracks[t];
        for c in tr.clips.iter_mut().filter(|c| c.start >= from && !skip.contains(&c.id)) {
            c.start += delta;
            c.end += delta;
            if c.start < 0 {
                return Err(invalid("ripple would move a clip before zero"));
            }
        }
        tr.clips.sort_by_key(|c| c.start);
    }
    Ok(())
}

pub fn ripple_trim(p: &mut Project, clip_id: &str, edge: Edge, to: Ticks) -> Result<()> {
    let ids = linked_ids(p, clip_id)?;
    let anchor = clip(p, clip_id)?.clone();
    let mut tracks = BTreeSet::new();
    let mut trimmed = Vec::new();
    for id in &ids {
        ensure_unlocked(p, id)?;
        let (s, t, c) = take_clip(p, id)?;
        tracks.insert((s, t));
        let (d, e) = match edge {
            Edge::Start => (to - anchor.start, c.duration()),
            Edge::End => (0, to - anchor.start),
        };
        if e <= d {
            return Err(invalid("ripple trim leaves a non-positive duration"));
        }
        let mut n = recut(&c, d, e, false);
        // The head trim keeps the clip in place; the gap closes after it.
        let back = n.start - c.start;
        n.start -= back;
        n.end -= back;
        if d < 0 || e > c.duration() {
            check_source(p, &n)?;
        }
        trimmed.push((s, t, c.end, n));
    }
    let delta = trimmed[0].3.end - trimmed[0].2;
    let old_end = trimmed.iter().map(|x| x.2).min().unwrap_or(anchor.end);
    shift_tracks(p, &tracks, old_end, delta, &[])?;
    for (s, t, _, n) in trimmed {
        insert_clip(&mut p.sequences[s].tracks[t], n)?;
    }
    Ok(())
}

pub fn roll_edit(p: &mut Project, clip_id: &str, at: Ticks) -> Result<()> {
    let mut pairs = Vec::new();
    for id in linked_ids(p, clip_id)? {
        let (s, t, ci) = locate(p, &id)?;
        let tr = &p.sequences[s].tracks[t];
        let left = &tr.clips[ci];
        match tr.clips.get(ci + 1).filter(|r| r.start == left.end) {
            Some(right) => pairs.push((left.id.clone(), right.id.clone())),
            None if id == clip_id => return Err(invalid("roll edit needs an adjacent next clip on the same track")),
            None => {}
        }
    }
    for (l, r) in pairs {
        ensure_unlocked(p, &l)?;
        let (s, t, lc) = take_clip(p, &l)?;
        let (_, _, rc) = take_clip(p, &r)?;
        if at <= lc.start || at >= rc.end {
            return Err(invalid("roll point must stay inside both clips"));
        }
        let nl = recut(&lc, 0, at - lc.start, false);
        let nr = recut(&rc, at - rc.start, rc.duration(), false);
        check_source(p, &nl)?;
        check_source(p, &nr)?;
        let tr = &mut p.sequences[s].tracks[t];
        insert_clip(tr, nl)?;
        insert_clip(tr, nr)?;
    }
    Ok(())
}

pub fn slip(p: &mut Project, clip_id: &str, delta: Ticks) -> Result<()> {
    for id in linked_ids(p, clip_id)? {
        ensure_unlocked(p, &id)?;
        let mut c = clip(p, &id)?.clone();
        if matches!(c.source, ClipSource::Title(_) | ClipSource::Color { .. } | ClipSource::Adjustment) {
            return Err(invalid("only media and nested clips can be slipped"));
        }
        c.source_in += delta;
        check_source(p, &c)?;
        *p.clip_mut(&id).expect("exists") = c;
    }
    Ok(())
}

pub fn duplicate(p: &mut Project, clip_id: &str, start: Option<Ticks>, track_id: Option<&str>) -> Result<()> {
    let ids = linked_ids(p, clip_id)?;
    let src = clip(p, clip_id)?.clone();
    let group_end = ids.iter().filter_map(|i| p.clip(i)).map(|c| c.end).max().unwrap_or(src.end);
    let delta = start.unwrap_or(group_end) - src.start;
    let new_group = src.link_group.as_ref().map(|_| new_id());
    for id in &ids {
        let (s, t, _) = locate(p, id)?;
        let mut c = clip(p, id)?.clone();
        renew_ids(&mut c);
        c.start += delta;
        c.end += delta;
        c.link_group = new_group.clone();
        c.group = None;
        let tr = match track_id.filter(|_| id == clip_id) {
            Some(tid) => track_mut(p, tid)?,
            None => &mut p.sequences[s].tracks[t],
        };
        if tr.locked {
            return Err(CommandError::Locked(tr.id.clone()));
        }
        insert_clip(tr, c)?;
    }
    Ok(())
}

pub fn replace_media(p: &mut Project, clip_id: &str, media_id: &str) -> Result<()> {
    let m = p.media(media_id).ok_or_else(|| not_found(format!("media {media_id}")))?.clone();
    ensure_unlocked(p, clip_id)?;
    let (s, t, mut c) = take_clip(p, clip_id)?;
    if !matches!(c.source, ClipSource::Media { .. }) {
        return Err(invalid("only media clips can have their media replaced"));
    }
    c.source = ClipSource::Media { media_id: media_id.to_string() };
    if m.kind != MediaKind::Image && m.duration > 0 {
        if c.source_in >= m.duration {
            c.source_in = 0;
        }
        let avail = m.duration - c.source_in;
        if speed::source_span(&c) > avail {
            // Shorten to what the new media can supply.
            let mut len = c.duration();
            while len > 1 && elapsed(&c, len) > avail {
                len = (len as f64 * avail as f64 / elapsed(&c, len).max(1) as f64).floor() as Ticks;
            }
            c = recut(&c, 0, len.max(1), false);
        }
    }
    insert_clip(&mut p.sequences[s].tracks[t], c)
}

pub fn ripple_insert(p: &mut Project, track_id: &str, c: Clip) -> Result<()> {
    let (s, t) = locate_track(p, track_id)?;
    if p.sequences[s].tracks[t].locked {
        return Err(CommandError::Locked(track_id.to_string()));
    }
    let at = c.start;
    if let Some(span) = p.sequences[s].tracks[t].clips.iter().find(|x| x.start < at && x.end > at).map(|x| x.id.clone())
    {
        split_clip(p, &span, at)?;
    }
    let after: Vec<Id> =
        p.sequences[s].tracks[t].clips.iter().filter(|x| x.start >= at).map(|x| x.id.clone()).collect();
    let mut ids: Vec<Id> = Vec::new();
    for id in &after {
        for x in moving_ids(p, id)? {
            if !ids.contains(&x) {
                ids.push(x);
            }
        }
    }
    let d = c.duration();
    let mut taken = Vec::new();
    for id in &ids {
        ensure_unlocked(p, id)?;
        taken.push(take_clip(p, id)?);
    }
    for (s, t, mut x) in taken {
        x.start += d;
        x.end += d;
        insert_clip(&mut p.sequences[s].tracks[t], x)?;
    }
    insert_clip(&mut p.sequences[s].tracks[t], c)
}

pub fn overwrite(p: &mut Project, track_id: &str, c: Clip) -> Result<()> {
    let (s, t) = locate_track(p, track_id)?;
    if p.sequences[s].tracks[t].locked {
        return Err(CommandError::Locked(track_id.to_string()));
    }
    let (a, b) = (c.start, c.end);
    let old = std::mem::take(&mut p.sequences[s].tracks[t].clips);
    let mut keep = Vec::new();
    for x in old {
        if x.end <= a || x.start >= b {
            keep.push(x);
            continue;
        }
        if x.start < a {
            let mut l = recut(&x, 0, a - x.start, true);
            l.transition_out = None;
            keep.push(l);
        }
        if x.end > b {
            let mut r = recut(&x, b - x.start, x.duration(), true);
            if x.start < a {
                renew_ids(&mut r);
            }
            r.animations.retain(|e| e.placement != Some(AnimationPlacement::In));
            keep.push(r);
        }
    }
    keep.sort_by_key(|x| x.start);
    p.sequences[s].tracks[t].clips = keep;
    insert_clip(&mut p.sequences[s].tracks[t], c)
}

pub fn freeze_frame(p: &mut Project, clip_id: &str, at: Ticks, duration: Ticks) -> Result<()> {
    if duration <= 0 {
        return Err(invalid("freeze duration must be positive"));
    }
    let c = clip(p, clip_id)?.clone();
    if !matches!(c.source, ClipSource::Media { .. } | ClipSource::Sequence { .. }) {
        return Err(invalid("freeze frame needs a media or nested clip"));
    }
    if at < c.start || at >= c.end {
        return Err(invalid("freeze point outside clip"));
    }
    let local = at - c.start;
    let mut frozen = c.clone();
    renew_ids(&mut frozen);
    frozen.source_in = crate::eval::source_time(&c, local);
    frozen.start = at;
    frozen.end = at + duration;
    frozen.speed = Speed { freeze: true, ..Speed::default() };
    edit::for_each_param(&mut frozen, |p| *p = Param::constant(p.sample(local)));
    frozen.effects.retain(|e| e.range.is_none());
    frozen.animations.clear();
    frozen.transition_out = None;
    frozen.markers.clear();
    frozen.link_group = None;
    frozen.group = None;
    frozen.audio = AudioSettings { muted: true, ..AudioSettings::default() };
    if at > c.start {
        split_clip(p, clip_id, at)?;
    }
    let mut tracks = BTreeSet::new();
    for id in linked_ids(p, clip_id)? {
        let (s, t, _) = locate(p, &id)?;
        ensure_unlocked(p, &id)?;
        tracks.insert((s, t));
    }
    let (s, t, _) = locate(p, clip_id)?;
    tracks.insert((s, t));
    shift_tracks(p, &tracks, at, duration, &[])?;
    insert_clip(&mut p.sequences[s].tracks[t], frozen)
}

pub fn detach_audio(p: &mut Project, clip_id: &str, track_id: Option<&str>) -> Result<()> {
    ensure_unlocked(p, clip_id)?;
    let (s, t, _) = locate(p, clip_id)?;
    let c = clip(p, clip_id)?.clone();
    if p.sequences[s].tracks[t].kind != TrackKind::Video {
        return Err(invalid("detach_audio applies to clips on video tracks"));
    }
    let ClipSource::Media { media_id } = &c.source else { return Err(invalid("clip has no media audio")) };
    let m = p.media(media_id).ok_or_else(|| not_found(format!("media {media_id}")))?;
    if !m.has_audio {
        return Err(invalid("the clip's media has no audio"));
    }
    // Camera clips already carry their audio on a linked audio clip.
    let partner = all_clips(p)
        .any(|x| x.id != c.id && c.link_group.is_some() && x.link_group == c.link_group && x.source == c.source);
    if partner {
        return set_group(p, std::slice::from_ref(&c.id), true, false);
    }
    let mut a = c.clone();
    renew_ids(&mut a);
    a.name = format!("{} (audio)", c.name);
    a.transform = Transform::default();
    a.opacity = Param::float(1.0);
    a.blend_mode = BlendMode::Normal;
    a.effects.clear();
    a.animations.clear();
    a.transition_out = None;
    a.markers.clear();
    a.link_group = None;
    a.group = None;
    let target = match track_id {
        Some(id) => {
            let (ts, tt) = locate_track(p, id)?;
            if p.sequences[ts].tracks[tt].kind != TrackKind::Audio {
                return Err(invalid(format!("track {id} is not an audio track")));
            }
            (ts, tt)
        }
        None => {
            let free = p.sequences[s].tracks.iter().position(|tr| {
                tr.kind == TrackKind::Audio
                    && !tr.locked
                    && tr.clips.iter().all(|x| x.end <= a.start || x.start >= a.end)
            });
            match free {
                Some(i) => (s, i),
                None => {
                    let sid = p.sequences[s].id.clone();
                    add_track(p, Some(&sid), TrackKind::Audio, None)?;
                    (s, p.sequences[s].tracks.len() - 1)
                }
            }
        }
    };
    insert_clip(&mut p.sequences[target.0].tracks[target.1], a)?;
    let v = p.clip_mut(clip_id).expect("exists");
    v.audio.muted = true;
    v.link_group = None;
    Ok(())
}

/// Link (`link = true`) or group clips; `on = false` removes the
/// links/groups of the given clips (and of their whole groups).
pub fn set_group(p: &mut Project, ids: &[Id], link: bool, on: bool) -> Result<()> {
    for id in ids {
        clip(p, id)?;
    }
    if on {
        if ids.len() < 2 {
            return Err(invalid("give at least two clips"));
        }
        let seq = locate(p, &ids[0])?.0;
        if ids.iter().any(|i| p.locate_clip(i).map(|l| l.0) != Some(seq)) {
            return Err(invalid("clips must be in the same sequence"));
        }
        let g = new_id();
        for id in ids {
            let c = p.clip_mut(id).expect("exists");
            if link {
                c.link_group = Some(g.clone());
            } else {
                c.group = Some(g.clone());
            }
        }
        return Ok(());
    }
    let groups: BTreeSet<Id> = ids
        .iter()
        .filter_map(|i| p.clip(i))
        .filter_map(|c| if link { c.link_group.clone() } else { c.group.clone() })
        .collect();
    if groups.is_empty() {
        return Err(invalid(if link { "clips are not linked" } else { "clips are not grouped" }));
    }
    for g in groups {
        let members: Vec<(usize, usize, usize)> = p
            .sequences
            .iter()
            .enumerate()
            .flat_map(|(si, s)| {
                s.tracks
                    .iter()
                    .enumerate()
                    .flat_map(move |(ti, t)| t.clips.iter().enumerate().map(move |(ci, c)| (si, ti, ci, c)))
            })
            .filter(|(_, _, _, c)| if link { c.link_group.as_ref() == Some(&g) } else { c.group.as_ref() == Some(&g) })
            .map(|(a, b, c, _)| (a, b, c))
            .collect();
        // Unlinked camera video must not play the audio its partner carries.
        let audio_sources: Vec<ClipSource> = members
            .iter()
            .filter(|(s, t, _)| p.sequences[*s].tracks[*t].kind == TrackKind::Audio)
            .map(|(s, t, c)| p.sequences[*s].tracks[*t].clips[*c].source.clone())
            .collect();
        for (s, t, ci) in members {
            let kind = p.sequences[s].tracks[t].kind;
            let c = &mut p.sequences[s].tracks[t].clips[ci];
            if link {
                c.link_group = None;
                if kind == TrackKind::Video && audio_sources.contains(&c.source) {
                    c.audio.muted = true;
                }
            } else {
                c.group = None;
            }
        }
    }
    Ok(())
}

pub fn compound(p: &mut Project, ids: &[Id], name: Option<&str>) -> Result<()> {
    if ids.is_empty() {
        return Err(invalid("give the clips to nest"));
    }
    let seq_idx = locate(p, &ids[0])?.0;
    let mut members = Vec::new();
    for id in ids {
        let (s, t, _) = locate(p, id)?;
        if s != seq_idx {
            return Err(invalid("clips must be in the same sequence"));
        }
        ensure_unlocked(p, id)?;
        members.push((t, id.clone()));
    }
    let start = ids.iter().filter_map(|i| p.clip(i)).map(|c| c.start).min().unwrap_or(0);
    let end = ids.iter().filter_map(|i| p.clip(i)).map(|c| c.end).max().unwrap_or(0);
    let src_tracks: BTreeSet<usize> = members.iter().map(|(t, _)| *t).collect();
    let mut nested = Sequence {
        id: new_id(),
        name: name.unwrap_or("Compound Clip").to_string(),
        tracks: Vec::new(),
        markers: Vec::new(),
        extra: Extra::Null,
    };
    for &ti in &src_tracks {
        let src = &p.sequences[seq_idx].tracks[ti];
        nested.tracks.push(Track {
            id: new_id(),
            kind: src.kind,
            name: src.name.clone(),
            locked: false,
            muted: src.muted,
            hidden: src.hidden,
            clips: Vec::new(),
            height: None,
            extra: Extra::Null,
        });
    }
    for (ti, id) in &members {
        let (_, _, mut c) = take_clip(p, id)?;
        c.start -= start;
        c.end -= start;
        let k = src_tracks.iter().position(|x| x == ti).expect("collected");
        insert_clip(&mut nested.tracks[k], c)?;
    }
    let seq = &p.sequences[seq_idx];
    let host = src_tracks
        .iter()
        .copied()
        .find(|t| seq.tracks[*t].kind == TrackKind::Video)
        .unwrap_or(*src_tracks.iter().next().expect("non-empty"));
    let compound_clip = Clip {
        id: new_id(),
        name: nested.name.clone(),
        source: ClipSource::Sequence { sequence_id: nested.id.clone() },
        start,
        end,
        source_in: 0,
        speed: Speed::default(),
        transform: Transform::default(),
        opacity: Param::float(1.0),
        blend_mode: BlendMode::Normal,
        audio: AudioSettings::default(),
        effects: Vec::new(),
        animations: Vec::new(),
        transition_out: None,
        link_group: None,
        group: None,
        disabled: false,
        markers: Vec::new(),
        extra: Extra::Null,
    };
    p.sequences.push(nested);
    insert_clip(&mut p.sequences[seq_idx].tracks[host], compound_clip)
}

pub fn paste_attributes(p: &mut Project, from: &str, to: &[Id], attrs: &[ClipAttribute]) -> Result<()> {
    let src = clip(p, from)?.clone();
    if attrs.is_empty() {
        return Err(invalid("choose at least one attribute"));
    }
    for id in to {
        if attrs.contains(&ClipAttribute::Speed) {
            match &src.speed.curve {
                Some(c) => set_speed_ramp(p, id, &c.preset, Some(&c.points))?,
                None => set_speed(p, id, src.speed.rate, Some(src.speed.reverse))?,
            }
        }
        with_clip(p, id, |c| {
            for a in attrs {
                match a {
                    ClipAttribute::Transform => {
                        let t = &mut c.transform;
                        let s = &src.transform;
                        t.position_x = s.position_x.clone();
                        t.position_y = s.position_y.clone();
                        t.scale_x = s.scale_x.clone();
                        t.scale_y = s.scale_y.clone();
                        t.rotation = s.rotation.clone();
                        t.flip_h = s.flip_h;
                        t.flip_v = s.flip_v;
                    }
                    ClipAttribute::Crop => {
                        c.transform.crop = src.transform.crop;
                        c.transform.fit = src.transform.fit;
                    }
                    ClipAttribute::Opacity => c.opacity = src.opacity.clone(),
                    ClipAttribute::BlendMode => c.blend_mode = src.blend_mode,
                    ClipAttribute::Effects => {
                        c.effects = src.effects.clone();
                        c.effects.iter_mut().for_each(|e| {
                            e.id = new_id();
                            e.extra = Extra::Null;
                        });
                    }
                    ClipAttribute::Animations => {
                        c.animations = src.animations.iter().filter(|a| a.placement.is_some()).cloned().collect();
                        c.animations.iter_mut().for_each(|e| {
                            e.id = new_id();
                            e.extra = Extra::Null;
                        });
                        edit::reanchor_animations(c);
                    }
                    ClipAttribute::Audio => {
                        let (fi, fo) = (src.audio.fade_in.min(c.duration()), src.audio.fade_out.min(c.duration()));
                        c.audio = AudioSettings { fade_in: fi, fade_out: fo, ..src.audio.clone() };
                    }
                    ClipAttribute::Speed => {}
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Scales clip-local times (keyframes, ranges, markers) by `k` after a retime.
fn scale_local(c: &mut Clip, k: f64) {
    if (k - 1.0).abs() < 1e-12 {
        return;
    }
    let sc = |t: Ticks| (t as f64 * k).round() as Ticks;
    edit::for_each_param(c, |p| p.keyframes.iter_mut().for_each(|kf| kf.time = sc(kf.time)));
    for e in c.effects.iter_mut().chain(c.animations.iter_mut()).filter(|e| e.placement.is_none()) {
        if let Some((a, b)) = &mut e.range {
            *a = sc(*a);
            *b = sc(*b);
        }
    }
    for m in &mut c.markers {
        m.time = sc(m.time);
    }
}

fn check_all_tracks(p: &Project) -> Result<()> {
    for s in &p.sequences {
        for t in &s.tracks {
            sort_and_check(t)?;
        }
    }
    Ok(())
}

pub fn set_speed(p: &mut Project, clip_id: &str, rate: f64, reverse: Option<bool>) -> Result<()> {
    if !(rate > 0.0 && rate <= 100.0) {
        return Err(invalid("speed must be in (0, 100]"));
    }
    for id in linked_ids(p, clip_id)? {
        with_clip(p, &id, |c| {
            let span = speed::source_span(c) as f64;
            let old = c.duration();
            c.speed.rate = rate;
            c.speed.ramp = None;
            c.speed.curve = None;
            c.speed.freeze = false;
            if let Some(r) = reverse {
                c.speed.reverse = r;
            }
            c.end = c.start + ((span / rate).round() as Ticks).max(1);
            scale_local(c, c.duration() as f64 / old.max(1) as f64);
            edit::reanchor_animations(c);
            Ok(())
        })?;
    }
    check_all_tracks(p)
}

pub fn set_speed_ramp(p: &mut Project, clip_id: &str, preset: &str, points: Option<&[SpeedPoint]>) -> Result<()> {
    let key = speed::preset_key(preset);
    let curve = match (key.as_str(), points) {
        ("none" | "off" | "normal", _) => None,
        (_, Some(pts)) => {
            let mut c = speed::custom_curve(pts).map_err(invalid)?;
            if speed::preset_curve(&key).is_some() {
                c.preset = key.clone();
            }
            Some(c)
        }
        ("custom", None) => return Err(invalid("a custom speed ramp needs `points`")),
        (k, None) => Some(speed::preset_curve(k).ok_or_else(|| {
            invalid(format!(
                "unknown speed ramp preset {preset}; use one of {}, custom or none",
                speed::PRESETS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            ))
        })?),
    };
    for id in linked_ids(p, clip_id)? {
        with_clip(p, &id, |c| {
            if c.speed.freeze {
                return Err(invalid("freeze-frame clips have no speed"));
            }
            let span = speed::source_span(c);
            let old = c.duration();
            match &curve {
                None => {
                    c.speed.ramp = None;
                    c.speed.curve = None;
                    c.speed.rate = 1.0;
                    c.end = c.start + span.max(1);
                }
                Some(cv) => {
                    let (ramp, dur) = speed::ramp_for(cv, span);
                    c.speed.rate = ramp.value.as_f64().unwrap_or(1.0);
                    c.speed.ramp = Some(ramp);
                    c.speed.curve = Some(cv.clone());
                    c.end = c.start + dur;
                }
            }
            scale_local(c, c.duration() as f64 / old.max(1) as f64);
            edit::reanchor_animations(c);
            Ok(())
        })?;
    }
    check_all_tracks(p)
}

// -- parameters ----------------------------------------------------------------

fn path(param: &str) -> Result<ParamPath> {
    ParamPath::parse(param)
        .ok_or_else(|| invalid(format!("unknown parameter path `{param}`; use {}", edit::PARAM_PATHS)))
}

fn check_value(path: &ParamPath, v: &Value) -> Result<()> {
    if !matches!(path, ParamPath::Effect { .. }) && !matches!(v, Value::Float(f) if f.is_finite()) {
        return Err(invalid("this parameter takes a float value: {\"type\":\"float\",\"value\":1.0}"));
    }
    if matches!(path, ParamPath::Speed) && v.as_f64().is_some_and(|r| r <= 0.0) {
        return Err(invalid("speed must be positive"));
    }
    Ok(())
}

fn speed_changed(c: &mut Clip, path: &ParamPath) {
    if *path == ParamPath::Speed {
        c.speed.curve = None;
        if let Some(r) = &c.speed.ramp {
            let span = crate::eval::integrate_rate(r, c.duration());
            c.speed.rate = span as f64 / c.duration().max(1) as f64;
        }
    }
}

pub fn set_param(p: &mut Project, clip_id: &str, param: &str, value: &Value) -> Result<()> {
    let path = path(param)?;
    check_value(&path, value)?;
    with_clip(p, clip_id, |c| {
        if path == ParamPath::Speed {
            let r = value.as_f64().unwrap_or(1.0);
            c.speed.ramp = None;
            c.speed.curve = None;
            c.speed.rate = r;
            return Ok(());
        }
        for prm in path.params_mut(c, Some(value)).map_err(not_found)? {
            if std::mem::discriminant(&prm.value) != std::mem::discriminant(value) {
                return Err(invalid("value type does not match the parameter"));
            }
            *prm = Param::constant(value.clone());
        }
        Ok(())
    })
}

pub fn add_keyframe(
    p: &mut Project,
    clip_id: &str,
    param: &str,
    time: Ticks,
    value: &Value,
    interp: Interpolation,
) -> Result<()> {
    let path = path(param)?;
    check_value(&path, value)?;
    with_clip(p, clip_id, |c| {
        if time < 0 || time > c.duration() {
            return Err(invalid(format!("keyframe time must be within the clip (0..={})", c.duration())));
        }
        for prm in path.params_mut(c, Some(value)).map_err(not_found)? {
            if std::mem::discriminant(&prm.value) != std::mem::discriminant(value) {
                return Err(invalid("value type does not match the parameter"));
            }
            edit::upsert_keyframe(prm, Keyframe { time, value: value.clone(), interpolation: interp });
        }
        speed_changed(c, &path);
        Ok(())
    })
}

pub fn remove_keyframe(p: &mut Project, clip_id: &str, param: &str, time: Ticks) -> Result<()> {
    let path = path(param)?;
    with_clip(p, clip_id, |c| {
        let mut any = false;
        for prm in path.params_mut(c, None).map_err(not_found)? {
            any |= edit::remove_keyframe(prm, time);
        }
        if !any {
            return Err(not_found(format!("keyframe at {time} on {param}")));
        }
        speed_changed(c, &path);
        Ok(())
    })
}

pub fn move_keyframe(p: &mut Project, clip_id: &str, param: &str, from: Ticks, to: Ticks) -> Result<()> {
    let path = path(param)?;
    with_clip(p, clip_id, |c| {
        if to < 0 || to > c.duration() {
            return Err(invalid("keyframe time must be within the clip"));
        }
        let mut any = false;
        for prm in path.params_mut(c, None).map_err(not_found)? {
            if let Some(i) = prm.keyframes.iter().position(|k| k.time == from) {
                let mut k = prm.keyframes.remove(i);
                k.time = to;
                prm.keyframes.retain(|x| x.time != to);
                edit::upsert_keyframe(prm, k);
                any = true;
            }
        }
        if !any {
            return Err(not_found(format!("keyframe at {from} on {param}")));
        }
        speed_changed(c, &path);
        Ok(())
    })
}

pub fn clear_keyframes(p: &mut Project, clip_id: &str, param: &str) -> Result<()> {
    let path = path(param)?;
    with_clip(p, clip_id, |c| {
        for prm in path.params_mut(c, None).map_err(not_found)? {
            edit::clear_keyframes(prm, 0);
        }
        speed_changed(c, &path);
        Ok(())
    })
}

// -- animations ----------------------------------------------------------------

const DEFAULT_ANIMATION: Ticks = TICKS_PER_SECOND;

pub fn add_animation(
    p: &mut Project,
    clip_id: &str,
    kind: &str,
    placement: AnimationPlacement,
    duration: Option<Ticks>,
    params: BTreeMap<String, Param>,
    id: Option<Id>,
) -> Result<()> {
    if kind.trim().is_empty() {
        return Err(invalid("animation kind is empty"));
    }
    if duration.is_some_and(|d| d <= 0) {
        return Err(invalid("animation duration must be positive"));
    }
    with_clip(p, clip_id, |c| {
        use AnimationPlacement as A;
        c.animations.retain(|a| match (a.placement, placement) {
            (None, _) => true,
            (Some(x), y) if x == y => false,
            (Some(A::Combo), _) | (_, A::Combo) => false,
            _ => true,
        });
        let len = c.duration();
        let d = duration.unwrap_or(DEFAULT_ANIMATION).min(len);
        let range = match placement {
            A::In => (0, d),
            A::Out => (len - d, len),
            A::Loop | A::Combo => (0, len),
        };
        c.animations.push(EffectInstance {
            id: id.unwrap_or_else(new_id),
            kind: kind.to_string(),
            enabled: true,
            params,
            range: Some(range),
            placement: Some(placement),
            extra: Extra::Null,
        });
        edit::reanchor_animations(c);
        Ok(())
    })
}

pub fn remove_animation(
    p: &mut Project,
    clip_id: &str,
    id: Option<&str>,
    placement: Option<AnimationPlacement>,
) -> Result<()> {
    if id.is_none() && placement.is_none() {
        return Err(invalid("give animation_id or placement"));
    }
    with_clip(p, clip_id, |c| {
        let before = c.animations.len();
        c.animations.retain(|a| !(id.is_some_and(|i| a.id == i) || (placement.is_some() && a.placement == placement)));
        if c.animations.len() == before {
            return Err(not_found("animation"));
        }
        Ok(())
    })
}

pub fn replace_animation(
    p: &mut Project,
    clip_id: &str,
    id: &str,
    kind: Option<&str>,
    duration: Option<Ticks>,
    params: Option<&BTreeMap<String, Param>>,
) -> Result<()> {
    with_clip(p, clip_id, |c| {
        let len = c.duration();
        let a = c.animations.iter_mut().find(|a| a.id == id).ok_or_else(|| not_found(format!("animation {id}")))?;
        if let Some(k) = kind {
            a.kind = k.to_string();
            a.extra = Extra::Null;
        }
        if let Some(prm) = params {
            a.params = prm.clone();
        }
        if let Some(d) = duration {
            if d <= 0 {
                return Err(invalid("animation duration must be positive"));
            }
            let d = d.min(len);
            a.range = Some(match a.placement {
                Some(AnimationPlacement::Out) => (len - d, len),
                _ => (a.range.map_or(0, |r| r.0), a.range.map_or(0, |r| r.0) + d),
            });
        }
        edit::reanchor_animations(c);
        Ok(())
    })
}

// -- markers -------------------------------------------------------------------

pub fn add_marker(p: &mut Project, sequence_id: Option<&str>, clip_id: Option<&str>, m: &Marker) -> Result<()> {
    if m.time < 0 || m.duration < 0 {
        return Err(invalid("marker time and duration must not be negative"));
    }
    let exists = p
        .sequences
        .iter()
        .flat_map(|s| s.markers.iter())
        .chain(all_clips(p).flat_map(|c| c.markers.iter()))
        .any(|x| x.id == m.id);
    if exists {
        return Err(invalid(format!("marker {} exists", m.id)));
    }
    match clip_id {
        Some(cid) => {
            let c = p.clip_mut(cid).ok_or_else(|| not_found(format!("clip {cid}")))?;
            if m.time >= c.duration() {
                return Err(invalid("clip marker time is clip-local and must be inside the clip"));
            }
            c.markers.push(m.clone());
            c.markers.sort_by_key(|x| x.time);
        }
        None => {
            let sid = sequence_id.map(str::to_string).unwrap_or_else(|| p.main_sequence.clone());
            let s = p.sequence_mut(&sid).ok_or_else(|| not_found(format!("sequence {sid}")))?;
            s.markers.push(m.clone());
            s.markers.sort_by_key(|x| x.time);
        }
    }
    Ok(())
}

fn markers_mut(p: &mut Project) -> impl Iterator<Item = &mut Vec<Marker>> {
    p.sequences.iter_mut().flat_map(|s| {
        std::iter::once(&mut s.markers)
            .chain(s.tracks.iter_mut().flat_map(|t| t.clips.iter_mut().map(|c| &mut c.markers)))
    })
}

pub fn update_marker(p: &mut Project, m: &Marker) -> Result<()> {
    for list in markers_mut(p) {
        if let Some(x) = list.iter_mut().find(|x| x.id == m.id) {
            *x = m.clone();
            list.sort_by_key(|x| x.time);
            return Ok(());
        }
    }
    Err(not_found(format!("marker {}", m.id)))
}

pub fn remove_marker(p: &mut Project, id: &str) -> Result<()> {
    for list in markers_mut(p) {
        if let Some(i) = list.iter().position(|x| x.id == id) {
            list.remove(i);
            return Ok(());
        }
    }
    Err(not_found(format!("marker {id}")))
}

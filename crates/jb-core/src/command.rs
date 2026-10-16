//! The command bus. Every mutation of a project goes through a `Command`, so
//! the UI, the CLI and agents share one serializable, undoable API.
//!
//! Undo is snapshot based: projects are small compared to media, so keeping
//! whole-project snapshots is simpler and more robust than hand-written
//! inverse operations.

use crate::model::*;
use crate::param::Param;
use crate::time::Ticks;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CommandError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("track is locked: {0}")]
    Locked(String),
}

pub type Result<T> = std::result::Result<T, CommandError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    AddMedia {
        asset: MediaAsset,
    },
    RemoveMedia {
        media_id: Id,
    },
    AddTrack {
        sequence_id: Option<Id>,
        kind: TrackKind,
        index: Option<usize>,
    },
    RemoveTrack {
        track_id: Id,
    },
    SetTrackFlags {
        track_id: Id,
        locked: Option<bool>,
        muted: Option<bool>,
        hidden: Option<bool>,
    },
    /// Inserts a clip; fails if it overlaps an existing clip on the track.
    AddClip {
        track_id: Id,
        clip: Clip,
    },
    RemoveClip {
        clip_id: Id,
        ripple: bool,
    },
    /// Moves a clip (and its link group) to a new start time and optionally another track.
    MoveClip {
        clip_id: Id,
        start: Ticks,
        track_id: Option<Id>,
    },
    /// Changes clip edges. `start`/`end` are new timeline positions; the source in-point follows the start edge.
    TrimClip {
        clip_id: Id,
        start: Option<Ticks>,
        end: Option<Ticks>,
    },
    /// Splits a clip (and its link group) at timeline time `at`.
    SplitClip {
        clip_id: Id,
        at: Ticks,
    },
    SetSpeed {
        clip_id: Id,
        rate: f64,
        reverse: Option<bool>,
    },
    SetTransform {
        clip_id: Id,
        transform: Transform,
    },
    SetOpacity {
        clip_id: Id,
        opacity: Param,
    },
    SetAudio {
        clip_id: Id,
        audio: AudioSettings,
    },
    AddEffect {
        clip_id: Id,
        effect: EffectInstance,
    },
    RemoveEffect {
        clip_id: Id,
        effect_id: Id,
    },
    SetEffectParams {
        clip_id: Id,
        effect_id: Id,
        params: BTreeMap<String, Param>,
    },
    SetTransition {
        clip_id: Id,
        transition: Option<Transition>,
    },
    SetTitle {
        clip_id: Id,
        title: TitleData,
    },
    RenameProject {
        name: String,
    },
    /// Applies several commands as one undo step.
    Batch {
        commands: Vec<Command>,
    },
}

fn track_mut<'a>(p: &'a mut Project, track_id: &str) -> Result<&'a mut Track> {
    p.sequences
        .iter_mut()
        .flat_map(|s| s.tracks.iter_mut())
        .find(|t| t.id == track_id)
        .ok_or_else(|| CommandError::NotFound(format!("track {track_id}")))
}

fn sort_and_check(track: &Track) -> Result<()> {
    for w in track.clips.windows(2) {
        if w[0].end > w[1].start {
            return Err(CommandError::Invalid(format!("clips {} and {} overlap", w[0].id, w[1].id)));
        }
    }
    Ok(())
}

fn insert_clip(track: &mut Track, clip: Clip) -> Result<()> {
    if clip.end <= clip.start {
        return Err(CommandError::Invalid("clip has non-positive duration".into()));
    }
    let pos = track.clips.partition_point(|c| c.start < clip.start);
    track.clips.insert(pos, clip);
    sort_and_check(track)
}

/// Ids of the clip plus every clip in its link group.
fn linked_ids(p: &Project, clip_id: &str) -> Result<Vec<Id>> {
    let clip = p.clip(clip_id).ok_or_else(|| CommandError::NotFound(format!("clip {clip_id}")))?;
    Ok(match &clip.link_group {
        None => vec![clip.id.clone()],
        Some(g) => p
            .sequences
            .iter()
            .flat_map(|s| s.tracks.iter().flat_map(|t| t.clips.iter()))
            .filter(|c| c.link_group.as_ref() == Some(g))
            .map(|c| c.id.clone())
            .collect(),
    })
}

fn ensure_unlocked(p: &Project, clip_id: &str) -> Result<()> {
    let (s, t, _) = p.locate_clip(clip_id).ok_or_else(|| CommandError::NotFound(format!("clip {clip_id}")))?;
    let track = &p.sequences[s].tracks[t];
    if track.locked {
        return Err(CommandError::Locked(track.id.clone()));
    }
    Ok(())
}

fn take_clip(p: &mut Project, clip_id: &str) -> Result<(usize, usize, Clip)> {
    let (s, t, c) = p.locate_clip(clip_id).ok_or_else(|| CommandError::NotFound(format!("clip {clip_id}")))?;
    Ok((s, t, p.sequences[s].tracks[t].clips.remove(c)))
}

fn with_clip<F: FnOnce(&mut Clip) -> Result<()>>(p: &mut Project, clip_id: &str, f: F) -> Result<()> {
    ensure_unlocked(p, clip_id)?;
    let clip = p.clip_mut(clip_id).ok_or_else(|| CommandError::NotFound(format!("clip {clip_id}")))?;
    f(clip)
}

/// Applies a command to a project in place. On error the project may be
/// partially modified; `History::execute` guards against that.
pub fn apply(p: &mut Project, cmd: &Command) -> Result<()> {
    match cmd {
        Command::AddMedia { asset } => {
            if p.media(&asset.id).is_some() {
                return Err(CommandError::Invalid(format!("media {} exists", asset.id)));
            }
            p.media.push(asset.clone());
        }
        Command::RemoveMedia { media_id } => {
            let used = p
                .sequences
                .iter()
                .flat_map(|s| s.tracks.iter().flat_map(|t| t.clips.iter()))
                .any(|c| matches!(&c.source, ClipSource::Media { media_id: m } if m == media_id));
            if used {
                return Err(CommandError::Invalid(format!("media {media_id} is in use")));
            }
            p.media.retain(|m| &m.id != media_id);
        }
        Command::AddTrack { sequence_id, kind, index } => {
            let sid = sequence_id.clone().unwrap_or_else(|| p.main_sequence.clone());
            let seq = p.sequence_mut(&sid).ok_or_else(|| CommandError::NotFound(format!("sequence {sid}")))?;
            let n = seq.tracks.iter().filter(|t| t.kind == *kind).count() + 1;
            let name = match kind {
                TrackKind::Video => format!("Video {n}"),
                TrackKind::Audio => format!("Audio {n}"),
            };
            let track = Track {
                id: new_id(),
                kind: *kind,
                name,
                locked: false,
                muted: false,
                hidden: false,
                clips: Vec::new(),
                extra: Extra::Null,
            };
            let i = index.unwrap_or(seq.tracks.len()).min(seq.tracks.len());
            seq.tracks.insert(i, track);
        }
        Command::RemoveTrack { track_id } => {
            let mut found = false;
            for s in &mut p.sequences {
                let before = s.tracks.len();
                s.tracks.retain(|t| &t.id != track_id);
                found |= s.tracks.len() != before;
            }
            if !found {
                return Err(CommandError::NotFound(format!("track {track_id}")));
            }
        }
        Command::SetTrackFlags { track_id, locked, muted, hidden } => {
            let t = track_mut(p, track_id)?;
            if let Some(v) = locked {
                t.locked = *v;
            }
            if let Some(v) = muted {
                t.muted = *v;
            }
            if let Some(v) = hidden {
                t.hidden = *v;
            }
        }
        Command::AddClip { track_id, clip } => {
            let t = track_mut(p, track_id)?;
            if t.locked {
                return Err(CommandError::Locked(track_id.clone()));
            }
            insert_clip(t, clip.clone())?;
        }
        Command::RemoveClip { clip_id, ripple } => {
            for id in linked_ids(p, clip_id)? {
                ensure_unlocked(p, &id)?;
                let (s, t, clip) = take_clip(p, &id)?;
                if *ripple {
                    let gap = clip.duration();
                    for c in p.sequences[s].tracks[t].clips.iter_mut().filter(|c| c.start >= clip.end) {
                        c.start -= gap;
                        c.end -= gap;
                    }
                }
            }
        }
        Command::MoveClip { clip_id, start, track_id } => {
            let anchor = p.clip(clip_id).ok_or_else(|| CommandError::NotFound(format!("clip {clip_id}")))?.start;
            let delta = start - anchor;
            for id in linked_ids(p, clip_id)? {
                ensure_unlocked(p, &id)?;
                let (s, t, mut clip) = take_clip(p, &id)?;
                if clip.start + delta < 0 {
                    return Err(CommandError::Invalid("clip would start before zero".into()));
                }
                clip.start += delta;
                clip.end += delta;
                let target = if &id == clip_id { track_id.clone() } else { None };
                match target {
                    Some(tid) => insert_clip(track_mut(p, &tid)?, clip)?,
                    None => insert_clip(&mut p.sequences[s].tracks[t], clip)?,
                }
            }
        }
        Command::TrimClip { clip_id, start, end } => {
            for id in linked_ids(p, clip_id)? {
                ensure_unlocked(p, &id)?;
                let (s, t, mut clip) = take_clip(p, &id)?;
                if let Some(ns) = start {
                    let d = ns - clip.start;
                    clip.source_in += (d as f64 * clip.speed.rate).round() as Ticks;
                    if clip.source_in < 0 {
                        return Err(CommandError::Invalid("trim before source start".into()));
                    }
                    clip.start = *ns;
                }
                if let Some(ne) = end {
                    clip.end = *ne;
                }
                insert_clip(&mut p.sequences[s].tracks[t], clip)?;
            }
        }
        Command::SplitClip { clip_id, at } => {
            for id in linked_ids(p, clip_id)? {
                ensure_unlocked(p, &id)?;
                let (s, t, clip) = take_clip(p, &id)?;
                if !clip.contains(*at) || *at == clip.start {
                    return Err(CommandError::Invalid("split point outside clip".into()));
                }
                let mut left = clip.clone();
                let mut right = clip;
                left.end = *at;
                left.transition_out = None;
                right.id = new_id();
                right.source_in = crate::eval::source_time(&left, at - left.start);
                right.start = *at;
                left.audio.fade_out = 0;
                right.audio.fade_in = 0;
                let cut = at - left.start;
                split_ranged(&mut left.animations, &mut right.animations, cut);
                split_ranged(&mut left.effects, &mut right.effects, cut);
                let track = &mut p.sequences[s].tracks[t];
                insert_clip(track, left)?;
                insert_clip(track, right)?;
            }
            // Split halves keep a shared link group so the right halves stay
            // linked to each other but not to the left halves.
            relink_right_halves(p, clip_id, *at);
        }
        Command::SetSpeed { clip_id, rate, reverse } => {
            if *rate <= 0.0 {
                return Err(CommandError::Invalid("speed must be positive".into()));
            }
            for id in linked_ids(p, clip_id)? {
                with_clip(p, &id, |c| {
                    let source_len = c.duration() as f64 * c.speed.rate;
                    c.speed.rate = *rate;
                    c.speed.ramp = None;
                    if let Some(r) = reverse {
                        c.speed.reverse = *r;
                    }
                    c.end = c.start + (source_len / rate).round() as Ticks;
                    Ok(())
                })?;
            }
            // Retiming changes clip ends; re-validate affected tracks.
            for s in &p.sequences {
                for t in &s.tracks {
                    sort_and_check(t)?;
                }
            }
        }
        Command::SetTransform { clip_id, transform } => with_clip(p, clip_id, |c| {
            c.transform = transform.clone();
            Ok(())
        })?,
        Command::SetOpacity { clip_id, opacity } => with_clip(p, clip_id, |c| {
            c.opacity = opacity.clone();
            Ok(())
        })?,
        Command::SetAudio { clip_id, audio } => with_clip(p, clip_id, |c| {
            c.audio = audio.clone();
            Ok(())
        })?,
        Command::AddEffect { clip_id, effect } => with_clip(p, clip_id, |c| {
            c.effects.push(effect.clone());
            Ok(())
        })?,
        Command::RemoveEffect { clip_id, effect_id } => with_clip(p, clip_id, |c| {
            let before = c.effects.len();
            c.effects.retain(|e| &e.id != effect_id);
            if c.effects.len() == before {
                return Err(CommandError::NotFound(format!("effect {effect_id}")));
            }
            Ok(())
        })?,
        Command::SetEffectParams { clip_id, effect_id, params } => with_clip(p, clip_id, |c| {
            let e = c
                .effects
                .iter_mut()
                .chain(c.animations.iter_mut())
                .find(|e| &e.id == effect_id)
                .ok_or_else(|| CommandError::NotFound(format!("effect {effect_id}")))?;
            for (k, v) in params {
                e.params.insert(k.clone(), v.clone());
            }
            Ok(())
        })?,
        Command::SetTransition { clip_id, transition } => with_clip(p, clip_id, |c| {
            c.transition_out = transition.clone();
            Ok(())
        })?,
        Command::SetTitle { clip_id, title } => with_clip(p, clip_id, |c| match &mut c.source {
            ClipSource::Title(t) => {
                *t = title.clone();
                Ok(())
            }
            _ => Err(CommandError::Invalid("clip is not a title".into())),
        })?,
        Command::RenameProject { name } => p.name = name.clone(),
        Command::Batch { commands } => {
            for c in commands {
                apply(p, c)?;
            }
        }
    }
    Ok(())
}

/// Distributes effects with an active range (entry/exit animations, timed
/// effects) between the two halves of a split at clip-local time `cut`.
/// Unranged effects stay on both halves.
fn split_ranged(left: &mut Vec<EffectInstance>, right: &mut Vec<EffectInstance>, cut: Ticks) {
    left.retain(|e| e.range.is_none_or(|(a, _)| a < cut));
    for e in left.iter_mut() {
        if let Some((_, b)) = &mut e.range {
            *b = (*b).min(cut);
        }
    }
    right.retain(|e| e.range.is_none_or(|(_, b)| b > cut));
    for e in right.iter_mut() {
        e.id = new_id();
        if let Some((a, b)) = &mut e.range {
            *a = (*a - cut).max(0);
            *b -= cut;
        }
    }
}

fn relink_right_halves(p: &mut Project, clip_id: &str, at: Ticks) {
    let Some(group) = p.clip(clip_id).and_then(|c| c.link_group.clone()) else { return };
    let new_group = new_id();
    for s in &mut p.sequences {
        for t in &mut s.tracks {
            for c in &mut t.clips {
                if c.link_group.as_ref() == Some(&group) && c.start == at {
                    c.link_group = Some(new_group.clone());
                }
            }
        }
    }
}

/// Undo/redo history around a project.
#[derive(Debug, Clone)]
pub struct History {
    pub project: Project,
    undo: Vec<Project>,
    redo: Vec<Project>,
    limit: usize,
}

impl History {
    pub fn new(project: Project) -> Self {
        Self { project, undo: Vec::new(), redo: Vec::new(), limit: 200 }
    }

    /// Applies a command atomically: on error the project is left untouched.
    pub fn execute(&mut self, cmd: &Command) -> Result<()> {
        let mut next = self.project.clone();
        apply(&mut next, cmd)?;
        let prev = std::mem::replace(&mut self.project, next);
        self.undo.push(prev);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.project, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.project, next));
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

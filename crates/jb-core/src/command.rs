//! The command bus. Every mutation of a project goes through a `Command`, so
//! the UI, the CLI and agents share one serializable, undoable API.
//!
//! Undo is snapshot based: projects are small compared to media, so keeping
//! whole-project snapshots is simpler and more robust than hand-written
//! inverse operations.

use crate::model::*;
use crate::ops;
use crate::param::{Interpolation, Param, Value};
use crate::time::Ticks;
use serde::{Deserialize, Deserializer, Serialize};
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

/// Distinguishes a missing field (`None`) from an explicit `null` (`Some(None)`).
fn double_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

/// Clip properties that `paste_attributes` can copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClipAttribute {
    /// Position, scale, rotation and flip (with their keyframes).
    Transform,
    /// Crop rectangle and fit mode.
    Crop,
    Opacity,
    BlendMode,
    /// Effects and color adjustments.
    Effects,
    /// Entry/exit/loop animations.
    Animations,
    /// Volume, pan, fades, pitch, mute, normalization and equalizer.
    Audio,
    /// Speed, reverse and speed ramp (retimes the target clips).
    Speed,
}

/// Which edge `ripple_trim` moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Start,
    End,
}

/// Animation applied by `set_title_style` (Filmora title text animations).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TitleAnimation {
    /// Animation kind (registry id such as `jb.anim.fade_in`, or `filmora:<id>`).
    pub kind: String,
    #[serde(default = "default_in")]
    pub placement: AnimationPlacement,
    /// Duration in ticks (default 1 s, capped by the clip).
    #[serde(default)]
    pub duration: Option<Ticks>,
}

fn default_in() -> AnimationPlacement {
    AnimationPlacement::In
}

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
    /// Moves a track to `index` within its sequence (0 = bottom).
    MoveTrack {
        track_id: Id,
        index: usize,
    },
    RenameTrack {
        track_id: Id,
        name: String,
    },
    /// Timeline display height in UI pixels; `null` restores the default.
    SetTrackHeight {
        track_id: Id,
        height: Option<u32>,
    },
    /// Inserts a clip; fails if it overlaps an existing clip on the track.
    AddClip {
        track_id: Id,
        clip: Clip,
    },
    /// Ripple insert at `clip.start`: a clip under that time is split and
    /// everything from there on (with linked/grouped clips) moves right by
    /// the new clip's duration.
    InsertClip {
        track_id: Id,
        clip: Clip,
    },
    /// Overwrite: whatever lies on the track in `[clip.start, clip.end)` is
    /// removed or trimmed away, then the clip is placed there.
    OverwriteClip {
        track_id: Id,
        clip: Clip,
    },
    RemoveClip {
        clip_id: Id,
        ripple: bool,
    },
    /// Moves a clip (and its link group and group) to a new start time and optionally another track.
    MoveClip {
        clip_id: Id,
        start: Ticks,
        track_id: Option<Id>,
    },
    /// Changes clip edges. `start`/`end` are new timeline positions; the
    /// source in-point follows the start edge, and keyframes, markers and
    /// animations stay attached to the content.
    TrimClip {
        clip_id: Id,
        start: Option<Ticks>,
        end: Option<Ticks>,
    },
    /// Trims an edge and closes (or opens) the gap: later clips on the
    /// affected tracks shift by the change in length. `start` trims the
    /// head as if the start moved there, but the clip keeps its position.
    RippleTrim {
        clip_id: Id,
        edge: Edge,
        /// New timeline time of the edge.
        to: Ticks,
    },
    /// Roll edit: moves the cut between this clip and the adjacent next clip
    /// on its track to `at`, trimming both (and their linked partners).
    RollEdit {
        clip_id: Id,
        at: Ticks,
    },
    /// Slip edit: shifts the source range under the clip by `delta` source
    /// ticks without moving or resizing it.
    SlipClip {
        clip_id: Id,
        delta: Ticks,
    },
    /// Splits a clip (and its link group) at timeline time `at`.
    SplitClip {
        clip_id: Id,
        at: Ticks,
    },
    /// Copies a clip (and its linked partners) to `start` (default: right
    /// after it) on `track_id` (default: the same track).
    DuplicateClip {
        clip_id: Id,
        start: Option<Ticks>,
        track_id: Option<Id>,
    },
    /// Points a media clip at another media asset, keeping its timing and
    /// properties (shortened if the new media is too short).
    ReplaceMedia {
        clip_id: Id,
        media_id: Id,
    },
    /// Enables or disables a clip (disabled clips are neither shown nor heard).
    SetClipEnabled {
        clip_id: Id,
        enabled: bool,
    },
    /// Freeze frame: at timeline time `at` the frame is held for `duration`
    /// (inserted, later clips on the affected tracks move right).
    FreezeFrame {
        clip_id: Id,
        at: Ticks,
        duration: Ticks,
    },
    /// Moves the audio of a video clip to its own clip on an audio track
    /// (`track_id`, or a free/new audio track). Already linked camera audio
    /// is simply unlinked.
    DetachAudio {
        clip_id: Id,
        track_id: Option<Id>,
    },
    /// Links clips so they move, trim and split together.
    LinkClips {
        clip_ids: Vec<Id>,
    },
    /// Removes the link groups of the given clips.
    UnlinkClips {
        clip_ids: Vec<Id>,
    },
    /// Groups clips so they move together.
    GroupClips {
        clip_ids: Vec<Id>,
    },
    UngroupClips {
        clip_ids: Vec<Id>,
    },
    /// Moves clips into a new nested sequence and puts one compound clip in
    /// their place (on the lowest of their tracks).
    CreateCompoundClip {
        clip_ids: Vec<Id>,
        name: Option<String>,
    },
    /// Copies attributes of one clip onto others ("Paste attributes").
    PasteAttributes {
        from_clip_id: Id,
        to_clip_ids: Vec<Id>,
        attributes: Vec<ClipAttribute>,
    },
    /// Constant playback rate; clears any speed ramp.
    SetSpeed {
        clip_id: Id,
        rate: f64,
        reverse: Option<bool>,
    },
    /// Speed ramping. `preset`: `montage`, `hero_moment`, `bullet_time`,
    /// `jumper`, `flash_in`, `flash_out`, `custom` (uses `points`) or `none`.
    /// The source range is kept, so the clip's duration changes.
    SetSpeedRamp {
        clip_id: Id,
        preset: String,
        #[serde(default)]
        points: Option<Vec<SpeedPoint>>,
    },
    SetReverse {
        clip_id: Id,
        reverse: bool,
    },
    SetTransform {
        clip_id: Id,
        transform: Transform,
    },
    /// Crop insets `[left, top, right, bottom]`, fractions of the source size.
    SetCrop {
        clip_id: Id,
        crop: [f64; 4],
    },
    SetFlip {
        clip_id: Id,
        horizontal: Option<bool>,
        vertical: Option<bool>,
    },
    /// Constant rotation in degrees (clockwise); clears rotation keyframes.
    SetRotation {
        clip_id: Id,
        degrees: f64,
    },
    SetFitMode {
        clip_id: Id,
        fit: FitMode,
    },
    SetBlendMode {
        clip_id: Id,
        blend_mode: BlendMode,
    },
    SetOpacity {
        clip_id: Id,
        opacity: Param,
    },
    /// Sets a parameter to a constant value (clearing its keyframes). See
    /// `add_keyframe` for the `param` paths.
    SetParam {
        clip_id: Id,
        param: String,
        value: Value,
    },
    /// Adds (or replaces) a keyframe at clip-local `time`. `param` is one of
    /// `transform.position_x`, `transform.position_y`, `transform.scale_x`,
    /// `transform.scale_y`, `transform.scale`, `transform.rotation`,
    /// `opacity`, `volume`, `pan`, `speed` or `effect.<effect_id>.<param>`.
    /// Position is relative to the canvas center in canvas units, scale is a
    /// factor, rotation degrees, opacity 0..1, volume dB, pan -1..1.
    AddKeyframe {
        clip_id: Id,
        param: String,
        time: Ticks,
        value: Value,
        #[serde(default)]
        interpolation: Option<Interpolation>,
    },
    RemoveKeyframe {
        clip_id: Id,
        param: String,
        time: Ticks,
    },
    /// Moves the keyframe at `from` to `to` (replacing one already there).
    MoveKeyframe {
        clip_id: Id,
        param: String,
        from: Ticks,
        to: Ticks,
    },
    /// Removes every keyframe of a parameter, keeping its value at clip start.
    ClearKeyframes {
        clip_id: Id,
        param: String,
    },
    SetAudio {
        clip_id: Id,
        audio: AudioSettings,
    },
    /// Constant clip volume in dB (clears volume keyframes). On a video clip
    /// whose audio lives on linked audio clips, those clips are changed.
    SetVolume {
        clip_id: Id,
        gain_db: f64,
    },
    /// Stereo balance, -1 (left) .. 1 (right).
    SetPan {
        clip_id: Id,
        pan: f64,
    },
    /// Pitch shift in semitones (-12..12), independent of speed.
    SetPitch {
        clip_id: Id,
        semitones: f64,
    },
    SetMute {
        clip_id: Id,
        muted: bool,
    },
    /// Whether speed changes keep the original pitch (time-stretch).
    SetMaintainPitch {
        clip_id: Id,
        maintain_pitch: bool,
    },
    SetAudioFades {
        clip_id: Id,
        fade_in: Option<Ticks>,
        fade_out: Option<Ticks>,
    },
    /// Auto normalization to `target_lufs` (-23, -16, -14 or -12 in Filmora;
    /// `null` turns it off). `measured_lufs` is the clip's integrated
    /// loudness when known (front ends measure it).
    SetNormalization {
        clip_id: Id,
        target_lufs: Option<f64>,
        #[serde(default)]
        measured_lufs: Option<f64>,
    },
    /// Equalizer: a preset (Filmora's `default`, `classic`, `dance`, `low_bass`,
    /// `sparkle`, `pop`, `rock_n_roll`, `soft`, `techno`, `hard_rock`, `ska`,
    /// `country`, `acoustic`, `folk`, `blues`; or `electronic`, `hip_hop`,
    /// `jazz`, `metal`, `rnb`, `speech`), custom `bands`
    /// (10 gains in dB), or neither to turn it off.
    SetEqualizer {
        clip_id: Id,
        #[serde(default)]
        preset: Option<String>,
        #[serde(default)]
        bands: Option<[f64; 10]>,
    },
    /// Audio channel routing: `stereo`, `left`, `right`, `mono` or `swap`.
    SetAudioChannels {
        clip_id: Id,
        channels: ChannelMode,
    },
    /// Voice filter: `telephone`, `radio`, `megaphone`, `echo`, `small_room`,
    /// `big_room`, `robot`, `lofi`, `underwater`, or `null` to turn it off.
    SetVoiceFilter {
        clip_id: Id,
        filter: Option<String>,
    },
    /// Audio cleanup. `hum_removal`: mains frequency (50 or 60 Hz) to notch
    /// out with its harmonics; `hiss_removal`: strength 0..100. Omitted
    /// fields are kept; `null` turns one off.
    SetAudioCleanup {
        clip_id: Id,
        #[serde(default, deserialize_with = "double_option")]
        #[schemars(with = "Option<f64>")]
        hum_removal: Option<Option<f64>>,
        #[serde(default, deserialize_with = "double_option")]
        #[schemars(with = "Option<f64>")]
        hiss_removal: Option<Option<f64>>,
    },
    AddEffect {
        clip_id: Id,
        effect: EffectInstance,
    },
    /// Turns an effect or animation on or off without removing it.
    SetEffectEnabled {
        clip_id: Id,
        effect_id: Id,
        enabled: bool,
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
    /// Adds an entry (`in`), exit (`out`), `loop` or `combo` animation,
    /// replacing the one in that slot. `duration` defaults to 1 s.
    AddAnimation {
        clip_id: Id,
        kind: String,
        placement: AnimationPlacement,
        #[serde(default)]
        duration: Option<Ticks>,
        #[serde(default)]
        params: BTreeMap<String, Param>,
        #[serde(default)]
        id: Option<Id>,
    },
    /// Removes an animation by id, or the one in `placement`.
    RemoveAnimation {
        clip_id: Id,
        #[serde(default)]
        animation_id: Option<Id>,
        #[serde(default)]
        placement: Option<AnimationPlacement>,
    },
    /// Changes the kind, duration or parameters of an animation.
    ReplaceAnimation {
        clip_id: Id,
        animation_id: Id,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        duration: Option<Ticks>,
        #[serde(default)]
        params: Option<BTreeMap<String, Param>>,
    },
    SetTransition {
        clip_id: Id,
        transition: Option<Transition>,
    },
    SetTitle {
        clip_id: Id,
        title: TitleData,
    },
    /// Edits title template parameters. Works on title clips and on title
    /// template instances (a compound clip holding one title). Omitted
    /// fields are kept; `stroke`, `shadow` and `background` accept `null` to
    /// remove them.
    SetTitleStyle {
        clip_id: Id,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        font_family: Option<String>,
        /// Fraction of the canvas height.
        #[serde(default)]
        font_size: Option<f64>,
        #[serde(default)]
        color: Option<[f64; 4]>,
        #[serde(default)]
        bold: Option<bool>,
        #[serde(default)]
        italic: Option<bool>,
        #[serde(default)]
        alignment: Option<TextAlignment>,
        /// Normalized text box center (0.5, 0.5 = centered).
        #[serde(default)]
        position: Option<[f64; 2]>,
        #[serde(default, deserialize_with = "double_option")]
        #[schemars(with = "Option<Stroke>")]
        stroke: Option<Option<Stroke>>,
        #[serde(default, deserialize_with = "double_option")]
        #[schemars(with = "Option<Shadow>")]
        shadow: Option<Option<Shadow>>,
        #[serde(default, deserialize_with = "double_option")]
        #[schemars(with = "Option<[f64; 4]>")]
        background: Option<Option<[f64; 4]>>,
        #[serde(default)]
        letter_spacing: Option<f64>,
        #[serde(default)]
        line_spacing: Option<f64>,
        /// Text animation; replaces the clip's animation in that slot.
        #[serde(default)]
        animation: Option<TitleAnimation>,
    },
    /// Adds a marker to a sequence (default: main) or, with `clip_id`, to a clip.
    AddMarker {
        #[serde(default)]
        sequence_id: Option<Id>,
        #[serde(default)]
        clip_id: Option<Id>,
        marker: Marker,
    },
    /// Replaces the marker with the same id.
    UpdateMarker {
        marker: Marker,
    },
    RemoveMarker {
        marker_id: Id,
    },
    RenameProject {
        name: String,
    },
    /// Changes the project canvas and timing. Omitted fields are kept.
    /// Sizes must be positive and even (encoders need even dimensions).
    SetProjectSettings {
        #[serde(default)]
        width: Option<u32>,
        #[serde(default)]
        height: Option<u32>,
        #[serde(default)]
        frame_rate: Option<crate::time::Rational>,
        /// Canvas background color (linear RGBA).
        #[serde(default)]
        background: Option<[f64; 4]>,
    },
    /// Renames a media asset in the media library.
    RenameMedia {
        media_id: Id,
        name: String,
    },
    /// Applies several commands as one undo step.
    Batch {
        commands: Vec<Command>,
    },
}

/// Applies a command to a project in place. On error the project may be
/// partially modified; `History::execute` guards against that.
pub fn apply(p: &mut Project, cmd: &Command) -> Result<()> {
    use Command as C;
    match cmd {
        C::AddMedia { asset } => {
            if p.media(&asset.id).is_some() {
                return Err(CommandError::Invalid(format!("media {} exists", asset.id)));
            }
            p.media.push(asset.clone());
        }
        C::RemoveMedia { media_id } => {
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
        C::AddTrack { sequence_id, kind, index } => ops::add_track(p, sequence_id.as_deref(), *kind, *index)?,
        C::RemoveTrack { track_id } => {
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
        C::SetTrackFlags { track_id, locked, muted, hidden } => {
            let t = ops::track_mut(p, track_id)?;
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
        C::MoveTrack { track_id, index } => ops::move_track(p, track_id, *index)?,
        C::RenameTrack { track_id, name } => {
            if name.trim().is_empty() {
                return Err(CommandError::Invalid("track name is empty".into()));
            }
            ops::track_mut(p, track_id)?.name = name.clone();
        }
        C::SetTrackHeight { track_id, height } => {
            if height.is_some_and(|h| h == 0 || h > 2000) {
                return Err(CommandError::Invalid("track height must be in 1..=2000".into()));
            }
            ops::track_mut(p, track_id)?.height = *height;
        }
        C::AddClip { track_id, clip } => {
            let t = ops::track_mut(p, track_id)?;
            if t.locked {
                return Err(CommandError::Locked(track_id.clone()));
            }
            ops::insert_clip(t, clip.clone())?;
        }
        C::InsertClip { track_id, clip } => ops::ripple_insert(p, track_id, clip.clone())?,
        C::OverwriteClip { track_id, clip } => ops::overwrite(p, track_id, clip.clone())?,
        C::RemoveClip { clip_id, ripple } => ops::remove_clip(p, clip_id, *ripple)?,
        C::MoveClip { clip_id, start, track_id } => ops::move_clip(p, clip_id, *start, track_id.as_deref())?,
        C::TrimClip { clip_id, start, end } => ops::trim_clip(p, clip_id, *start, *end)?,
        C::RippleTrim { clip_id, edge, to } => ops::ripple_trim(p, clip_id, *edge, *to)?,
        C::RollEdit { clip_id, at } => ops::roll_edit(p, clip_id, *at)?,
        C::SlipClip { clip_id, delta } => ops::slip(p, clip_id, *delta)?,
        C::SplitClip { clip_id, at } => {
            ops::split_clip(p, clip_id, *at)?;
        }
        C::DuplicateClip { clip_id, start, track_id } => ops::duplicate(p, clip_id, *start, track_id.as_deref())?,
        C::ReplaceMedia { clip_id, media_id } => ops::replace_media(p, clip_id, media_id)?,
        C::SetClipEnabled { clip_id, enabled } => ops::with_clip(p, clip_id, |c| {
            c.disabled = !enabled;
            Ok(())
        })?,
        C::FreezeFrame { clip_id, at, duration } => ops::freeze_frame(p, clip_id, *at, *duration)?,
        C::DetachAudio { clip_id, track_id } => ops::detach_audio(p, clip_id, track_id.as_deref())?,
        C::LinkClips { clip_ids } => ops::set_group(p, clip_ids, true, true)?,
        C::UnlinkClips { clip_ids } => ops::set_group(p, clip_ids, true, false)?,
        C::GroupClips { clip_ids } => ops::set_group(p, clip_ids, false, true)?,
        C::UngroupClips { clip_ids } => ops::set_group(p, clip_ids, false, false)?,
        C::CreateCompoundClip { clip_ids, name } => ops::compound(p, clip_ids, name.as_deref())?,
        C::PasteAttributes { from_clip_id, to_clip_ids, attributes } => {
            ops::paste_attributes(p, from_clip_id, to_clip_ids, attributes)?
        }
        C::SetSpeed { clip_id, rate, reverse } => ops::set_speed(p, clip_id, *rate, *reverse)?,
        C::SetSpeedRamp { clip_id, preset, points } => ops::set_speed_ramp(p, clip_id, preset, points.as_deref())?,
        C::SetReverse { clip_id, reverse } => {
            for id in ops::linked_ids(p, clip_id)? {
                ops::with_clip(p, &id, |c| {
                    c.speed.reverse = *reverse;
                    Ok(())
                })?;
            }
        }
        C::SetTransform { clip_id, transform } => ops::with_clip(p, clip_id, |c| {
            c.transform = transform.clone();
            Ok(())
        })?,
        C::SetCrop { clip_id, crop } => {
            let ok = crop.iter().all(|v| (0.0..1.0).contains(v)) && crop[0] + crop[2] < 1.0 && crop[1] + crop[3] < 1.0;
            if !ok {
                return Err(CommandError::Invalid(
                    "crop insets must be in 0..1 and leave a non-empty rectangle (left+right < 1, top+bottom < 1)"
                        .into(),
                ));
            }
            ops::with_clip(p, clip_id, |c| {
                c.transform.crop = *crop;
                Ok(())
            })?
        }
        C::SetFlip { clip_id, horizontal, vertical } => ops::with_clip(p, clip_id, |c| {
            if let Some(h) = horizontal {
                c.transform.flip_h = *h;
            }
            if let Some(v) = vertical {
                c.transform.flip_v = *v;
            }
            Ok(())
        })?,
        C::SetRotation { clip_id, degrees } => ops::with_clip(p, clip_id, |c| {
            finite(*degrees, "rotation")?;
            c.transform.rotation = Param::float(*degrees);
            Ok(())
        })?,
        C::SetFitMode { clip_id, fit } => ops::with_clip(p, clip_id, |c| {
            c.transform.fit = *fit;
            Ok(())
        })?,
        C::SetBlendMode { clip_id, blend_mode } => ops::with_clip(p, clip_id, |c| {
            c.blend_mode = *blend_mode;
            Ok(())
        })?,
        C::SetOpacity { clip_id, opacity } => ops::with_clip(p, clip_id, |c| {
            c.opacity = opacity.clone();
            Ok(())
        })?,
        C::SetParam { clip_id, param, value } => ops::set_param(p, clip_id, param, value)?,
        C::AddKeyframe { clip_id, param, time, value, interpolation } => {
            ops::add_keyframe(p, clip_id, param, *time, value, interpolation.unwrap_or_default())?
        }
        C::RemoveKeyframe { clip_id, param, time } => ops::remove_keyframe(p, clip_id, param, *time)?,
        C::MoveKeyframe { clip_id, param, from, to } => ops::move_keyframe(p, clip_id, param, *from, *to)?,
        C::ClearKeyframes { clip_id, param } => ops::clear_keyframes(p, clip_id, param)?,
        C::SetAudio { clip_id, audio } => ops::with_clip(p, clip_id, |c| {
            c.audio = audio.clone();
            Ok(())
        })?,
        C::SetVolume { clip_id, gain_db } => {
            finite(*gain_db, "volume")?;
            ops::with_audio(p, clip_id, |a| a.gain_db = Param::float(*gain_db))?
        }
        C::SetPan { clip_id, pan } => {
            if !(-1.0..=1.0).contains(pan) {
                return Err(CommandError::Invalid("pan must be in -1..1".into()));
            }
            ops::with_audio(p, clip_id, |a| a.pan = Param::float(*pan))?
        }
        C::SetPitch { clip_id, semitones } => {
            if !(-24.0..=24.0).contains(semitones) {
                return Err(CommandError::Invalid("pitch must be in -24..24 semitones".into()));
            }
            ops::with_audio(p, clip_id, |a| a.pitch = *semitones)?
        }
        C::SetMute { clip_id, muted } => ops::with_audio(p, clip_id, |a| a.muted = *muted)?,
        C::SetMaintainPitch { clip_id, maintain_pitch } => {
            for id in ops::linked_ids(p, clip_id)? {
                ops::with_clip(p, &id, |c| {
                    c.speed.maintain_pitch = *maintain_pitch;
                    Ok(())
                })?;
            }
        }
        C::SetAudioFades { clip_id, fade_in, fade_out } => {
            if fade_in.is_some_and(|f| f < 0) || fade_out.is_some_and(|f| f < 0) {
                return Err(CommandError::Invalid("fades must not be negative".into()));
            }
            ops::with_audio(p, clip_id, |a| {
                if let Some(f) = fade_in {
                    a.fade_in = *f;
                }
                if let Some(f) = fade_out {
                    a.fade_out = *f;
                }
            })?
        }
        C::SetNormalization { clip_id, target_lufs, measured_lufs } => {
            if target_lufs.is_some_and(|t| !(-70.0..=0.0).contains(&t)) {
                return Err(CommandError::Invalid("target loudness must be in -70..0 LUFS".into()));
            }
            ops::with_audio(p, clip_id, |a| {
                a.normalize = target_lufs.map(|t| Normalization {
                    target_lufs: t,
                    measured_lufs: measured_lufs.or(a.normalize.and_then(|n| n.measured_lufs)),
                })
            })?
        }
        C::SetEqualizer { clip_id, preset, bands } => {
            let eq = match (preset, bands) {
                (_, Some(b)) => {
                    if b.iter().any(|g| !(-24.0..=24.0).contains(g)) {
                        return Err(CommandError::Invalid("equalizer gains must be in -24..24 dB".into()));
                    }
                    Some(Equalizer { preset: preset.clone().unwrap_or_else(|| "custom".into()), bands: *b })
                }
                (Some(name), None) if name == "none" || name == "off" => None,
                (Some(name), None) => Some(Equalizer::preset(name).ok_or_else(|| {
                    CommandError::Invalid(format!(
                        "unknown equalizer preset {name}; use one of {}",
                        Equalizer::PRESETS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
                    ))
                })?),
                (None, None) => None,
            };
            ops::with_audio(p, clip_id, |a| a.equalizer = eq.clone())?
        }
        C::SetAudioChannels { clip_id, channels } => ops::with_audio(p, clip_id, |a| a.channels = *channels)?,
        C::SetVoiceFilter { clip_id, filter } => {
            let f = match filter.as_deref().map(|f| f.trim().to_ascii_lowercase().replace([' ', '-'], "_")) {
                None => None,
                Some(f) if f.is_empty() || f == "none" || f == "off" => None,
                Some(f) if VOICE_FILTERS.contains(&f.as_str()) => Some(f),
                Some(f) => {
                    return Err(CommandError::Invalid(format!(
                        "unknown voice filter {f}; use one of {} or null",
                        VOICE_FILTERS.join(", ")
                    )))
                }
            };
            ops::with_audio(p, clip_id, |a| a.voice_filter = f.clone())?
        }
        C::SetAudioCleanup { clip_id, hum_removal, hiss_removal } => {
            if let Some(Some(hz)) = hum_removal {
                if !(40.0..=70.0).contains(hz) {
                    return Err(CommandError::Invalid("hum_removal is the mains frequency: 50 or 60 (Hz)".into()));
                }
            }
            if let Some(Some(v)) = hiss_removal {
                if !(0.0..=100.0).contains(v) {
                    return Err(CommandError::Invalid("hiss_removal strength must be within 0..100".into()));
                }
            }
            ops::with_audio(p, clip_id, |a| {
                if let Some(h) = hum_removal {
                    a.hum_removal = *h;
                }
                if let Some(h) = hiss_removal {
                    a.hiss_removal = h.filter(|v| *v > 0.0);
                }
            })?
        }
        C::SetEffectEnabled { clip_id, effect_id, enabled } => ops::with_clip(p, clip_id, |c| {
            let e = c
                .effects
                .iter_mut()
                .chain(c.animations.iter_mut())
                .find(|e| &e.id == effect_id)
                .ok_or_else(|| CommandError::NotFound(format!("effect {effect_id}")))?;
            e.enabled = *enabled;
            Ok(())
        })?,
        C::AddEffect { clip_id, effect } => ops::with_clip(p, clip_id, |c| {
            c.effects.push(effect.clone());
            Ok(())
        })?,
        C::RemoveEffect { clip_id, effect_id } => ops::with_clip(p, clip_id, |c| {
            let before = c.effects.len() + c.animations.len();
            c.effects.retain(|e| &e.id != effect_id);
            c.animations.retain(|e| &e.id != effect_id);
            if c.effects.len() + c.animations.len() == before {
                return Err(CommandError::NotFound(format!("effect {effect_id}")));
            }
            Ok(())
        })?,
        C::SetEffectParams { clip_id, effect_id, params } => ops::with_clip(p, clip_id, |c| {
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
        C::AddAnimation { clip_id, kind, placement, duration, params, id } => {
            ops::add_animation(p, clip_id, kind, *placement, *duration, params.clone(), id.clone())?
        }
        C::RemoveAnimation { clip_id, animation_id, placement } => {
            ops::remove_animation(p, clip_id, animation_id.as_deref(), *placement)?
        }
        C::ReplaceAnimation { clip_id, animation_id, kind, duration, params } => {
            ops::replace_animation(p, clip_id, animation_id, kind.as_deref(), *duration, params.as_ref())?
        }
        C::SetTransition { clip_id, transition } => ops::with_clip(p, clip_id, |c| {
            c.transition_out = transition.clone();
            Ok(())
        })?,
        C::SetTitle { clip_id, title } => ops::with_title(p, clip_id, |t| {
            *t = title.clone();
            Ok(())
        })?,
        C::SetTitleStyle {
            clip_id,
            text,
            font_family,
            font_size,
            color,
            bold,
            italic,
            alignment,
            position,
            stroke,
            shadow,
            background,
            letter_spacing,
            line_spacing,
            animation,
        } => {
            if font_size.is_some_and(|s| !(s > 0.0 && s <= 2.0)) {
                return Err(CommandError::Invalid("font_size is a fraction of the canvas height (0..2]".into()));
            }
            ops::with_title(p, clip_id, |t| {
                macro_rules! set {
                    ($($f:ident),*) => {$( if let Some(v) = $f { t.$f = v.clone(); } )*};
                }
                set!(
                    text,
                    font_family,
                    font_size,
                    color,
                    bold,
                    italic,
                    alignment,
                    position,
                    letter_spacing,
                    line_spacing
                );
                set!(stroke, shadow, background);
                Ok(())
            })?;
            if let Some(a) = animation {
                ops::add_animation(p, clip_id, &a.kind, a.placement, a.duration, BTreeMap::new(), None)?;
            }
        }
        C::AddMarker { sequence_id, clip_id, marker } => {
            ops::add_marker(p, sequence_id.as_deref(), clip_id.as_deref(), marker)?
        }
        C::UpdateMarker { marker } => ops::update_marker(p, marker)?,
        C::RemoveMarker { marker_id } => ops::remove_marker(p, marker_id)?,
        C::RenameProject { name } => p.name = name.clone(),
        C::SetProjectSettings { width, height, frame_rate, background } => {
            for (v, what) in [(width, "width"), (height, "height")] {
                if let Some(v) = v {
                    if *v == 0 || *v % 2 != 0 || *v > 16384 {
                        return Err(CommandError::Invalid(format!(
                            "{what} must be a positive even number up to 16384"
                        )));
                    }
                }
            }
            if let Some(r) = frame_rate {
                if r.num <= 0 || r.den <= 0 || r.num as f64 / r.den as f64 > 240.0 {
                    return Err(CommandError::Invalid("frame rate must be positive (at most 240 fps)".into()));
                }
            }
            if let Some(c) = background {
                if c.iter().any(|v| !(0.0..=1.0).contains(v)) {
                    return Err(CommandError::Invalid("background components must be in 0..1".into()));
                }
            }
            let s = &mut p.settings;
            if let Some(v) = width {
                s.width = *v;
            }
            if let Some(v) = height {
                s.height = *v;
            }
            if let Some(v) = frame_rate {
                s.frame_rate = *v;
            }
            if let Some(v) = background {
                s.background = *v;
            }
        }
        C::RenameMedia { media_id, name } => {
            if name.trim().is_empty() {
                return Err(CommandError::Invalid("media name is empty".into()));
            }
            let m = p
                .media
                .iter_mut()
                .find(|m| &m.id == media_id)
                .ok_or_else(|| CommandError::NotFound(format!("media {media_id}")))?;
            m.name = name.trim().to_string();
        }
        C::Batch { commands } => {
            for c in commands {
                apply(p, c)?;
            }
        }
    }
    Ok(())
}

fn finite(v: f64, what: &str) -> Result<()> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(CommandError::Invalid(format!("{what} must be a finite number")))
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

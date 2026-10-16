//! The project model. This is the single source of truth that the UI, the
//! CLI, the renderer and agents all operate on. It is deliberately
//! independent of any third-party project format; importers translate into
//! it and keep whatever they cannot represent in `extra` fields so that
//! exports can round-trip losslessly.

use crate::param::Param;
use crate::time::{Rational, Ticks};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Id = String;

pub fn new_id() -> Id {
    uuid::Uuid::new_v4().to_string()
}

/// Opaque data preserved from an imported format.
pub type Extra = serde_json::Value;

fn is_null(v: &Extra) -> bool {
    v.is_null()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: Id,
    pub name: String,
    pub settings: ProjectSettings,
    pub media: Vec<MediaAsset>,
    pub sequences: Vec<Sequence>,
    /// Sequence that is rendered and shown in the main timeline.
    pub main_sequence: Id,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate: Rational,
    pub sample_rate: u32,
    /// Canvas background color (linear RGBA).
    #[serde(default = "black")]
    pub background: [f64; 4],
}

fn black() -> [f64; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Audio,
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: Id,
    pub name: String,
    /// Absolute path on the local filesystem.
    pub path: String,
    pub kind: MediaKind,
    pub duration: Ticks,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<Rational>,
    #[serde(default)]
    pub has_audio: bool,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub id: Id,
    pub name: String,
    /// Tracks in compositing order: index 0 is the bottom-most track.
    pub tracks: Vec<Track>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

impl Sequence {
    pub fn duration(&self) -> Ticks {
        self.tracks.iter().flat_map(|t| t.clips.iter()).map(|c| c.end).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: Id,
    pub kind: TrackKind,
    pub name: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub hidden: bool,
    /// Clips sorted by `start`, never overlapping.
    pub clips: Vec<Clip>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

/// What a clip plays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipSource {
    Media {
        media_id: Id,
    },
    /// A nested sequence (compound clip, title template, etc).
    Sequence {
        sequence_id: Id,
    },
    Title(TitleData),
    /// A solid color.
    Color {
        color: [f64; 4],
    },
    /// An adjustment layer: applies its effects to everything below it.
    Adjustment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TitleData {
    pub text: String,
    pub font_family: String,
    /// Font size as a fraction of the canvas height.
    pub font_size: f64,
    pub color: [f64; 4],
    /// Center position in normalized canvas coordinates (0..1).
    pub position: [f64; 2],
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<[f64; 4]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: [f64; 4],
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub color: [f64; 4],
    pub offset: [f64; 2],
    pub blur: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: Id,
    pub name: String,
    pub source: ClipSource,
    /// Timeline range `[start, end)`.
    pub start: Ticks,
    pub end: Ticks,
    /// Source time that plays at `start` (before speed mapping).
    #[serde(default)]
    pub source_in: Ticks,
    #[serde(default)]
    pub speed: Speed,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default = "full_opacity")]
    pub opacity: Param,
    #[serde(default)]
    pub blend_mode: BlendMode,
    #[serde(default)]
    pub audio: AudioSettings,
    /// Effects applied in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectInstance>,
    /// Entry/exit animation presets ("Animation" tab).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<EffectInstance>,
    /// Transition that blends this clip into the next one on the same track.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_out: Option<Transition>,
    /// Clips sharing a link group move and trim together (e.g. camera video + its audio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_group: Option<Id>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

fn full_opacity() -> Param {
    Param::float(1.0)
}

impl Clip {
    pub fn duration(&self) -> Ticks {
        self.end - self.start
    }

    pub fn contains(&self, t: Ticks) -> bool {
        t >= self.start && t < self.end
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    /// Constant playback rate (1.0 = normal).
    pub rate: f64,
    #[serde(default)]
    pub reverse: bool,
    /// Speed ramping: rate keyframes over clip-local time. Overrides `rate`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramp: Option<Param>,
    #[serde(default = "yes")]
    pub maintain_pitch: bool,
}

fn yes() -> bool {
    true
}

impl Default for Speed {
    fn default() -> Self {
        Self { rate: 1.0, reverse: false, ramp: None, maintain_pitch: true }
    }
}

/// 2D transform in canvas space. Position is the offset of the clip center
/// from the canvas center, in normalized canvas units (1.0 = full width/height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub position_x: Param,
    pub position_y: Param,
    pub scale_x: Param,
    pub scale_y: Param,
    /// Degrees, clockwise.
    pub rotation: Param,
    #[serde(default)]
    pub flip_h: bool,
    #[serde(default)]
    pub flip_v: bool,
    /// Normalized crop rectangle in source space: [left, top, right, bottom] insets.
    #[serde(default)]
    pub crop: [f64; 4],
    #[serde(default)]
    pub fit: FitMode,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position_x: Param::float(0.0),
            position_y: Param::float(0.0),
            scale_x: Param::float(1.0),
            scale_y: Param::float(1.0),
            rotation: Param::float(0.0),
            flip_h: false,
            flip_v: false,
            crop: [0.0; 4],
            fit: FitMode::Fit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    /// Letterbox into the canvas preserving aspect ratio.
    #[default]
    Fit,
    /// Cover the canvas preserving aspect ratio.
    Fill,
    Stretch,
    /// Use source pixels 1:1.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Add,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Gain in dB.
    pub gain_db: Param,
    /// -1 (left) .. 1 (right).
    pub pan: Param,
    pub fade_in: Ticks,
    pub fade_out: Ticks,
    /// Pitch shift in semitones.
    #[serde(default)]
    pub pitch: f64,
    #[serde(default)]
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { gain_db: Param::float(0.0), pan: Param::float(0.0), fade_in: 0, fade_out: 0, pitch: 0.0, muted: false }
    }
}

/// An effect, filter, animation or color adjustment. `kind` is a stable
/// identifier from the effect registry (e.g. `"jb.blur.gaussian"`); effects
/// imported from another format that have no registered equivalent keep
/// their original identifier and render as a pass-through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectInstance {
    pub id: Id,
    pub kind: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub params: BTreeMap<String, Param>,
    /// Optional clip-local active range (animations, timed effects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<(Ticks, Ticks)>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub id: Id,
    pub kind: String,
    /// Total transition length, centered on the cut between the two clips
    /// unless `alignment` says otherwise.
    pub duration: Ticks,
    #[serde(default)]
    pub alignment: TransitionAlignment,
    #[serde(default)]
    pub params: BTreeMap<String, Param>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionAlignment {
    #[default]
    Center,
    /// Transition ends at the cut.
    EndAtCut,
    /// Transition starts at the cut.
    StartAtCut,
}

impl Project {
    pub fn new(name: impl Into<String>, settings: ProjectSettings) -> Self {
        let seq = Sequence { id: new_id(), name: "Main".into(), tracks: Vec::new(), extra: Extra::Null };
        Self {
            id: new_id(),
            name: name.into(),
            settings,
            media: Vec::new(),
            main_sequence: seq.id.clone(),
            sequences: vec![seq],
            extra: Extra::Null,
        }
    }

    pub fn sequence(&self, id: &str) -> Option<&Sequence> {
        self.sequences.iter().find(|s| s.id == id)
    }

    pub fn sequence_mut(&mut self, id: &str) -> Option<&mut Sequence> {
        self.sequences.iter_mut().find(|s| s.id == id)
    }

    pub fn main(&self) -> &Sequence {
        self.sequence(&self.main_sequence).expect("main sequence exists")
    }

    pub fn media(&self, id: &str) -> Option<&MediaAsset> {
        self.media.iter().find(|m| m.id == id)
    }

    /// Finds a clip anywhere in the project, returning (sequence, track, clip) indices.
    pub fn locate_clip(&self, clip_id: &str) -> Option<(usize, usize, usize)> {
        for (si, s) in self.sequences.iter().enumerate() {
            for (ti, t) in s.tracks.iter().enumerate() {
                if let Some(ci) = t.clips.iter().position(|c| c.id == clip_id) {
                    return Some((si, ti, ci));
                }
            }
        }
        None
    }

    pub fn clip(&self, clip_id: &str) -> Option<&Clip> {
        let (s, t, c) = self.locate_clip(clip_id)?;
        Some(&self.sequences[s].tracks[t].clips[c])
    }

    pub fn clip_mut(&mut self, clip_id: &str) -> Option<&mut Clip> {
        let (s, t, c) = self.locate_clip(clip_id)?;
        Some(&mut self.sequences[s].tracks[t].clips[c])
    }
}

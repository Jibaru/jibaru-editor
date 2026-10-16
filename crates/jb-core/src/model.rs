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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Audio,
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Sequence {
    pub id: Id,
    pub name: String,
    /// Tracks in compositing order: index 0 is the bottom-most track.
    pub tracks: Vec<Track>,
    /// Timeline markers; `time` is in sequence time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

/// A named point (or range, when `duration > 0`) on a sequence or clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Marker {
    pub id: Id,
    /// Sequence time for sequence markers, clip-local time for clip markers.
    pub time: Ticks,
    #[serde(default)]
    pub duration: Ticks,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
    /// Linear RGBA.
    #[serde(default = "marker_color")]
    pub color: [f64; 4],
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

/// Filmora's default marker color.
pub fn marker_color() -> [f64; 4] {
    [0.0, 206.0 / 255.0, 208.0 / 255.0, 1.0]
}

impl Sequence {
    pub fn duration(&self) -> Ticks {
        self.tracks.iter().flat_map(|t| t.clips.iter()).map(|c| c.end).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Display height in the timeline (UI pixels); `None` = default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

/// What a clip plays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
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
    #[serde(default, skip_serializing_if = "is_default")]
    pub alignment: TextAlignment,
    /// Extra space between characters, as a fraction of the font size.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub letter_spacing: f64,
    /// Extra space between lines, as a fraction of the font size.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub line_spacing: f64,
}

impl Default for TitleData {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_family: "Arial".into(),
            font_size: 0.048,
            color: [1.0; 4],
            position: [0.5, 0.5],
            bold: false,
            italic: false,
            stroke: None,
            shadow: None,
            background: None,
            alignment: TextAlignment::Center,
            letter_spacing: 0.0,
            line_spacing: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Left,
    #[default]
    Center,
    Right,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Stroke {
    pub color: [f64; 4],
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Shadow {
    pub color: [f64; 4],
    pub offset: [f64; 2],
    pub blur: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Clips sharing a group move together (Filmora "Group").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<Id>,
    #[serde(default)]
    pub disabled: bool,
    /// Clip markers; `time` is clip-local.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Speed {
    /// Constant playback rate (1.0 = normal).
    pub rate: f64,
    #[serde(default)]
    pub reverse: bool,
    /// Speed ramping: rate keyframes over clip-local time. Overrides `rate`
    /// (which then holds the average rate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramp: Option<Param>,
    /// The speed curve `ramp` was built from, in Filmora's terms (rate over
    /// the normalized source range). Kept so the curve survives round trips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<SpeedCurve>,
    /// Freeze frame: the clip shows the frame at `source_in` for its whole
    /// duration and is silent.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub freeze: bool,
    #[serde(default = "yes")]
    pub maintain_pitch: bool,
}

/// A speed ramp: playback rate as a function of the position in the clip's
/// source range, like Filmora's speed ramping panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SpeedCurve {
    /// Preset name (`montage`, `hero_moment`, `bullet_time`, `jumper`,
    /// `flash_in`, `flash_out`) or `custom`.
    pub preset: String,
    /// Control points sorted by `position`, linearly interpolated.
    pub points: Vec<SpeedPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SpeedPoint {
    /// Position in the source range, 0..=1.
    pub position: f64,
    /// Playback rate at that position (> 0).
    pub speed: f64,
}

fn yes() -> bool {
    true
}

impl Default for Speed {
    fn default() -> Self {
        Self { rate: 1.0, reverse: false, ramp: None, curve: None, freeze: false, maintain_pitch: true }
    }
}

/// 2D transform in canvas space. Position is the offset of the clip center
/// from the canvas center, in normalized canvas units (1.0 = full width/height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Linear dodge.
    Add,
    LinearBurn,
    DarkerColor,
    LighterColor,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Subtract,
    Divide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Auto normalization (loudness matching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalize: Option<Normalization>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equalizer: Option<Equalizer>,
    /// Noise reduction (Filmora AI Denoise): maximum attenuation of noise in dB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denoise_db: Option<f64>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            gain_db: Param::float(0.0),
            pan: Param::float(0.0),
            fade_in: 0,
            fade_out: 0,
            pitch: 0.0,
            muted: false,
            normalize: None,
            equalizer: None,
            denoise_db: None,
        }
    }
}

/// Loudness normalization of a clip to a target integrated loudness.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Normalization {
    /// Target in LUFS. Filmora offers -23 (EBU R128), -16, -14 and -12.
    pub target_lufs: f64,
    /// Measured integrated loudness of the clip's source range, in LUFS.
    /// Filled in by tools that can decode media; until then no gain is applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured_lufs: Option<f64>,
}

impl Normalization {
    /// Gain in dB that brings the measured loudness to the target (0 when unmeasured).
    pub fn gain_db(&self) -> f64 {
        self.measured_lufs.filter(|m| m.is_finite()).map_or(0.0, |m| (self.target_lufs - m).clamp(-40.0, 40.0))
    }
}

/// Ten-band graphic equalizer (31 Hz .. 16 kHz octave bands), like Filmora's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Equalizer {
    /// Preset name (see `Equalizer::PRESETS`) or `custom`.
    pub preset: String,
    /// Gain per band in dB (-12..12), lowest band first.
    pub bands: [f64; 10],
}

impl Equalizer {
    /// Center frequencies of the bands in Hz.
    pub const FREQUENCIES: [f64; 10] = [31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

    /// Built-in presets. The first fifteen are Filmora 15's equalizer menu
    /// with the band values Filmora writes for them (recorded from project
    /// files it saved); the rest are Jibaru's own voicings, which Filmora
    /// shows as "Custom".
    pub const PRESETS: &'static [(&'static str, [f64; 10])] = &[
        ("default", [0.0; 10]),
        ("classic", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.16, 0.36, -7.92, -10.08]),
        ("dance", [-0.72, 3.96, 5.04, 0.72, -1.08, -1.08, 0.0, 0.36, 3.96, 3.96]),
        ("low_bass", [2.88, 3.96, 2.88, 2.88, 2.16, -2.16, -7.92, -10.08, -12.96, -10.08]),
        ("sparkle", [-3.96, -3.96, -3.96, -2.16, -1.08, 2.16, 5.76, 7.92, 7.92, 7.92]),
        ("pop", [-1.08, -2.16, 3.96, 5.76, 5.76, 3.96, -1.08, -2.16, -2.16, -2.16]),
        ("rock_n_roll", [-1.08, 2.88, 2.16, 2.88, -1.08, -1.08, 0.0, 2.16, 2.16, 3.96]),
        ("soft", [0.0, 3.96, 2.16, -1.08, -2.16, -1.08, 3.96, 5.76, 5.76, 7.92]),
        ("techno", [-5.76, 1.08, 3.96, -1.08, -1.08, -2.16, 0.0, 0.0, 5.04, 5.04]),
        ("hard_rock", [-3.96, 0.0, 0.0, 0.0, 0.0, 0.0, 2.88, 0.0, 2.88, 1.08]),
        ("ska", [0.0, -2.16, -3.96, -2.16, 0.0, 3.96, 5.76, 7.92, 5.76, 5.76]),
        ("country", [0.0, -1.08, 0.0, 0.0, 2.16, 2.16, 0.0, 0.0, 2.88, 2.88]),
        ("acoustic", [0.0, 1.08, 2.16, 0.0, 0.0, 0.0, 0.0, 0.0, 2.16, 2.16]),
        ("folk", [-1.08, 0.0, 1.08, 0.0, 2.16, 0.0, 0.0, 0.0, 2.16, 0.0]),
        ("blues", [-1.44, 0.0, 2.16, 1.08, 0.0, 0.0, 0.0, 0.0, -1.08, -2.88]),
        ("electronic", [4.0, 3.0, 1.0, 0.0, -2.0, 2.0, 1.0, 1.0, 3.0, 4.0]),
        ("hip_hop", [5.0, 4.0, 1.0, 3.0, -1.0, -1.0, 1.0, -1.0, 2.0, 3.0]),
        ("jazz", [4.0, 3.0, 1.0, 2.0, -2.0, -2.0, 0.0, 1.0, 3.0, 4.0]),
        ("metal", [4.0, 5.0, 5.0, 2.0, 0.0, 1.0, 3.0, 4.0, 3.0, 2.0]),
        ("rnb", [3.0, 7.0, 6.0, 1.0, -2.0, -1.0, 2.0, 3.0, 3.0, 4.0]),
        ("speech", [-3.0, -2.0, 0.0, 2.0, 4.0, 4.0, 3.0, 2.0, 0.0, -2.0]),
    ];

    /// A preset by name (case and separators are ignored: "Hip-Hop" = "hip_hop").
    pub fn preset(name: &str) -> Option<Self> {
        let key = name.trim().to_ascii_lowercase().replace(['\'', '\u{2019}'], "").replace([' ', '-', '&'], "_");
        let key = match key.as_str() {
            "classical" => "classic",
            "rock" | "rocknroll" | "rock_and_roll" | "rock_n_roll" => "rock_n_roll",
            "hardrock" => "hard_rock",
            "r_b" | "r_and_b" | "randb" => "rnb",
            "hiphop" => "hip_hop",
            "lowbass" => "low_bass",
            "none" | "flat" => "default",
            k => k,
        };
        Self::PRESETS.iter().find(|(n, _)| *n == key).map(|(n, b)| Self { preset: n.to_string(), bands: *b })
    }
}

/// An effect, filter, animation or color adjustment. `kind` is a stable
/// identifier from the effect registry (e.g. `"jb.blur.gaussian"`); effects
/// imported from another format that have no registered equivalent keep
/// their original identifier and render as a pass-through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// For entry/exit animations: the slot the animation occupies. Its
    /// `range` follows the clip edges when the clip is trimmed or retimed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<AnimationPlacement>,
    #[serde(default, skip_serializing_if = "is_null")]
    pub extra: Extra,
}

/// Slots of Filmora's "Animation" tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnimationPlacement {
    /// Entry animation, from the clip start.
    In,
    /// Exit animation, ending at the clip end.
    Out,
    /// Repeats over the whole clip.
    Loop,
    /// One animation spanning the whole clip (Filmora "Combo").
    Combo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
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
        let seq =
            Sequence { id: new_id(), name: "Main".into(), tracks: Vec::new(), markers: Vec::new(), extra: Extra::Null };
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

//! Timeline evaluation: turns a sequence and a time into a flat, fully
//! resolved description of what must be drawn and mixed for that instant.
//! The renderer and the audio mixer consume this; they never walk the
//! project tree themselves.

use crate::model::*;
use crate::param::Param;
use crate::time::Ticks;
use std::collections::BTreeMap;

/// Everything visible at one instant, bottom layer first.
#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct FramePlan {
    pub time: Ticks,
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct Layer {
    pub clip_id: Id,
    pub content: LayerContent,
    /// Clip-local time (time since the clip started on the timeline).
    pub local_time: Ticks,
    pub transform: ResolvedTransform,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    pub effects: Vec<ResolvedEffect>,
    /// Set when this layer is the outgoing side of an active transition;
    /// `incoming` is the layer it blends into.
    pub transition: Option<Box<ActiveTransition>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LayerContent {
    /// A frame of a media asset at the given source time.
    Media {
        media_id: Id,
        source_time: Ticks,
    },
    /// A nested sequence evaluated at the given time.
    Nested(Box<FramePlan>),
    Title(TitleData),
    Color([f64; 4]),
    Adjustment,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct ResolvedTransform {
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation: f64,
    pub flip_h: bool,
    pub flip_v: bool,
    pub crop: [f64; 4],
    pub fit: FitMode,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct ResolvedEffect {
    pub kind: String,
    pub params: BTreeMap<String, crate::param::Value>,
    /// 0..1 progress through the effect's active range (animations).
    pub progress: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct ActiveTransition {
    pub kind: String,
    /// 0 at the start of the transition, 1 at the end.
    pub progress: f64,
    pub params: BTreeMap<String, crate::param::Value>,
    pub incoming: Layer,
}

/// Maps a clip-local timeline time to a source time, honoring speed.
pub fn source_time(clip: &Clip, local: Ticks) -> Ticks {
    if clip.speed.freeze {
        return clip.source_in;
    }
    let elapsed = match &clip.speed.ramp {
        Some(ramp) => integrate_rate(ramp, local),
        None => (local as f64 * clip.speed.rate).round() as Ticks,
    };
    if clip.speed.reverse {
        let total = match &clip.speed.ramp {
            Some(ramp) => integrate_rate(ramp, clip.duration()),
            None => (clip.duration() as f64 * clip.speed.rate).round() as Ticks,
        };
        clip.source_in + (total - elapsed).max(0)
    } else {
        clip.source_in + elapsed
    }
}

/// Integrates a rate curve from 0 to `t`: exact for linear and hold
/// segments, Simpson's rule for eased ones.
pub fn integrate_rate(rate: &Param, t: Ticks) -> Ticks {
    use crate::param::Interpolation;
    if t <= 0 {
        return 0;
    }
    let kfs = &rate.keyframes;
    if kfs.is_empty() {
        return (rate.sample_f64(0) * t as f64).round() as Ticks;
    }
    let (first, last) = (kfs[0].time, kfs[kfs.len() - 1].time);
    let mut cuts: Vec<Ticks> = vec![0];
    cuts.extend(kfs.iter().map(|k| k.time).filter(|&x| x > 0 && x < t));
    cuts.push(t);
    let mut acc = 0.0;
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b <= a {
            continue;
        }
        let dx = (b - a) as f64;
        let interp = kfs.iter().rev().find(|k| k.time <= a).map(|k| k.interpolation);
        acc += if a < first || a >= last {
            rate.sample_f64(a) * dx
        } else {
            match interp {
                Some(Interpolation::Linear) => 0.5 * (rate.sample_f64(a) + rate.sample_f64(b)) * dx,
                Some(Interpolation::Hold) => rate.sample_f64(a) * dx,
                _ => {
                    const N: i64 = 16;
                    let h = dx / N as f64;
                    let f = |i: i64| rate.sample_f64(a + ((b - a) * i) / N);
                    let mut s = f(0) + f(N);
                    for i in 1..N {
                        s += f(i) * if i % 2 == 1 { 4.0 } else { 2.0 };
                    }
                    s * h / 3.0
                }
            }
        };
    }
    acc.round() as Ticks
}

fn resolve_transform(t: &Transform, local: Ticks) -> ResolvedTransform {
    ResolvedTransform {
        position: [t.position_x.sample_f64(local), t.position_y.sample_f64(local)],
        scale: [t.scale_x.sample_f64(local), t.scale_y.sample_f64(local)],
        rotation: t.rotation.sample_f64(local),
        flip_h: t.flip_h,
        flip_v: t.flip_v,
        crop: t.crop,
        fit: t.fit,
    }
}

fn resolve_effects(effects: &[EffectInstance], local: Ticks, clip_len: Ticks) -> Vec<ResolvedEffect> {
    effects
        .iter()
        .filter(|e| e.enabled)
        .filter_map(|e| {
            let (a, b) = e.range.unwrap_or((0, clip_len));
            if local < a || local >= b {
                return None;
            }
            Some(ResolvedEffect {
                kind: e.kind.clone(),
                params: e.params.iter().map(|(k, p)| (k.clone(), p.sample(local))).collect(),
                progress: (local - a) as f64 / (b - a).max(1) as f64,
            })
        })
        .collect()
}

/// Returns the transition window `[from, to)` around the cut at `cut`.
pub fn transition_window(tr: &Transition, cut: Ticks) -> (Ticks, Ticks) {
    match tr.alignment {
        TransitionAlignment::Center => (cut - tr.duration / 2, cut + tr.duration - tr.duration / 2),
        TransitionAlignment::EndAtCut => (cut - tr.duration, cut),
        TransitionAlignment::StartAtCut => (cut, cut + tr.duration),
    }
}

fn layer_for(project: &Project, clip: &Clip, t: Ticks, depth: usize) -> Option<Layer> {
    let local = t - clip.start;
    let content = match &clip.source {
        ClipSource::Media { media_id } => {
            LayerContent::Media { media_id: media_id.clone(), source_time: source_time(clip, local) }
        }
        ClipSource::Sequence { sequence_id } => {
            if depth > 16 {
                return None;
            }
            let seq = project.sequence(sequence_id)?;
            let nested_t = source_time(clip, local);
            LayerContent::Nested(Box::new(evaluate_at_depth(project, seq, nested_t, depth + 1)))
        }
        ClipSource::Title(d) => LayerContent::Title(d.clone()),
        ClipSource::Color { color } => LayerContent::Color(*color),
        ClipSource::Adjustment => LayerContent::Adjustment,
    };
    let mut effects = resolve_effects(&clip.animations, local, clip.duration());
    effects.extend(resolve_effects(&clip.effects, local, clip.duration()));
    Some(Layer {
        clip_id: clip.id.clone(),
        content,
        local_time: local,
        transform: resolve_transform(&clip.transform, local),
        opacity: clip.opacity.sample_f64(local),
        blend_mode: clip.blend_mode,
        effects,
        transition: None,
    })
}

/// Evaluates the visual layers of `seq` at time `t`.
pub fn evaluate(project: &Project, seq: &Sequence, t: Ticks) -> FramePlan {
    evaluate_at_depth(project, seq, t, 0)
}

fn evaluate_at_depth(project: &Project, seq: &Sequence, t: Ticks, depth: usize) -> FramePlan {
    let mut layers = Vec::new();
    for track in seq.tracks.iter().filter(|tr| tr.kind == TrackKind::Video && !tr.hidden) {
        let clips: Vec<&Clip> = track.clips.iter().filter(|c| !c.disabled).collect();
        let mut handled = false;
        // Transitions may extend past clip edges, so check them first.
        for pair in clips.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let Some(tr) = &a.transition_out else { continue };
            let (from, to) = transition_window(tr, a.end);
            if t >= from && t < to {
                // Clips are held on their edge frame when the window exceeds them.
                let ta = t.min(a.end - 1).max(a.start);
                let tb = t.max(b.start).min(b.end - 1);
                if let (Some(mut out), Some(inc)) = (layer_for(project, a, ta, depth), layer_for(project, b, tb, depth))
                {
                    out.transition = Some(Box::new(ActiveTransition {
                        kind: tr.kind.clone(),
                        progress: (t - from) as f64 / (to - from).max(1) as f64,
                        params: tr.params.iter().map(|(k, p)| (k.clone(), p.sample(t - from))).collect(),
                        incoming: inc,
                    }));
                    layers.push(out);
                    handled = true;
                }
                break;
            }
        }
        // A transition out of the last clip of a track (Filmora's fade to black
        // at the end of a timeline) blends into nothing: a transparent layer.
        if !handled {
            if let Some(a) = clips.last().filter(|a| a.transition_out.is_some() && a.contains(t)) {
                let tr = a.transition_out.as_ref().unwrap();
                let (from, to) = transition_window(tr, a.end);
                if t >= from && t < to {
                    if let Some(mut out) = layer_for(project, a, t, depth) {
                        let empty = Layer {
                            clip_id: a.id.clone(),
                            content: LayerContent::Color([0.0; 4]),
                            local_time: 0,
                            transform: out.transform.clone(),
                            opacity: 1.0,
                            blend_mode: Default::default(),
                            effects: Vec::new(),
                            transition: None,
                        };
                        out.transition = Some(Box::new(ActiveTransition {
                            kind: tr.kind.clone(),
                            progress: (t - from) as f64 / (to - from).max(1) as f64,
                            params: tr.params.iter().map(|(k, p)| (k.clone(), p.sample(t - from))).collect(),
                            incoming: empty,
                        }));
                        layers.push(out);
                        handled = true;
                    }
                }
            }
        }
        if handled {
            continue;
        }
        if let Some(clip) = clips.iter().find(|c| c.contains(t)) {
            if let Some(l) = layer_for(project, clip, t, depth) {
                layers.push(l);
            }
        }
    }
    FramePlan { time: t, layers }
}

/// One audible clip segment at an instant, for the audio mixer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, schemars::JsonSchema)]
pub struct AudioVoice {
    pub clip_id: Id,
    pub media_id: Id,
    pub source_time: Ticks,
    pub rate: f64,
    pub gain: f64,
    pub pan: f64,
    /// Plays the source backwards.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reverse: bool,
    /// Keep the original pitch when `rate != 1` (time-stretch instead of resample).
    pub maintain_pitch: bool,
    /// Pitch shift in semitones on top of any speed change.
    #[serde(skip_serializing_if = "is_zero")]
    pub pitch: f64,
    /// Equalizer band gains in dB (see `Equalizer::FREQUENCIES`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equalizer: Option<[f64; 10]>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Linear gain for a clip at clip-local time, including fades.
pub fn clip_gain(clip: &Clip, local: Ticks) -> f64 {
    if clip.audio.muted {
        return 0.0;
    }
    let norm = clip.audio.normalize.map_or(0.0, |n| n.gain_db());
    let mut g = 10f64.powf((clip.audio.gain_db.sample_f64(local) + norm) / 20.0);
    if clip.audio.fade_in > 0 && local < clip.audio.fade_in {
        g *= local as f64 / clip.audio.fade_in as f64;
    }
    let remaining = clip.duration() - local;
    if clip.audio.fade_out > 0 && remaining < clip.audio.fade_out {
        g *= remaining.max(0) as f64 / clip.audio.fade_out as f64;
    }
    g
}

/// Audible voices of `seq` at time `t`, recursing into nested sequences.
pub fn audio_voices(project: &Project, seq: &Sequence, t: Ticks) -> Vec<AudioVoice> {
    let mut out = Vec::new();
    collect_voices(project, seq, t, 1.0, 1.0, 0, &mut out);
    out
}

fn collect_voices(
    project: &Project,
    seq: &Sequence,
    t: Ticks,
    gain: f64,
    rate: f64,
    depth: usize,
    out: &mut Vec<AudioVoice>,
) {
    if depth > 16 {
        return;
    }
    for track in seq.tracks.iter().filter(|tr| !tr.muted) {
        for clip in track.clips.iter().filter(|c| !c.disabled && !c.speed.freeze && c.contains(t)) {
            let local = t - clip.start;
            let g = gain * clip_gain(clip, local);
            let r = rate * clip.speed.ramp.as_ref().map_or(clip.speed.rate, |p| p.sample_f64(local));
            match &clip.source {
                ClipSource::Media { media_id } => {
                    let has_audio = project.media(media_id).is_some_and(|m| m.has_audio || m.kind == MediaKind::Audio);
                    // A video track clip only contributes audio when it carries it unlinked.
                    if has_audio && (track.kind == TrackKind::Audio || clip.link_group.is_none()) {
                        out.push(AudioVoice {
                            clip_id: clip.id.clone(),
                            media_id: media_id.clone(),
                            source_time: source_time(clip, local),
                            rate: r,
                            gain: g,
                            pan: clip.audio.pan.sample_f64(local),
                            reverse: clip.speed.reverse,
                            maintain_pitch: clip.speed.maintain_pitch,
                            pitch: clip.audio.pitch,
                            equalizer: clip
                                .audio
                                .equalizer
                                .as_ref()
                                .map(|e| e.bands)
                                .filter(|b| b.iter().any(|g| *g != 0.0)),
                        });
                    }
                }
                ClipSource::Sequence { sequence_id } => {
                    if let Some(s) = project.sequence(sequence_id) {
                        collect_voices(project, s, source_time(clip, local), g, r, depth + 1, out);
                    }
                }
                _ => {}
            }
        }
    }
}

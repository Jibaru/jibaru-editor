//! Jibaru-private state stored inside a `.wfp` next to Filmora's documents.
//!
//! Some clip properties edited in Jibaru have no (verified) place in
//! Filmora's format: effects without a Filmora equivalent (chroma key,
//! masks, shadows, library effects and filters, color panel adjustments),
//! audio noise reduction changed in Jibaru and audio channel routing. They
//! are written to the archive entry [`SIDECAR`] so they survive a save and
//! reopen in Jibaru; Filmora ignores the entry (and won't show them).
//!
//! ```json
//! {"version": 1,
//!  "clips": {"<clip thisUId>": {"effects": [{"index": 2, "effect": {EffectInstance}}],
//!                               "denoise_db": 15.0, "channels": "left"}},
//!  "native_kinds": {"<effect thisUId>": "jb.auto_enhance"}}
//! ```
//!
//! Clips are keyed by the `thisUId` written to the timeline (the identity
//! the reader gives imported clips). `index` is the effect's position in
//! the clip's effect list. `denoise_db` is present only when it differs
//! from what Filmora's own fields say (`null` = off). `native_kinds` maps
//! effects written as Filmora built-ins back to the Jibaru kind the user
//! picked, so the model reads back exactly as it was saved.

use crate::mapping::marker_str;
use jb_core::{ChannelMode, EffectInstance, Project};
use serde_json::{json, Map, Value};

/// Archive path of the sidecar document.
pub const SIDECAR: &str = "ProjectFolder/jibaru.json";

/// Effects written as a Filmora built-in: (Jibaru kind, Filmora id, display, chain).
pub const NATIVE_EFFECTS: &[(&str, &str, &str, &str)] =
    &[("jb.auto_enhance", "45D9AD95-4F03-4B2F-A8C7-8F7817B73FA4", "AutoEnhance", "BasicFollow")];

pub fn native(kind: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    NATIVE_EFFECTS.iter().find(|n| n.0 == kind)
}

/// Effects that only Jibaru keeps (neither `filmora:` kinds nor native ones).
pub fn is_private(e: &EffectInstance) -> bool {
    !e.kind.starts_with("filmora:") && native(&e.kind).is_none()
}

/// Per-clip private state collected by the writer.
#[derive(Default)]
pub struct ClipState {
    pub effects: Vec<(usize, EffectInstance)>,
    /// `Some(value)` when it differs from Filmora's fields.
    pub denoise_db: Option<Option<f64>>,
    pub channels: ChannelMode,
}

impl ClipState {
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty() && self.denoise_db.is_none() && self.channels == ChannelMode::Stereo
    }

    fn to_json(&self) -> Value {
        let mut o = Map::new();
        if !self.effects.is_empty() {
            let list: Vec<Value> = self
                .effects
                .iter()
                .map(|(i, e)| json!({ "index": i, "effect": serde_json::to_value(e).unwrap_or_default() }))
                .collect();
            o.insert("effects".into(), Value::Array(list));
        }
        if let Some(d) = self.denoise_db {
            o.insert("denoise_db".into(), json!(d));
        }
        if self.channels != ChannelMode::Stereo {
            o.insert("channels".into(), serde_json::to_value(self.channels).unwrap_or_default());
        }
        Value::Object(o)
    }
}

/// Builds the sidecar document; `None` when there is nothing to keep.
pub fn document(clips: &[(String, ClipState)], native_kinds: &[(String, String)]) -> Option<Value> {
    let clips: Map<String, Value> =
        clips.iter().filter(|(_, s)| !s.is_empty()).map(|(uid, s)| (uid.clone(), s.to_json())).collect();
    if clips.is_empty() && native_kinds.is_empty() {
        return None;
    }
    let kinds: Map<String, Value> = native_kinds.iter().map(|(u, k)| (u.clone(), json!(k))).collect();
    Some(json!({ "version": 1, "clips": clips, "native_kinds": kinds }))
}

/// Re-attaches the sidecar state to an imported project.
pub fn apply(project: &mut Project, doc: &Value) {
    let clips = doc.get("clips").and_then(Value::as_object);
    let kinds = doc.get("native_kinds").and_then(Value::as_object);
    for seq in &mut project.sequences {
        for track in &mut seq.tracks {
            for clip in &mut track.clips {
                if let Some(k) = kinds {
                    for e in clip.effects.iter_mut().chain(clip.animations.iter_mut()) {
                        if let Some(kind) = marker_str(&e.extra, "uid").and_then(|u| k.get(u)).and_then(Value::as_str) {
                            e.kind = kind.to_string();
                        }
                    }
                }
                let Some(state) = marker_str(&clip.extra, "uid").and_then(|u| clips?.get(u)) else { continue };
                let mut list: Vec<(usize, EffectInstance)> = state
                    .get("effects")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|x| {
                        let i = x.get("index").and_then(Value::as_u64).unwrap_or(u64::MAX) as usize;
                        Some((i, serde_json::from_value(x.get("effect")?.clone()).ok()?))
                    })
                    .collect();
                list.sort_by_key(|(i, _)| *i);
                for (i, e) in list {
                    if clip.effects.iter().any(|x| x.id == e.id) {
                        continue;
                    }
                    let at = i.min(clip.effects.len());
                    clip.effects.insert(at, e);
                }
                if let Some(d) = state.get("denoise_db") {
                    clip.audio.denoise_db = d.as_f64();
                }
                if let Some(c) = state.get("channels").and_then(|c| serde_json::from_value(c.clone()).ok()) {
                    clip.audio.channels = c;
                }
            }
        }
    }
}

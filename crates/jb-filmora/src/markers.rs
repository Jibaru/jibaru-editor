//! Markers live in the timeline's `extra.json`, under `allMarkersInfo`,
//! keyed by the userData GUID (key 3) of the timeline (sequence markers,
//! verified) or of a clip (clip markers, inferred):
//! `{"<GUID>": [{"name", "position", "comments", "color"}]}`. Positions are
//! ticks: sequence time for timelines, clip-local time for clips.

use crate::import::{timelines, tl_clips};
use crate::json::*;
use crate::mapping::*;
use jb_core::*;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

fn marker_of(key: &str, i: usize, v: &Value) -> Marker {
    Marker {
        id: format!("marker-{key}-{i}"),
        time: i64_of(v, "position").unwrap_or(0),
        duration: i64_of(v, "duration").unwrap_or(0).max(0),
        name: str_of(v, "name").unwrap_or_default().to_string(),
        comment: str_of(v, "comments").unwrap_or_default().to_string(),
        color: i64_of(v, "color").map_or_else(marker_color, |c| color_from_int(c, 255)),
        extra: Extra::Null,
    }
}

fn marker_json(m: &Marker) -> Value {
    let mut v = json!({
        "name": m.name, "position": m.time, "comments": m.comment, "color": color_to_int(m.color).0
    });
    if m.duration > 0 {
        v["duration"] = json!(m.duration);
    }
    v
}

pub fn import_markers(project: &mut Project, timeline: &Value, extra: &Value) {
    let Some(all) = extra.get("allMarkersInfo").and_then(Value::as_object) else { return };
    let mut by_guid: HashMap<String, Vec<Marker>> = HashMap::new();
    for (k, v) in all {
        if let Some(list) = v.as_array() {
            by_guid.insert(k.clone(), list.iter().enumerate().map(|(i, m)| marker_of(k, i, m)).collect());
        }
    }
    if by_guid.is_empty() {
        return;
    }
    let mut clip_guid: HashMap<String, String> = HashMap::new();
    for tl in timelines(timeline) {
        let id = i64_of(tl, "timelineId").unwrap_or(0);
        if let Some(g) = user_data_str(tl, ud::GUID) {
            if let Some(ms) = by_guid.remove(&g) {
                if let Some(s) = project.sequence_mut(&crate::import::sequence_id(id)) {
                    s.markers = ms;
                }
            }
        }
        for c in tl_clips(tl) {
            if let (Some(u), Some(g)) = (str_of(c, "thisUId"), user_data_str(c, ud::GUID)) {
                clip_guid.insert(u.to_string(), g);
            }
        }
    }
    for s in &mut project.sequences {
        // Video clips first: a camera clip and its audio share the GUID.
        let mut order: Vec<usize> = (0..s.tracks.len()).collect();
        order.sort_by_key(|i| s.tracks[*i].kind == TrackKind::Audio);
        for i in order {
            let t = &mut s.tracks[i];
            for c in &mut t.clips {
                let Some(g) = marker_str(&c.extra, "uid").and_then(|u| clip_guid.get(u)) else { continue };
                // Linked clips share the GUID; the first one holds the markers.
                if let Some(ms) = by_guid.remove(g) {
                    c.markers = ms;
                }
            }
        }
    }
}

fn all_markers(p: &Project) -> Vec<(&str, &[Marker])> {
    let mut out: Vec<(&str, &[Marker])> = Vec::new();
    for s in &p.sequences {
        out.push((s.id.as_str(), &s.markers));
        for t in &s.tracks {
            for c in &t.clips {
                out.push((c.id.as_str(), &c.markers));
            }
        }
    }
    out
}

/// Rewrites `allMarkersInfo` when markers changed. `tl_guid` maps sequence
/// ids and `clip_guid` clip ids to the GUIDs of the written timeline.
pub fn export_markers(
    p: &Project,
    base: &Project,
    extra: &Value,
    tl_guid: &HashMap<String, String>,
    clip_guid: &HashMap<String, String>,
    known_guids: &HashSet<String>,
) -> Option<Value> {
    let marked = |p: &Project| {
        all_markers(p)
            .into_iter()
            .filter(|(_, m)| !m.is_empty())
            .map(|(id, m)| (id.to_string(), m.to_vec()))
            .collect::<Vec<_>>()
    };
    let now = marked(p);
    if now == marked(base) {
        return None;
    }
    let mut doc = extra.clone();
    if !doc.is_object() {
        return None;
    }
    if doc.get("allMarkersInfo").is_none_or(|v| !v.is_object()) {
        put(&mut doc, "allMarkersInfo", Value::Object(Map::new()));
    }
    let all = doc["allMarkersInfo"].as_object_mut().expect("object");
    all.retain(|k, v| !(v.is_array() && known_guids.contains(k)));
    let mut merged: Vec<(String, Vec<Value>)> = Vec::new();
    for (id, ms) in now {
        let Some(g) = tl_guid.get(&id).or_else(|| clip_guid.get(&id)) else { continue };
        let list: Vec<Value> = ms.iter().map(marker_json).collect();
        match merged.iter_mut().find(|(k, _)| k == g) {
            Some((_, l)) => l.extend(list),
            None => merged.push((g.clone(), list)),
        }
    }
    for (g, l) in merged {
        all.insert(g, Value::Array(l));
    }
    Some(doc)
}

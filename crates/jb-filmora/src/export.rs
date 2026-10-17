//! `jb_core::Project` → `.wfp`, by patching the original documents.
//!
//! Every entity is diffed against the baseline captured at read time and
//! only fields that changed are rewritten, so untouched values keep their
//! exact original representation. Entities created in Jibaru get JSON
//! shaped like what Filmora writes.

use crate::archive::Entry;
use crate::import::{self, all_effects, is_surfaced, timelines};
use crate::json::*;
use crate::mapping::*;
use crate::props;
use crate::title::{default_script, estimate_box_width, patch_title_clip};
use crate::{serialize_like, Result, WfpProject, WriteReport, MEDIAS_INFO, PROJECT_INFO};
use jb_core::eval::{self, transition_window};
use jb_core::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub fn export(wfp: &WfpProject) -> Result<(Vec<Entry>, WriteReport)> {
    let mut ex = Exporter::new(wfp);
    let timeline = ex.timeline();
    let medias_info = ex.medias_info();
    let project_info = ex.project_info();
    let extra = ex.extra_doc(&timeline);

    let raw = &wfp.raw;
    let mut entries = Vec::with_capacity(raw.entries.len());
    for e in &raw.entries {
        let patched = if e.name == raw.timeline_path {
            Some((&raw.timeline, &timeline))
        } else if Some(&e.name) == raw.extra_path.as_ref() {
            raw.extra.as_ref().zip(extra.as_ref())
        } else if e.name == PROJECT_INFO {
            Some((&raw.project_info, &project_info))
        } else if e.name == MEDIAS_INFO {
            raw.medias_info.as_ref().zip(medias_info.as_ref())
        } else {
            None
        };
        let mut out = e.clone();
        if let Some((orig, new)) = patched {
            if orig != new {
                out.data = serialize_like(&e.data, new);
            }
        }
        entries.push(out);
    }
    let sidecar = ex.sidecar_doc();
    let mut has_sidecar = false;
    entries.retain_mut(|e| {
        if e.name != crate::sidecar::SIDECAR {
            return true;
        }
        has_sidecar = true;
        match &sidecar {
            Some(doc) => {
                if crate::parse_json(&e.name, &e.data).ok().as_ref() != Some(doc) {
                    e.data = serde_json::to_vec(doc).unwrap_or_default();
                }
                true
            }
            None => false,
        }
    });
    if let (false, Some(doc)) = (has_sidecar, &sidecar) {
        let info = raw.entry(PROJECT_INFO);
        entries.push(Entry {
            name: crate::sidecar::SIDECAR.to_string(),
            data: serde_json::to_vec(doc).unwrap_or_default(),
            deflated: true,
            modified: info.and_then(|e| e.modified),
            unix_mode: info.and_then(|e| e.unix_mode),
        });
    }
    if let (None, Some(doc)) = (&raw.extra_path, &extra) {
        // Markers need the timeline's extra.json; create it next to the timeline.
        let dir = raw.timeline_path.strip_suffix("timeline.wesproj").unwrap_or_default();
        let tl = raw.entry(&raw.timeline_path);
        entries.push(Entry {
            name: format!("{dir}extra.json"),
            data: serde_json::to_vec(doc).unwrap_or_default(),
            deflated: true,
            modified: tl.and_then(|e| e.modified),
            unix_mode: tl.and_then(|e| e.unix_mode),
        });
    }
    Ok((entries, WriteReport { warnings: ex.warnings, notes: ex.notes }))
}

/// Filmora ids of transitions that registry kinds map onto.
const KNOWN_TRANSITIONS: &[(&str, &str, &str)] = &[
    ("jb.dissolve", "2981D185-D52E-44f4-ABD5-3CE83890E32E", "Dissolve"),
    ("jb.fade_black", "3B49DBEE-9A15-4844-B613-E2C2497EA236", "fade_black"),
    ("jb.audio_fade", "audio/blender/transition-fade", "audio fade"),
];

const DEFAULT_PIP: &str =
    "{\"Algorithm\":\"Bilinear\",\"BlendMode\":0,\"BlenderName\":\"video/blender/simple-pip\",\"Enable\":true,\"Opacity\":100.0}";

struct Exporter<'a> {
    wfp: &'a WfpProject,
    p: &'a Project,
    base: &'a Project,
    orig_clips: HashMap<&'a str, &'a Value>,
    orig_effects: HashMap<&'a str, &'a Value>,
    orig_transitions: HashMap<&'a str, &'a Value>,
    orig_timelines: HashMap<i64, &'a Value>,
    base_clips: HashMap<&'a str, &'a Clip>,
    base_media: HashMap<&'a str, &'a MediaAsset>,
    seq_tl: HashMap<&'a str, i64>,
    main_tl: i64,
    serial: i64,
    used_uids: HashSet<String>,
    used_guids: HashSet<String>,
    extra_timelines: Vec<Value>,
    new_resources: Vec<Value>,
    warnings: Vec<String>,
    /// Model clip id -> written `thisUId`.
    clip_uid: HashMap<String, String>,
    /// Model effect id -> written `thisUId` of effects created this write.
    effect_uid: HashMap<String, String>,
    notes: Vec<String>,
}

fn changed<T: PartialEq>(old: Option<&Clip>, new: &Clip, f: impl Fn(&Clip) -> T) -> bool {
    old.is_none_or(|o| f(o) != f(new))
}

impl<'a> Exporter<'a> {
    fn new(wfp: &'a WfpProject) -> Self {
        let raw = &wfp.raw;
        let mut ex = Exporter {
            wfp,
            p: &wfp.project,
            base: &raw.baseline,
            orig_clips: HashMap::new(),
            orig_effects: HashMap::new(),
            orig_transitions: HashMap::new(),
            orig_timelines: HashMap::new(),
            base_clips: HashMap::new(),
            base_media: HashMap::new(),
            seq_tl: HashMap::new(),
            main_tl: 0,
            serial: i64_of(&raw.timeline, "serialNumber").unwrap_or(1000),
            used_uids: HashSet::new(),
            used_guids: HashSet::new(),
            extra_timelines: Vec::new(),
            new_resources: Vec::new(),
            warnings: Vec::new(),
            clip_uid: HashMap::new(),
            effect_uid: HashMap::new(),
            notes: Vec::new(),
        };
        for tl in timelines(&raw.timeline) {
            let id = i64_of(tl, "timelineId").unwrap_or(0);
            ex.orig_timelines.insert(id, tl);
            ex.serial = ex.serial.max(id + 1);
            for c in import::tl_clips(tl) {
                if let Some(u) = str_of(c, "thisUId") {
                    ex.orig_clips.insert(u, c);
                }
                for (_, e) in all_effects(c) {
                    if let Some(u) = str_of(e, "thisUId") {
                        ex.orig_effects.insert(u, e);
                    }
                }
                for key in ["preTransition", "postTransition"] {
                    if let Some(u) = c.get(key).and_then(|t| str_of(t, "thisUId")) {
                        ex.orig_transitions.insert(u, &c[key]);
                    }
                }
            }
        }
        for s in &ex.base.sequences {
            for t in &s.tracks {
                for c in &t.clips {
                    if let Some(u) = marker_str(&c.extra, "uid") {
                        ex.base_clips.insert(u, c);
                    }
                }
            }
        }
        for m in &ex.base.media {
            ex.base_media.insert(m.id.as_str(), m);
        }
        let mut serial = ex.serial;
        for s in &ex.p.sequences {
            let tl = marker_i64(&s.extra, "timelineId").filter(|id| ex.orig_timelines.contains_key(id)).unwrap_or_else(
                || {
                    serial += 1;
                    serial - 1
                },
            );
            ex.seq_tl.insert(s.id.as_str(), tl);
        }
        ex.serial = serial;
        ex.main_tl = ex.seq_tl.get(ex.p.main_sequence.as_str()).copied().unwrap_or(0);
        ex
    }

    fn next_timeline_id(&mut self) -> i64 {
        self.serial += 1;
        self.serial - 1
    }

    fn fresh_uid(&mut self, preferred: Option<&str>) -> String {
        if let Some(p) = preferred.filter(|p| uuid::Uuid::parse_str(p).is_ok()) {
            if self.used_uids.insert(p.to_string()) {
                return p.to_string();
            }
        }
        let u = new_uid();
        self.used_uids.insert(u.clone());
        u
    }

    /// The `filename` clips use for a media file: the original reference
    /// (which may be relative to Filmora's resource folder) unless relinked.
    fn media_url(&self, m: &MediaAsset) -> String {
        let unchanged = self.base_media.get(m.id.as_str()).is_some_and(|b| b.path == m.path);
        match marker_str(&m.extra, "filename") {
            Some(f) if unchanged => f.to_string(),
            _ => path_to_url(&m.path),
        }
    }

    fn settings_changed(&self) -> bool {
        self.p.settings != self.base.settings
    }

    // -- documents ---------------------------------------------------------

    fn timeline(&mut self) -> Value {
        let raw = &self.wfp.raw;
        let mut doc = raw.timeline.clone();
        let by_tl: HashMap<i64, &Sequence> = self.p.sequences.iter().map(|s| (self.seq_tl[s.id.as_str()], s)).collect();
        let mut out = Vec::new();
        for tl in timelines(&raw.timeline) {
            let id = i64_of(tl, "timelineId").unwrap_or(0);
            match by_tl.get(&id) {
                Some(seq) => out.push(self.build_timeline(Some(tl), seq, id)),
                None => out.push(tl.clone()),
            }
        }
        for seq in &self.p.sequences {
            let id = self.seq_tl[seq.id.as_str()];
            if !self.orig_timelines.contains_key(&id) {
                out.push(self.build_timeline(None, seq, id));
            }
        }
        out.append(&mut self.extra_timelines);
        if self.settings_changed() {
            let s = &self.p.settings;
            for tl in &mut out {
                put(tl, "resolutionWidth", json!(s.width));
                put(tl, "resolutionHeight", json!(s.height));
                put(tl, "frameRate", json!({ "den": s.frame_rate.den, "num": s.frame_rate.num }));
                if tl.get("aspectRatioX").is_some() {
                    let g = gcd(s.width, s.height).max(1);
                    put(tl, "aspectRatioX", json!(s.width / g));
                    put(tl, "aspectRatioY", json!(s.height / g));
                }
            }
        }
        put(&mut doc, "timelineInfos", Value::Array(out));
        if i64_of(&doc, "serialNumber").is_some_and(|n| n < self.serial) {
            put(&mut doc, "serialNumber", json!(self.serial));
        }
        if !self.new_resources.is_empty() {
            if let Some(list) = doc.get_mut("resources").and_then(Value::as_array_mut) {
                list.append(&mut self.new_resources);
            }
        }
        // Relinked media: keep resource records pointing at the new file.
        for m in &self.p.media {
            if let Some(b) = self.base_media.get(m.id.as_str()) {
                if b.path != m.path {
                    let old = marker_str(&b.extra, "filename").map_or_else(|| path_to_url(&b.path), str::to_string);
                    let new = self.media_url(m);
                    for r in doc.get_mut("resources").and_then(Value::as_array_mut).into_iter().flatten() {
                        if str_of(r, "filename") == Some(old.as_str()) {
                            put(r, "filename", json!(new));
                        }
                    }
                }
            }
        }
        doc
    }

    fn project_info(&mut self) -> Value {
        let mut info = self.wfp.raw.project_info.clone();
        if self.p.name != self.base.name {
            put(&mut info, "project_file_name", json!(self.p.name));
        }
        if self.settings_changed() {
            let s = &self.p.settings;
            put(&mut info, "project_timeline_resolution", json!([s.width, s.height]));
            put(&mut info, "project_timeline_framerate", json!([s.frame_rate.num, s.frame_rate.den]));
            put(&mut info, "project_sample_rate", json!(s.sample_rate));
        }
        let (d, bd) = (self.p.main().duration(), self.base.main().duration());
        if d != bd {
            put(&mut info, "project_timeline_duration", json!(d));
        }
        info
    }

    fn medias_info(&mut self) -> Option<Value> {
        let mut doc = self.wfp.raw.medias_info.clone()?;
        let duration = (self.p.main().duration() != self.base.main().duration()).then(|| self.p.main().duration());
        let timeline_media = str_of(&self.wfp.raw.project_info, "timeline_mediaId").map(str::to_string);
        let items = doc.get_mut("media_items").and_then(Value::as_object_mut)?;
        if let (Some(d), Some(id)) = (duration, timeline_media) {
            if let Some(item) = items.get_mut(&id) {
                put(item, "duration", json!(d));
            }
        }
        let mut added = Vec::new();
        for m in &self.p.media {
            match self.base_media.get(m.id.as_str()) {
                Some(b) => {
                    if b.path != m.path || b.name != m.name {
                        let key = marker_str(&m.extra, "mediaId").unwrap_or(&m.id);
                        if let Some(item) = items.get_mut(key) {
                            put(item, "download_url", json!(m.path));
                            put(item, "name", json!(m.name));
                        }
                    }
                }
                None => {
                    let id = to_guid(&m.id);
                    if items.contains_key(&id) {
                        continue;
                    }
                    let media_type = match m.kind {
                        MediaKind::Video => 8,
                        MediaKind::Image => 16,
                        MediaKind::Audio => 262_144,
                    };
                    items.insert(
                        id.clone(),
                        json!({
                            "download_url": m.path, "id": id, "media_type": media_type,
                            "media_length": if m.kind == MediaKind::Image { 50_000_000 } else { m.duration },
                            "name": m.name, "mediaCreationInfo": "{\"creationType\":0}",
                            "stream_idx": if m.kind == MediaKind::Image { -1 } else { 0 },
                            "mark_info_list": [{"mark_in": -1, "mark_out": -1}], "scence_info": []
                        }),
                    );
                    added.push(id);
                }
            }
        }
        if !added.is_empty() {
            if let Some(root) = doc.get_mut("media_structure") {
                let list = match root.get("media_item") {
                    Some(Value::Array(a)) => a.clone(),
                    Some(Value::String(s)) => vec![json!(s)],
                    _ => Vec::new(),
                };
                let mut list = list;
                list.extend(added.into_iter().map(Value::String));
                put(root, "media_item", Value::Array(list));
            }
        }
        Some(doc)
    }

    // -- timelines and tracks ----------------------------------------------

    fn build_timeline(&mut self, orig: Option<&Value>, seq: &Sequence, tl_id: i64) -> Value {
        let mut tl = match orig {
            Some(o) => o.clone(),
            None => self.template_timeline(tl_id, tl_id == self.main_tl),
        };
        let is_main = i64_of(&tl, "type") == Some(0);
        let orig_tracks: Vec<Value> = tl.get("trackInfos").and_then(Value::as_array).cloned().unwrap_or_default();
        let bus = tl.get("audioBusInfos").and_then(|b| b.get(0)).and_then(|b| str_of(b, "busUid")).map(str::to_string);
        let mut next_tag = orig_tracks.iter().filter_map(|t| i64_of(t, "trackTag")).max().unwrap_or(0) + 1;
        let mut tracks = Vec::new();
        for track in &seq.tracks {
            let same_tl = marker_i64(&track.extra, "timeline") == Some(tl_id) && orig.is_some();
            let base = marker_i64(&track.extra, "index")
                .filter(|_| same_tl)
                .and_then(|i| orig_tracks.get(i as usize))
                .cloned();
            let mut t = base.unwrap_or_else(|| {
                let tag = next_tag;
                next_tag += 1;
                match track.kind {
                    TrackKind::Video => json!({ "clipList": [], "trackTag": tag, "trackType": TRACK_VIDEO, "userData": [], "uuid": new_uid() }),
                    TrackKind::Audio => json!({
                        "busUuids": bus.iter().collect::<Vec<_>>(), "clipList": [], "trackTag": tag,
                        "trackType": TRACK_AUDIO, "userData": [], "uuid": new_uid()
                    }),
                }
            });
            self.patch_track_flags(&mut t, track);
            let mut clips = Vec::new();
            for clip in &track.clips {
                if let Some(v) = self.clip_json(clip, tl_id, track.kind, is_main) {
                    clips.push(v);
                }
            }
            put(&mut t, "clipList", Value::Array(clips));
            tracks.push(t);
        }
        self.patch_camera_audio(seq, &mut tracks);
        put(&mut tl, "trackInfos", Value::Array(tracks));
        if !seq.markers.is_empty() && user_data(&tl, ud::GUID).is_none() {
            // Markers are keyed by the timeline GUID.
            set_user_data(&mut tl, ud::GUID, &guid_blob(&new_guid()));
        }
        tl
    }

    /// Track flags as Filmora writes them: `locked: true`, `visible: false`
    /// (hidden video track) and `mute: true` are present only while set.
    /// A video track's sound lives on its camera-audio tracks (userData 20 =
    /// its `trackTag`), so hiding or muting it mutes those (verified in
    /// Filmora 15.7.3). Names are userData 50, heights userData 12000.
    fn patch_track_flags(&mut self, t: &mut Value, track: &Track) {
        let base = self.base_track(&track.id);
        let flags = |x: &Track| (x.locked, x.muted, x.hidden);
        let unchanged = base.map_or(flags(track) == (false, false, false), |b| flags(b) == flags(track));
        if !unchanged {
            set_flag(t, "locked", track.locked);
            match track.kind {
                TrackKind::Video => {
                    if track.hidden {
                        put(t, "visible", json!(false));
                    } else {
                        remove_key(t, "visible");
                    }
                }
                TrackKind::Audio => {
                    if track.hidden {
                        self.warnings.push(format!("track {}: audio tracks cannot be hidden in Filmora", track.id));
                    }
                    remove_key(t, "visible");
                    set_flag(t, "mute", track.muted);
                }
            }
        }
        if base.is_some_and(|b| b.name != track.name) {
            set_user_data(t, ud::NAME, track.name.as_bytes());
        }
        if base.map_or(track.height.is_some(), |b| b.height != track.height) {
            let h = track.height.map_or(import::default_track_height(track.kind), |h| h as i32);
            set_user_data(t, ud::TRACK_HEIGHT, &h.to_le_bytes());
        }
    }

    fn base_track(&self, id: &str) -> Option<&'a Track> {
        self.base.sequences.iter().flat_map(|s| s.tracks.iter()).find(|b| b.id == id)
    }

    /// Mutes the camera-audio tracks of hidden or muted video tracks.
    fn patch_camera_audio(&mut self, seq: &Sequence, tracks: &mut [Value]) {
        for (i, vt) in seq.tracks.iter().enumerate() {
            if vt.kind != TrackKind::Video {
                continue;
            }
            let base = self.base_track(&vt.id);
            if base.map_or(!vt.hidden && !vt.muted, |b| (b.hidden, b.muted) == (vt.hidden, vt.muted)) {
                continue;
            }
            let Some(tag) = i64_of(&tracks[i], "trackTag") else { continue };
            for (j, at) in seq.tracks.iter().enumerate() {
                if at.kind == TrackKind::Audio && user_data_i32(&tracks[j], ud::CAMERA_AUDIO_OF) == Some(tag as i32) {
                    set_flag(&mut tracks[j], "mute", at.muted || vt.hidden || vt.muted);
                }
            }
        }
    }

    /// `extra.json` with the markers of the project, when they changed.
    fn extra_doc(&mut self, timeline: &Value) -> Option<Value> {
        let raw = &self.wfp.raw;
        let mut tl_guid = HashMap::new();
        let mut clip_guid = HashMap::new();
        let mut clip_in = HashMap::new();
        let mut known = HashSet::new();
        let by_uid: HashMap<&str, &str> = self.clip_uid.iter().map(|(k, v)| (v.as_str(), k.as_str())).collect();
        for doc in [timeline, &raw.timeline] {
            for tl in timelines(doc) {
                let id = i64_of(tl, "timelineId").unwrap_or(0);
                if let Some(g) = user_data_str(tl, ud::GUID) {
                    known.insert(g.clone());
                    if let Some((sid, _)) = self.seq_tl.iter().find(|(_, t)| **t == id) {
                        tl_guid.entry(sid.to_string()).or_insert(g);
                    }
                }
                for c in import::tl_clips(tl) {
                    if let (Some(u), Some(g)) = (str_of(c, "thisUId"), user_data_str(c, ud::GUID)) {
                        known.insert(g.clone());
                        if let Some(cid) = by_uid.get(u) {
                            clip_guid.entry(cid.to_string()).or_insert(g);
                            clip_in.entry(cid.to_string()).or_insert(i64_of(c, "inPoint").unwrap_or(0));
                        }
                    }
                }
            }
        }
        let empty = json!({
            "fontNameInfo": [], "usedBizFont": [], "usedTemplateResInfo": {}, "mediaClipsMapInfo": {},
            "allMarkersInfo": {}, "pendingMarkersInfo": {}, "highlightInfo": {}, "TextSentence": {"TextSentence": []}
        });
        crate::markers::export_markers(
            self.p,
            self.base,
            raw.extra.as_ref().unwrap_or(&empty),
            &tl_guid,
            &clip_guid,
            &clip_in,
            &known,
        )
    }

    /// Jibaru-only clip state for the archive sidecar (see `crate::sidecar`).
    fn sidecar_doc(&mut self) -> Option<Value> {
        use crate::sidecar::{is_private, native, ClipState};
        let mut clips = Vec::new();
        let mut kinds = Vec::new();
        let mut notes = Vec::new();
        for clip in self.p.sequences.iter().flat_map(|s| s.tracks.iter()).flat_map(|t| t.clips.iter()) {
            let Some(uid) = self.clip_uid.get(&clip.id) else { continue };
            let mut st = ClipState::default();
            for (i, e) in clip.effects.iter().enumerate() {
                if is_private(e) {
                    notes.push(format!("clip {}: effect {} kept for Jibaru; Filmora won't show it", clip.id, e.kind));
                    st.effects.push((i, e.clone()));
                } else if native(&e.kind).is_some() {
                    let euid =
                        self.effect_uid.get(&e.id).cloned().or_else(|| marker_str(&e.extra, "uid").map(str::to_string));
                    if let Some(u) = euid {
                        kinds.push((u, e.kind.clone()));
                    }
                }
            }
            let filmora_denoise = marker_str(&clip.extra, "uid")
                .and_then(|u| self.orig_clips.get(u))
                .and_then(|c| import::audio_of(c, i64_of(c, "inPoint").unwrap_or(0)).denoise_db);
            if clip.audio.denoise_db != filmora_denoise {
                st.denoise_db = Some(clip.audio.denoise_db);
                notes.push(format!("clip {}: noise reduction kept for Jibaru; Filmora won't show it", clip.id));
            }
            st.channels = clip.audio.channels;
            if st.channels != ChannelMode::Stereo {
                notes.push(format!("clip {}: audio channels kept for Jibaru; Filmora won't show it", clip.id));
            }
            st.voice_filter = clip.audio.voice_filter.clone();
            st.hum_removal = clip.audio.hum_removal;
            st.hiss_removal = clip.audio.hiss_removal;
            if st.voice_filter.is_some() || st.hum_removal.is_some() || st.hiss_removal.is_some() {
                notes.push(format!("clip {}: voice filter and audio cleanup kept for Jibaru; Filmora won't show them", clip.id));
            }
            clips.push((uid.clone(), st));
        }
        self.notes.extend(notes);
        crate::sidecar::document(&clips, &kinds)
    }

    fn template_timeline(&self, tl_id: i64, main: bool) -> Value {
        let main_tl = self.orig_timelines.get(&self.main_tl).copied();
        let s = &self.p.settings;
        let get = |k: &str, d: Value| main_tl.and_then(|m| m.get(k)).cloned().unwrap_or(d);
        json!({
            "aspectRatioX": get("aspectRatioX", json!(16)),
            "aspectRatioY": get("aspectRatioY", json!(9)),
            "audioBusInfos": [{ "balance": 0.5, "busUid": new_uid(), "gain": 0.0 }],
            "channels": get("channels", json!(2)),
            "format": get("format", json!("f32")),
            "frameRate": { "den": s.frame_rate.den, "num": s.frame_rate.num },
            "hdrMode": 0,
            "hdrMode_force_16bit": 0,
            "resolutionHeight": s.height,
            "resolutionWidth": s.width,
            "sampleRate": get("sampleRate", json!(s.sample_rate)),
            "timelineId": tl_id,
            "trackInfos": [],
            "type": if main { 0 } else { 1 },
            "userData": []
        })
    }

    // -- clips ---------------------------------------------------------------

    fn clip_json(&mut self, clip: &Clip, tl_id: i64, track_kind: TrackKind, in_main: bool) -> Option<Value> {
        let uid = marker_str(&clip.extra, "uid");
        let orig = uid.and_then(|u| self.orig_clips.get(u).copied());
        let base = uid.and_then(|u| self.base_clips.get(u).copied());
        let (mut v, old) = match (orig, base) {
            (Some(o), Some(b)) => (o.clone(), Some(b)),
            _ => (self.template_clip(clip, tl_id, track_kind, in_main)?, None),
        };
        let this = self.fresh_uid(Some(&clip.id));
        self.clip_uid.insert(clip.id.clone(), this.clone());
        if str_of(&v, "thisUId") != Some(this.as_str()) {
            put(&mut v, "thisUId", json!(this));
        }
        if old.is_none() || user_data_i32(&v, ud::OWNER_TIMELINE).is_some_and(|o| o as i64 != tl_id) {
            set_user_data(&mut v, ud::OWNER_TIMELINE, &(tl_id as i32).to_le_bytes());
        }
        self.patch_clip(&mut v, old, clip);
        if old.is_some() && uid != Some(clip.id.as_str()) {
            // A clip derived from another one (split, duplicate): its nested
            // objects need identities of their own.
            self.renew_nested_ids(&mut v);
        }
        Some(v)
    }

    fn renew_nested_ids(&mut self, v: &mut Value) {
        let mut objs: Vec<&mut Value> = Vec::new();
        if let Some(chains) = v.get_mut("effectChainList").and_then(Value::as_array_mut) {
            for ch in chains {
                if let Some(list) = ch.get_mut("effectList").and_then(Value::as_array_mut) {
                    objs.extend(list.iter_mut());
                }
            }
        }
        for o in objs {
            let u = self.fresh_uid(None);
            put(o, "thisUId", json!(u));
            if user_data(o, ud::GUID).is_some() {
                set_user_data(o, ud::GUID, &guid_blob(&new_guid()));
            }
        }
        for key in ["preTransition", "postTransition"] {
            if let Some(t) = v.get_mut(key).filter(|t| t.is_object()) {
                let u = new_uid();
                put(t, "thisUId", json!(u));
                if user_data(t, ud::GUID).is_some() {
                    set_user_data(t, ud::GUID, &guid_blob(&new_guid()));
                }
            }
        }
    }

    fn template_clip(&mut self, clip: &Clip, tl_id: i64, track_kind: TrackKind, in_main: bool) -> Option<Value> {
        let dur = clip.duration();
        match &clip.source {
            ClipSource::Media { media_id } => {
                let Some(m) = self.p.media(media_id) else {
                    self.warnings.push(format!("clip {}: media {media_id} not found, skipped", clip.id));
                    return None;
                };
                let source_uuid = self.resource_for(m);
                let mut v = if track_kind == TrackKind::Audio || m.kind == MediaKind::Audio {
                    audio_clip_template()
                } else {
                    video_clip_template()
                };
                put(&mut v, "filename", json!(self.media_url(m)));
                put(&mut v, "sourceUuid", json!(source_uuid));
                let stream = if track_kind == TrackKind::Audio && m.kind == MediaKind::Video { 1 } else { 0 };
                put(&mut v, "streamId", json!(stream));
                if m.kind == MediaKind::Image {
                    if let Some(o) = v.as_object_mut() {
                        o.remove("speed");
                    }
                } else {
                    let total = ticks_to_secs(m.duration);
                    let kf = KfParam {
                        keys: vec![Kf::new(0.0, 1.0), Kf::new(total, 1.0)],
                        total_time: Some(total),
                        ..KfParam::empty()
                    };
                    put(&mut v["speed"], "speedParam", json!(kf.to_text()));
                }
                let kind = if v["type"] == json!(CLIP_AUDIO) { 2 } else { 1 };
                set_user_data(&mut v, ud::KIND, &(kind as i32).to_le_bytes());
                set_user_data(&mut v, ud::GUID, &guid_blob(&new_guid()));
                set_user_data(&mut v, ud::MEDIA_ITEM, to_guid(media_id).as_bytes());
                Some(v)
            }
            ClipSource::Sequence { sequence_id } => {
                let Some(target) = self.seq_tl.get(sequence_id.as_str()).copied() else {
                    self.warnings.push(format!("clip {}: sequence {sequence_id} not found, skipped", clip.id));
                    return None;
                };
                let mut v = timeline_ref_template();
                put(&mut v, "timelineId", json!(target));
                put(&mut v, "outPoint", json!(dur));
                set_user_data(&mut v, ud::GUID, &guid_blob(&new_guid()));
                Some(v)
            }
            ClipSource::Title(title) => {
                if !in_main {
                    return Some(text_clip_template(dur));
                }
                // Top-level titles are a nested timeline holding the text clip.
                let sub_id = self.next_timeline_id();
                let mut sub = self.template_timeline(sub_id, false);
                let mut text = text_clip_template(dur);
                put(&mut text, "thisUId", json!(self.fresh_uid(None)));
                set_user_data(&mut text, ud::OWNER_TIMELINE, &(sub_id as i32).to_le_bytes());
                patch_title_clip(&mut text, None, title);
                let aspect = self.p.settings.width as f64 / self.p.settings.height.max(1) as f64;
                let box_h = title.font_size / TITLE_GLYPH_RATIO;
                let mut script = nested(&text, "scriptBuf").unwrap_or_else(default_script);
                let bw = estimate_box_width(&title.text, box_h, aspect);
                script["ScaleX"] = json!(bw);
                script["ViewSize"] = json!(format!("{}:{}", self.p.settings.width, self.p.settings.height));
                set_nested(&mut text, "scriptBuf", &script);
                if let Some(e) = transform_effect_mut(&mut text) {
                    set_effect_param(e, "Scale_x", json!(bw * 100.0), 3);
                }
                put(
                    &mut sub,
                    "trackInfos",
                    json!([{ "clipList": [text], "trackType": TRACK_VIDEO, "uuid": new_uid() }]),
                );
                self.extra_timelines.push(sub);
                let mut v = timeline_ref_template();
                put(&mut v, "timelineId", json!(sub_id));
                put(&mut v, "outPoint", json!(dur));
                set_user_data(&mut v, ud::GUID, &guid_blob(&new_guid()));
                let _ = tl_id;
                Some(v)
            }
            ClipSource::Color { .. } | ClipSource::Adjustment => {
                self.warnings.push(format!(
                    "clip {}: color/adjustment clips cannot be written to Filmora yet, skipped",
                    clip.id
                ));
                None
            }
        }
    }

    /// Ensures a `resources` record exists for a media file; returns its sourceUuid.
    fn resource_for(&mut self, m: &MediaAsset) -> String {
        let url = self.media_url(m);
        let existing = self
            .wfp
            .raw
            .timeline
            .get("resources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .chain(self.new_resources.iter());
        for r in existing {
            if str_of(r, "filename") == Some(url.as_str()) {
                return str_of(r, "sourceUuid").unwrap_or_default().to_string();
            }
        }
        let source_uuid = new_uid();
        let (stream_type, vids, auds) = match m.kind {
            MediaKind::Video => (2, 1, m.has_audio as i64),
            MediaKind::Audio => (3, 0, 1),
            MediaKind::Image => (5, 1, 0),
        };
        let fr = m.frame_rate.unwrap_or(self.p.settings.frame_rate);
        let mut r = json!({
            "audioStreamCount": auds, "filename": url, "mediaLength": if m.kind == MediaKind::Image { 0 } else { m.duration },
            "programCount": 1, "sourceUuid": source_uuid, "streamType": stream_type, "subPicStreamCount": 0,
            "videoStreamCount": vids
        });
        if vids > 0 {
            put(
                &mut r,
                "vidStreamInfo",
                json!([{ "frameRate": { "den": fr.den, "num": fr.num }, "height": m.height, "vidStreamId": 0, "width": m.width }]),
            );
        }
        self.new_resources.push(r);
        source_uuid
    }

    fn patch_clip(&mut self, v: &mut Value, old: Option<&Clip>, new: &Clip) {
        let ty = i64_of(v, "type").unwrap_or(0);
        let dur = new.duration();

        // Timing.
        if changed(old, new, |c| (c.start, c.end)) {
            put(v, "tlBegin", json!(new.start));
            put(v, "tlEnd", json!(new.end));
        }
        let retimed =
            changed(old, new, |c| (c.source_in, c.duration(), c.speed.rate, c.speed.ramp.clone(), c.speed.freeze));
        let has_speed = v.get("speed").is_some_and(Value::is_object);
        if retimed {
            if has_speed {
                let rate = if new.speed.freeze { 1.0 } else { new.speed.rate.max(1e-6) };
                // The importer snaps retimed in-points to Filmora's frame
                // grid (constant rates); an unchanged in-point keeps the
                // project's offset.
                let plain = |c: &Clip| c.speed.ramp.is_none() && !c.speed.freeze;
                let kept = old
                    .filter(|o| o.source_in == new.source_in && plain(o) && plain(new))
                    .and_then(|_| v["speed"]["offset"].as_f64());
                let offset = kept.unwrap_or_else(|| ticks_to_secs(new.source_in));
                // A freeze frame counts its frozen duration as source time.
                let span = if new.speed.freeze { dur } else { jb_core::speed::source_span(new) };
                put(&mut v["speed"], "offset", json!(offset));
                put(&mut v["speed"], "offsetEnd", json!(offset + ticks_to_secs(span)));
                let in_point = (new.source_in as f64 / rate).round() as i64;
                put(v, "inPoint", json!(in_point));
                put(v, "outPoint", json!(in_point + dur));
            } else {
                put(v, "inPoint", json!(new.source_in));
                put(v, "outPoint", json!(new.source_in + dur));
            }
        }
        if has_speed && changed(old, new, |c| c.speed.clone()) {
            put(&mut v["speed"], "reverse", json!(new.speed.reverse));
            if changed(old, new, |c| {
                (c.speed.rate, c.speed.ramp.clone(), c.speed.curve.clone(), c.speed.freeze, c.speed.reverse)
            }) || retimed
            {
                let text = self.speed_param(v, new);
                put(&mut v["speed"], "speedParam", json!(text));
            }
            // Fields of earlier Jibaru versions that Filmora does not have.
            remove_key(&mut v["speed"], "rampPreset");
            remove_key(&mut v["speed"], "keepPitch");
            remove_key(v, "freezeFrame");
        } else if !has_speed
            && (new.speed.rate != 1.0 || new.speed.ramp.is_some())
            && changed(old, new, |c| c.speed.clone())
        {
            self.warnings.push(format!("clip {}: clip type {ty} has no speed settings in Filmora", new.id));
        }
        // Maintain pitch is the default; audio clips flag the opposite.
        if ty == CLIP_AUDIO && changed(old, new, |c| c.speed.maintain_pitch) {
            set_flag(v, "speedWithPitch", !new.speed.maintain_pitch);
        }
        let in_point = i64_of(v, "inPoint").unwrap_or(0);

        // Source.
        match &new.source {
            ClipSource::Media { media_id } => {
                let media = self.p.media(media_id);
                let relinked = media.zip(self.base_media.get(media_id.as_str())).is_some_and(|(m, b)| m.path != b.path);
                if old.is_some() && (changed(old, new, |c| c.source.clone()) || relinked) {
                    if let Some(m) = media {
                        let sid = self.resource_for(m);
                        put(v, "filename", json!(self.media_url(m)));
                        put(v, "sourceUuid", json!(sid));
                        set_user_data(v, ud::MEDIA_ITEM, to_guid(media_id).as_bytes());
                    }
                }
            }
            ClipSource::Sequence { sequence_id } => {
                if changed(old, new, |c| c.source.clone()) {
                    if let Some(tl) = self.seq_tl.get(sequence_id.as_str()) {
                        put(v, "timelineId", json!(tl));
                    }
                }
            }
            ClipSource::Title(t) => {
                let old_title = old.and_then(|o| match &o.source {
                    ClipSource::Title(t) => Some(t),
                    _ => None,
                });
                if ty == CLIP_TEXT && old_title != Some(t) {
                    patch_title_clip(v, old_title, t);
                }
            }
            _ => {
                if changed(old, new, |c| c.source.clone()) {
                    self.warnings
                        .push(format!("clip {}: changing the source to color/adjustment is not written", new.id));
                }
            }
        }

        if changed(old, new, |c| c.name.clone()) && !(old.is_none() && matches!(new.source, ClipSource::Title(_))) {
            set_user_data(v, ud::NAME, new.name.as_bytes());
        }

        // Visual properties.
        if ty != CLIP_TEXT && ty != CLIP_AUDIO && changed(old, new, |c| c.transform.clone()) {
            let media = match &new.source {
                ClipSource::Media { media_id } => self.p.media(media_id).map(|m| (m.width, m.height)),
                _ => None,
            };
            self.patch_transform(v, old.map(|o| &o.transform), &new.transform, &new.id, in_point, media);
        } else if ty == CLIP_TEXT && new.transform != Transform::default() && changed(old, new, |c| c.transform.clone())
        {
            // The text clip's transform mirrors the text box; position and
            // size are edited through the title data instead.
            self.warnings.push(format!("clip {}: text clip transform is derived from the title, not written", new.id));
        }
        if ty != CLIP_AUDIO && changed(old, new, |c| c.opacity.clone()) {
            let mut pip = nested(v, "pipBuf").unwrap_or_else(|| serde_json::from_str(DEFAULT_PIP).unwrap_or_default());
            put(&mut pip, "Opacity", json!(new.opacity.value.as_f64().unwrap_or(1.0) * 100.0));
            if pip.get("OpacityKeyFrame").is_some() || !new.opacity.keyframes.is_empty() {
                let tb = props::TimeBase::ticks(in_point);
                put(&mut pip, "OpacityKeyFrame", json!(props::kf_param(&new.opacity, 100.0, 0.0, tb).to_text()));
            }
            set_nested(v, "pipBuf", &pip);
        }
        if changed(old, new, |c| c.blend_mode) {
            if ty == CLIP_AUDIO {
                if new.blend_mode != BlendMode::Normal {
                    self.warnings.push(format!("clip {}: audio clips have no blend mode", new.id));
                }
            } else if let Some(name) = props::blend_name(new.blend_mode) {
                let mut pip =
                    nested(v, "pipBuf").unwrap_or_else(|| serde_json::from_str(DEFAULT_PIP).unwrap_or_default());
                put(&mut pip, "BlendMode", json!(name));
                set_nested(v, "pipBuf", &pip);
            } else {
                self.warnings.push(format!(
                    "clip {}: blend mode {:?} does not exist in Filmora, not written",
                    new.id, new.blend_mode
                ));
            }
        }
        // Groups: userData 53 on the video clips of the group.
        if ty != CLIP_AUDIO && changed(old, new, |c| c.group.clone()) {
            match &new.group {
                Some(g) => set_user_data(v, ud::GROUP, &guid_blob(&to_guid(g))),
                None => remove_user_data(v, ud::GROUP),
            }
        }
        if changed(old, new, |c| c.disabled) {
            if new.disabled {
                put(v, "enable", json!(false));
            } else {
                remove_key(v, "enable");
            }
        }

        // Audio.
        if changed(old, new, |c| c.audio.clone()) {
            self.patch_audio(v, old.map(|o| &o.audio), &new.audio, ty, &new.id, in_point);
        }

        // Effects, animations and transitions.
        let pre = |c: &Clip| c.animations.iter().find(|a| is_pre_transition(a)).cloned();
        let surfaced = |c: &Clip| {
            let mut all: Vec<EffectInstance> = c.effects.clone();
            all.extend(c.animations.iter().filter(|a| is_legacy_animation(a)).cloned());
            all
        };
        if changed(old, new, surfaced) {
            self.rebuild_effects(v, old, new, in_point);
        }
        let motions = |c: &Clip| {
            c.animations
                .iter()
                .filter(|a| is_motion(a))
                .map(|a| (a.kind.clone(), a.placement, a.range))
                .collect::<Vec<_>>()
        };
        if changed(old, new, motions) {
            self.patch_motions(v, new);
        }
        if changed(old, new, |c| (pre(c), c.start)) {
            self.patch_pre_transition(v, old, new, pre(new));
        }
        if changed(old, new, |c| (c.transition_out.clone(), c.end)) {
            self.patch_post_transition(v, old, new);
        }

        // Link group (userData GUID; unlinked clips need a unique one).
        let current = user_data_str(v, ud::GUID);
        match &new.link_group {
            Some(g) => {
                let guid = to_guid(g);
                if current.as_deref() != Some(guid.as_str()) {
                    set_user_data(v, ud::GUID, &guid_blob(&guid));
                }
            }
            None => {
                let taken = current.as_ref().is_none_or(|c| self.used_guids.contains(c));
                if taken || old.is_some_and(|o| o.link_group.is_some()) {
                    let g = new_guid();
                    set_user_data(v, ud::GUID, &guid_blob(&g));
                }
            }
        }
        if let Some(g) = user_data_str(v, ud::GUID) {
            if new.link_group.is_none() {
                self.used_guids.insert(g);
            }
        }
    }

    /// `inAnimation` / `outAnimation` Motion presets.
    fn patch_motions(&mut self, v: &mut Value, new: &Clip) {
        for placement in [AnimationPlacement::In, AnimationPlacement::Out] {
            let field = props::anim_field(placement).expect("slot");
            let want = new.animations.iter().find(|a| is_motion(a) && a.placement == Some(placement));
            let Some(a) = want else {
                remove_key(v, field);
                continue;
            };
            let len = a.range.map_or(0, |(x, y)| (y - x).max(0));
            let existing = v.get(field).filter(|e| e.is_object());
            let same =
                existing.is_some_and(|e| props::motion_kind(str_of(e, "name").unwrap_or(""), placement) == a.kind);
            if same {
                put(&mut v[field], "duration", json!(len));
                continue;
            }
            match props::motion_json(&a.kind, len) {
                Some(m) => {
                    let name = str_of(&m, "name").unwrap_or_default().to_string();
                    put(v, field, m);
                    set_user_data(v, props::UD_ANIM_NAME, name.as_bytes());
                }
                None => self
                    .warnings
                    .push(format!("clip {}: animation {} has no Filmora Motion preset, not written", new.id, a.kind)),
            }
        }
        for a in new.animations.iter().filter(|a| !is_pre_transition(a) && !is_legacy_animation(a) && !is_motion(a)) {
            self.warnings.push(format!(
                "clip {}: {:?} animation {} has no slot in Filmora 15 (Motion presets are in/out), not written",
                new.id, a.placement, a.kind
            ));
        }
    }

    fn speed_param(&self, v: &Value, new: &Clip) -> String {
        let prev = v.get("speed").and_then(|s| str_of(s, "speedParam")).and_then(KfParam::parse);
        let media_total = match &new.source {
            ClipSource::Media { media_id } => self.p.media(media_id).map(|m| ticks_to_secs(m.duration)),
            _ => None,
        };
        let total = prev.as_ref().and_then(|k| k.total_time).filter(|t| *t > 0.0).or(media_total).unwrap_or(0.0);
        let version = prev.as_ref().map_or(3, |k| k.version);
        let span = ticks_to_secs(jb_core::speed::source_span(new));
        let mut freeze = Vec::new();
        let keys = match (&new.speed.curve, &new.speed.ramp) {
            _ if new.speed.freeze => {
                // Filmora: an extra key at the frozen time and `_freeze`.
                let at = ticks_to_secs(new.source_in);
                freeze.push((at, ticks_to_secs(new.duration())));
                let mut keys = vec![Kf::new(0.0, 1.0)];
                if at > 0.0 && at < total {
                    keys.push(Kf::new(at, 1.0));
                }
                keys.push(Kf::new(total, 1.0));
                keys
            }
            // Ramp keys sit at source seconds (code 9, smooth tangents).
            (Some(c), Some(_)) => ramp_keys(
                c.points.iter().map(|p| (ticks_to_secs(new.source_in) + p.position * span, p.speed)).collect(),
            ),
            (None, Some(r)) if !r.keyframes.is_empty() => ramp_keys(
                r.keyframes
                    .iter()
                    .map(|k| {
                        let fwd = Clip { speed: Speed { reverse: false, ..new.speed.clone() }, ..new.clone() };
                        (ticks_to_secs(eval::source_time(&fwd, k.time)), k.value.as_f64().unwrap_or(1.0))
                    })
                    .collect(),
            ),
            _ => {
                // Filmora's uniform-speed convention: `rate → 1.0` over the media.
                let r = new.speed.rate;
                let mut a = Kf::new(0.0, r);
                let mut b = Kf::new(total, 1.0);
                if r != 1.0 && total > 0.0 {
                    let d = (1.0 - r) / total;
                    a.right_derivative = Some(d);
                    b.left_derivative = Some(d);
                }
                vec![a, b]
            }
        };
        KfParam { version, keys, total_time: Some(total), reverse: new.speed.reverse, freeze, ..KfParam::empty() }
            .to_text()
    }

    fn patch_transform(
        &mut self,
        v: &mut Value,
        old: Option<&Transform>,
        new: &Transform,
        clip_id: &str,
        in_point: Ticks,
        media: Option<(u32, u32)>,
    ) {
        use props::*;
        let tb = TimeBase::seconds(in_point);
        if transform_effect_mut(v).is_none() {
            let basic = chain_mut(v, "Basic", 0);
            if let Some(list) = basic.get_mut("effectList").and_then(Value::as_array_mut) {
                list.push(new_effect_json(FX_TRANSFORM, "transform", &[]));
            }
        }
        let ch = |f: &dyn Fn(&Transform) -> Param| old.is_none_or(|o| f(o) != f(new));
        {
            let Some(e) = transform_effect_mut(v) else { return };
            if ch(&|t| t.position_x.clone()) {
                set_effect_param_full(e, "Position_x", &new.position_x, 1.0, 0.5, 3, tb);
            }
            if ch(&|t| t.position_y.clone()) {
                set_effect_param_full(e, "Position_y", &new.position_y, 1.0, 0.5, 3, tb);
            }
            if ch(&|t| t.scale_x.clone()) {
                set_effect_param_full(e, "Scale_x", &new.scale_x, 100.0, 0.0, 3, tb);
            }
            if ch(&|t| t.scale_y.clone()) {
                set_effect_param_full(e, "Scale_y", &new.scale_y, 100.0, 0.0, 3, tb);
            }
            if ch(&|t| t.rotation.clone()) {
                set_effect_param_full(e, TF_ROTATION, &new.rotation, 1.0, 0.0, 3, tb);
            }
        }
        for (on, was, id, display) in [
            (new.flip_h, old.map(|o| o.flip_h), FX_FLIP_H, "horizontal_filp"),
            (new.flip_v, old.map(|o| o.flip_v), FX_FLIP_V, "vertical_filp"),
        ] {
            if was == Some(on) || (was.is_none() && !on) {
                continue;
            }
            if !on && import::find_effect(v, id).is_none() {
                continue;
            }
            let e = ensure_effect(v, "BasicFollow", id, display);
            set_user_data(e, UD_FLIP_ACTIVE, &(on as i32).to_le_bytes());
            if on {
                remove_key(e, "enable");
            } else {
                put(e, "enable", json!(false));
            }
        }
        if matches!(new.fit, FitMode::Stretch | FitMode::None) && old.is_none_or(|o| o.fit != new.fit) {
            self.warnings
                .push(format!("clip {clip_id}: fit mode {:?} does not exist in Filmora, not written", new.fit));
        }
        let fill = new.fit == FitMode::Fill;
        if old.is_none_or(|o| o.crop != new.crop || (o.fit == FitMode::Fill) != fill) {
            let mut rect = rect_of(new.crop);
            if fill {
                // "Crop to Fit": the centered part of the crop with the canvas aspect.
                let s = &self.p.settings;
                let (mw, mh) = media.filter(|(w, h)| *w > 0 && *h > 0).unwrap_or((s.width, s.height));
                let src_aspect = rect[2] * mw as f64 / (rect[3] * mh as f64).max(1e-9);
                let canvas = s.width as f64 / s.height.max(1) as f64;
                if src_aspect > canvas {
                    let w = rect[2] * canvas / src_aspect;
                    rect[0] += (rect[2] - w) / 2.0;
                    rect[2] = w;
                } else {
                    let h = rect[3] * src_aspect / canvas;
                    rect[1] += (rect[3] - h) / 2.0;
                    rect[3] = h;
                }
            }
            let full = rect == [0.0, 0.0, 1.0, 1.0];
            if !full || import::find_effect(v, FX_CROP).is_some() {
                let e = ensure_effect(v, "Basic", FX_CROP, "crop-pan-zoom");
                if full {
                    remove_effect_param(e, CROP_SCRIPT);
                } else {
                    let key = CropKey { ori_time: 0, time: 0.0, rect };
                    set_effect_param(e, CROP_SCRIPT, json!(crop_script(&[key])), 6);
                }
                if user_data_i32(e, UD_PAN_ZOOM) == Some(1) {
                    set_user_data(e, UD_PAN_ZOOM, &0i32.to_le_bytes());
                }
                if fill {
                    set_user_data(e, UD_CROP_MODE, &CROP_TO_FIT.to_le_bytes());
                }
                if e.get("paramList").and_then(Value::as_array).is_some_and(Vec::is_empty) {
                    remove_key(e, "paramList");
                }
            }
        }
    }

    fn patch_audio(
        &mut self,
        v: &mut Value,
        old: Option<&AudioSettings>,
        new: &AudioSettings,
        ty: i64,
        clip_id: &str,
        in_point: Ticks,
    ) {
        use props::*;
        let tb = TimeBase::seconds(in_point);
        if ty != CLIP_AUDIO {
            // Video clips' sound is carried by their linked audio clips, so
            // a muted video clip is what Filmora has anyway.
            // Noise reduction and channel routing live in the Jibaru sidecar.
            let only_mute =
                AudioSettings {
                    muted: false,
                    denoise_db: None,
                    channels: ChannelMode::Stereo,
                    voice_filter: None,
                    hum_removal: None,
                    hiss_removal: None,
                    ..new.clone()
                }
                    == AudioSettings::default();
            if !only_mute {
                self.warnings.push(format!("clip {clip_id}: audio settings only apply to audio clips in Filmora"));
            }
            return;
        }
        let ch = |f: &dyn Fn(&AudioSettings) -> String| old.is_none_or(|o| f(o) != f(new));
        if ch(&|a| format!("{:?}", a.gain_db)) {
            let e = ensure_effect(v, "BasicFollow", FX_VOLUME, "volume");
            set_effect_param_full(e, AU_GAIN, &new.gain_db, 1.0, 0.0, 2, tb);
            // Keyframes of older projects would apply on top.
            let legacy = v.get("volumeKeyframe").and_then(|k| str_of(k, "parameter")).and_then(KfParam::parse);
            if legacy.is_some_and(|k| !k.keys.is_empty()) {
                put(v, "volumeKeyframe", json!({ "parameter": KfParam::empty().to_text() }));
            }
        }
        if ch(&|a| format!("{:?}", a.pan)) {
            let e = ensure_effect(v, "BasicFollow", FX_VOLUME, "volume");
            set_effect_param_full(e, AU_BALANCE, &new.pan, 0.5, 0.5, 2, tb);
        }
        if old.is_none_or(|o| (o.fade_in, o.fade_out) != (new.fade_in, new.fade_out)) {
            let e = ensure_effect(v, "BasicFollow", FX_FADE, "fade");
            if old.is_none_or(|o| o.fade_in != new.fade_in) {
                set_effect_param(e, "FadeInTime", json!(ticks_to_secs(new.fade_in)), 2);
            }
            if old.is_none_or(|o| o.fade_out != new.fade_out) {
                set_effect_param(e, "FadeOutTime", json!(ticks_to_secs(new.fade_out)), 2);
            }
        }
        if ch(&|a| a.pitch.to_string()) && (new.pitch != 0.0 || import::find_effect(v, FX_PITCH).is_some()) {
            let e = ensure_effect(v, "BasicFollow", FX_PITCH, FX_PITCH_DISPLAY);
            set_effect_param(e, AU_PITCH, json!(new.pitch), 2);
        }
        if old.is_none_or(|o| o.muted != new.muted) {
            let e = ensure_effect(v, "BasicFollow", FX_VOLUME, "volume");
            if new.muted {
                set_effect_param(e, AU_MUTE, json!(true), 1);
            } else {
                remove_effect_param(e, AU_MUTE);
            }
        }
        if old.is_none_or(|o| o.normalize != new.normalize) {
            let e = ensure_effect(v, "BasicFollow", FX_VOLUME, "volume");
            match new.normalize {
                Some(n) => {
                    let gain = n.measured_lufs.map_or(1.0, |m| 10f64.powf((n.target_lufs - m) / 20.0));
                    set_effect_param(e, AU_LOUDNESS_GAIN, json!(gain), 3);
                    set_effect_param(e, AU_LOUDNESS_ON, json!(true), 1);
                    set_user_data(v, UD_LOUDNESS_TARGET, &(n.target_lufs.round() as i32).to_le_bytes());
                }
                None => remove_effect_param(e, AU_LOUDNESS_ON),
            }
        }
        if ch(&|a| format!("{:?}", a.equalizer)) {
            let e = ensure_effect(v, "BasicFollow", FX_EQUALIZER, "equalizer");
            let bands = new.equalizer.as_ref().map_or([0.0; 10], |q| q.bands);
            for (n, g) in EQ_BANDS.iter().zip(bands) {
                if g == 0.0 {
                    remove_effect_param(e, n);
                } else {
                    set_effect_param(e, n, json!(g), 2);
                }
            }
            if e.get("paramList").and_then(Value::as_array).is_some_and(Vec::is_empty) {
                remove_key(e, "paramList");
            }
        }
    }

    fn rebuild_effects(&mut self, v: &mut Value, old: Option<&Clip>, new: &Clip, in_point: Ticks) {
        let tb = props::TimeBase::seconds(in_point);
        let mut desired: Vec<(&EffectInstance, bool)> = new.effects.iter().map(|e| (e, false)).collect();
        desired.extend(new.animations.iter().filter(|a| is_legacy_animation(a)).map(|e| (e, true)));
        let old_inst = |id: &str| old.and_then(|o| o.effects.iter().chain(o.animations.iter()).find(|e| e.id == id));
        let mut used = vec![false; desired.len()];

        if let Some(chains) = v.get_mut("effectChainList").and_then(Value::as_array_mut) {
            for ch in chains.iter_mut() {
                let Some(list) = ch.get_mut("effectList").and_then(Value::as_array_mut) else { continue };
                let mut out = Vec::with_capacity(list.len());
                for e in list.drain(..) {
                    let id = str_of(&e, "id").unwrap_or("").to_string();
                    if !is_surfaced(&id, &e) {
                        out.push(e);
                        continue;
                    }
                    let uid = str_of(&e, "thisUId").map(str::to_string);
                    let found = desired.iter().position(|(d, _)| uid.as_deref() == Some(d.id.as_str()));
                    if let Some(i) = found {
                        if !used[i] {
                            used[i] = true;
                            let (inst, anim) = desired[i];
                            let mut e = e;
                            patch_effect(&mut e, old_inst(&inst.id), inst, anim, tb);
                            out.push(e);
                        }
                    }
                }
                *list = out;
            }
        }
        for (i, (inst, anim)) in desired.iter().enumerate() {
            if used[i] {
                continue;
            }
            let native = crate::sidecar::native(&inst.kind);
            let fid = match (inst.kind.strip_prefix("filmora:"), native) {
                (Some(f), _) => f.to_string(),
                (None, Some(n)) => n.1.to_string(),
                // Kept in the Jibaru sidecar (see `sidecar_doc`).
                (None, None) => continue,
            };
            let display =
                native.map_or_else(|| marker_str(&inst.extra, "display").unwrap_or(&fid), |n| n.2).to_string();
            let src = marker_str(&inst.extra, "uid").and_then(|u| self.orig_effects.get(u).copied());
            let mut e = match src {
                Some(s) if str_of(s, "id") == Some(fid.as_str()) => s.clone(),
                _ => new_effect_json(&fid, &display, &[]),
            };
            let uid = self.fresh_uid(Some(&inst.id));
            self.effect_uid.insert(inst.id.clone(), uid.clone());
            put(&mut e, "thisUId", json!(uid));
            set_user_data(&mut e, ud::GUID, &guid_blob(&new_guid()));
            set_store_record(&mut e, &inst.extra);
            let baseline = src.map(|s| import::effect_of(marker_str(&inst.extra, "chain").unwrap_or(""), s, in_point));
            patch_effect(&mut e, baseline.as_ref(), inst, *anim, tb);
            let chain = marker_str(&inst.extra, "chain")
                .or(native.map(|n| n.3))
                .map(str::to_string)
                .unwrap_or_else(|| "Effect".into());
            let ch = chain_mut(v, &chain, usize::MAX);
            if let Some(list) = ch.get_mut("effectList").and_then(Value::as_array_mut) {
                list.push(e);
            }
        }
    }

    fn transition_json(&mut self, existing: Option<&Value>, t: &Transition, clip_id: &str) -> Option<Value> {
        let (fid, display) = match t.kind.strip_prefix("filmora:") {
            Some(id) => (id.to_string(), marker_str(&t.extra, "display").unwrap_or(id).to_string()),
            None => match KNOWN_TRANSITIONS.iter().find(|(k, _, _)| *k == t.kind) {
                Some((_, id, name)) => (id.to_string(), name.to_string()),
                None => {
                    self.warnings
                        .push(format!("clip {clip_id}: transition {} has no Filmora equivalent, not written", t.kind));
                    return None;
                }
            },
        };
        let uid = marker_str(&t.extra, "uid");
        let mut tj = match existing.filter(|e| str_of(e, "thisUId") == uid && uid.is_some()) {
            Some(e) => e.clone(),
            None => {
                let mut tj = match uid.and_then(|u| self.orig_transitions.get(u).copied()) {
                    Some(src) => src.clone(),
                    None => json!({
                        "display": display, "id": fid, "placeholder": false, "placeholderStatus": 0,
                        "thisUId": "", "tlBegin": 0, "tlEnd": 0, "type": 5, "userData": []
                    }),
                };
                let u = self.fresh_uid(Some(&t.id));
                put(&mut tj, "thisUId", json!(u));
                set_user_data(&mut tj, ud::GUID, &guid_blob(&new_guid()));
                tj
            }
        };
        if str_of(&tj, "id") != Some(fid.as_str()) {
            put(&mut tj, "id", json!(fid));
            put(&mut tj, "display", json!(display));
            if let Some(list) = tj.get_mut("userData").and_then(Value::as_array_mut) {
                list.retain(|u| u.get("key").and_then(Value::as_i64) != Some(ud::RESOURCE));
            }
        }
        set_store_record(&mut tj, &t.extra);
        Some(tj)
    }

    fn patch_post_transition(&mut self, v: &mut Value, old: Option<&Clip>, new: &Clip) {
        let Some(t) = &new.transition_out else {
            if let Some(o) = v.as_object_mut() {
                o.remove("postTransition");
            }
            return;
        };
        let existing = v.get("postTransition").cloned();
        let Some(mut tj) = self.transition_json(existing.as_ref(), t, &new.id) else {
            if let Some(o) = v.as_object_mut() {
                o.remove("postTransition");
            }
            return;
        };
        let old_t = old.and_then(|o| o.transition_out.as_ref()).filter(|o| o.id == t.id);
        let same_shape = old_t.is_some_and(|o| o.duration == t.duration && o.alignment == t.alignment);
        let was_here = existing.as_ref().and_then(|e| str_of(e, "thisUId")) == str_of(&tj, "thisUId");
        if same_shape && was_here {
            let shift = new.end - old.map_or(new.end, |o| o.end);
            if shift != 0 {
                for k in ["tlBegin", "tlEnd"] {
                    let x = i64_of(&tj, k).unwrap_or(0) + shift;
                    put(&mut tj, k, json!(x));
                }
            }
        } else {
            let (a, b) = transition_window(t, new.end);
            put(&mut tj, "tlBegin", json!(a));
            put(&mut tj, "tlEnd", json!(b));
        }
        put(v, "postTransition", tj);
    }

    fn patch_pre_transition(&mut self, v: &mut Value, old: Option<&Clip>, new: &Clip, pre: Option<EffectInstance>) {
        let Some(a) = pre else {
            if let Some(o) = v.as_object_mut() {
                o.remove("preTransition");
            }
            return;
        };
        let t = Transition {
            id: a.id.clone(),
            kind: a.kind.clone(),
            duration: a.range.map_or(0, |(x, y)| y - x),
            alignment: TransitionAlignment::StartAtCut,
            params: a.params.clone(),
            extra: a.extra.clone(),
        };
        let existing = v.get("preTransition").cloned();
        let Some(mut tj) = self.transition_json(existing.as_ref(), &t, &new.id) else { return };
        let (x, y) = a.range.unwrap_or((0, 0));
        put(&mut tj, "tlBegin", json!(new.start + x));
        put(&mut tj, "tlEnd", json!(new.start + y));
        let _ = old;
        put(v, "preTransition", tj);
    }
}

fn is_pre_transition(a: &EffectInstance) -> bool {
    marker_str(&a.extra, "role") == Some("preTransition")
}

/// Animations of pre-15 projects: effects in an "Animation" chain.
fn is_legacy_animation(a: &EffectInstance) -> bool {
    !is_pre_transition(a) && marker_str(&a.extra, "chain").is_some()
}

/// Entry/exit animations written as Filmora 15 Motion presets.
fn is_motion(a: &EffectInstance) -> bool {
    !is_pre_transition(a)
        && !is_legacy_animation(a)
        && matches!(a.placement, Some(AnimationPlacement::In | AnimationPlacement::Out))
}

/// Speed ramp keys as Filmora writes them: code 9 with equal left and right
/// derivatives (the curve's slope at the key).
fn ramp_keys(points: Vec<(f64, f64)>) -> Vec<Kf> {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i.saturating_sub(1)], points[(i + 1).min(n - 1)]);
            let d = if b.0 > a.0 { (b.1 - a.1) / (b.0 - a.0) } else { 0.0 };
            let mut k = Kf::new(points[i].0, points[i].1);
            k.interpolation = props::KF_RAMP;
            k.left_derivative = Some(d);
            k.right_derivative = Some(d);
            k
        })
        .collect()
}

fn set_flag(v: &mut Value, key: &str, on: bool) {
    if on {
        put(v, key, json!(true));
    } else {
        remove_key(v, key);
    }
}

/// Writes the fields of `new` that differ from `old` into an effect object,
/// including parameter keyframes (`paramMapList`) and, for legacy
/// animations, the slot (`AnimationScene`) and range (`inPoint`/`outPoint`).
fn patch_effect(e: &mut Value, old: Option<&EffectInstance>, new: &EffectInstance, anim: bool, tb: props::TimeBase) {
    if let Some(fid) = new.kind.strip_prefix("filmora:") {
        if str_of(e, "id") != Some(fid) {
            put(e, "id", json!(fid));
        }
    }
    if old.is_none_or(|o| o.enabled != new.enabled) {
        if new.enabled {
            remove_key(e, "enable");
        } else {
            put(e, "enable", json!(false));
        }
    }
    for (name, p) in &new.params {
        let prev = old.and_then(|o| o.params.get(name));
        if prev == Some(p) {
            continue;
        }
        let (val, ty) = match &p.value {
            jb_core::Value::Float(f) => (json!(f), 3),
            jb_core::Value::Bool(b) => (json!(*b as i64), 5),
            jb_core::Value::Text(s) => (json!(s), 6),
            jb_core::Value::Vec2(v) => (json!(v), 3),
            jb_core::Value::Color(c) => (json!(color_to_int(*c).0), 5),
        };
        set_effect_param(e, name, val, ty);
        if !p.keyframes.is_empty() || prev.is_some_and(|o| !o.keyframes.is_empty()) {
            let text = (!p.keyframes.is_empty()).then(|| props::kf_param(p, 1.0, 0.0, tb).to_text());
            props::set_param_map(e, name, text);
        }
    }
    if let Some(o) = old {
        for name in o.params.keys().filter(|k| !new.params.contains_key(*k)) {
            remove_effect_param(e, name);
            props::set_param_map(e, name, None);
        }
    }
    if anim && old.is_none_or(|o| o.range != new.range) {
        if let Some((a, b)) = new.range {
            put(e, "inPoint", json!(a));
            put(e, "outPoint", json!(b));
        }
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Writes `extra.filmora.resource` (Filmora's store record: `id`, `name`
/// slug, `res_type`, ...) as userData 12. A fresh Filmora session resolves a
/// store transition or effect GUID only through this record; without it the
/// item is dropped unless it was already loaded earlier in the session.
fn set_store_record(v: &mut Value, extra: &Extra) {
    if let Some(res) = extra.get("filmora").and_then(|f| f.get("resource")).filter(|r| r.is_object()) {
        set_user_data(v, ud::RESOURCE, res.to_string().as_bytes());
    }
}

fn new_effect_json(id: &str, display: &str, params: &[(&str, Value)]) -> Value {
    let mut e = json!({
        "display": display, "id": id, "placeholder": false, "placeholderStatus": 0,
        "thisUId": new_uid(), "type": 3,
        "userData": [{ "data": "AQ==", "key": 7, "size": 1 }]
    });
    set_user_data(&mut e, ud::GUID, &guid_blob(&new_guid()));
    for (n, v) in params {
        set_effect_param(&mut e, n, v.clone(), 3);
    }
    e
}

fn transform_effect_mut(clip: &mut Value) -> Option<&mut Value> {
    clip.get_mut("effectChainList")?
        .as_array_mut()?
        .iter_mut()
        .flat_map(|ch| ch.get_mut("effectList").and_then(Value::as_array_mut).into_iter().flatten())
        .find(|e| str_of(e, "id") == Some(FX_TRANSFORM))
}

/// The chain named `name`, created at `index` (or the end) when missing.
fn chain_mut<'v>(clip: &'v mut Value, name: &str, index: usize) -> &'v mut Value {
    if clip.get("effectChainList").is_none_or(|c| !c.is_array()) {
        put(clip, "effectChainList", json!([]));
    }
    let chains = clip["effectChainList"].as_array_mut().expect("array");
    let pos = match chains.iter().position(|c| str_of(c, "name") == Some(name)) {
        Some(p) => p,
        None => {
            let at = index.min(chains.len());
            chains.insert(at, json!({ "effectList": [], "name": name }));
            at
        }
    };
    &mut chains[pos]
}

fn ensure_effect<'v>(clip: &'v mut Value, chain: &str, id: &str, display: &str) -> &'v mut Value {
    let has = clip
        .get("effectChainList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|c| c.get("effectList").and_then(Value::as_array).into_iter().flatten())
        .any(|e| str_of(e, "id") == Some(id));
    if !has {
        let at = match chain {
            "BasicFollow" => clip
                .get("effectChainList")
                .and_then(Value::as_array)
                .and_then(|c| c.iter().position(|c| str_of(c, "name") == Some("Basic")))
                .map_or(0, |i| i + 1),
            _ => 0,
        };
        let ch = chain_mut(clip, chain, at);
        ch["effectList"].as_array_mut().expect("array").push(new_effect_json(id, display, &[]));
    }
    clip["effectChainList"]
        .as_array_mut()
        .expect("array")
        .iter_mut()
        .flat_map(|c| c["effectList"].as_array_mut().into_iter().flatten())
        .find(|e| str_of(e, "id") == Some(id))
        .expect("effect exists")
}

fn base_clip_fields(ty: i64) -> Value {
    json!({
        "inPoint": 0, "outPoint": 0, "placeholder": false, "placeholderStatus": 0,
        "thisUId": "", "tlBegin": 0, "tlEnd": 0, "type": ty, "userData": []
    })
}

fn merge(mut a: Value, b: Value) -> Value {
    if let Value::Object(o) = b {
        for (k, v) in o {
            put(&mut a, &k, v);
        }
    }
    a
}

fn video_chains() -> Value {
    json!([
        { "effectList": [new_effect_json(FX_CROP, "crop-pan-zoom", &[]),
            new_effect_json(FX_TRANSFORM, "transform", &[("EnableTransform", json!(1))])], "name": "Basic" },
        { "effectList": [], "name": "Effect" }
    ])
}

fn video_clip_template() -> Value {
    merge(
        base_clip_fields(CLIP_VIDEO),
        json!({
            "backgroundFillBluredness": 20,
            "effectChainList": video_chains(),
            "filename": "",
            "picSequence": false,
            "pipBuf": DEFAULT_PIP,
            "pipBufSize": DEFAULT_PIP.len() + 1,
            "sourceUuid": "",
            "speed": { "offset": 0.0, "offsetEnd": 0.0, "reverse": false, "speedParam": "" },
            "streamId": 0
        }),
    )
}

fn audio_clip_template() -> Value {
    let empty = KfParam::empty().to_text();
    let fx = |id: &str, display: &str| new_effect_json(id, display, &[]);
    merge(
        base_clip_fields(CLIP_AUDIO),
        json!({
            "audioDuckingframe": { "parameter": empty },
            "denoiseV3Strength": 50.0,
            "effectChainList": [{ "effectList": [
                fx("audio/effect/clip_volume", "clip_volume"),
                fx("audio/effect/change_channel", "change_channel"),
                fx(FX_VOLUME, "volume"),
                fx("audio/effect/ducking", "ducking"),
                fx(FX_FADE, "fade"),
                fx("audio/effect/equalizer", "equalizer")
            ], "name": "BasicFollow" }],
            "filename": "",
            "sourceUuid": "",
            "speed": { "offset": 0.0, "offsetEnd": 0.0, "reverse": false, "speedParam": "" },
            "streamId": 0,
            "volumeKeyframe": { "parameter": empty }
        }),
    )
}

fn timeline_ref_template() -> Value {
    let mut v = merge(
        base_clip_fields(CLIP_TIMELINE),
        json!({
            "effectChainList": [
                video_chains()[0].clone(), { "effectList": [], "name": "Effect" },
                { "effectList": [], "name": "_wes_hold_effectchain_" }
            ],
            "isAudio": 0,
            "keyframeScaleMode": 1,
            "pipBuf": DEFAULT_PIP,
            "pipBufSize": DEFAULT_PIP.len() + 1,
            "timelineId": 0
        }),
    );
    set_user_data(&mut v, ud::KIND, &12i32.to_le_bytes());
    set_user_data(&mut v, ud::NAME, b"Title");
    v
}

fn text_clip_template(dur: Ticks) -> Value {
    let script = default_script();
    let mut v = merge(
        base_clip_fields(CLIP_TEXT),
        json!({
            "effectChainList": [
                { "effectList": [new_effect_json(FX_CROP, "crop-pan-zoom", &[]),
                    new_effect_json(FX_TRANSFORM, "transform", &[("Scale_y", json!(6.666666507720947)), ("EnableTransform", json!(1))])],
                  "name": "Basic" },
                { "effectList": [], "name": "Effect" },
                { "effectList": [], "name": "_wes_hold_effectchain_" }
            ],
            "pipBuf": DEFAULT_PIP,
            "pipBufSize": DEFAULT_PIP.len() + 1
        }),
    );
    set_nested(&mut v, "scriptBuf", &script);
    put(&mut v, "outPoint", json!(dur));
    put(&mut v, "tlEnd", json!(dur));
    set_user_data(&mut v, ud::KIND, &5i32.to_le_bytes());
    set_user_data(&mut v, ud::NAME, b"Layer 01");
    set_user_data(&mut v, ud::GUID, &guid_blob(&new_guid()));
    v
}

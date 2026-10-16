//! `.wfp` → `jb_core::Project`.

use crate::archive::Entry;
use crate::json::*;
use crate::mapping::*;
use crate::title::title_from_script;
use crate::{parse_json, Error, Raw, Result, WfpProject, MEDIAS_INFO, PROJECT_INFO};
use jb_core::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

pub fn import(entries: Vec<Entry>) -> Result<WfpProject> {
    let info_entry = entries
        .iter()
        .find(|e| e.name == PROJECT_INFO)
        .or_else(|| entries.iter().find(|e| e.name.ends_with("project_info.json")))
        .ok_or_else(|| Error::Format("missing project_info.json".into()))?;
    let project_info = parse_json(&info_entry.name, &info_entry.data)?;

    let timeline_entry = match str_of(&project_info, "timeline_mediaId") {
        Some(id) => entries.iter().find(|e| e.name.ends_with(&format!("{id}/timeline.wesproj"))),
        None => None,
    }
    .or_else(|| entries.iter().find(|e| e.name.ends_with("timeline.wesproj")))
    .ok_or_else(|| Error::Format("missing timeline.wesproj".into()))?;
    let timeline_path = timeline_entry.name.clone();
    let timeline = parse_json(&timeline_path, &timeline_entry.data)?;

    let medias_info = match entries.iter().find(|e| e.name == MEDIAS_INFO) {
        Some(e) => Some(parse_json(&e.name, &e.data)?),
        None => None,
    };

    let mut ctx = Ctx::new(&timeline, medias_info.as_ref());
    let project = ctx.project(&project_info)?;
    let warnings = std::mem::take(&mut ctx.warnings);
    let raw = Raw { entries, project_info, timeline_path, timeline, medias_info, baseline: project.clone() };
    Ok(WfpProject { project, raw, warnings })
}

pub fn sequence_id(timeline_id: i64) -> Id {
    format!("timeline-{timeline_id}")
}

struct Ctx<'a> {
    doc: &'a Value,
    timelines: HashMap<i64, &'a Value>,
    resources: HashMap<String, &'a Value>,
    media: Vec<MediaAsset>,
    media_by_path: HashMap<String, Id>,
    link_counts: HashMap<String, usize>,
    used_ids: HashSet<String>,
    used_tracks: HashSet<String>,
    warnings: Vec<String>,
}

impl<'a> Ctx<'a> {
    fn new(doc: &'a Value, medias_info: Option<&'a Value>) -> Self {
        let mut ctx = Ctx {
            doc,
            timelines: HashMap::new(),
            resources: HashMap::new(),
            media: Vec::new(),
            media_by_path: HashMap::new(),
            link_counts: HashMap::new(),
            used_ids: HashSet::new(),
            used_tracks: HashSet::new(),
            warnings: Vec::new(),
        };
        for tl in timelines(doc) {
            if let Some(id) = i64_of(tl, "timelineId") {
                ctx.timelines.insert(id, tl);
            }
            for c in tl_clips(tl) {
                if let Some(g) = user_data_str(c, ud::GUID) {
                    *ctx.link_counts.entry(g).or_default() += 1;
                }
            }
        }
        for r in doc.get("resources").and_then(Value::as_array).into_iter().flatten() {
            if let Some(url) = str_of(r, "filename") {
                ctx.resources.insert(url.to_string(), r);
            }
        }
        if let Some(items) = medias_info.and_then(|m| m.get("media_items")).and_then(Value::as_object) {
            for (id, item) in items {
                let Some(path) = str_of(item, "download_url").filter(|p| !p.is_empty()) else { continue };
                let media_type = i64_of(item, "media_type").unwrap_or(0);
                if media_type == 1_048_576 {
                    continue; // the timeline itself
                }
                let resource = ctx.resources.get(&path_to_url(path)).copied();
                let asset = media_asset(id, item, path, media_type, resource);
                ctx.media_by_path.insert(path.to_string(), asset.id.clone());
                ctx.media.push(asset);
            }
        }
        ctx
    }

    fn project(&mut self, info: &Value) -> Result<Project> {
        let res = info.get("project_timeline_resolution");
        let fps = info.get("project_timeline_framerate");
        let main_tl = timelines(self.doc)
            .find(|t| i64_of(t, "type") == Some(0))
            .or_else(|| {
                let cur = i64_of(self.doc, "currentTimelineId")?;
                self.timelines.get(&cur).copied()
            })
            .or_else(|| timelines(self.doc).next())
            .ok_or_else(|| Error::Format("timeline has no timelineInfos".into()))?;
        let first = |v: Option<&Value>, i: usize| v.and_then(|a| a.get(i)).and_then(Value::as_i64);
        let settings = ProjectSettings {
            width: first(res, 0).or_else(|| i64_of(main_tl, "resolutionWidth")).unwrap_or(1920) as u32,
            height: first(res, 1).or_else(|| i64_of(main_tl, "resolutionHeight")).unwrap_or(1080) as u32,
            frame_rate: Rational::new(first(fps, 0).unwrap_or(30), first(fps, 1).unwrap_or(1).max(1)),
            sample_rate: i64_of(info, "project_sample_rate").unwrap_or(48_000) as u32,
            background: [0.0, 0.0, 0.0, 1.0],
        };
        let mut sequences = Vec::new();
        for tl in timelines(self.doc) {
            let main = std::ptr::eq(tl, main_tl);
            sequences.push(self.sequence(tl, main));
        }
        let main_id = i64_of(main_tl, "timelineId").unwrap_or(0);
        Ok(Project {
            id: str_of(info, "project_guid").map_or_else(new_id, str::to_string),
            name: str_of(info, "project_file_name").unwrap_or("Untitled").to_string(),
            settings,
            media: std::mem::take(&mut self.media),
            sequences,
            main_sequence: sequence_id(main_id),
            extra: Extra::Null,
        })
    }

    fn sequence(&mut self, tl: &Value, main: bool) -> Sequence {
        let tl_id = i64_of(tl, "timelineId").unwrap_or(0);
        let mut tracks = Vec::new();
        let (mut nv, mut na) = (0, 0);
        for (index, t) in tl.get("trackInfos").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let kind = if i64_of(t, "trackType") == Some(TRACK_AUDIO) { TrackKind::Audio } else { TrackKind::Video };
            let name = match kind {
                TrackKind::Video => {
                    nv += 1;
                    format!("Video {nv}")
                }
                TrackKind::Audio => {
                    na += 1;
                    format!("Audio {na}")
                }
            };
            let uuid = str_of(t, "uuid").map_or_else(new_id, str::to_string);
            let mut id = uuid.clone();
            if !self.used_tracks.insert(id.clone()) {
                id = format!("{uuid}@{tl_id}.{index}");
                self.used_tracks.insert(id.clone());
            }
            let mut clips: Vec<Clip> = t
                .get("clipList")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|c| self.clip(c, tl_id))
                .collect();
            clips.sort_by_key(|c| c.start);
            tracks.push(Track {
                id,
                kind,
                name,
                locked: false,
                muted: false,
                hidden: false,
                clips,
                extra: marker(json!({ "timeline": tl_id, "index": index })),
            });
        }
        Sequence {
            id: sequence_id(tl_id),
            name: if main { "Main".into() } else { format!("Timeline {tl_id}") },
            tracks,
            extra: marker(json!({ "timelineId": tl_id })),
        }
    }

    fn unique_id(&mut self, uid: Option<&str>) -> Id {
        let id = uid.map_or_else(new_id, str::to_string);
        if self.used_ids.insert(id.clone()) {
            return id;
        }
        let id = new_id();
        self.used_ids.insert(id.clone());
        id
    }

    fn clip(&mut self, c: &Value, tl_id: i64) -> Clip {
        let ty = i64_of(c, "type").unwrap_or(0);
        let uid = str_of(c, "thisUId");
        let id = self.unique_id(uid);
        let start = i64_of(c, "tlBegin").unwrap_or(0);
        let end = i64_of(c, "tlEnd").unwrap_or(start);
        let url = str_of(c, "filename").unwrap_or("");
        let path = url_to_path(url).or_else(|| resource_path(url));
        let name = user_data_str(c, ud::NAME).filter(|n| !n.is_empty()).or_else(|| path.as_deref().map(file_stem));

        let source = match ty {
            CLIP_VIDEO | CLIP_AUDIO if path.is_some() => {
                ClipSource::Media { media_id: self.media_for_clip(c, path.as_deref().unwrap_or_default(), url) }
            }
            CLIP_TIMELINE => match i64_of(c, "timelineId") {
                Some(sub) if self.timelines.contains_key(&sub) => {
                    ClipSource::Sequence { sequence_id: sequence_id(sub) }
                }
                _ => {
                    self.warnings.push(format!("clip {id}: nested timeline missing, kept as placeholder"));
                    ClipSource::Color { color: [0.0; 4] }
                }
            },
            CLIP_TEXT => {
                let script = nested(c, "scriptBuf").unwrap_or(Value::Null);
                let sy = find_effect(c, FX_TRANSFORM).and_then(|e| effect_param_f64(e, "Scale_y"));
                ClipSource::Title(title_from_script(&script, sy))
            }
            _ => {
                self.warnings.push(format!(
                    "clip {id} (type {ty}, timeline {tl_id}) has no Jibaru equivalent; kept as transparent placeholder"
                ));
                ClipSource::Color { color: [0.0; 4] }
            }
        };

        let mut clip = Clip {
            id,
            name: name.unwrap_or_else(|| format!("Clip type {ty}")),
            source,
            start,
            end,
            source_in: i64_of(c, "inPoint").unwrap_or(0),
            speed: Speed::default(),
            transform: Transform::default(),
            opacity: Param::float(1.0),
            blend_mode: BlendMode::Normal,
            audio: AudioSettings::default(),
            effects: Vec::new(),
            animations: Vec::new(),
            transition_out: None,
            link_group: None,
            disabled: false,
            extra: marker(json!({ "uid": uid, "type": ty })),
        };
        let (source_in, speed) = speed_of(c, end - start);
        clip.source_in = source_in;
        clip.speed = speed;
        if ty != CLIP_TEXT {
            clip.transform = transform_of(c);
        }
        let (opacity, blend) = opacity_of(c);
        clip.opacity = opacity;
        if blend != 0 {
            self.warnings.push(format!("clip {}: blend mode {blend} not mapped, rendered as normal", clip.id));
        }
        clip.audio = audio_of(c);
        let (effects, animations) = effects_of(c);
        clip.effects = effects;
        clip.animations = animations;
        if let Some(pre) = c.get("preTransition").filter(|v| v.is_object()) {
            clip.animations.insert(0, pre_transition_of(pre, start));
        }
        clip.transition_out = c.get("postTransition").filter(|v| v.is_object()).map(|t| transition_of(t, end));
        if let Some(g) = user_data_str(c, ud::GUID) {
            if self.link_counts.get(&g).copied().unwrap_or(0) > 1 {
                clip.link_group = Some(g);
            }
        }
        clip
    }

    fn media_for_clip(&mut self, c: &Value, path: &str, url: &str) -> Id {
        if let Some(item) = user_data_str(c, ud::MEDIA_ITEM) {
            if self.media.iter().any(|m| m.id == item && m.path == path) {
                return item;
            }
        }
        if let Some(id) = self.media_by_path.get(path) {
            return id.clone();
        }
        // Media used on the timeline but absent from the media bin.
        let resource = self.resources.get(url).copied();
        let id = to_guid(str_of(c, "sourceUuid").unwrap_or(path));
        let item = json!({ "name": file_stem(path) });
        let mut asset = media_asset(&id, &item, path, 0, resource);
        asset.extra = marker(json!({ "mediaId": null, "sourceUuid": str_of(c, "sourceUuid"), "filename": url }));
        self.media_by_path.insert(path.to_string(), id.clone());
        self.media.push(asset);
        id
    }
}

/// Store resources (sound effects, stickers) are referenced relative to the
/// resource download folder, e.g. `6_Cartoon_Whoosh_02/Data/x.m4a`. Resolves
/// them against the local Filmora installation when present.
fn resource_path(rel: &str) -> Option<String> {
    if rel.is_empty() || rel.contains("://") {
        return None;
    }
    for root in crate::ResourceIndex::default_roots() {
        let Ok(cats) = std::fs::read_dir(&root) else { continue };
        for cat in cats.flatten() {
            let candidate = cat.path().join(rel);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    Some(rel.to_string())
}

pub fn timelines(doc: &Value) -> impl Iterator<Item = &Value> {
    doc.get("timelineInfos").and_then(Value::as_array).into_iter().flatten()
}

pub fn tl_clips(tl: &Value) -> impl Iterator<Item = &Value> {
    tl.get("trackInfos")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|t| t.get("clipList").and_then(Value::as_array).into_iter().flatten())
}

pub fn all_effects(c: &Value) -> impl Iterator<Item = (&str, &Value)> {
    c.get("effectChainList").and_then(Value::as_array).into_iter().flatten().flat_map(|ch| {
        let name = str_of(ch, "name").unwrap_or("");
        ch.get("effectList").and_then(Value::as_array).into_iter().flatten().map(move |e| (name, e))
    })
}

pub fn find_effect<'v>(c: &'v Value, id: &str) -> Option<&'v Value> {
    all_effects(c).map(|(_, e)| e).find(|e| str_of(e, "id") == Some(id))
}

fn media_asset(id: &str, item: &Value, path: &str, media_type: i64, resource: Option<&Value>) -> MediaAsset {
    let vid = resource.and_then(|r| r.get("vidStreamInfo")).and_then(|v| v.get(0));
    let width = vid.and_then(|v| i64_of(v, "width")).unwrap_or(0) as u32;
    let height = vid.and_then(|v| i64_of(v, "height")).unwrap_or(0) as u32;
    let stream_type = resource.and_then(|r| i64_of(r, "streamType")).unwrap_or(0);
    let kind = match media_type {
        8 => MediaKind::Video,
        16 => MediaKind::Image,
        262_144 => MediaKind::Audio,
        _ => match stream_type {
            5 => MediaKind::Image,
            3 => MediaKind::Audio,
            2 if width > 0 => MediaKind::Video,
            2 => MediaKind::Audio,
            _ => guess_kind(path),
        },
    };
    let frame_rate = vid
        .and_then(|v| v.get("frameRate"))
        .and_then(|f| Some(Rational::new(i64_of(f, "num")?, i64_of(f, "den")?)))
        .filter(|r| r.num > 1 && r.den > 0 && kind == MediaKind::Video);
    let duration = i64_of(item, "media_length")
        .filter(|d| *d > 0)
        .or_else(|| resource.and_then(|r| i64_of(r, "mediaLength")))
        .unwrap_or(0);
    MediaAsset {
        id: id.to_string(),
        name: str_of(item, "name").map_or_else(|| file_stem(path), str::to_string),
        path: path.to_string(),
        kind,
        duration,
        width,
        height,
        frame_rate,
        has_audio: kind == MediaKind::Audio || resource.and_then(|r| i64_of(r, "audioStreamCount")).unwrap_or(0) > 0,
        extra: marker(json!({ "mediaId": id })),
    }
}

fn guess_kind(path: &str) -> MediaKind {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "heic" | "tif" | "tiff" => MediaKind::Image,
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" | "aif" | "aiff" => MediaKind::Audio,
        _ => MediaKind::Video,
    }
}

/// Source in-point and speed. `speed.offset/offsetEnd` are true source
/// seconds while `inPoint/outPoint` are in the retimed source timebase
/// (`inPoint = offset / rate`). `speedParam` stores the rate curve over the
/// whole media; a uniform rate `r` is written as two keys `r → 1.0` spanning
/// `_totalTime` (Filmora's own convention), so only longer curves are ramps.
pub fn speed_of(c: &Value, dur: Ticks) -> (Ticks, Speed) {
    let mut speed = Speed::default();
    let Some(sp) = c.get("speed").filter(|v| v.is_object()) else {
        return (i64_of(c, "inPoint").unwrap_or(0), speed);
    };
    let offset = f64_of(sp, "offset").unwrap_or(0.0);
    let offset_end = f64_of(sp, "offsetEnd").unwrap_or(offset);
    let source_in = secs_to_ticks(offset);
    speed.reverse = sp.get("reverse").and_then(Value::as_bool).unwrap_or(false);
    let ratio = if dur > 0 { (offset_end - offset) * TICKS_PER_SECOND as f64 / dur as f64 } else { 0.0 };
    let kf = str_of(sp, "speedParam").and_then(KfParam::parse);
    let keys = kf.as_ref().map(|k| k.keys.as_slice()).unwrap_or_default();
    let uniform = keys.len() <= 2 && keys.last().is_none_or(|k| k.value == 1.0 || keys.len() == 1);
    let all_equal = keys.windows(2).all(|w| w[0].value == w[1].value);
    speed.rate = if ratio <= 0.0 {
        1.0
    } else if let (true, Some(k)) = (uniform, keys.first()) {
        k.value
    } else if all_equal {
        keys.first().map_or(ratio, |k| k.value)
    } else {
        // A real speed curve: keyframe times are source seconds; map them to
        // clip-local time through the clip's average rate (approximation).
        let avg = ratio.max(1e-6);
        let ramp: Vec<Keyframe> = keys
            .iter()
            .map(|k| Keyframe {
                time: ((secs_to_ticks(k.time) - source_in) as f64 / avg).round() as Ticks,
                value: jb_core::Value::Float(k.value),
                interpolation: Interpolation::Linear,
            })
            .collect();
        speed.ramp = Some(Param { value: jb_core::Value::Float(avg), keyframes: ramp });
        avg
    };
    if speed.rate <= 0.0 {
        speed.rate = 1.0;
    }
    (source_in, speed)
}

pub fn transform_of(c: &Value) -> Transform {
    let mut t = Transform::default();
    let Some(e) = find_effect(c, FX_TRANSFORM) else { return t };
    let p = |n: &str| effect_param_f64(e, n);
    if let Some(x) = p("Position_x") {
        t.position_x = Param::float(x - 0.5);
    }
    if let Some(y) = p("Position_y") {
        t.position_y = Param::float(y - 0.5);
    }
    let sx = p("Scale_x");
    let sy = p("Scale_y").or(sx);
    if let Some(sx) = sx {
        t.scale_x = Param::float(sx / 100.0);
    }
    if let Some(sy) = sy {
        t.scale_y = Param::float(sy / 100.0);
    }
    if let Some(r) = ROTATION_PARAMS.iter().find_map(|n| p(n)) {
        t.rotation = Param::float(r);
    }
    t
}

/// Candidate names of the rotation parameter (not present in the corpus).
pub const ROTATION_PARAMS: &[&str] = &["Rotation", "Rotate", "Rotate_z"];

fn keyframes_of(text: Option<&str>, scale: f64) -> Vec<Keyframe> {
    text.and_then(KfParam::parse)
        .map(|k| {
            k.keys
                .iter()
                .map(|k| Keyframe {
                    time: secs_to_ticks(k.time),
                    value: jb_core::Value::Float(k.value * scale),
                    interpolation: Interpolation::Linear,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Opacity (0..1) and the raw blend mode from `pipBuf`.
pub fn opacity_of(c: &Value) -> (Param, i64) {
    let Some(pip) = nested(c, "pipBuf") else { return (Param::float(1.0), 0) };
    let value = f64_of(&pip, "Opacity").unwrap_or(100.0) / 100.0;
    let keyframes = keyframes_of(str_of(&pip, "OpacityKeyFrame"), 0.01);
    (Param { value: jb_core::Value::Float(value), keyframes }, i64_of(&pip, "BlendMode").unwrap_or(0))
}

pub fn audio_of(c: &Value) -> AudioSettings {
    let mut a = AudioSettings::default();
    if let Some(e) = find_effect(c, FX_VOLUME) {
        if let Some(g) = effect_param_f64(e, "VolumeGain") {
            a.gain_db = Param::float(g);
        }
    }
    a.gain_db.keyframes =
        keyframes_of(c.get("volumeKeyframe").and_then(|v| v.get("parameter")).and_then(Value::as_str), 1.0);
    if let Some(e) = find_effect(c, FX_FADE) {
        a.fade_in = secs_to_ticks(effect_param_f64(e, "FadeInTime").unwrap_or(0.0));
        a.fade_out = secs_to_ticks(effect_param_f64(e, "FadeOutTime").unwrap_or(0.0));
    }
    a
}

/// Effects surfaced in the model: everything except the ones mapped into
/// core clip fields and parameterless structural defaults.
pub fn is_surfaced(id: &str, effect: &Value) -> bool {
    if matches!(id, FX_TRANSFORM | FX_VOLUME | FX_FADE) {
        return false;
    }
    let has_params = effect.get("paramList").and_then(Value::as_array).is_some_and(|l| !l.is_empty());
    !(STRUCTURAL_EFFECTS.contains(&id) && !has_params)
}

pub fn is_animation_chain(chain: &str) -> bool {
    chain.to_ascii_lowercase().contains("anim")
}

pub fn effect_kind(id: &str) -> String {
    format!("filmora:{id}")
}

pub fn effect_of(chain: &str, e: &Value) -> EffectInstance {
    let id = str_of(e, "id").unwrap_or("");
    EffectInstance {
        id: str_of(e, "thisUId").map_or_else(new_id, str::to_string),
        kind: effect_kind(id),
        enabled: e.get("enable").and_then(Value::as_bool).unwrap_or(true),
        params: params_of(e),
        range: None,
        extra: marker(json!({ "uid": str_of(e, "thisUId"), "chain": chain, "display": str_of(e, "display") })),
    }
}

pub fn params_of(e: &Value) -> BTreeMap<String, Param> {
    let mut out = BTreeMap::new();
    for p in e.get("paramList").and_then(Value::as_array).into_iter().flatten() {
        let (Some(name), Some(fx)) = (str_of(p, "name"), p.get("fxParam")) else { continue };
        let value = match fx.get("unValue") {
            Some(Value::String(s)) => jb_core::Value::Text(s.clone()),
            Some(Value::Bool(b)) => jb_core::Value::Bool(*b),
            Some(v) if v.is_number() => jb_core::Value::Float(v.as_f64().unwrap_or(0.0)),
            _ => continue,
        };
        out.insert(name.to_string(), Param::constant(value));
    }
    out
}

fn effects_of(c: &Value) -> (Vec<EffectInstance>, Vec<EffectInstance>) {
    let (mut effects, mut animations) = (Vec::new(), Vec::new());
    for (chain, e) in all_effects(c) {
        let id = str_of(e, "id").unwrap_or("");
        if !is_surfaced(id, e) {
            continue;
        }
        let inst = effect_of(chain, e);
        if is_animation_chain(chain) {
            animations.push(inst);
        } else {
            effects.push(inst);
        }
    }
    (effects, animations)
}

/// Human name of a transition/effect from its store resource record.
pub fn resource_name(v: &Value) -> Option<String> {
    let text = user_data_str(v, ud::RESOURCE)?;
    let j: Value = serde_json::from_str(&text).ok()?;
    str_of(&j, "name").map(str::to_string)
}

pub fn transition_of(t: &Value, cut: Ticks) -> Transition {
    let begin = i64_of(t, "tlBegin").unwrap_or(cut);
    let end = i64_of(t, "tlEnd").unwrap_or(cut);
    let alignment = if end == cut && begin < cut {
        TransitionAlignment::EndAtCut
    } else if begin == cut && end > cut {
        TransitionAlignment::StartAtCut
    } else {
        TransitionAlignment::Center
    };
    Transition {
        id: str_of(t, "thisUId").map_or_else(new_id, str::to_string),
        kind: effect_kind(str_of(t, "id").unwrap_or("")),
        duration: (end - begin).max(0),
        alignment,
        params: BTreeMap::new(),
        extra: marker(
            json!({ "uid": str_of(t, "thisUId"), "display": str_of(t, "display"), "name": resource_name(t) }),
        ),
    }
}

/// `preTransition` (a transition into the clip from nothing) has no model
/// equivalent; it is surfaced as an entry animation over its range.
pub fn pre_transition_of(t: &Value, clip_start: Ticks) -> EffectInstance {
    let begin = i64_of(t, "tlBegin").unwrap_or(clip_start);
    let end = i64_of(t, "tlEnd").unwrap_or(begin);
    EffectInstance {
        id: str_of(t, "thisUId").map_or_else(new_id, str::to_string),
        kind: effect_kind(str_of(t, "id").unwrap_or("")),
        enabled: true,
        params: BTreeMap::new(),
        range: Some(((begin - clip_start).max(0), (end - clip_start).max(0))),
        extra: marker(json!({
            "uid": str_of(t, "thisUId"), "role": "preTransition",
            "display": str_of(t, "display"), "name": resource_name(t)
        })),
    }
}

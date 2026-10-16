//! Tests against `fixtures/synthetic.wfp`, an anonymized project shaped
//! like Filmora 15 output (see `fixtures/make_synthetic.py`).

use jb_core::*;
use jb_filmora::{CoverageCategory, WfpProject};
use serde_json::Value as Json;
use std::collections::HashSet;

const S: Ticks = TICKS_PER_SECOND;
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/synthetic.wfp");
const DISSOLVE: &str = "filmora:2981D185-D52E-44f4-ABD5-3CE83890E32E";
const ADJUST: &str = "filmora:662E16ED-4524-4D13-AAE9-11DBA0C63E17";

fn load() -> WfpProject {
    jb_filmora::read(FIXTURE).unwrap()
}

fn clips(p: &Project) -> Vec<(&Track, &Clip)> {
    p.sequences.iter().flat_map(|s| &s.tracks).flat_map(|t| t.clips.iter().map(move |c| (t, c))).collect()
}

fn find<'a>(p: &'a Project, kind: TrackKind, start: Ticks, name: &str) -> &'a Clip {
    clips(p)
        .into_iter()
        .find(|(t, c)| t.kind == kind && c.start == start && c.name == name)
        .map(|(_, c)| c)
        .unwrap_or_else(|| panic!("no {kind:?} clip {name} at {start}"))
}

fn f(p: &Param) -> f64 {
    p.value.as_f64().unwrap()
}

fn timeline(w: &WfpProject) -> &Json {
    &w.raw.timeline
}

fn all_clip_json(doc: &Json) -> Vec<&Json> {
    doc["timelineInfos"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|tl| tl["trackInfos"].as_array().unwrap())
        .flat_map(|t| t["clipList"].as_array().unwrap())
        .collect()
}

fn clip_json<'a>(doc: &'a Json, uid: &str) -> &'a Json {
    all_clip_json(doc).into_iter().find(|c| c["thisUId"] == uid).unwrap_or_else(|| panic!("no clip {uid}"))
}

/// Structural invariants Filmora relies on.
fn check_invariants(doc: &Json) {
    let mut uids = HashSet::new();
    let mut walk = vec![doc];
    while let Some(v) = walk.pop() {
        match v {
            Json::Object(o) => {
                if let Some(Json::String(u)) = o.get("thisUId") {
                    assert!(uids.insert(u.clone()), "duplicate thisUId {u}");
                }
                for key in ["pipBuf", "scriptBuf"] {
                    if let Some(Json::String(s)) = o.get(key) {
                        assert_eq!(o[&format!("{key}Size")], s.len() + 1, "{key}Size");
                        serde_json::from_str::<Json>(s).unwrap();
                    }
                }
                if let Some(Json::Array(list)) = o.get("userData") {
                    for u in list {
                        use base64::Engine;
                        let n = base64::engine::general_purpose::STANDARD
                            .decode(u["data"].as_str().unwrap())
                            .unwrap()
                            .len();
                        assert_eq!(u["size"], n);
                    }
                }
                walk.extend(o.values());
            }
            Json::Array(a) => walk.extend(a),
            _ => {}
        }
    }
}

#[test]
fn maps_the_fixture() {
    let w = load();
    let p = &w.project;
    assert_eq!(p.name, "Synthetic Trip");
    assert_eq!((p.settings.width, p.settings.height), (1920, 1080));
    assert_eq!(p.settings.frame_rate, Rational::new(30000, 1001));
    assert_eq!(p.sequences.len(), 2);
    assert_eq!(p.main().tracks.len(), 5);
    assert_eq!(p.media.len(), 4);
    let cam = p.media.iter().find(|m| m.path == "/Users/demo/Movies/Trip/CAM_0001.MP4").unwrap();
    assert_eq!(
        (cam.kind, cam.width, cam.has_audio, cam.duration),
        (MediaKind::Video, 1920, true, 60 * S + S / 1000 * 60)
    );
    assert_eq!(p.media.iter().find(|m| m.path.ends_with("logo.png")).unwrap().kind, MediaKind::Image);
    assert_eq!(p.media.iter().find(|m| m.path.ends_with("theme.mp3")).unwrap().kind, MediaKind::Audio);

    let a = find(p, TrackKind::Video, 0, "CAM_0001");
    assert_eq!((a.end, a.source_in, a.speed.rate), (5 * S, 2 * S, 1.0));
    let tr = a.transition_out.as_ref().unwrap();
    assert_eq!((tr.kind.as_str(), tr.duration, tr.alignment), (DISSOLVE, S, TransitionAlignment::Center));
    assert_eq!(a.animations[0].kind, DISSOLVE);
    assert_eq!(a.animations[0].range, Some((0, S / 2)));
    let adj = a.effects.iter().find(|e| e.kind == ADJUST).unwrap();
    assert_eq!(f(&adj.params["u_exposure"]), 40.0);
    assert_eq!(a.effects.len(), 1, "structural effects are not surfaced");

    let aa = find(p, TrackKind::Audio, 0, "CAM_0001");
    assert!(a.link_group.is_some());
    assert_eq!(a.link_group, aa.link_group);
    assert_eq!(f(&aa.audio.gain_db), -3.0);
    assert_eq!(aa.transition_out.as_ref().unwrap().kind, "filmora:audio/blender/transition-fade");

    let b = find(p, TrackKind::Video, 5 * S, "CAM_0002");
    assert_eq!((b.speed.rate, b.source_in, b.speed.ramp.is_none()), (2.0, 10 * S, true));
    // 3 s on the timeline at 2x consume 6 s of source.
    assert_eq!(eval::source_time(b, 3 * S), 16 * S);

    let music = find(p, TrackKind::Audio, 0, "theme");
    assert_eq!(f(&music.audio.gain_db), -12.5);
    assert_eq!((music.audio.fade_in, music.audio.fade_out), (S, 2 * S));
    assert!(music.link_group.is_none());

    let img = find(p, TrackKind::Video, 2 * S, "logo");
    assert_eq!((f(&img.transform.position_x), f(&img.transform.position_y)), (-0.25, 0.25));
    assert_eq!((f(&img.transform.scale_x), f(&img.transform.scale_y)), (0.5, 0.5));
    assert!((f(&img.opacity) - 0.8).abs() < 1e-9);

    let title_ref = find(p, TrackKind::Video, 6 * S, "Basic 1");
    let ClipSource::Sequence { sequence_id } = &title_ref.source else { panic!("title is a nested sequence") };
    let sub = p.sequence(sequence_id).unwrap();
    let text = &sub.tracks[0].clips[0];
    let ClipSource::Title(t) = &text.source else { panic!("text clip maps to a title") };
    assert_eq!(t.text, "Hello World");
    assert_eq!(t.position, [0.5, 0.85]);
    assert_eq!(t.color, [1.0, 0.8, 0.0, 1.0]);
    assert!((t.font_size - 0.048).abs() < 1e-3, "{}", t.font_size);
    assert!(t.stroke.is_some() && t.shadow.is_none());
    assert_eq!(text.transform, Transform::default(), "the text box transform is folded into the title");

    let graphic = clips(p)
        .into_iter()
        .find(|(_, c)| c.start == 8 * S && c.end == 9 * S && matches!(c.source, ClipSource::Color { .. }))
        .unwrap()
        .1;
    assert!(graphic.effects.iter().any(|e| e.kind == "filmora:graphic/effect/path_stroke"));
    assert_eq!(w.warnings.len(), 1, "{:?}", w.warnings);
}

#[test]
fn unedited_round_trip_is_byte_identical() {
    let w = load();
    let (bytes, report) = jb_filmora::write_bytes(&w).unwrap();
    assert!(report.warnings.is_empty());
    let again = jb_filmora::read_bytes(&bytes).unwrap();
    assert_eq!(again.project, w.project);
    for (a, b) in w.raw.entries.iter().zip(&again.raw.entries) {
        assert_eq!((&a.name, &a.data, a.deflated), (&b.name, &b.data, b.deflated));
    }
    assert_eq!(w.raw.entries.len(), again.raw.entries.len());
}

#[test]
fn edits_are_patched_in_place() {
    let mut w = load();
    let orig = w.clone();
    let p = &w.project;
    let a = find(p, TrackKind::Video, 0, "CAM_0001").clone();
    let b = find(p, TrackKind::Video, 5 * S, "CAM_0002").clone();
    let music = find(p, TrackKind::Audio, 0, "theme").clone();
    let img = find(p, TrackKind::Video, 2 * S, "logo").clone();
    let sub_id = match &find(p, TrackKind::Video, 6 * S, "Basic 1").source {
        ClipSource::Sequence { sequence_id } => sequence_id.clone(),
        _ => unreachable!(),
    };
    let text = p.sequence(&sub_id).unwrap().tracks[0].clips[0].clone();
    let adj = a.effects[0].id.clone();

    let mut h = History::new(w.project.clone());
    let mut title = match &text.source {
        ClipSource::Title(t) => t.clone(),
        _ => unreachable!(),
    };
    title.text = "Bye".into();
    title.position = [0.5, 0.2];
    let mut music_audio = music.audio.clone();
    music_audio.gain_db = Param::float(-6.0);
    music_audio.fade_out = 3 * S;
    let mut tf = img.transform.clone();
    tf.scale_x = Param::float(0.75);
    let new_fx = EffectInstance {
        id: new_id(),
        kind: "filmora:video/effect/blur".into(),
        enabled: true,
        params: [("radius".to_string(), Param::float(12.0))].into(),
        range: None,
        extra: Extra::Null,
    };
    let mut tr = a.transition_out.clone().unwrap();
    tr.duration = 2 * S;
    h.execute(&Command::Batch {
        commands: vec![
            Command::RenameProject { name: "Edited".into() },
            Command::SetTitle { clip_id: text.id.clone(), title },
            Command::SetAudio { clip_id: music.id.clone(), audio: music_audio },
            Command::SetTransform { clip_id: img.id.clone(), transform: tf },
            Command::SetOpacity { clip_id: img.id.clone(), opacity: Param::float(0.5) },
            Command::SetEffectParams {
                clip_id: a.id.clone(),
                effect_id: adj.clone(),
                params: [("u_exposure".to_string(), Param::float(10.0))].into(),
            },
            Command::AddEffect { clip_id: img.id.clone(), effect: new_fx },
            Command::SetTransition { clip_id: a.id.clone(), transition: Some(tr) },
            Command::SetSpeed { clip_id: b.id.clone(), rate: 1.5, reverse: None },
            Command::SplitClip { clip_id: a.id.clone(), at: 2 * S },
        ],
    })
    .unwrap();
    w.project = h.project.clone();
    let (bytes, report) = jb_filmora::write_bytes(&w).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let r = jb_filmora::read_bytes(&bytes).unwrap();
    check_invariants(timeline(&r));
    let p = &r.project;
    assert_eq!(p.name, "Edited");
    assert_eq!(r.raw.project_info["project_file_name"], "Edited");

    // Split of a linked pair: four clips, right halves linked to each other.
    let l = find(p, TrackKind::Video, 0, "CAM_0001");
    let rv = find(p, TrackKind::Video, 2 * S, "CAM_0001");
    let la = find(p, TrackKind::Audio, 0, "CAM_0001");
    let ra = find(p, TrackKind::Audio, 2 * S, "CAM_0001");
    assert_eq!((l.end, rv.source_in, rv.end), (2 * S, 4 * S, 5 * S));
    assert_eq!(l.link_group, la.link_group);
    assert_eq!(rv.link_group, ra.link_group);
    assert_ne!(l.link_group, rv.link_group);
    assert!(l.transition_out.is_none());
    let tr = rv.transition_out.as_ref().unwrap();
    assert_eq!((tr.kind.as_str(), tr.duration), (DISSOLVE, 2 * S));
    assert_eq!(f(&rv.effects[0].params["u_exposure"]), 10.0);
    // Unknown fields survive on both halves.
    for c in [l, rv] {
        assert_eq!(clip_json(timeline(&r), &c.id)["futureField"]["nested"], serde_json::json!([1, 2, 3]));
    }
    // SplitClip copies animations, so the right half carries its own copy
    // of the entry transition, with a fresh identity.
    let pre_l = &clip_json(timeline(&r), &l.id)["preTransition"];
    let pre_r = &clip_json(timeline(&r), &rv.id)["preTransition"];
    assert_eq!(pre_l, &clip_json(timeline(&orig), &a.id)["preTransition"]);
    assert_eq!((pre_r["tlBegin"].as_i64(), pre_r["tlEnd"].as_i64()), (Some(2 * S), Some(2 * S + S / 2)));
    assert_ne!(pre_l["thisUId"], pre_r["thisUId"]);

    let b2 = find(p, TrackKind::Video, 5 * S, "CAM_0002");
    assert_eq!((b2.speed.rate, b2.end), (1.5, 5 * S + 4 * S));
    let bj = clip_json(timeline(&r), &b2.id);
    assert_eq!(bj["speed"]["offset"], 10.0);
    assert_eq!(bj["speed"]["offsetEnd"], 16.0);

    let m = find(p, TrackKind::Audio, 0, "theme");
    assert_eq!((f(&m.audio.gain_db), m.audio.fade_out, m.audio.fade_in), (-6.0, 3 * S, S));

    let i = find(p, TrackKind::Video, 2 * S, "logo");
    assert_eq!((f(&i.transform.scale_x), f(&i.transform.scale_y)), (0.75, 0.5));
    assert!((f(&i.opacity) - 0.5).abs() < 1e-9);
    let blur = i.effects.iter().find(|e| e.kind == "filmora:video/effect/blur").unwrap();
    assert_eq!(f(&blur.params["radius"]), 12.0);

    let sub = p.sequence(&sub_id).unwrap();
    let ClipSource::Title(t) = &sub.tracks[0].clips[0].source else { panic!() };
    assert_eq!((t.text.as_str(), t.position), ("Bye", [0.5, 0.2]));
    assert!(t.stroke.is_some(), "untouched title styling survives");

    // Untouched clips are byte-for-byte the same JSON objects.
    let untouched = find(&orig.project, TrackKind::Video, 6 * S, "Basic 1");
    assert_eq!(clip_json(timeline(&r), &untouched.id), clip_json(timeline(&orig), &untouched.id));
    assert_eq!(r.raw.medias_info, orig.raw.medias_info);
    // Non-JSON entries are untouched.
    let thumb = |w: &WfpProject| w.raw.entry("ProjectFolder/Anon/Cover/thumb.fsthumb").unwrap().data.clone();
    assert_eq!(thumb(&r), thumb(&orig));
    // A second write of the re-read project changes nothing.
    let (bytes2, _) = jb_filmora::write_bytes(&r).unwrap();
    assert_eq!(jb_filmora::read_bytes(&bytes2).unwrap().raw.timeline, r.raw.timeline);
}

#[test]
fn new_clips_get_filmora_shaped_json() {
    let mut w = load();
    let mut h = History::new(w.project.clone());
    let asset = MediaAsset {
        id: new_id(),
        name: "extra".into(),
        path: "/Users/demo/Movies/extra.mov".into(),
        kind: MediaKind::Video,
        duration: 20 * S,
        width: 1280,
        height: 720,
        frame_rate: Some(Rational::new(25, 1)),
        has_audio: true,
        extra: Extra::Null,
    };
    let video_track = h.project.main().tracks.iter().find(|t| t.kind == TrackKind::Video).unwrap().id.clone();
    h.execute(&Command::AddMedia { asset: asset.clone() }).unwrap();
    h.execute(&Command::AddTrack { sequence_id: None, kind: TrackKind::Video, index: None }).unwrap();
    let new_track = h.project.main().tracks.last().unwrap().id.clone();
    let clip = |id: &str, source: ClipSource, start: Ticks, end: Ticks| -> Clip {
        let mut c: Clip = serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "source": {"type": "adjustment"}, "start": start, "end": end
        }))
        .unwrap();
        c.source = source;
        c
    };
    let mut v = clip("new-video", ClipSource::Media { media_id: asset.id.clone() }, 9 * S, 12 * S);
    v.source_in = 4 * S;
    h.execute(&Command::AddClip { track_id: video_track, clip: v }).unwrap();
    let title = TitleData {
        text: "New title".into(),
        font_family: "Arial".into(),
        font_size: 0.06,
        color: [1.0, 0.0, 0.0, 1.0],
        position: [0.5, 0.9],
        bold: true,
        italic: false,
        stroke: None,
        shadow: None,
        background: None,
    };
    h.execute(&Command::AddClip {
        track_id: new_track,
        clip: clip("new-title", ClipSource::Title(title.clone()), 0, 3 * S),
    })
    .unwrap();
    w.project = h.project.clone();
    let (bytes, report) = jb_filmora::write_bytes(&w).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let r = jb_filmora::read_bytes(&bytes).unwrap();
    check_invariants(timeline(&r));
    let p = &r.project;
    let nv = find(p, TrackKind::Video, 9 * S, "new-video");
    assert_eq!((nv.end, nv.source_in, nv.speed.rate), (12 * S, 4 * S, 1.0));
    let m = p.media(match &nv.source {
        ClipSource::Media { media_id } => media_id,
        _ => panic!(),
    });
    assert_eq!(m.unwrap().path, "/Users/demo/Movies/extra.mov");
    assert_eq!(m.unwrap().width, 1280);

    // The title was wrapped in a nested timeline like Filmora does.
    let wrapper = p.main().tracks.last().unwrap().clips.first().unwrap();
    let ClipSource::Sequence { sequence_id } = &wrapper.source else { panic!("wrapped") };
    let ClipSource::Title(t) = &p.sequence(sequence_id).unwrap().tracks[0].clips[0].source else { panic!() };
    assert_eq!((t.text.as_str(), t.bold, t.position), ("New title", true, [0.5, 0.9]));
    assert!((t.font_size - 0.06).abs() < 1e-9);
    assert_eq!(t.color, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn coverage_lists_everything_used() {
    let w = load();
    let cov = jb_filmora::coverage(&w);
    let dissolve = cov.get(CoverageCategory::Transition, "2981D185-D52E-44f4-ABD5-3CE83890E32E").unwrap();
    assert_eq!(dissolve.count, 2);
    assert_eq!(dissolve.resource.as_ref().unwrap().res_id, Some(39146));
    let adj = cov.get(CoverageCategory::ColorAdjustment, "662E16ED-4524-4D13-AAE9-11DBA0C63E17").unwrap();
    assert_eq!(adj.name.as_deref(), Some("AdjustColor"));
    assert_eq!(adj.params["u_exposure"].samples, vec!["40.0"]);
    assert!(cov.get(CoverageCategory::TitleTemplate, "Basic_1").is_some());
    assert!(cov.get(CoverageCategory::Font, "Arial").is_some());
    assert!(cov.get(CoverageCategory::Graphic, "graphic/effect/path_stroke").is_some());
    assert_eq!(cov.get(CoverageCategory::ClipType, "2").unwrap().count, 3);
    let md = cov.to_markdown();
    assert!(md.contains("## Transitions") && md.contains("AdjustColor"));
    let mut merged = cov.clone();
    merged.merge(&cov);
    assert_eq!(merged.get(CoverageCategory::Transition, "2981D185-D52E-44f4-ABD5-3CE83890E32E").unwrap().count, 4);
}

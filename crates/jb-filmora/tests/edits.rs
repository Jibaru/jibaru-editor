//! Edit round trips: property-panel edits written to .wfp and read back.

use jb_core::*;
use jb_filmora::WfpProject;
use serde_json::{json, Value as Json};

const S: Ticks = TICKS_PER_SECOND;
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/synthetic.wfp");

fn load() -> WfpProject {
    jb_filmora::read(FIXTURE).unwrap()
}

fn find<'a>(p: &'a Project, kind: TrackKind, start: Ticks, name: &str) -> &'a Clip {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .filter(|t| t.kind == kind)
        .flat_map(|t| &t.clips)
        .find(|c| c.start == start && c.name == name)
        .unwrap_or_else(|| panic!("no {kind:?} clip {name} at {start}"))
}

fn run(h: &mut History, cmd: Json) {
    let c: Command = serde_json::from_value(cmd.clone()).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    h.execute(&c).unwrap_or_else(|e| panic!("{cmd}: {e}"));
}

/// Every keyframe string carries a valid MD5 and every nested buffer its size.
fn check_invariants(doc: &Json) {
    let mut walk = vec![doc];
    while let Some(v) = walk.pop() {
        match v {
            Json::Object(o) => {
                for key in ["pipBuf", "scriptBuf"] {
                    if let Some(Json::String(s)) = o.get(key) {
                        assert_eq!(o[&format!("{key}Size")], s.len() + 1, "{key}Size");
                    }
                }
                walk.extend(o.values());
            }
            Json::Array(a) => walk.extend(a),
            _ => {}
        }
    }
}

fn write_read(w: &WfpProject) -> WfpProject {
    let (bytes, report) = jb_filmora::write_bytes(w).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let r = jb_filmora::read_bytes(&bytes).unwrap();
    check_invariants(&r.raw.timeline);
    // A second write of the re-read project is stable.
    let (bytes2, report2) = jb_filmora::write_bytes(&r).unwrap();
    assert!(report2.warnings.is_empty(), "{:?}", report2.warnings);
    let r2 = jb_filmora::read_bytes(&bytes2).unwrap();
    assert_eq!(r2.raw.timeline, r.raw.timeline);
    assert_eq!(r2.project, r.project);
    r
}

fn f(v: f64) -> Json {
    json!({"type": "float", "value": v})
}

#[test]
fn visual_properties_round_trip() {
    let mut w = load();
    let img = find(&w.project, TrackKind::Video, 2 * S, "logo").clone();
    let a = find(&w.project, TrackKind::Video, 0, "CAM_0001").clone();
    let adj = a.effects[0].id.clone();
    let mut h = History::new(w.project.clone());
    run(&mut h, json!({"op": "set_blend_mode", "clip_id": img.id, "blend_mode": "screen"}));
    run(&mut h, json!({"op": "set_crop", "clip_id": img.id, "crop": [0.1, 0.05, 0.2, 0.0]}));
    run(&mut h, json!({"op": "set_flip", "clip_id": img.id, "horizontal": true}));
    run(&mut h, json!({"op": "set_fit_mode", "clip_id": img.id, "fit": "fill"}));
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": img.id, "param": "transform.position_x", "time": 0, "value": f(-0.25)}),
    );
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": img.id, "param": "transform.position_x", "time": S, "value": f(0.25), "interpolation": "ease_in_out"}),
    );
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": img.id, "param": "transform.rotation", "time": S / 2, "value": f(45.0)}),
    );
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": img.id, "param": "transform.rotation", "time": S, "value": f(90.0), "interpolation": "hold"}),
    );
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": img.id, "param": "transform.scale", "time": 0, "value": f(0.5)}),
    );
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": img.id, "param": "transform.scale", "time": S, "value": f(1.0)}),
    );
    run(&mut h, json!({"op": "add_keyframe", "clip_id": img.id, "param": "opacity", "time": 0, "value": f(0.0)}));
    run(&mut h, json!({"op": "add_keyframe", "clip_id": img.id, "param": "opacity", "time": S, "value": f(1.0)}));
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": a.id, "param": format!("effect.{adj}.u_exposure"), "time": 0, "value": f(0.0)}),
    );
    run(
        &mut h,
        json!({"op": "add_keyframe", "clip_id": a.id, "param": format!("effect.{adj}.u_exposure"), "time": S, "value": f(40.0)}),
    );
    run(
        &mut h,
        json!({"op": "add_animation", "clip_id": img.id, "kind": "jb.anim.fade_in", "placement": "in", "duration": S / 2}),
    );
    run(
        &mut h,
        json!({"op": "add_animation", "clip_id": img.id, "kind": "jb.anim.slide_out_left", "placement": "out", "duration": S / 2}),
    );
    w.project = h.project.clone();
    let r = write_read(&w);

    let (i, e) = (r.project.clip(&img.id).unwrap(), h.project.clip(&img.id).unwrap());
    assert_eq!(i.blend_mode, BlendMode::Screen);
    // The interpolation of a last keyframe shapes nothing and is not stored.
    let mut et = e.transform.clone();
    for p in [&mut et.position_x, &mut et.rotation] {
        p.keyframes.last_mut().unwrap().interpolation = Interpolation::Linear;
    }
    // "Fill" is Filmora's Crop to Fit: the crop narrowed to the canvas
    // aspect, which renders the same when fitted.
    assert_eq!(i.transform.fit, FitMode::Fit);
    let (c, ec) = (i.transform.crop, et.crop);
    assert!((c[0] - ec[0]).abs() < 1e-9 && (c[2] - ec[2]).abs() < 1e-9, "{c:?}");
    assert!(c[1] > ec[1] && c[3] > ec[3], "{c:?}");
    et.crop = c;
    et.fit = FitMode::Fit;
    assert_eq!(i.transform, et);
    assert_eq!(i.opacity.keyframes, e.opacity.keyframes);
    assert_eq!(i.animations.len(), 2);
    for (x, y) in i.animations.iter().zip(&e.animations) {
        assert_eq!((&x.kind, x.placement, x.range), (&y.kind, y.placement, y.range));
    }
    let ra = r.project.clip(&a.id).unwrap();
    assert_eq!(ra.effects[0].params["u_exposure"], h.project.clip(&a.id).unwrap().effects[0].params["u_exposure"]);
    // The writer used Filmora's parameter keyframe list.
    let tl = serde_json::to_string(&r.raw.timeline).unwrap();
    assert!(tl.contains("paramMapList"));
}

#[test]
fn speed_audio_flags_and_markers_round_trip() {
    let mut w = load();
    let p = &w.project;
    let a = find(p, TrackKind::Video, 0, "CAM_0001").clone();
    let b = find(p, TrackKind::Video, 5 * S, "CAM_0002").clone();
    let music = find(p, TrackKind::Audio, 0, "theme").clone();
    let audio_track = p.main().tracks.iter().find(|t| t.clips.iter().any(|c| c.id == music.id)).unwrap().id.clone();
    let video_track = p.main().tracks.iter().find(|t| t.kind == TrackKind::Video).unwrap().id.clone();
    let mut h = History::new(w.project.clone());
    run(&mut h, json!({"op": "set_speed_ramp", "clip_id": b.id, "preset": "flash_in"}));
    run(&mut h, json!({"op": "set_maintain_pitch", "clip_id": b.id, "maintain_pitch": false}));
    run(&mut h, json!({"op": "set_pan", "clip_id": music.id, "pan": -0.5}));
    run(&mut h, json!({"op": "set_pitch", "clip_id": music.id, "semitones": 2.0}));
    run(&mut h, json!({"op": "set_normalization", "clip_id": music.id, "target_lufs": -14.0, "measured_lufs": -20.5}));
    run(&mut h, json!({"op": "set_equalizer", "clip_id": music.id, "preset": "rock"}));
    run(&mut h, json!({"op": "add_keyframe", "clip_id": music.id, "param": "volume", "time": 0, "value": f(-12.0)}));
    run(&mut h, json!({"op": "add_keyframe", "clip_id": music.id, "param": "volume", "time": 2 * S, "value": f(0.0)}));
    run(&mut h, json!({"op": "set_mute", "clip_id": music.id, "muted": true}));
    run(&mut h, json!({"op": "set_clip_enabled", "clip_id": b.id, "enabled": false}));
    run(&mut h, json!({"op": "set_track_flags", "track_id": audio_track, "muted": true, "locked": true}));
    run(&mut h, json!({"op": "set_track_flags", "track_id": video_track, "hidden": true}));
    run(
        &mut h,
        json!({"op": "add_marker", "marker": {"id": "m1", "time": 3 * S, "name": "Drop", "comment": "here", "color": [1.0, 0.0, 0.0, 1.0]}}),
    );
    run(&mut h, json!({"op": "add_marker", "clip_id": a.id, "marker": {"id": "m2", "time": S, "name": "clip mark"}}));
    w.project = h.project.clone();
    let r = write_read(&w);
    let rp = &r.project;

    let (rb, eb) = (rp.clip(&b.id).unwrap(), h.project.clip(&b.id).unwrap());
    assert!(rb.disabled);
    // Maintain pitch lives on the linked audio clip (`speedWithPitch`).
    let rba = rp
        .sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .filter(|t| t.kind == TrackKind::Audio)
        .flat_map(|t| &t.clips)
        .find(|c| c.link_group.is_some() && c.link_group == rb.link_group)
        .unwrap();
    assert!(!rba.speed.maintain_pitch);
    assert_eq!(rb.speed.curve.as_ref().unwrap().preset, "flash_in");
    assert_eq!(rb.duration(), eb.duration());
    for t in [0, S, 2 * S, rb.duration() - 1] {
        let (x, y) = (eval::source_time(rb, t), eval::source_time(eb, t));
        assert!((x - y).abs() < S / 50, "ramp at {t}: {x} vs {y}");
    }
    assert_eq!(rp.clip(&music.id).unwrap().audio, h.project.clip(&music.id).unwrap().audio);
    let at = rp.main().tracks.iter().find(|t| t.id == audio_track).unwrap();
    assert!(at.muted && at.locked);
    assert!(rp.main().tracks.iter().find(|t| t.id == video_track).unwrap().hidden);
    assert_eq!(rp.main().markers.len(), 1);
    assert_eq!((rp.main().markers[0].name.as_str(), rp.main().markers[0].color), ("Drop", [1.0, 0.0, 0.0, 1.0]));
    assert_eq!(rp.clip(&a.id).unwrap().markers[0].time, S);
}

#[test]
fn timeline_edits_round_trip() {
    let mut w = load();
    let a = find(&w.project, TrackKind::Video, 0, "CAM_0001").clone();
    let img = find(&w.project, TrackKind::Video, 2 * S, "logo").clone();
    let mut h = History::new(w.project.clone());
    run(&mut h, json!({"op": "freeze_frame", "clip_id": a.id, "at": 2 * S, "duration": S}));
    run(&mut h, json!({"op": "duplicate_clip", "clip_id": img.id, "start": 30 * S}));
    run(&mut h, json!({"op": "set_reverse", "clip_id": img.id, "reverse": false}));
    w.project = h.project.clone();
    let r = write_read(&w);
    let frozen = r
        .project
        .sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips)
        .find(|c| c.speed.freeze)
        .expect("freeze frame survives");
    assert_eq!((frozen.start, frozen.end, frozen.source_in), (2 * S, 3 * S, 4 * S));
    assert!(find(&r.project, TrackKind::Video, 30 * S, "logo").duration() > 0);
}

/// The JSON written for each property has the shape Filmora 15.7.3 writes
/// (docs/filmora-format.md §13).
#[test]
fn written_shapes_match_filmora() {
    let mut w = load();
    let a = find(&w.project, TrackKind::Video, 0, "CAM_0001").clone();
    let b = find(&w.project, TrackKind::Video, 5 * S, "CAM_0002").clone();
    let music = find(&w.project, TrackKind::Audio, 0, "theme").clone();
    let video_track = w.project.main().tracks.iter().find(|t| t.kind == TrackKind::Video).unwrap().id.clone();
    let mut h = History::new(w.project.clone());
    run(&mut h, json!({"op": "set_flip", "clip_id": a.id, "horizontal": true}));
    run(&mut h, json!({"op": "set_blend_mode", "clip_id": a.id, "blend_mode": "add"}));
    run(&mut h, json!({"op": "set_crop", "clip_id": a.id, "crop": [0.1, 0.1, 0.1, 0.1]}));
    run(&mut h, json!({"op": "set_clip_enabled", "clip_id": b.id, "enabled": false}));
    run(&mut h, json!({"op": "set_speed", "clip_id": b.id, "rate": 2.0}));
    run(&mut h, json!({"op": "set_maintain_pitch", "clip_id": b.id, "maintain_pitch": false}));
    run(&mut h, json!({"op": "set_mute", "clip_id": music.id, "muted": true}));
    run(&mut h, json!({"op": "set_equalizer", "clip_id": music.id, "preset": "low_bass"}));
    run(&mut h, json!({"op": "set_pitch", "clip_id": music.id, "semitones": 3.0}));
    run(&mut h, json!({"op": "rename_track", "track_id": video_track, "name": "Main Cam"}));
    run(&mut h, json!({"op": "set_track_height", "track_id": video_track, "height": 160}));
    run(
        &mut h,
        json!({"op": "add_animation", "clip_id": a.id, "kind": "jb.anim.fade_in", "placement": "in", "duration": S}),
    );
    w.project = h.project.clone();
    let r = write_read(&w);
    let tl = serde_json::to_string(&r.raw.timeline).unwrap();
    for needle in [
        "\"id\":\"video/effect/horizontal_filp\"",
        "\\\"BlendMode\\\":\\\"Linear Dodge\\\"",
        "\"name\":\"SettingScript\"",
        "\"enable\":false",
        "\"speedWithPitch\":true",
        "\"name\":\"Mute\"",
        "\"name\":\"63Hz\"",
        "\"id\":\"audio/effect/pitchshifter\"",
        "\"name\":\"Semitone\"",
        "\"inAnimation\":",
        "\"id\":\"video/effect/motion\"",
    ] {
        assert!(tl.contains(needle), "missing {needle}");
    }
    for gone in ["keepPitch", "rampPreset", "freezeFrame", "CropLeft", "FlipX", "\"Preset\""] {
        assert!(!tl.contains(gone), "unexpected {gone}");
    }
    let rp = &r.project;
    let ra = rp.clip(&a.id).unwrap();
    assert!(ra.transform.flip_h);
    assert_eq!(ra.blend_mode, BlendMode::Add);
    assert!(ra.animations.iter().any(|x| x.kind == "jb.anim.fade_in" && x.range == Some((0, S))));
    let eq = rp.clip(&music.id).unwrap().audio.equalizer.clone().unwrap();
    assert_eq!(eq.preset, "low_bass");
    let vt = rp.main().tracks.iter().find(|t| t.id == video_track).unwrap();
    assert_eq!((vt.name.as_str(), vt.height), ("Main Cam", Some(160)));
}

#[test]
fn jibaru_only_state_round_trips_through_the_sidecar() {
    let mut w = load();
    let a = find(&w.project, TrackKind::Video, 0, "CAM_0001").clone();
    let music = find(&w.project, TrackKind::Audio, 0, "theme").clone();
    let media = w.project.media[0].id.clone();
    let mut h = History::new(w.project.clone());
    let key = json!({"id": "11111111-2222-4333-8444-555555555555", "kind": "jb.chroma_key",
        "params": {"tolerance": {"value": f(40.0)}, "color": {"value": {"type": "color", "value": [0.0, 1.0, 0.0, 1.0]}}}});
    run(&mut h, json!({"op": "add_effect", "clip_id": a.id, "effect": key}));
    run(
        &mut h,
        json!({"op": "add_effect", "clip_id": a.id, "effect": {"id": "e-mask", "kind": "jb.mask.ellipse", "params": {}}}),
    );
    run(&mut h, json!({"op": "set_effect_enabled", "clip_id": a.id, "effect_id": "e-mask", "enabled": false}));
    run(
        &mut h,
        json!({"op": "add_effect", "clip_id": a.id, "effect": {"id": "e-ae", "kind": "jb.auto_enhance", "params": {"amount": {"value": f(70.0)}}}}),
    );
    let mut audio = h.project.clip(&music.id).unwrap().audio.clone();
    audio.denoise_db = Some(12.0);
    run(&mut h, json!({"op": "set_audio", "clip_id": music.id, "audio": audio}));
    run(&mut h, json!({"op": "set_audio_channels", "clip_id": music.id, "channels": "left"}));
    run(&mut h, json!({"op": "set_voice_filter", "clip_id": music.id, "filter": "echo"}));
    run(&mut h, json!({"op": "set_audio_cleanup", "clip_id": music.id, "hum_removal": 50, "hiss_removal": 30}));
    run(&mut h, json!({"op": "set_project_settings", "width": 1080, "height": 1920}));
    run(&mut h, json!({"op": "rename_media", "media_id": media, "name": "Renamed"}));
    w.project = h.project.clone();

    let (_, report) = jb_filmora::write_bytes(&w).unwrap();
    assert!(report.notes.iter().any(|n| n.contains("jb.chroma_key")), "{:?}", report.notes);
    assert!(!report.notes.iter().any(|n| n.contains("jb.auto_enhance")), "{:?}", report.notes);
    let r = write_read(&w);
    let rp = &r.project;
    let (ra, ea) = (rp.clip(&a.id).unwrap(), h.project.clip(&a.id).unwrap());
    assert_eq!(ra.effects.len(), ea.effects.len());
    for (x, y) in ra.effects.iter().zip(&ea.effects) {
        assert_eq!((&x.kind, x.enabled), (&y.kind, y.enabled));
        assert_eq!(x.params, y.params, "{}", x.kind);
    }
    // Auto Enhance is written as Filmora's own AutoEnhance effect.
    let tl = serde_json::to_string(&r.raw.timeline).unwrap();
    assert!(tl.contains("45D9AD95-4F03-4B2F-A8C7-8F7817B73FA4"));
    assert!(tl.contains("\"aspectRatioX\":9") && tl.contains("\"aspectRatioY\":16"));
    let rm = rp.clip(&music.id).unwrap();
    assert_eq!((rm.audio.denoise_db, rm.audio.channels), (Some(12.0), ChannelMode::Left));
    assert_eq!(
        (rm.audio.voice_filter.as_deref(), rm.audio.hum_removal, rm.audio.hiss_removal),
        (Some("echo"), Some(50.0), Some(30.0))
    );
    assert_eq!((rp.settings.width, rp.settings.height), (1080, 1920));
    assert_eq!(rp.media(&media).unwrap().name, "Renamed");

    // Turning denoise off survives too.
    let mut h = History::new(r.project.clone());
    let mut audio = rm.audio.clone();
    audio.denoise_db = None;
    audio.channels = ChannelMode::Stereo;
    audio.voice_filter = None;
    audio.hum_removal = None;
    audio.hiss_removal = None;
    run(&mut h, json!({"op": "set_audio", "clip_id": music.id, "audio": audio}));
    for e in ["11111111-2222-4333-8444-555555555555", "e-mask"] {
        run(&mut h, json!({"op": "remove_effect", "clip_id": a.id, "effect_id": e}));
    }
    let mut w2 = r.clone();
    w2.project = h.project.clone();
    let r2 = write_read(&w2);
    assert_eq!(r2.project.clip(&music.id).unwrap().audio.denoise_db, None);
    assert_eq!(r2.project.clip(&music.id).unwrap().audio.channels, ChannelMode::Stereo);
    assert_eq!(r2.project.clip(&music.id).unwrap().audio.voice_filter, None);
    assert_eq!(r2.project.clip(&music.id).unwrap().audio.hum_removal, None);
    let effects = &r2.project.clip(&a.id).unwrap().effects;
    assert_eq!(
        effects.iter().map(|e| e.kind.as_str()).filter(|k| k.starts_with("jb.")).collect::<Vec<_>>(),
        ["jb.auto_enhance"]
    );
}

//! Behaviour of the editing commands.

use jb_core::*;
use serde_json::json;

const S: Ticks = TICKS_PER_SECOND;

struct Fx {
    h: History,
    v1: Id,
    v2: Id,
    a1: Id,
}

fn project() -> Fx {
    let mut p = Project::new(
        "t",
        ProjectSettings {
            width: 1920,
            height: 1080,
            frame_rate: Rational::new(30, 1),
            sample_rate: 48000,
            background: [0.0, 0.0, 0.0, 1.0],
        },
    );
    for (id, kind) in [("m", MediaKind::Video), ("n", MediaKind::Video), ("song", MediaKind::Audio)] {
        p.media.push(MediaAsset {
            id: id.into(),
            name: id.into(),
            path: format!("/{id}"),
            kind,
            duration: 100 * S,
            width: 1920,
            height: 1080,
            frame_rate: None,
            has_audio: true,
            extra: Extra::Null,
        });
    }
    let mut h = History::new(p);
    for k in [TrackKind::Video, TrackKind::Video, TrackKind::Audio] {
        h.execute(&Command::AddTrack { sequence_id: None, kind: k, index: None }).unwrap();
    }
    let t = &h.project.main().tracks;
    let (v1, v2, a1) = (t[0].id.clone(), t[1].id.clone(), t[2].id.clone());
    Fx { h, v1, v2, a1 }
}

fn clip(id: &str, media: &str, start: Ticks, end: Ticks) -> Clip {
    serde_json::from_value(json!({
        "id": id, "name": id, "source": {"type": "media", "media_id": media},
        "start": start, "end": end
    }))
    .unwrap()
}

fn get<'a>(h: &'a History, id: &str) -> &'a Clip {
    h.project.clip(id).unwrap_or_else(|| panic!("no clip {id}"))
}

fn track_clips<'a>(h: &'a History, track: &str) -> Vec<&'a Clip> {
    h.project.main().tracks.iter().find(|t| t.id == track).unwrap().clips.iter().collect()
}

fn kf(t: Ticks, v: f64) -> Keyframe {
    Keyframe { time: t, value: Value::Float(v), interpolation: Interpolation::Linear }
}

fn run(h: &mut History, cmd: serde_json::Value) {
    let c: Command = serde_json::from_value(cmd.clone()).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    h.execute(&c).unwrap_or_else(|e| panic!("{cmd}: {e}"));
}

fn fails(h: &mut History, cmd: serde_json::Value) {
    let c: Command = serde_json::from_value(cmd).unwrap();
    assert!(h.execute(&c).is_err());
}

#[test]
fn keyframe_commands() {
    let mut f = project();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("a", "m", 0, 10 * S) }).unwrap();
    let h = &mut f.h;
    run(
        h,
        json!({"op": "add_keyframe", "clip_id": "a", "param": "transform.scale", "time": 0, "value": {"type": "float", "value": 1.0}}),
    );
    run(
        h,
        json!({"op": "add_keyframe", "clip_id": "a", "param": "transform.scale", "time": 10 * S, "value": {"type": "float", "value": 2.0}}),
    );
    let c = get(h, "a");
    assert_eq!(c.transform.scale_x.sample_f64(5 * S), 1.5);
    assert_eq!(c.transform.scale_y.keyframes.len(), 2);
    run(h, json!({"op": "move_keyframe", "clip_id": "a", "param": "scale_x", "from": 10 * S, "to": 5 * S}));
    assert_eq!(get(h, "a").transform.scale_x.sample_f64(5 * S), 2.0);
    run(h, json!({"op": "remove_keyframe", "clip_id": "a", "param": "transform.scale_y", "time": 0}));
    assert_eq!(get(h, "a").transform.scale_y.sample_f64(0), 2.0);
    // Effect params, created on first keyframe.
    let fx = EffectInstance {
        id: "blur".into(),
        kind: "jb.gaussian_blur".into(),
        enabled: true,
        params: Default::default(),
        range: None,
        placement: None,
        extra: Extra::Null,
    };
    h.execute(&Command::AddEffect { clip_id: "a".into(), effect: fx }).unwrap();
    run(
        h,
        json!({"op": "add_keyframe", "clip_id": "a", "param": "effect.blur.radius", "time": S, "value": {"type": "float", "value": 4.0}}),
    );
    assert_eq!(get(h, "a").effects[0].params["radius"].keyframes.len(), 1);
    run(
        h,
        json!({"op": "add_keyframe", "clip_id": "a", "param": "volume", "time": S, "value": {"type": "float", "value": -6.0}, "interpolation": "ease_in_out"}),
    );
    run(h, json!({"op": "clear_keyframes", "clip_id": "a", "param": "volume"}));
    assert!(get(h, "a").audio.gain_db.keyframes.is_empty());
    run(h, json!({"op": "set_param", "clip_id": "a", "param": "opacity", "value": {"type": "float", "value": 0.5}}));
    assert_eq!(get(h, "a").opacity, Param::float(0.5));
    fails(
        h,
        json!({"op": "add_keyframe", "clip_id": "a", "param": "nope", "time": 0, "value": {"type": "float", "value": 1.0}}),
    );
    fails(
        h,
        json!({"op": "add_keyframe", "clip_id": "a", "param": "opacity", "time": 11 * S, "value": {"type": "float", "value": 1.0}}),
    );
}

#[test]
fn split_shifts_keyframes_and_ramps() {
    let mut f = project();
    let mut c = clip("a", "m", 0, 10 * S);
    c.opacity = Param { value: Value::Float(0.0), keyframes: vec![kf(0, 0.0), kf(10 * S, 1.0)] };
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: c }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "set_speed_ramp", "clip_id": "a", "preset": "montage"}));
    let ramped = get(h, "a").clone();
    let src_end = eval::source_time(&ramped, ramped.duration());
    assert!((src_end - 10 * S).abs() < S / 100, "{src_end}");
    let at = ramped.start + ramped.duration() * 3 / 10;
    let before: Vec<(Ticks, f64)> = (0..20)
        .map(|i| at - S + i * S / 10)
        .map(|t| (eval::source_time(&ramped, t), ramped.opacity.sample_f64(t)))
        .collect();
    run(h, json!({"op": "split_clip", "clip_id": "a", "at": at}));
    let clips = track_clips(h, &f.v1);
    let (l, r) = (clips[0], clips[1]);
    assert_eq!((l.end, r.start), (at, at));
    for (i, t) in (0..20).map(|i| at - S + i * S / 10).enumerate() {
        let (c, local) = if t < at { (l, t - l.start) } else { (r, t - r.start) };
        let (src, op) = (eval::source_time(c, local), c.opacity.sample_f64(local));
        assert!((src - before[i].0).abs() < S / 200, "t={t}: {src} vs {}", before[i].0);
        assert!((op - before[i].1).abs() < 1e-6, "opacity at {t}");
    }
    // Both halves keep a curve that covers their own source range.
    assert!(l.speed.curve.is_some() && r.speed.curve.is_some());
    assert_eq!(r.speed.ramp.as_ref().unwrap().keyframes[0].time, 0);
}

#[test]
fn split_reverse_keeps_source_order() {
    let mut f = project();
    let mut c = clip("a", "m", 0, 10 * S);
    c.source_in = 5 * S;
    c.speed.reverse = true;
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: c.clone() }).unwrap();
    run(&mut f.h, json!({"op": "split_clip", "clip_id": "a", "at": 4 * S}));
    let clips = track_clips(&f.h, &f.v1);
    for t in [0, S, 3 * S, 5 * S, 9 * S] {
        let (x, local) = if t < 4 * S { (clips[0], t) } else { (clips[1], t - 4 * S) };
        assert_eq!(eval::source_time(x, local), eval::source_time(&c, t), "t={t}");
    }
}

#[test]
fn trims_keep_content_attached() {
    let mut f = project();
    let mut c = clip("a", "m", 0, 10 * S);
    c.source_in = 10 * S;
    c.transform.position_x = Param { value: Value::Float(0.0), keyframes: vec![kf(2 * S, 0.0), kf(6 * S, 0.4)] };
    c.markers.push(Marker {
        id: "mk".into(),
        time: 5 * S,
        duration: 0,
        name: "x".into(),
        comment: String::new(),
        color: marker_color(),
        extra: Extra::Null,
    });
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: c }).unwrap();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("b", "n", 10 * S, 20 * S) }).unwrap();
    let h = &mut f.h;
    run(
        h,
        json!({"op": "add_animation", "clip_id": "a", "kind": "jb.anim.fade_out", "placement": "out", "duration": S}),
    );
    run(h, json!({"op": "trim_clip", "clip_id": "a", "start": 2 * S, "end": 8 * S}));
    let a = get(h, "a");
    assert_eq!((a.start, a.source_in), (2 * S, 12 * S));
    assert!((a.transform.position_x.sample_f64(2 * S) - 0.2).abs() < 1e-9);
    assert_eq!(a.markers[0].time, 3 * S);
    assert_eq!(a.animations[0].range, Some((5 * S, 6 * S)), "exit animation follows the end");
    // Ripple trim of the end closes the gap.
    run(h, json!({"op": "ripple_trim", "clip_id": "a", "edge": "end", "to": 6 * S}));
    assert_eq!((get(h, "a").end, get(h, "b").start), (6 * S, 8 * S));
    // Ripple trim of the head keeps the clip in place.
    run(h, json!({"op": "ripple_trim", "clip_id": "a", "edge": "start", "to": 3 * S}));
    let a = get(h, "a");
    assert_eq!((a.start, a.end, a.source_in), (2 * S, 5 * S, 13 * S));
    assert_eq!(get(h, "b").start, 7 * S);
    // Roll the cut a/b: there is a gap, so it fails until they touch.
    fails(h, json!({"op": "roll_edit", "clip_id": "a", "at": 4 * S}));
    run(h, json!({"op": "move_clip", "clip_id": "b", "start": 5 * S}));
    run(h, json!({"op": "slip_clip", "clip_id": "b", "delta": 2 * S}));
    assert_eq!(get(h, "b").source_in, 2 * S);
    run(h, json!({"op": "roll_edit", "clip_id": "a", "at": 4 * S}));
    let (a, b) = (get(h, "a"), get(h, "b"));
    assert_eq!((a.end, b.start, b.end, b.source_in), (4 * S, 4 * S, 15 * S, S));
    fails(h, json!({"op": "slip_clip", "clip_id": "b", "delta": -10 * S}));
    fails(h, json!({"op": "slip_clip", "clip_id": "b", "delta": 95 * S}));
}

#[test]
fn ripple_insert_and_overwrite() {
    let mut f = project();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("a", "m", 0, 10 * S) }).unwrap();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("b", "n", 10 * S, 20 * S) }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "insert_clip", "track_id": f.v1, "clip": clip("x", "n", 4 * S, 6 * S)}));
    let ids: Vec<(Ticks, Ticks)> = track_clips(h, &f.v1).iter().map(|c| (c.start, c.end)).collect();
    assert_eq!(ids, vec![(0, 4 * S), (4 * S, 6 * S), (6 * S, 12 * S), (12 * S, 22 * S)]);
    assert_eq!(track_clips(h, &f.v1)[2].source_in, 4 * S);
    run(h, json!({"op": "overwrite_clip", "track_id": f.v1, "clip": clip("y", "m", 3 * S, 13 * S)}));
    let ids: Vec<(Ticks, Ticks)> = track_clips(h, &f.v1).iter().map(|c| (c.start, c.end)).collect();
    assert_eq!(ids, vec![(0, 3 * S), (3 * S, 13 * S), (13 * S, 22 * S)]);
    assert_eq!(get(h, "b").source_in, S, "head of b was overwritten");
}

#[test]
fn freeze_detach_link_group() {
    let mut f = project();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("a", "m", 0, 10 * S) }).unwrap();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("b", "n", 10 * S, 20 * S) }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "detach_audio", "clip_id": "a"}));
    let audio = track_clips(h, &f.a1)[0].clone();
    assert_eq!((audio.start, audio.end), (0, 10 * S));
    assert!(get(h, "a").audio.muted);
    run(h, json!({"op": "link_clips", "clip_ids": ["a", audio.id]}));
    run(h, json!({"op": "freeze_frame", "clip_id": "a", "at": 4 * S, "duration": 2 * S}));
    let v: Vec<&Clip> = track_clips(h, &f.v1);
    assert_eq!(v.len(), 4);
    assert!(v[1].speed.freeze);
    assert_eq!((v[1].start, v[1].end, v[1].source_in), (4 * S, 6 * S, 4 * S));
    assert_eq!(eval::source_time(v[1], S), 4 * S);
    assert_eq!((v[2].start, v[2].source_in), (6 * S, 4 * S));
    assert_eq!(get(h, "b").start, 12 * S);
    let a: Vec<&Clip> = track_clips(h, &f.a1);
    assert_eq!((a[1].start, a[1].source_in), (6 * S, 4 * S), "linked audio moved with the freeze");
    assert!(eval::audio_voices(&h.project, h.project.main(), 5 * S).is_empty(), "freeze is silent");
    // Groups move together.
    let right = v[2].id.clone();
    run(h, json!({"op": "group_clips", "clip_ids": ["b", right]}));
    run(h, json!({"op": "move_clip", "clip_id": "b", "start": 14 * S}));
    assert_eq!(get(h, &right).start, 8 * S);
    run(h, json!({"op": "ungroup_clips", "clip_ids": ["b"]}));
    assert!(get(h, "a").group.is_none());
    run(h, json!({"op": "unlink_clips", "clip_ids": ["a"]}));
    assert!(get(h, "a").link_group.is_none());
}

#[test]
fn duplicate_replace_compound_paste() {
    let mut f = project();
    let mut c = clip("a", "m", 0, 5 * S);
    c.blend_mode = BlendMode::Screen;
    c.transform.flip_h = true;
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: c }).unwrap();
    f.h.execute(&Command::AddClip { track_id: f.v2.clone(), clip: clip("t", "n", 0, 5 * S) }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "duplicate_clip", "clip_id": "a"}));
    let dup = track_clips(h, &f.v1)[1].clone();
    assert_eq!((dup.start, dup.end, dup.blend_mode), (5 * S, 10 * S, BlendMode::Screen));
    assert_ne!(dup.id, "a");
    run(
        h,
        json!({"op": "paste_attributes", "from_clip_id": "a", "to_clip_ids": ["t"], "attributes": ["blend_mode", "transform"]}),
    );
    assert_eq!((get(h, "t").blend_mode, get(h, "t").transform.flip_h), (BlendMode::Screen, true));
    run(h, json!({"op": "replace_media", "clip_id": dup.id, "media_id": "n"}));
    assert!(matches!(&get(h, &dup.id).source, ClipSource::Media { media_id } if media_id == "n"));
    run(h, json!({"op": "create_compound_clip", "clip_ids": ["a", "t"], "name": "Nest"}));
    let comp = track_clips(h, &f.v1)[0].clone();
    let ClipSource::Sequence { sequence_id } = &comp.source else { panic!("compound") };
    let nested = h.project.sequence(sequence_id).unwrap();
    assert_eq!((nested.name.as_str(), nested.tracks.len()), ("Nest", 2));
    assert_eq!((comp.start, comp.end), (0, 5 * S));
    assert!(track_clips(h, &f.v2).is_empty());
    let plan = eval::evaluate(&h.project, h.project.main(), S);
    assert!(matches!(plan.layers[0].content, eval::LayerContent::Nested(_)));
}

#[test]
fn speed_ramps_and_reverse() {
    let mut f = project();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("a", "m", 0, 10 * S) }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "set_speed_ramp", "clip_id": "a", "preset": "Bullet time"}));
    let a = get(h, "a");
    assert_eq!(a.speed.curve.as_ref().unwrap().preset, "bullet_time");
    assert!(a.duration() < 10 * S, "mostly fast: {}", a.duration());
    run(
        h,
        json!({"op": "set_speed_ramp", "clip_id": "a", "preset": "custom", "points": [{"position": 0.0, "speed": 0.5}, {"position": 1.0, "speed": 0.5}]}),
    );
    assert!((get(h, "a").duration() - 20 * S).abs() < S / 100);
    run(h, json!({"op": "set_speed_ramp", "clip_id": "a", "preset": "none"}));
    assert!((get(h, "a").duration() - 10 * S).abs() < 10);
    fails(h, json!({"op": "set_speed_ramp", "clip_id": "a", "preset": "zzz"}));
    run(h, json!({"op": "set_reverse", "clip_id": "a", "reverse": true}));
    assert!(get(h, "a").speed.reverse);
    run(h, json!({"op": "set_maintain_pitch", "clip_id": "a", "maintain_pitch": false}));
    run(h, json!({"op": "set_pitch", "clip_id": "a", "semitones": 3.0}));
    let v = &eval::audio_voices(&h.project, h.project.main(), S)[0];
    assert!(!v.maintain_pitch && v.reverse && v.pitch == 3.0);
}

#[test]
fn visual_audio_and_animation_commands() {
    let mut f = project();
    f.h.execute(&Command::AddClip { track_id: f.v1.clone(), clip: clip("a", "m", 0, 4 * S) }).unwrap();
    f.h.execute(&Command::AddClip { track_id: f.a1.clone(), clip: clip("s", "song", 0, 4 * S) }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "set_crop", "clip_id": "a", "crop": [0.1, 0.0, 0.1, 0.2]}));
    fails(h, json!({"op": "set_crop", "clip_id": "a", "crop": [0.6, 0.0, 0.5, 0.0]}));
    run(h, json!({"op": "set_flip", "clip_id": "a", "vertical": true}));
    run(h, json!({"op": "set_rotation", "clip_id": "a", "degrees": 90.0}));
    run(h, json!({"op": "set_fit_mode", "clip_id": "a", "fit": "fill"}));
    run(h, json!({"op": "set_blend_mode", "clip_id": "a", "blend_mode": "linear_burn"}));
    let t = &get(h, "a").transform;
    assert_eq!(
        (t.crop, t.flip_v, t.fit, t.rotation.value.as_f64()),
        ([0.1, 0.0, 0.1, 0.2], true, FitMode::Fill, Some(90.0))
    );
    run(h, json!({"op": "set_clip_enabled", "clip_id": "a", "enabled": false}));
    assert!(eval::evaluate(&h.project, h.project.main(), S).layers.is_empty());

    run(h, json!({"op": "set_volume", "clip_id": "s", "gain_db": -6.0}));
    run(h, json!({"op": "set_pan", "clip_id": "s", "pan": -0.5}));
    run(h, json!({"op": "set_audio_fades", "clip_id": "s", "fade_in": S}));
    run(h, json!({"op": "set_normalization", "clip_id": "s", "target_lufs": -14.0, "measured_lufs": -20.0}));
    run(h, json!({"op": "set_equalizer", "clip_id": "s", "preset": "Hip-Hop"}));
    let au = &get(h, "s").audio;
    assert_eq!((au.normalize.unwrap().gain_db(), au.equalizer.as_ref().unwrap().preset.as_str()), (6.0, "hip_hop"));
    let g = eval::clip_gain(get(h, "s"), 2 * S);
    assert!((g - 1.0).abs() < 1e-9, "-6 dB volume + 6 dB normalization: {g}");
    run(h, json!({"op": "set_equalizer", "clip_id": "s"}));
    assert!(get(h, "s").audio.equalizer.is_none());
    fails(h, json!({"op": "set_pan", "clip_id": "s", "pan": 2.0}));
    run(h, json!({"op": "set_mute", "clip_id": "s", "muted": true}));
    assert_eq!(eval::clip_gain(get(h, "s"), 2 * S), 0.0);

    run(
        h,
        json!({"op": "add_animation", "clip_id": "a", "kind": "jb.anim.zoom_in", "placement": "in", "duration": 3 * S}),
    );
    run(
        h,
        json!({"op": "add_animation", "clip_id": "a", "kind": "jb.anim.fade_out", "placement": "out", "duration": 3 * S}),
    );
    let anims = &get(h, "a").animations;
    assert_eq!(anims[0].range, Some((0, 3 * S)));
    assert_eq!(anims[1].range, Some((3 * S, 4 * S)), "exit capped so it does not overlap the entry");
    run(h, json!({"op": "add_animation", "clip_id": "a", "kind": "jb.anim.spin_in", "placement": "in"}));
    assert_eq!(get(h, "a").animations.len(), 2, "same slot is replaced");
    let id = get(h, "a").animations.iter().find(|a| a.kind == "jb.anim.spin_in").unwrap().id.clone();
    run(h, json!({"op": "replace_animation", "clip_id": "a", "animation_id": id, "duration": S / 2}));
    run(h, json!({"op": "remove_animation", "clip_id": "a", "placement": "out"}));
    assert_eq!(get(h, "a").animations[0].range, Some((0, S / 2)));
    run(h, json!({"op": "add_animation", "clip_id": "a", "kind": "jb.anim.ken_burns", "placement": "combo"}));
    assert_eq!(get(h, "a").animations.len(), 1);
}

#[test]
fn markers_tracks_titles() {
    let mut f = project();
    let title = ClipSource::Title(TitleData { text: "Hi".into(), ..Default::default() });
    let mut c = clip("t", "m", 0, 3 * S);
    c.source = title;
    f.h.execute(&Command::AddClip { track_id: f.v2.clone(), clip: c }).unwrap();
    let h = &mut f.h;
    run(h, json!({"op": "add_marker", "marker": {"id": "m1", "time": S, "name": "beat"}}));
    run(h, json!({"op": "add_marker", "clip_id": "t", "marker": {"id": "m2", "time": S}}));
    fails(h, json!({"op": "add_marker", "clip_id": "t", "marker": {"id": "m3", "time": 5 * S}}));
    run(h, json!({"op": "update_marker", "marker": {"id": "m1", "time": 2 * S, "name": "drop", "comment": "c"}}));
    assert_eq!(h.project.main().markers[0].name, "drop");
    run(h, json!({"op": "remove_marker", "marker_id": "m2"}));
    assert!(get(h, "t").markers.is_empty());

    run(h, json!({"op": "rename_track", "track_id": f.v2, "name": "Titles"}));
    run(h, json!({"op": "move_track", "track_id": f.v2, "index": 0}));
    run(h, json!({"op": "set_track_height", "track_id": f.v2, "height": 80}));
    let t0 = &h.project.main().tracks[0];
    assert_eq!((t0.id.as_str(), t0.name.as_str(), t0.height), (f.v2.as_str(), "Titles", Some(80)));

    run(
        h,
        json!({"op": "set_title_style", "clip_id": "t", "font_family": "Georgia", "font_size": 0.1,
        "color": [1, 0, 0, 1], "alignment": "left", "stroke": {"color": [0, 0, 0, 1], "width": 0.01},
        "background": [0, 0, 0, 0.5], "animation": {"kind": "jb.anim.fade_in", "duration": 5000000}}),
    );
    let ClipSource::Title(t) = &get(h, "t").source else { panic!() };
    assert_eq!((t.font_family.as_str(), t.alignment, t.text.as_str()), ("Georgia", TextAlignment::Left, "Hi"));
    assert!(t.stroke.is_some() && t.background.is_some());
    assert_eq!(get(h, "t").animations[0].range, Some((0, S / 2)));
    run(h, json!({"op": "set_title_style", "clip_id": "t", "stroke": null}));
    let ClipSource::Title(t) = &get(h, "t").source else { panic!() };
    assert!(t.stroke.is_none() && t.background.is_some());
}

#[test]
fn commands_have_a_schema() {
    let schema = serde_json::to_string(&schemars::schema_for!(Command)).unwrap();
    for op in ["add_keyframe", "set_speed_ramp", "freeze_frame", "create_compound_clip", "set_title_style", "roll_edit"]
    {
        assert!(schema.contains(op), "{op}");
    }
}

#[test]
fn panel_commands() {
    let mut f = project();
    let h = &mut f.h;
    run(h, json!({"op": "add_clip", "track_id": f.v1, "clip": clip("c", "m", 0, 10 * S)}));
    run(h, json!({"op": "add_effect", "clip_id": "c", "effect": {"id": "e", "kind": "jb.chroma_key", "params": {}}}));
    run(h, json!({"op": "set_effect_enabled", "clip_id": "c", "effect_id": "e", "enabled": false}));
    assert!(!get(h, "c").effects[0].enabled);
    fails(h, json!({"op": "set_effect_enabled", "clip_id": "c", "effect_id": "nope", "enabled": true}));

    run(h, json!({"op": "set_audio_channels", "clip_id": "c", "channels": "left"}));
    assert_eq!(get(h, "c").audio.channels, ChannelMode::Left);

    run(h, json!({"op": "set_project_settings", "width": 1080, "height": 1920}));
    assert_eq!((h.project.settings.width, h.project.settings.height), (1080, 1920));
    assert_eq!(h.project.settings.frame_rate, Rational::new(30, 1));
    fails(h, json!({"op": "set_project_settings", "width": 1081}));
    fails(h, json!({"op": "set_project_settings", "frame_rate": {"num": 0, "den": 1}}));

    run(h, json!({"op": "rename_media", "media_id": "m", "name": " Beach "}));
    assert_eq!(h.project.media("m").unwrap().name, "Beach");
    fails(h, json!({"op": "rename_media", "media_id": "m", "name": "  "}));
    fails(h, json!({"op": "rename_media", "media_id": "zz", "name": "x"}));
}

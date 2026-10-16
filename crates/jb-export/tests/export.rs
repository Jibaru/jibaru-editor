// Shares the clip generators of jb-media's tests.
#[path = "../../jb-media/tests/common/mod.rs"]
mod common;

use common::*;
use jb_core::model::*;
use jb_core::time::{Rational, TICKS_PER_SECOND};
use jb_export::{export, CpuCompositor, ExportSettings};
use jb_media::audio::AudioDecoder;
use jb_media::{DecodeMode, MediaPool, PoolConfig, VideoDecoder};
use std::path::Path;

fn project(w: u32, h: u32) -> Project {
    Project::new(
        "t",
        ProjectSettings {
            width: w,
            height: h,
            frame_rate: Rational::new(FPS, 1),
            sample_rate: 48_000,
            background: [0.0, 0.0, 0.0, 1.0],
        },
    )
}

fn add_media(p: &mut Project, id: &str, path: &Path) {
    let info = jb_media::probe_info(path).unwrap();
    p.media.push(info.to_asset(id));
}

fn add_track(p: &mut Project, kind: TrackKind, clips: Vec<serde_json::Value>) {
    let n = p.main().tracks.len();
    let track: Track = serde_json::from_value(serde_json::json!({
        "id": format!("track{n}"), "kind": kind, "name": "t", "clips": clips
    }))
    .unwrap();
    let id = p.main_sequence.clone();
    p.sequence_mut(&id).unwrap().tracks.push(track);
}

fn media_clip(id: &str, media: &str, start: i64, end: i64) -> serde_json::Value {
    serde_json::json!({"id": id, "name": id, "source": {"type": "media", "media_id": media}, "start": start, "end": end})
}

fn run_export(
    p: &Project,
    settings: &ExportSettings,
    progress: &mut dyn FnMut(&jb_export::ExportProgress) -> bool,
) -> anyhow::Result<jb_export::ExportReport> {
    let pool = MediaPool::for_project(p, PoolConfig::default());
    let mut comp = CpuCompositor::new(&pool, p.settings.width, p.settings.height);
    export(p, &p.main_sequence, settings, &pool, &mut |plan| comp.render(plan).unwrap(), progress)
}

fn rms(s: &[f32]) -> f64 {
    (s.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / s.len().max(1) as f64).sqrt()
}

#[test]
fn exports_video_and_audio_end_to_end() {
    let mut p = project(320, 240);
    add_media(&mut p, "m", &index_clip());
    add_track(&mut p, TrackKind::Video, vec![media_clip("a", "m", 0, 2 * TICKS_PER_SECOND)]);
    let out = out("export-basic.mp4");
    let mut calls = 0;
    let report = run_export(&p, &ExportSettings::new(&out), &mut |pr| {
        calls += 1;
        assert!(pr.frame <= pr.total_frames);
        true
    })
    .unwrap();
    eprintln!("{report:?}");
    assert_eq!(report.frames, FRAMES as u64);
    assert_eq!(calls, FRAMES);

    let info = jb_media::probe_info(&out).unwrap();
    assert_eq!((info.width, info.height), (320, 240));
    assert_eq!(info.frame_rate, Some(Rational::new(25, 1)));
    assert!(info.has_audio);
    assert_eq!(info.sample_rate, 48_000);
    assert!((info.duration - 2 * TICKS_PER_SECOND).abs() < TICKS_PER_SECOND / 10, "{}", info.duration);
    let rate = Rational::new(FPS, 1);
    let mut d = VideoDecoder::open(&out, true).unwrap();
    for i in [0, 1, 17, 30, 49] {
        let f = d.frame_at(rate.frame_start(i), None, DecodeMode::Accurate).unwrap().unwrap();
        let got = index_from_luma(mean_luma(&f));
        assert!((got - i).abs() <= 1, "frame {i} looks like {got}");
    }
    // The 440 Hz tone (lavfi sine, amplitude 1/8) is present throughout.
    let mut a = AudioDecoder::open(&out, 48_000).unwrap();
    let mid = a.read(TICKS_PER_SECOND, 4800).unwrap();
    assert!((rms(&mid) - 0.088).abs() < 0.02, "{}", rms(&mid));
    cleanup(&out);
}

#[test]
fn speed_change_resamples_audio() {
    let mut p = project(64, 48);
    add_media(&mut p, "tone", &late_tone());
    add_track(
        &mut p,
        TrackKind::Video,
        vec![serde_json::json!({"id": "c", "name": "c", "source": {"type": "color", "color": [1.0, 0.0, 0.0, 1.0]},
            "start": 0, "end": TICKS_PER_SECOND})],
    );
    let mut clip = media_clip("s", "tone", 0, TICKS_PER_SECOND);
    clip["speed"] = serde_json::json!({"rate": 2.0});
    clip["audio"] = serde_json::json!({"gain_db": {"value": {"type": "float", "value": -6.0206}},
        "pan": {"value": {"type": "float", "value": 0.0}}, "fade_in": 0, "fade_out": 0});
    add_track(&mut p, TrackKind::Audio, vec![clip]);
    let out = out("export-speed.mp4");
    run_export(&p, &ExportSettings::new(&out), &mut |_| true).unwrap();

    let samples = jb_media::audio::decode_all(&out, 48_000).unwrap();
    let left: Vec<f32> = samples.chunks(2).map(|c| c[0]).collect();
    // The tone started at 1 s of the source: 0.5 s on the 2x timeline.
    let onset = left.iter().position(|x| x.abs() > 0.05).unwrap() as f64 / 48_000.0;
    assert!((onset - 0.5).abs() < 0.01, "onset {onset}");
    // 1 kHz at 2x is 2 kHz.
    let w = &left[(0.6 * 48_000.0) as usize..(0.8 * 48_000.0) as usize];
    let crossings = w.windows(2).filter(|p| (p[0] < 0.0) != (p[1] < 0.0)).count();
    assert!((crossings as f64 - 800.0).abs() < 40.0, "{crossings} zero crossings");
    // 0.5 amplitude at -6 dB: RMS 0.177.
    assert!((rms(w) - 0.177).abs() < 0.03, "{}", rms(w));
    // Red canvas.
    let f = VideoDecoder::open(&out, true).unwrap().frame_at(0, None, DecodeMode::Accurate).unwrap().unwrap();
    assert!((mean_luma(&f) - 63.0).abs() < 4.0, "{}", mean_luma(&f));
    cleanup(&out);
}

#[test]
fn exports_a_range() {
    let mut p = project(320, 240);
    add_media(&mut p, "m", &index_clip());
    add_track(&mut p, TrackKind::Video, vec![media_clip("a", "m", 0, 2 * TICKS_PER_SECOND)]);
    let out = out("export-range.mp4");
    let mut s = ExportSettings::new(&out);
    s.range = Some((TICKS_PER_SECOND * 2 / 5, TICKS_PER_SECOND * 6 / 5));
    s.audio = false;
    let r = run_export(&p, &s, &mut |_| true).unwrap();
    assert_eq!(r.frames, 20);
    let info = jb_media::probe_info(&out).unwrap();
    assert!(!info.has_audio);
    let f = VideoDecoder::open(&out, true).unwrap().frame_at(0, None, DecodeMode::Accurate).unwrap().unwrap();
    assert!((index_from_luma(mean_luma(&f)) - 10).abs() <= 1);
    cleanup(&out);
}

#[test]
fn cancel_removes_partial_output() {
    let mut p = project(320, 240);
    add_media(&mut p, "m", &index_clip());
    add_track(&mut p, TrackKind::Video, vec![media_clip("a", "m", 0, 2 * TICKS_PER_SECOND)]);
    let out = out("export-cancel.mp4");
    let err = run_export(&p, &ExportSettings::new(&out), &mut |pr| pr.frame < 5).unwrap_err();
    assert!(err.to_string().contains("cancel"), "{err}");
    assert!(!out.exists());
    assert!(!out.with_extension("partial.mp4").exists());
}

#[test]
fn software_encoder_fallback_works() {
    let mut p = project(320, 240);
    add_media(&mut p, "m", &index_clip());
    add_track(&mut p, TrackKind::Video, vec![media_clip("a", "m", 0, TICKS_PER_SECOND)]);
    let out = out("export-sw.mp4");
    let mut s = ExportSettings::new(&out);
    s.hardware = false;
    let r = run_export(&p, &s, &mut |_| true).unwrap();
    assert!(!r.hardware_encoder);
    assert!(r.encoder == "libopenh264" || r.encoder == "mpeg4", "{}", r.encoder);
    let f = VideoDecoder::open(&out, false).unwrap().frame_at(Rational::new(FPS, 1).frame_start(12), None, DecodeMode::Accurate);
    assert!((index_from_luma(mean_luma(&f.unwrap().unwrap())) - 12).abs() <= 1);
    cleanup(&out);
}

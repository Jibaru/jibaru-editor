mod common;

use common::*;
use jb_core::frame::{FrameSource, PixelFormat};
use jb_core::model::MediaKind;
use jb_core::time::{Rational, TICKS_PER_SECOND};
use jb_media::{DecodeMode, MediaCache, MediaPool, PoolConfig, ProxyOptions, VideoDecoder};

const RATE: Rational = Rational::new(FPS, 1);

#[test]
fn probe_reports_stream_properties() {
    let info = jb_media::probe_info(index_clip()).unwrap();
    assert_eq!(info.kind, MediaKind::Video);
    assert_eq!((info.width, info.height), (320, 240));
    assert_eq!(info.frame_rate, Some(Rational::new(25, 1)));
    assert!(info.has_audio);
    assert_eq!(info.sample_rate, 44100);
    assert_eq!(info.rotation, 0);
    assert!((info.duration - 2 * TICKS_PER_SECOND).abs() < TICKS_PER_SECOND / 10, "{}", info.duration);
    let asset = jb_media::probe(index_clip()).unwrap();
    assert_eq!(asset.width, 320);
    assert!(asset.has_audio);
}

#[test]
fn sequential_decode_hits_every_frame_without_seeking() {
    let mut d = VideoDecoder::open(&index_clip(), true).unwrap();
    for i in 0..FRAMES {
        let f = d.frame_at(RATE.frame_start(i), None, DecodeMode::Accurate).unwrap().unwrap();
        assert_eq!(f.format, PixelFormat::Nv12);
        assert_eq!(index_from_luma(mean_luma(&f)), i, "frame {i}");
        assert_eq!(f.pts, RATE.frame_start(i));
    }
    assert_eq!(d.stats.seeks, 1, "only the initial seek");
    assert_eq!(d.stats.frames_decoded, FRAMES as u64);
}

#[test]
fn random_access_is_frame_accurate() {
    let mut d = VideoDecoder::open(&index_clip(), true).unwrap();
    for i in [40, 3, 27, 26, 49, 0, 12, 13, 38] {
        // The nearest frame is shown: 0.4 frame in selects frame i, 0.6 in
        // the next one.
        let t = RATE.frame_start(i) + RATE.frame_duration() * 2 / 5;
        let f = d.frame_at(t, None, DecodeMode::Accurate).unwrap().unwrap();
        assert_eq!(index_from_luma(mean_luma(&f)), i, "frame {i}");
        let t = RATE.frame_start(i) - RATE.frame_duration() * 2 / 5;
        let f = d.frame_at(t.max(0), None, DecodeMode::Accurate).unwrap().unwrap();
        assert_eq!(index_from_luma(mean_luma(&f)), i, "frame {i} from 0.6 of the previous one");
    }
    // Past the end holds the last frame.
    let f = d.frame_at(10 * TICKS_PER_SECOND, None, DecodeMode::Accurate).unwrap().unwrap();
    assert_eq!(index_from_luma(mean_luma(&f)), FRAMES - 1);
}

#[test]
fn max_size_scales_down_in_decode() {
    let mut d = VideoDecoder::open(&index_clip(), true).unwrap();
    let f = d.frame_at(0, Some((160, 160)), DecodeMode::Accurate).unwrap().unwrap();
    assert_eq!((f.width, f.height), (160, 120));
    assert_eq!(f.data.len(), 160 * 120 * 3 / 2);
    assert_eq!(index_from_luma(mean_luma(&f)), 0);
}

#[test]
fn ten_bit_decodes_to_p010_and_alpha_to_rgba() {
    let mut d = VideoDecoder::open(&ten_bit_clip(), false).unwrap();
    let f = d.frame_at(0, None, DecodeMode::Accurate).unwrap().unwrap();
    assert_eq!(f.format, PixelFormat::P010);
    assert_eq!(f.stride, 128 * 2);
    assert!((mean_luma(&f) - 126.0).abs() < 4.0, "{}", mean_luma(&f));

    let mut d = VideoDecoder::open(&alpha_clip(), false).unwrap();
    let f = d.frame_at(0, None, DecodeMode::Accurate).unwrap().unwrap();
    assert_eq!(f.format, PixelFormat::Rgba8);
    let px = &f.data[0..4];
    assert!(px[0] > 240 && px[1] < 10 && (px[3] as i32 - 128).abs() < 3, "{px:?}");
}

#[test]
fn display_rotation_is_applied() {
    let info = jb_media::probe_info(rotated_clip()).unwrap();
    assert_eq!(info.rotation, 270, "90 degrees counter-clockwise");
    assert_eq!((info.width, info.height), (240, 320));
    let mut d = VideoDecoder::open(&rotated_clip(), true).unwrap();
    let f = d.frame_at(RATE.frame_start(7), None, DecodeMode::Accurate).unwrap().unwrap();
    assert_eq!((f.width, f.height), (240, 320));
    assert_eq!(index_from_luma(mean_luma(&f)), 7);
}

#[test]
fn hardware_hevc_decode() {
    let Some(clip) = index_clip_hevc() else {
        eprintln!("no hardware HEVC encoder in the ffmpeg CLI; skipping");
        return;
    };
    let mut d = VideoDecoder::open(&clip, true).unwrap();
    eprintln!("hevc decoder: {:?}", d.hardware());
    // CI virtual machines may not expose VideoToolbox.
    if cfg!(target_os = "macos") && std::env::var_os("CI").is_none() {
        assert_eq!(d.hardware(), Some("videotoolbox"));
    }
    for i in [0, 1, 2, 30, 10, 49] {
        let f = d.frame_at(RATE.frame_start(i), None, DecodeMode::Accurate).unwrap().unwrap();
        assert_eq!(f.format, PixelFormat::Nv12);
        // Lossy HEVC: allow one step of error.
        assert!((index_from_luma(mean_luma(&f)) - i).abs() <= 1, "frame {i}: {}", mean_luma(&f));
    }
}

#[test]
fn software_fallback_matches() {
    let mut d = VideoDecoder::open(&index_clip(), false).unwrap();
    assert_eq!(d.hardware(), None);
    let f = d.frame_at(RATE.frame_start(33), None, DecodeMode::Accurate).unwrap().unwrap();
    assert_eq!(index_from_luma(mean_luma(&f)), 33);
}

#[test]
fn pool_serves_frames_and_bounds_decoders() {
    let pool = MediaPool::new(PoolConfig { max_video_decoders: 2, ..Default::default() });
    pool.add("a", index_clip());
    pool.add("b", index_clip_large());
    pool.add("c", rotated_clip());
    for i in 0..10 {
        let t = RATE.frame_start(i);
        for id in ["a", "b", "c"] {
            let f = pool.frame(id, t, None).unwrap().unwrap();
            assert_eq!(index_from_luma(mean_luma(&f)), i, "{id} frame {i}");
        }
    }
    let st = pool.stats();
    assert!(st.open_video_decoders <= 2, "{st:?}");
    assert!(st.video_decoders_evicted > 0);

    // A repeated request is served from the frame ring.
    let before = pool.stats().ring_hits;
    pool.frame("a", RATE.frame_start(9), None).unwrap().unwrap();
    assert!(pool.stats().ring_hits > before || pool.stats().open_video_decoders <= 2);
    assert!(pool.frame("missing", 0, None).is_err());
}

#[test]
fn pool_alternating_times_use_two_decoders() {
    // A transition between two parts of the same clip must not thrash.
    let pool = MediaPool::new(PoolConfig::default());
    pool.add("a", index_clip());
    for i in 0..20 {
        let fa = pool.frame("a", RATE.frame_start(i), None).unwrap().unwrap();
        let fb = pool.frame("a", RATE.frame_start(i + 28), None).unwrap().unwrap();
        assert_eq!(index_from_luma(mean_luma(&fa)), i);
        assert_eq!(index_from_luma(mean_luma(&fb)), i + 28);
    }
    assert_eq!(pool.stats().video_decoders_opened, 2);
}

#[test]
fn images_load_as_rgba_with_max_size() {
    let p = out("still.png");
    let img = image::RgbaImage::from_pixel(400, 200, image::Rgba([10, 200, 30, 255]));
    img.save(&p).unwrap();
    let info = jb_media::probe_info(&p).unwrap();
    assert_eq!(info.kind, MediaKind::Image);
    assert_eq!((info.width, info.height), (400, 200));
    let pool = MediaPool::new(PoolConfig::default());
    pool.add("img", &p);
    let f = pool.frame("img", 123, Some((100, 100))).unwrap().unwrap();
    assert_eq!(f.format, PixelFormat::Rgba8);
    assert_eq!((f.width, f.height), (100, 50));
    assert_eq!(&f.data[0..4], &[10, 200, 30, 255]);
    let full = pool.frame("img", 0, None).unwrap().unwrap();
    assert_eq!(full.width, 400);
    cleanup(&p);
}

fn rms(s: &[f32]) -> f64 {
    (s.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / s.len().max(1) as f64).sqrt()
}

#[test]
fn audio_is_resampled_and_sample_accurate() {
    let pool = MediaPool::new(PoolConfig { sample_rate: 44_100, ..Default::default() });
    pool.add("t", late_tone());
    // Silence then tone: amplitude 0.5 sine -> RMS 0.354 on each channel.
    let quiet = pool.audio_samples("t", TICKS_PER_SECOND / 2, 4410).unwrap();
    assert_eq!(quiet.len(), 4410 * 2);
    assert!(rms(&quiet) < 0.01, "{}", rms(&quiet));
    let loud = pool.audio_samples("t", TICKS_PER_SECOND * 3 / 2, 4410).unwrap();
    assert!((rms(&loud) - 0.354).abs() < 0.03, "{}", rms(&loud));
    // Onset lands within a few ms of 1.0 s even after random access.
    let around = pool.audio_samples("t", TICKS_PER_SECOND * 9 / 10, 8820).unwrap();
    let onset = around.chunks(2).position(|p| p[0].abs() > 0.05).unwrap();
    let onset_t = 0.9 + onset as f64 / 44_100.0;
    assert!((onset_t - 1.0).abs() < 0.005, "onset at {onset_t}");
    // Before the start and past the end is silence.
    let tail = pool.audio_samples("t", 5 * TICKS_PER_SECOND, 100).unwrap();
    assert!(tail.iter().all(|x| *x == 0.0));
}

#[test]
fn thumbnails_and_waveform_are_cached_on_disk() {
    let cache = MediaCache::new(out("cache")).unwrap();
    let thumbs = cache.filmstrip(&index_clip(), &[0, TICKS_PER_SECOND, TICKS_PER_SECOND / 2], 60).unwrap();
    assert_eq!(thumbs.len(), 3);
    for t in &thumbs {
        assert!(t.path.exists());
        assert_eq!((t.width, t.height), (80, 60));
    }
    let mtime = std::fs::metadata(&thumbs[0].path).unwrap().modified().unwrap();
    let again = cache.thumbnail(&index_clip(), 0, 60).unwrap();
    assert_eq!(again.path, thumbs[0].path);
    assert_eq!(std::fs::metadata(&again.path).unwrap().modified().unwrap(), mtime);

    let w = cache.waveform(&late_tone()).unwrap();
    assert_eq!(w.rate, jb_media::waveform::PEAKS_PER_SECOND);
    assert!((w.duration() - 2 * TICKS_PER_SECOND).abs() < TICKS_PER_SECOND / 10);
    let b = w.buckets(0, 2 * TICKS_PER_SECOND, 4);
    assert!(b[0][1] < 0.02 && b[3][1] > 0.4, "{b:?}");
    let reloaded = jb_media::Waveform::load(
        &cache.dir().join("waveforms").join(format!("{}.peaks", MediaCache::key(&late_tone()).unwrap())),
    )
    .unwrap();
    assert_eq!(reloaded.min.len(), w.min.len());
    cleanup(cache.dir());
}

#[test]
fn proxy_keeps_frame_timing() {
    let src = index_clip_large();
    let dst = out("proxy.mp4");
    let report =
        jb_media::generate_proxy(&src, &dst, &ProxyOptions { height: 240, ..Default::default() }, &mut |_| true)
            .unwrap();
    eprintln!("proxy: {report:?}");
    assert_eq!((report.width, report.height), (320, 240));
    assert_eq!(report.frames, FRAMES as u64);
    let info = jb_media::probe_info(&dst).unwrap();
    assert_eq!((info.width, info.height), (320, 240));
    let mut a = VideoDecoder::open(&src, true).unwrap();
    let mut b = VideoDecoder::open(&dst, true).unwrap();
    for i in 0..FRAMES {
        let t = RATE.frame_start(i);
        let fa = a.frame_at(t, None, DecodeMode::Accurate).unwrap().unwrap();
        let fb = b.frame_at(t, None, DecodeMode::Accurate).unwrap().unwrap();
        assert_eq!(fa.pts, fb.pts, "frame {i}");
        assert!((index_from_luma(mean_luma(&fb)) - i).abs() <= 1, "frame {i}");
    }
    cleanup(&dst);
}

#[test]
fn pool_uses_proxy_for_previews_only() {
    let src = index_clip_large();
    let dst = out("proxy-pool.mp4");
    jb_media::generate_proxy(&src, &dst, &ProxyOptions { height: 240, ..Default::default() }, &mut |_| true).unwrap();
    let pool = MediaPool::new(PoolConfig::default());
    pool.add("m", &src);
    pool.set_proxy("m", Some(dst.clone()));
    let preview = pool.frame("m", RATE.frame_start(5), Some((1000, 1000))).unwrap().unwrap();
    assert_eq!(preview.width, 320, "proxy resolution");
    let full = pool.frame("m", RATE.frame_start(5), None).unwrap().unwrap();
    assert_eq!(full.width, 640, "original for export");
    pool.clear();
    cleanup(&dst);
}

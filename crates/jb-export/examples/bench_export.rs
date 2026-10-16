//! Export throughput benchmark:
//! `cargo run --release -p jb-export --example bench_export -- <clip> [seconds] [h264|hevc] [WxH|source] [rgba]`
//!
//! Builds a one-clip project at the clip's frame rate and exports it with
//! the CPU compositor (decode -> compose -> convert -> hardware encode -> mux).
//! With `rgba` the compositor output is converted to RGBA first, like the
//! GPU renderer's readback, so the encoder stage pays the RGBA -> NV12 cost.

use jb_core::frame::ColorSpace;
use jb_core::model::*;
use jb_core::time::TICKS_PER_SECOND;
use jb_core::PixelFormat;
use jb_export::{export, CpuCompositor, ExportSettings, VideoCodec};
use jb_media::convert::{convert_frame, Scaler};
use jb_media::{MediaPool, PoolConfig};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = std::path::PathBuf::from(
        args.next().expect("usage: bench_export <clip> [seconds] [h264|hevc] [WxH|source] [rgba]"),
    );
    let seconds: f64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(10.0);
    let codec = match args.next().as_deref() {
        Some("hevc") => VideoCodec::Hevc,
        _ => VideoCodec::H264,
    };
    let info = jb_media::probe_info(&path)?;
    let (w, h) = match args.next().filter(|s| s != "source") {
        Some(s) => {
            let (a, b) = s.split_once('x').expect("size as WxH");
            (a.parse()?, b.parse()?)
        }
        None => (info.width, info.height),
    };
    let rgba = args.next().as_deref() == Some("rgba");
    let rate = info.frame_rate.unwrap_or(jb_core::Rational::new(30, 1));
    let mut p = Project::new(
        "bench",
        ProjectSettings { width: w, height: h, frame_rate: rate, sample_rate: 48_000, background: [0.0, 0.0, 0.0, 1.0] },
    );
    p.media.push(info.to_asset("m"));
    let len = ((seconds * TICKS_PER_SECOND as f64) as i64).min(info.duration);
    let track: Track = serde_json::from_value(serde_json::json!({
        "id": "v", "kind": "video", "name": "V1",
        "clips": [{"id": "c", "name": "c", "source": {"type": "media", "media_id": "m"}, "start": 0, "end": len}]
    }))?;
    let id = p.main_sequence.clone();
    p.sequence_mut(&id).unwrap().tracks.push(track);

    let pool = MediaPool::for_project(&p, PoolConfig::default());
    let mut comp = CpuCompositor::new(&pool, w, h);
    let out = std::env::temp_dir().join("jb-bench-export.mp4");
    let mut settings = ExportSettings::new(&out);
    settings.codec = codec;
    println!(
        "{}: {}x{} -> {w}x{h} {:?}, {seconds} s{}",
        path.display(),
        info.width,
        info.height,
        codec,
        if rgba { ", RGBA frames" } else { "" }
    );
    let mut render_time = std::time::Duration::ZERO;
    let mut scaler = Scaler::bilinear();
    let report = export(
        &p,
        &id,
        &settings,
        &pool,
        &mut |plan| {
            let t = std::time::Instant::now();
            let mut f = comp.render(plan).expect("render");
            if rgba {
                f = convert_frame(&mut scaler, &f, f.width, f.height, PixelFormat::Rgba8, ColorSpace::Bt709, true)
                    .expect("to rgba");
            }
            render_time += t.elapsed();
            f
        },
        &mut |pr| {
            if pr.frame % 60 == 0 {
                eprint!("\r{}/{} frames, {:.1} fps", pr.frame, pr.total_frames, pr.fps);
            }
            true
        },
    )?;
    eprintln!();
    println!(
        "encoder {} (hw {}), {} frames in {:.2} s: {:.1} fps ({:.2}x realtime); compositor {:.1} ms/frame",
        report.encoder,
        report.hardware_encoder,
        report.frames,
        report.elapsed.as_secs_f64(),
        report.fps,
        report.fps / rate.as_f64(),
        render_time.as_secs_f64() * 1000.0 / report.frames as f64
    );
    let size = std::fs::metadata(&out)?.len();
    println!("output {} ({:.1} MB)", out.display(), size as f64 / 1e6);
    let _ = std::fs::remove_file(&out);
    Ok(())
}

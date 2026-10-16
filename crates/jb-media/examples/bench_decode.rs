//! Decode benchmark: `cargo run --release -p jb-media --example bench_decode -- <file> [frames]`
//!
//! Reports sequential decode throughput (hardware and software, full size
//! and preview size) and random seek latency.

use jb_core::time::TICKS_PER_SECOND;
use jb_media::{DecodeMode, VideoDecoder};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = std::path::PathBuf::from(args.next().expect("usage: bench_decode <file> [frames]"));
    let frames: i64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(300);
    let info = jb_media::probe_info(&path)?;
    println!(
        "{}: {}x{} {:?} {:?} {} rot={} range={}",
        path.display(),
        info.width,
        info.height,
        info.video_codec,
        info.frame_rate,
        info.pixel_format.clone().unwrap_or_default(),
        info.rotation,
        if info.color.full_range { "full" } else { "limited" }
    );
    let rate = info.frame_rate.unwrap_or(jb_core::Rational::new(30, 1));
    let start = 5 * TICKS_PER_SECOND.min(info.duration / 4);
    for (hw, max) in [(true, None), (true, Some((960u32, 540u32))), (false, None), (false, Some((960, 540)))] {
        let mut d = VideoDecoder::open(&path, hw)?;
        let label = format!(
            "{:<12} {:<10}",
            d.hardware().unwrap_or("software"),
            max.map_or("full".to_string(), |(w, h)| format!("{w}x{h}"))
        );
        // Warm up (seek + first frame).
        d.frame_at(start, max, DecodeMode::Accurate)?;
        let t0 = Instant::now();
        let first = rate.frame_at(start);
        let mut bytes = 0usize;
        for i in 1..=frames {
            let f = d.frame_at(rate.frame_start(first + i), max, DecodeMode::Accurate)?.unwrap();
            bytes += f.data.len();
        }
        let el = t0.elapsed().as_secs_f64();
        println!("sequential {label}: {:>7.1} fps ({:.0} MB/s out)", frames as f64 / el, bytes as f64 / el / 1e6);
        if max.is_none() {
            // Random seeks over the whole file.
            let n = 20;
            let t0 = Instant::now();
            for k in 0..n {
                let t = (info.duration as i128 * ((k * 7919) % 97) as i128 / 97) as i64;
                d.frame_at(t, max, DecodeMode::Accurate)?;
            }
            println!("random seek {label}: {:>7.1} ms avg", t0.elapsed().as_secs_f64() * 1000.0 / n as f64);
        }
    }
    Ok(())
}

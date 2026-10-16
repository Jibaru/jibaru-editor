//! Generates a proxy: `cargo run --release -p jb-media --example make_proxy -- <src> <dst.mp4> [height]`

use jb_media::ProxyOptions;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let src = std::path::PathBuf::from(args.next().expect("usage: make_proxy <src> <dst.mp4> [height]"));
    let dst = std::path::PathBuf::from(args.next().expect("usage: make_proxy <src> <dst.mp4> [height]"));
    let height = args.next().map(|s| s.parse().unwrap()).unwrap_or(540);
    let info = jb_media::probe_info(&src)?;
    let report = jb_media::generate_proxy(&src, &dst, &ProxyOptions { height, ..Default::default() }, &mut |p| {
        eprint!("\r{:.0}%", p * 100.0);
        true
    })?;
    eprintln!();
    let secs = info.duration as f64 / 1e7;
    println!(
        "{} frames {}x{} via {} (decode {:?}) in {:.1} s: {:.1} fps, {:.1}x realtime, {:.1} MB",
        report.frames,
        report.width,
        report.height,
        report.encoder,
        report.hardware_decode,
        report.seconds,
        report.frames as f64 / report.seconds,
        secs / report.seconds,
        std::fs::metadata(&dst)?.len() as f64 / 1e6
    );
    Ok(())
}

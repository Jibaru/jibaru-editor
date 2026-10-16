//! Compares one of our transitions with a reference render (e.g. the preview
//! video of a Filmora transition, used only as a visual reference). The first
//! and last frames of the reference are taken as clips A and B; we render the
//! transition at the same instants and write a PNG with the reference row on
//! top and ours below, printing the mean absolute difference per frame.
//!
//! cargo run -p jb-render --release --example fx-compare -- REF.mp4 KIND OUT.png \
//!     [--window START LEN] [--span T0 T1] [--frames N]
//!
//! `--window` is where the transition sits in the reference (default 0.5 s,
//! 2.0 s long, which is how Filmora's transition previews are laid out).
//! Requires `ffmpeg` on PATH.

use jb_core::frame::{ColorSpace, PixelFormat, VideoFrame};
use jb_render::testing::*;
use jb_render::Renderer;
use std::io::BufWriter;
use std::process::Command;
use std::sync::Arc;

fn decode_all(path: &str, w: u32, h: u32) -> anyhow::Result<(Vec<Vec<u8>>, f64)> {
    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=r_frame_rate", "-of", "csv=p=0", path])
        .output()?;
    let rate = String::from_utf8_lossy(&probe.stdout).trim().to_string();
    let fps = match rate.split_once('/') {
        Some((a, b)) => a.parse::<f64>()? / b.parse::<f64>()?,
        None => rate.parse()?,
    };
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i", path, "-vf", &format!("scale={w}:{h}"), "-fps_mode", "passthrough"])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .output()?;
    anyhow::ensure!(out.status.success(), "ffmpeg failed: {}", String::from_utf8_lossy(&out.stderr));
    let n = (w * h * 4) as usize;
    Ok((out.stdout.chunks_exact(n).map(|c| c.to_vec()).collect(), fps))
}

fn frame(w: u32, h: u32, data: Vec<u8>) -> VideoFrame {
    VideoFrame {
        width: w,
        height: h,
        format: PixelFormat::Rgba8,
        color_space: ColorSpace::Bt709,
        full_range: true,
        stride: (w * 4) as usize,
        data: Arc::new(data),
        pts: 0,
    }
}

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut start = 0.5;
    let mut len = 2.0;
    let mut count = 12usize;
    let mut auto = false;
    if let Some(i) = args.iter().position(|a| a == "--window") {
        if args[i + 1] == "auto" {
            auto = true;
            args.drain(i..i + 2);
        } else {
            start = args[i + 1].parse()?;
            len = args[i + 2].parse()?;
            args.drain(i..i + 3);
        }
    }
    let mut span: Option<(f64, f64)> = None;
    if let Some(i) = args.iter().position(|a| a == "--span") {
        span = Some((args[i + 1].parse()?, args[i + 2].parse()?));
        args.drain(i..i + 3);
    }
    if let Some(i) = args.iter().position(|a| a == "--frames") {
        count = args[i + 1].parse()?;
        args.drain(i..i + 2);
    }
    let [reference, kind, out] = <[String; 3]>::try_from(args).map_err(|_| anyhow::anyhow!("usage: REF KIND OUT.png"))?;
    let (w, h) = (256u32, 144u32);
    let (frames, fps) = decode_all(&reference, w, h)?;
    anyhow::ensure!(frames.len() > 2, "reference has no frames");
    // Some previews end on black frames: B is the last non-black frame.
    let mean = |f: &[u8]| f.iter().map(|v| *v as f64).sum::<f64>() / f.len() as f64;
    let b_index = frames.iter().rposition(|f| mean(f) > 70.0).unwrap_or(frames.len() - 1);
    let frames: Vec<Vec<u8>> = frames[..=b_index].to_vec();
    let src = MemorySource::new()
        .with("a", frame(w, h, frames[0].clone()))
        .with("b", frame(w, h, frames.last().unwrap().clone()));
    if auto {
        // The transition spans from the first frame that leaves A to the
        // first frame that reaches B.
        let (a, b) = (&frames[0], frames.last().unwrap());
        let s0 = frames.iter().position(|f| mean_abs_diff(f, a) > 1.0).unwrap_or(0);
        let s1 = frames.iter().position(|f| mean_abs_diff(f, b) < 1.0).unwrap_or(frames.len() - 1);
        start = (s0 as f64 - 0.5) / fps;
        len = ((s1 as f64 + 0.5) / fps - start).max(1.0 / fps);
        println!("auto window: {start:.2}s + {len:.2}s");
    }
    let mut r = Renderer::new(w, h)?;
    let project = project(w, h);
    // Reference frame indices covering the window (plus a little margin).
    let (t0, t1) = span.unwrap_or((start - 0.1, start + len + 0.1));
    let first = (t0 * fps).floor().max(0.0) as usize;
    let last = ((t1 * fps).ceil() as usize).min(frames.len() - 1);
    let step = ((last - first) as f64 / count as f64).max(1.0);
    let picks: Vec<usize> = (0..count).map(|i| first + (i as f64 * step).round() as usize).filter(|i| *i <= last).collect();
    let cols = picks.len() as u32;
    let sheet_w = cols * w;
    let mut sheet = vec![0u8; (sheet_w * h * 2 * 4) as usize];
    let mut total = 0.0;
    for (c, &i) in picks.iter().enumerate() {
        let t = i as f64 / fps;
        let p = ((t - start) / len).clamp(0.0, 1.0);
        let ours = r.render(&project, &transition_plan(&kind, p, &[]), &src);
        if !r.last_report().unknown_kinds.is_empty() {
            anyhow::bail!("unknown kind {kind}");
        }
        let d = mean_abs_diff(&ours.data, &frames[i]);
        total += d;
        println!("frame {i:3} t={t:.2}s p={p:.3} diff={d:.2}");
        for (row, data) in [(0u32, &frames[i]), (1, &*ours.data)] {
            for y in 0..h {
                let s = (y * w * 4) as usize;
                let dst = (((row * h + y) * sheet_w + c as u32 * w) * 4) as usize;
                sheet[dst..dst + (w * 4) as usize].copy_from_slice(&data[s..s + (w * 4) as usize]);
            }
        }
    }
    println!("mean diff {:.2}", total / picks.len() as f64);
    let file = std::fs::File::create(&out)?;
    let mut enc = png::Encoder::new(BufWriter::new(file), sheet_w, h * 2);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&sheet)?;
    Ok(())
}

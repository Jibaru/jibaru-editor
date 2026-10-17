//! Fidelity harness: renders a range of a project with Jibaru and compares it,
//! frame by frame, with a reference export of the same range (for example
//! Filmora's own export of the same `.wfp`).
//!
//! ```sh
//! cargo run --release -p jb-cli --example fidelity -- PROJECT.wfp REF.mp4 \
//!     --range 1255s-1265s [--ref-at 1255] [--size 960x540] [--out DIR] \
//!     [--search 4] [--worst 6] [--at 3.2,4.5] [--ours OURS.mp4] [--proxies]
//! ```
//!
//! * `--ref-at` is where the range starts inside REF (default: the range
//!   start, i.e. REF is a whole-project export; use `0` for a range export).
//! * Our render is encoded at `--size` (no camera proxies unless `--proxies`)
//!   to `DIR/ours.mp4`, or `--ours` reuses an existing render of the range.
//! * Both videos are decoded with the same FFmpeg conversion to RGB, aligned
//!   by searching `±search` frames, and compared: per-frame mean absolute
//!   difference (0–255, over RGB), PSNR and signed channel bias go to
//!   `DIR/frames.csv`; the worst frames (plus `--at` times) are stacked in
//!   `DIR/sheet.png` as reference / ours / |diff|×4 rows.
//! * Audio: RMS envelopes in 50 ms windows are compared (mean |ΔdB| over
//!   windows louder than −50 dBFS) and the audio lag is estimated.
//!
//! Requires `ffmpeg` on PATH.

use anyhow::{bail, ensure, Context, Result};
use jb_core::Ticks;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Args {
    project: PathBuf,
    reference: PathBuf,
    range: String,
    ref_at: Option<f64>,
    size: (u32, u32),
    out: PathBuf,
    search: i64,
    worst: usize,
    at: Vec<f64>,
    ours: Option<PathBuf>,
    proxies: bool,
}

fn parse_args() -> Result<Args> {
    let mut it = std::env::args().skip(1);
    let mut pos = Vec::new();
    let mut a = Args {
        project: PathBuf::new(),
        reference: PathBuf::new(),
        range: String::new(),
        ref_at: None,
        size: (960, 540),
        out: PathBuf::from("fidelity-out"),
        search: 4,
        worst: 6,
        at: Vec::new(),
        ours: None,
        proxies: false,
    };
    while let Some(arg) = it.next() {
        let mut val = || it.next().context(format!("{arg} needs a value"));
        match arg.as_str() {
            "--range" => a.range = val()?,
            "--ref-at" => a.ref_at = Some(val()?.parse()?),
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once('x').context("--size WxH")?;
                a.size = (w.parse()?, h.parse()?);
            }
            "--out" => a.out = val()?.into(),
            "--search" => a.search = val()?.parse()?,
            "--worst" => a.worst = val()?.parse()?,
            "--at" => a.at = val()?.split(',').map(|s| s.parse()).collect::<Result<_, _>>()?,
            "--strip" => {
                // `--strip T N`: the sheet shows N consecutive frames from timeline time T.
                let t: f64 = val()?.parse()?;
                let n: usize = val()?.parse()?;
                a.at = (0..n).map(|i| t + i as f64 * 1001.0 / 30000.0).collect();
                a.worst = 0;
            }
            "--ours" => a.ours = Some(val()?.into()),
            "--proxies" => a.proxies = true,
            _ if arg.starts_with("--") => bail!("unknown option {arg}"),
            _ => pos.push(arg),
        }
    }
    ensure!(pos.len() == 2 && !a.range.is_empty(), "usage: fidelity PROJECT REF --range A-B [options]");
    a.project = pos[0].clone().into();
    a.reference = pos[1].clone().into();
    Ok(a)
}

fn secs(t: Ticks) -> f64 {
    t as f64 / 1e7
}

/// Renders `range` of the main sequence to `out` at `size`.
fn render(
    project: &jb_core::Project,
    range: (Ticks, Ticks),
    size: (u32, u32),
    out: &Path,
    proxies: bool,
) -> Result<()> {
    let pool = jb_media::MediaPool::for_project(project, jb_media::PoolConfig::default());
    if proxies {
        for m in &project.media {
            if let Some(p) = jb_media::cache::camera_proxy(Path::new(&m.path)) {
                pool.set_proxy(&m.id, Some(p));
            }
        }
    }
    let mut renderer = jb_render::Renderer::new(size.0, size.1)?;
    renderer.options.source_max_size = Some(size);
    // `JB_FIDELITY_LEVELS=none`: no Filmora level offsets (the reference batch exports
    // of synthetic cards do not carry the offset measured on the user projects).
    if std::env::var("JB_FIDELITY_LEVELS").as_deref() == Ok("none") {
        renderer.options.output_bias = [0.0; 3];
        renderer.options.input_bias = [0.0; 3];
    }
    let mut settings = jb_export::ExportSettings::new(out);
    settings.width = size.0;
    settings.height = size.1;
    settings.range = Some(range);
    settings.video_bit_rate = (size.0 as u64 * size.1 as u64 * 30).max(8_000_000);
    settings.prefetch_max_size = Some(size);
    // `JB_FIDELITY_ENCODER=libx264`: the hardware encoder smears fine test-card
    // detail (~4 MAD at 960x540 on the grid cards), which hides fitting errors.
    if let Ok(enc) = std::env::var("JB_FIDELITY_ENCODER") {
        settings.encoder = Some(enc);
        settings.hardware = false;
    }
    let mut unknown = std::collections::BTreeSet::new();
    let report = jb_export::export(
        project,
        &project.main_sequence,
        &settings,
        &pool,
        &mut |plan| {
            let f = renderer.render(project, plan, &pool);
            unknown.extend(renderer.last_report().unknown_kinds.iter().cloned());
            f
        },
        &mut |_| true,
    )?;
    eprintln!("rendered {} frames with {} in {:.1}s", report.frames, report.encoder, report.elapsed.as_secs_f64());
    if !unknown.is_empty() {
        eprintln!("unknown effects: {unknown:?}");
    }
    Ok(())
}

/// Streams decoded RGB frames of `path` from `start` for `len` seconds.
struct Decoder {
    child: std::process::Child,
    frame: usize,
}

impl Decoder {
    fn open(path: &Path, start: f64, len: f64, w: u32, h: u32, fmt: &str) -> Result<Self> {
        let child = Command::new("ffmpeg")
            .args(["-v", "error", "-nostdin", "-ss", &format!("{start:.6}"), "-i"])
            .arg(path)
            .args(["-t", &format!("{len:.6}"), "-an", "-vf", &format!("scale={w}:{h}:flags=area")])
            .args(["-fps_mode", "passthrough", "-f", "rawvideo", "-pix_fmt", fmt, "-"])
            .stdout(Stdio::piped())
            .spawn()
            .context("cannot run ffmpeg")?;
        let bpp = if fmt == "gray" { 1 } else { 3 };
        Ok(Decoder { child, frame: (w * h * bpp) as usize })
    }
    fn next(&mut self) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; self.frame];
        self.child.stdout.as_mut()?.read_exact(&mut buf).ok()?;
        Some(buf)
    }
    fn all(mut self) -> Vec<Vec<u8>> {
        std::iter::from_fn(|| self.next()).collect()
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn mad(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).unsigned_abs() as u64).sum::<u64>() as f64 / a.len() as f64
}

/// Best offset `o` (ours frame `i` matches reference frame `i + o`).
/// `k0` is the reference frame index where the range starts; seeks land
/// half a frame early so FFmpeg's accurate seek starts exactly on a frame.
fn align(reference: &Path, k0: i64, dt: f64, ours: &Path, len: f64, search: i64) -> Result<i64> {
    let first = (k0 - search).max(0);
    let span = len + (2 * search + 1) as f64 * dt;
    let r = Decoder::open(reference, (first as f64 - 0.5).max(0.0) * dt, span, 96, 54, "gray")?.all();
    let o = Decoder::open(ours, 0.0, len, 96, 54, "gray")?.all();
    let base = k0 - first;
    let mut best = (f64::MAX, 0);
    for off in -search..=search {
        let (mut sum, mut n) = (0.0, 0);
        for (i, f) in o.iter().enumerate() {
            let j = i as i64 + base + off;
            if j >= 0 && (j as usize) < r.len() {
                sum += mad(f, &r[j as usize]);
                n += 1;
            }
        }
        if n > o.len() / 2 && sum / (n as f64) < best.0 {
            best = (sum / n as f64, off);
        }
    }
    Ok(best.1)
}

struct FrameStat {
    index: usize,
    t: f64,
    mad: f64,
    psnr: f64,
    bias: [f64; 3],
    luma: (f64, f64),
}

fn stats(index: usize, t: f64, r: &[u8], o: &[u8]) -> FrameStat {
    let mut abs = 0u64;
    let mut sq = 0u64;
    let mut bias = [0i64; 3];
    let (mut lr, mut lo) = (0.0, 0.0);
    for (i, (pr, po)) in r.chunks_exact(3).zip(o.chunks_exact(3)).enumerate() {
        for c in 0..3 {
            let d = po[c] as i64 - pr[c] as i64;
            abs += d.unsigned_abs();
            sq += (d * d) as u64;
            bias[c] += d;
        }
        if i % 7 == 0 {
            lr += 0.2126 * pr[0] as f64 + 0.7152 * pr[1] as f64 + 0.0722 * pr[2] as f64;
            lo += 0.2126 * po[0] as f64 + 0.7152 * po[1] as f64 + 0.0722 * po[2] as f64;
        }
    }
    let n = r.len() as f64;
    let px = (r.len() / 3) as f64;
    let mse = sq as f64 / n;
    let ls = (r.len() / 3).div_ceil(7) as f64;
    FrameStat {
        index,
        t,
        mad: abs as f64 / n,
        psnr: if mse == 0.0 { 99.0 } else { 10.0 * (255.0f64 * 255.0 / mse).log10() },
        bias: bias.map(|b| b as f64 / px),
        luma: (lr / ls, lo / ls),
    }
}

/// RMS envelope (dBFS) of the mono mix in `win` second windows.
fn envelope(path: &Path, start: f64, len: f64, win: f64) -> Result<Vec<f64>> {
    let rate = 16000.0;
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-ss", &format!("{start:.6}"), "-i"])
        .arg(path)
        // Pad to the first timestamp: a stream whose first packet starts after
        // 0 (AAC priming trimmed by the edit list) must not shift the samples.
        .args(["-t", &format!("{len:.6}"), "-vn", "-af", "aresample=async=1:first_pts=0"])
        .args(["-ac", "1", "-ar", "16000", "-f", "f32le", "-"])
        .output()?;
    let s: Vec<f32> = out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    let n = (win * rate) as usize;
    Ok(s.chunks(n)
        .filter(|c| c.len() == n)
        .map(|c| {
            let rms = (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / n as f64).sqrt();
            20.0 * rms.max(1e-6).log10()
        })
        .collect())
}

fn audio_report(a: &Args, ref_at: f64, len: f64) -> Result<String> {
    let ours = a.out.join("ours.mp4");
    let ours = a.ours.as_deref().unwrap_or(&ours);
    // Lag at 5 ms resolution over ±200 ms.
    let fine_r = envelope(&a.reference, ref_at, len, 0.005)?;
    let fine_o = envelope(ours, 0.0, len, 0.005)?;
    if fine_r.is_empty() || fine_o.is_empty() {
        return Ok("audio: missing\n".into());
    }
    let mut best = (f64::MAX, 0i64);
    for lag in -40i64..=40 {
        let (mut s, mut n) = (0.0, 0);
        for (i, v) in fine_o.iter().enumerate() {
            let j = i as i64 + lag;
            if j >= 0 && (j as usize) < fine_r.len() && (*v > -50.0 || fine_r[j as usize] > -50.0) {
                s += (v - fine_r[j as usize]).abs();
                n += 1;
            }
        }
        if n > 0 && s / (n as f64) < best.0 {
            best = (s / n as f64, lag);
        }
    }
    let r = envelope(&a.reference, ref_at, len, 0.05)?;
    let o = envelope(ours, 0.0, len, 0.05)?;
    let pairs: Vec<(f64, f64)> =
        r.iter().zip(&o).filter(|(x, y)| **x > -50.0 || **y > -50.0).map(|(x, y)| (*x, *y)).collect();
    let mean = pairs.iter().map(|(x, y)| (y - x).abs()).sum::<f64>() / pairs.len().max(1) as f64;
    let signed = pairs.iter().map(|(x, y)| y - x).sum::<f64>() / pairs.len().max(1) as f64;
    let mut text = format!("audio: mean |ΔdB| {mean:.2}, mean ΔdB (ours-ref) {signed:+.2}, lag {:+} ms\n", best.1 * 5);
    text.push_str("audio envelope every 0.5 s (ref/ours dBFS):");
    for (i, (x, y)) in r.iter().zip(&o).enumerate().step_by(10) {
        text.push_str(&format!(" {:.1}:{:.0}/{:.0}", i as f64 * 0.05, x, y));
    }
    text.push('\n');
    Ok(text)
}

fn main() -> Result<()> {
    let a = parse_args()?;
    std::fs::create_dir_all(&a.out)?;
    let project = jb_cli::project_io::load_project(&a.project).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let fps = project.settings.frame_rate;
    let range = jb_cli::parse_range(&a.range, fps).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    // Renders start on the frame containing the range start.
    let range = (fps.frame_start(fps.frame_at(range.0)), range.1);
    let len = secs(range.1 - range.0);

    let ours = match &a.ours {
        Some(p) => p.clone(),
        None => {
            let p = a.out.join("ours.mp4");
            // `JB_FIDELITY_RENDER_SIZE=1920x1080` renders (and encodes) at that size and
            // compares at `--size`, like a full-resolution Filmora export decoded at
            // `--size`: at 960x540 our own 4:2:0 encode halves chroma resolution again
            // (Cb error ~3.4 vs ~1.2 on the grid cards).
            let rs = std::env::var("JB_FIDELITY_RENDER_SIZE")
                .ok()
                .and_then(|v| v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))))
                .unwrap_or(a.size);
            render(&project, range, rs, &p, a.proxies)?;
            p
        }
    };
    let frame_dt = fps.den as f64 / fps.num as f64;
    let k0 = match a.ref_at {
        Some(t) => (t / frame_dt).round() as i64,
        None => fps.frame_at(range.0),
    };
    let offset = align(&a.reference, k0, frame_dt, &ours, len, a.search)?;
    println!("alignment: reference is {offset:+} frame(s) relative to ours");
    let (w, h) = a.size;
    ensure!(k0 + offset >= 0, "reference starts before 0");
    let ref_start = (k0 + offset) as f64 * frame_dt;
    // Alignment for follow-up tools: reference path, range start, where ours frame 0 is in the reference.
    std::fs::write(a.out.join("ref"), a.reference.to_string_lossy().as_bytes())?;
    std::fs::write(a.out.join("start"), format!("{}", secs(range.0)))?;
    std::fs::write(a.out.join("refat"), format!("{ref_start}"))?;
    let mut rd = Decoder::open(&a.reference, (ref_start - 0.5 * frame_dt).max(0.0), len, w, h, "rgb24")?;
    let mut od = Decoder::open(&ours, 0.0, len, w, h, "rgb24")?;
    let mut all = Vec::new();
    let mut kept: Vec<(FrameStat, Vec<u8>, Vec<u8>)> = Vec::new();
    let forced: Vec<usize> = a.at.iter().map(|t| ((t - secs(range.0)) / frame_dt).round().max(0.0) as usize).collect();
    // Tone curve: mean ours value per reference value bin, per channel.
    let mut tone = [[(0u64, 0u64); 16]; 3];
    let mut i = 0;
    while let (Some(r), Some(o)) = (rd.next(), od.next()) {
        for (pr, po) in r.chunks_exact(3).zip(o.chunks_exact(3)).step_by(5) {
            for c in 0..3 {
                let b = &mut tone[c][pr[c] as usize / 16];
                b.0 += 1;
                b.1 += po[c] as u64;
            }
        }
        let s = stats(i, secs(range.0) + i as f64 * frame_dt, &r, &o);
        let st = FrameStat { bias: s.bias, ..s };
        all.push((st.index, st.t, st.mad, st.psnr, st.bias, st.luma));
        // Keep the worst `worst` frames (spaced ≥ 10 frames apart) plus forced ones.
        let is_forced = forced.contains(&i);
        if is_forced
            || kept.iter().filter(|k| !forced.contains(&k.0.index)).count() < a.worst
            || kept.iter().any(|k| k.0.mad < st.mad)
        {
            if let Some(k) =
                kept.iter_mut().find(|k| !forced.contains(&k.0.index) && (k.0.index as i64 - i as i64).abs() < 10)
            {
                if k.0.mad < st.mad || is_forced {
                    *k = (st, r, o);
                }
            } else {
                kept.push((st, r, o));
                let free: Vec<usize> = (0..kept.len()).filter(|j| !forced.contains(&kept[*j].0.index)).collect();
                if free.len() > a.worst {
                    let worst_min = *free.iter().min_by(|x, y| kept[**x].0.mad.total_cmp(&kept[**y].0.mad)).unwrap();
                    kept.remove(worst_min);
                }
            }
        }
        i += 1;
    }
    ensure!(!all.is_empty(), "no frames decoded");
    let mut csv = String::from("frame,time,mad,psnr,bias_r,bias_g,bias_b,luma_ref,luma_ours\n");
    for (i, t, m, p, b, l) in &all {
        csv.push_str(&format!("{i},{t:.3},{m:.3},{p:.2},{:.2},{:.2},{:.2},{:.1},{:.1}\n", b[0], b[1], b[2], l.0, l.1));
    }
    std::fs::write(a.out.join("frames.csv"), csv)?;
    let mut mads: Vec<f64> = all.iter().map(|x| x.2).collect();
    mads.sort_by(f64::total_cmp);
    let mean = mads.iter().sum::<f64>() / mads.len() as f64;
    let psnr = all.iter().map(|x| x.3).sum::<f64>() / all.len() as f64;
    let bias: Vec<f64> = (0..3).map(|c| all.iter().map(|x| x.4[c]).sum::<f64>() / all.len() as f64).collect();
    let lum = all.iter().fold((0.0, 0.0), |s, x| (s.0 + x.5 .0, s.1 + x.5 .1));
    println!(
        "frames {} (ours) | MAD mean {mean:.2} median {:.2} p95 {:.2} max {:.2} | PSNR mean {psnr:.2} dB",
        all.len(),
        mads[mads.len() / 2],
        mads[(mads.len() * 95 / 100).min(mads.len() - 1)],
        mads[mads.len() - 1]
    );
    println!(
        "bias ours-ref R {:+.2} G {:+.2} B {:+.2} | mean luma ref {:.1} ours {:.1}",
        bias[0],
        bias[1],
        bias[2],
        lum.0 / all.len() as f64,
        lum.1 / all.len() as f64
    );
    // Per-second profile.
    let mut prof = String::from("MAD per second:");
    for chunk in all.chunks((1.0 / frame_dt).round() as usize) {
        prof.push_str(&format!(
            " {:.0}:{:.1}",
            chunk[0].1,
            chunk.iter().map(|x| x.2).sum::<f64>() / chunk.len() as f64
        ));
    }
    println!("{prof}");
    for (c, name) in ["R", "G", "B"].iter().enumerate() {
        print!("tone {name} (ref bin center: ours-ref):");
        for (k, (n, s)) in tone[c].iter().enumerate() {
            if *n > 100 {
                print!(" {}:{:+.1}", k * 16 + 8, *s as f64 / *n as f64 - (k * 16) as f64 - 7.5);
            }
        }
        println!();
    }
    kept.sort_by_key(|k| k.0.index);
    print!("sheet frames:");
    for k in &kept {
        print!(" {:.3}s(MAD {:.1})", k.0.t, k.0.mad);
    }
    println!();
    // Contact sheet: one column per kept frame, rows ref / ours / diff×4.
    let cols = kept.len() as u32;
    let mut sheet = image::RgbImage::new(w * cols, h * 3);
    for (c, (_, r, o)) in kept.iter().enumerate() {
        for y in 0..h {
            for x in 0..w {
                let p = ((y * w + x) * 3) as usize;
                let rp = [r[p], r[p + 1], r[p + 2]];
                let op = [o[p], o[p + 1], o[p + 2]];
                let dp = [0, 1, 2].map(|k| ((r[p + k] as i32 - o[p + k] as i32).unsigned_abs() * 4).min(255) as u8);
                let cx = c as u32 * w + x;
                sheet.put_pixel(cx, y, image::Rgb(rp));
                sheet.put_pixel(cx, h + y, image::Rgb(op));
                sheet.put_pixel(cx, 2 * h + y, image::Rgb(dp));
            }
        }
    }
    sheet.save(a.out.join("sheet.png"))?;
    print!("{}", audio_report(&a, ref_start, len)?);
    let ref_frames = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=duration,nb_frames", "-of", "csv=p=0"])
        .arg(&a.reference)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let dur = project.main().duration();
    println!(
        "project: {:.3}s = {} frames | reference (duration,frames): {ref_frames}",
        secs(dur),
        jb_export::frame_count(fps, 0, dur)
    );
    Ok(())
}

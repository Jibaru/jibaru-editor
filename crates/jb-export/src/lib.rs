//! Render pipeline that encodes sequences to video files.
//!
//! [`export`] walks the frames of a sequence with [`jb_core::eval::evaluate`],
//! asks a renderer callback for each composed frame, encodes video with a
//! hardware encoder (LGPL software fallbacks), mixes audio with
//! [`jb_core::eval::audio_voices`] and muxes MP4 (H.264/HEVC + AAC).
//!
//! The work runs as a pipeline of threads joined by bounded channels:
//!
//! ```text
//! plan + prefetch decode ──▶ render (caller's thread) ──▶ convert + video encode ──┐
//!                                                          audio mix + AAC encode ──┴▶ mux
//! ```
//!
//! The renderer is a plain closure so this crate does not depend on the GPU
//! compositor; [`CpuCompositor`] is a simple CPU stand-in for tests.

#![allow(clippy::unnecessary_cast)]

mod cpu;

pub use cpu::CpuCompositor;
pub use jb_media::encode::VideoCodec;

use anyhow::{Context as _, Result};
use jb_core::eval::{evaluate, FramePlan, Layer, LayerContent};
use jb_core::frame::{FrameSource, PixelFormat, VideoFrame};
use jb_core::model::Project;
use jb_core::time::{Rational, Ticks, TICKS_PER_SECOND};
use jb_media::convert::{bytes_per_pixel, wrap_video_frame, AvFrame, Scaler};
use jb_media::encode::{open_aac_encoder, open_video_encoder, EncoderConfig};
use jb_media::ff;
use jb_media::MediaPool;
use ff::ffi;
use ff::packet::Mut as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::time::{Duration, Instant};

const EAGAIN: i32 = ffi::AVERROR(libc::EAGAIN);
/// AAC frame size.
const AUDIO_CHUNK: usize = 1024;

#[derive(Debug, Clone)]
pub struct ExportSettings {
    /// Output file; the container follows the extension (.mp4, .mov, .mkv).
    pub path: PathBuf,
    /// Output size; 0 uses the project size.
    pub width: u32,
    pub height: u32,
    /// Output frame rate; `None` uses the project rate.
    pub frame_rate: Option<Rational>,
    pub codec: VideoCodec,
    /// Video bit rate in bits per second; 0 picks one from size and rate.
    pub video_bit_rate: u64,
    pub audio_bit_rate: u64,
    pub audio: bool,
    /// Timeline range `[start, end)`; `None` exports the whole sequence.
    pub range: Option<(Ticks, Ticks)>,
    /// Allow hardware encoders.
    pub hardware: bool,
    /// Force a specific FFmpeg encoder by name.
    pub encoder: Option<String>,
    /// Decode upcoming frames ahead of the renderer on a separate thread.
    pub prefetch: bool,
    /// `max_size` the renderer passes to the frame source; prefetching must
    /// use the same value to be useful.
    pub prefetch_max_size: Option<(u32, u32)>,
    /// Frames buffered between pipeline stages.
    pub queue_depth: usize,
}

impl ExportSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            width: 0,
            height: 0,
            frame_rate: None,
            codec: VideoCodec::H264,
            video_bit_rate: 0,
            audio_bit_rate: 192_000,
            audio: true,
            range: None,
            hardware: true,
            encoder: None,
            prefetch: true,
            prefetch_max_size: None,
            queue_depth: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportProgress {
    pub frame: u64,
    pub total_frames: u64,
    pub elapsed: Duration,
    /// Average frames per second so far.
    pub fps: f64,
}

impl ExportProgress {
    pub fn fraction(&self) -> f64 {
        self.frame as f64 / self.total_frames.max(1) as f64
    }
}

#[derive(Debug, Clone)]
pub struct ExportReport {
    pub path: PathBuf,
    /// FFmpeg encoder used, e.g. "h264_videotoolbox".
    pub encoder: String,
    pub hardware_encoder: bool,
    pub width: u32,
    pub height: u32,
    pub frames: u64,
    pub duration: Ticks,
    pub elapsed: Duration,
    pub fps: f64,
}

/// Frame count of `[start, end)` at `rate`.
pub fn frame_count(rate: Rational, start: Ticks, end: Ticks) -> u64 {
    if end <= start {
        return 0;
    }
    (rate.frame_at(end - start - 1) + 1) as u64
}

fn prefetch(layers: &[Layer], media: &MediaPool, max: Option<(u32, u32)>) {
    for l in layers {
        match &l.content {
            LayerContent::Media { media_id, source_time } => {
                let _ = media.frame(media_id, *source_time, max);
            }
            LayerContent::Nested(p) => prefetch(&p.layers, media, max),
            _ => {}
        }
        if let Some(tr) = &l.transition {
            prefetch(std::slice::from_ref(&tr.incoming), media, max);
        }
    }
}

enum Pkt {
    Video(ff::Packet),
    Audio(ff::Packet),
}

/// Exports `sequence_id` of `project`. `media` provides audio and is used to
/// prefetch video frames; `renderer` composes each frame (any size or pixel
/// format; it is converted as needed). `progress` returns false to cancel,
/// in which case the partial file is removed and an error returned.
pub fn export(
    project: &Project,
    sequence_id: &str,
    settings: &ExportSettings,
    media: &MediaPool,
    renderer: &mut dyn FnMut(&FramePlan) -> VideoFrame,
    progress: &mut dyn FnMut(&ExportProgress) -> bool,
) -> Result<ExportReport> {
    jb_media::init();
    let started = Instant::now();
    let seq = project.sequence(sequence_id).with_context(|| format!("unknown sequence {sequence_id}"))?;
    let rate = settings.frame_rate.unwrap_or(project.settings.frame_rate);
    let (start, end) = settings.range.unwrap_or((0, seq.duration()));
    // Ranges start on the sequence's frame grid (the frame containing
    // `start`), so a range export shows exactly the frames of a full export.
    let first_frame = rate.frame_at(start);
    let start = rate.frame_start(first_frame);
    let total = frame_count(rate, start, end);
    anyhow::ensure!(total > 0, "nothing to export: the range is empty");
    let width = if settings.width == 0 { project.settings.width } else { settings.width } & !1;
    let height = if settings.height == 0 { project.settings.height } else { settings.height } & !1;
    let duration = rate.frame_start(total as i64);
    let sample_rate = media.config().sample_rate;
    let with_audio = settings.audio;

    // Encoders.
    let fr = ffi::AVRational { num: rate.num as i32, den: rate.den as i32 };
    let mut cfg = EncoderConfig::new(settings.codec, width, height, fr);
    cfg.hardware = settings.hardware;
    cfg.encoder = settings.encoder.clone();
    if settings.video_bit_rate > 0 {
        cfg.bit_rate = settings.video_bit_rate;
    }
    let mut venc = open_video_encoder(&cfg)?;
    let mut aenc = if with_audio { Some(open_aac_encoder(sample_rate, settings.audio_bit_rate, true)?) } else { None };

    // Muxer.
    let tmp = settings.path.with_extension(format!(
        "partial.{}",
        settings.path.extension().and_then(|e| e.to_str()).unwrap_or("mp4")
    ));
    let mut out = ff::format::output(&tmp).with_context(|| format!("cannot create {}", tmp.display()))?;
    unsafe {
        let mut s = out.add_stream(ff::encoder::find(ff::codec::Id::None))?;
        let st = s.as_mut_ptr();
        check(ffi::avcodec_parameters_from_context((*st).codecpar, venc.encoder.as_ptr()), "video params")?;
        (*(*st).codecpar).codec_tag = venc.codec_tag();
        (*st).time_base = venc.time_base;
        (*st).avg_frame_rate = fr;
        if let Some(a) = &aenc {
            let mut s = out.add_stream(ff::encoder::find(ff::codec::Id::None))?;
            let st = s.as_mut_ptr();
            check(ffi::avcodec_parameters_from_context((*st).codecpar, a.as_ptr()), "audio params")?;
            (*st).time_base = (*a.as_ptr()).time_base;
        }
    }
    let mut opts = ff::Dictionary::new();
    opts.set("movflags", "+faststart");
    out.write_header_with(opts)?;
    let tb_of = |i: usize| unsafe { (**(*out.as_ptr()).streams.add(i)).time_base };
    let v_tb_out = tb_of(0);
    let a_tb_out = if aenc.is_some() { tb_of(1) } else { v_tb_out };
    let v_tb_enc = venc.time_base;
    let a_tb_enc = aenc.as_ref().map_or(v_tb_enc, |a| unsafe { (*a.as_ptr()).time_base });
    let encoder_name = venc.name.clone();
    let hardware_encoder = venc.hardware;

    let cancel = AtomicBool::new(false);
    let depth = settings.queue_depth.max(1);
    let result = std::thread::scope(|s| -> Result<u64> {
        let cancel = &cancel;
        let (plan_tx, plan_rx) = sync_channel::<(u64, FramePlan)>(depth);
        let (frame_tx, frame_rx) = sync_channel::<(u64, VideoFrame)>(depth);
        let (vpkt_tx, vpkt_rx) = sync_channel::<Pkt>(64);
        let (apkt_tx, apkt_rx) = sync_channel::<Pkt>(64);

        // 1. Planning and prefetch decode.
        let planner = s.spawn(move || {
            for i in 0..total {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let t = rate.frame_start(first_frame + i as i64);
                let plan = evaluate(project, seq, t);
                if settings.prefetch {
                    prefetch(&plan.layers, media, settings.prefetch_max_size);
                }
                if plan_tx.send((i, plan)).is_err() {
                    break;
                }
            }
        });

        // 3. Pixel conversion + video encode.
        let video = s.spawn(move || -> Result<u64> {
            let mut scaler = Scaler::bilinear();
            let mut pkt = ff::Packet::empty();
            let mut n = 0u64;
            let drain = |venc: &mut jb_media::VideoEncoder, pkt: &mut ff::Packet| -> Result<bool> {
                loop {
                    let r = unsafe { ffi::avcodec_receive_packet(venc.encoder.as_mut_ptr(), pkt.as_mut_ptr()) };
                    if r == EAGAIN || r == ffi::AVERROR_EOF {
                        return Ok(true);
                    }
                    check(r, "video encode")?;
                    let p = std::mem::replace(pkt, ff::Packet::empty());
                    if vpkt_tx.send(Pkt::Video(p)).is_err() {
                        return Ok(false);
                    }
                }
            };
            for (i, frame) in frame_rx {
                let f = to_encoder_frame(&mut scaler, &frame, width, height, venc.pix_fmt)?;
                unsafe {
                    (*f.0).pts = i as i64;
                    (*f.0).duration = 1;
                    check(ffi::avcodec_send_frame(venc.encoder.as_mut_ptr(), f.0), "send video frame")?;
                }
                n += 1;
                if !drain(&mut venc, &mut pkt)? {
                    return Ok(n);
                }
            }
            unsafe { ffi::avcodec_send_frame(venc.encoder.as_mut_ptr(), std::ptr::null()) };
            drain(&mut venc, &mut pkt)?;
            Ok(n)
        });

        // 4. Audio mix + AAC encode.
        let mut apkt_tx = Some(apkt_tx);
        let audio = aenc.take().map(|mut aenc| {
            let apkt_tx = apkt_tx.take().unwrap();
            s.spawn(move || -> Result<()> {
                let total_samples = jb_media::audio::ticks_to_samples(duration, sample_rate);
                let first = jb_media::audio::ticks_to_samples(start, sample_rate);
                let mut pkt = ff::Packet::empty();
                let mut pos = 0i64;
                let drain = |aenc: &mut ff::encoder::Audio, pkt: &mut ff::Packet| -> Result<bool> {
                    loop {
                        let r = unsafe { ffi::avcodec_receive_packet(aenc.as_mut_ptr(), pkt.as_mut_ptr()) };
                        if r == EAGAIN || r == ffi::AVERROR_EOF {
                            return Ok(true);
                        }
                        check(r, "audio encode")?;
                        let p = std::mem::replace(pkt, ff::Packet::empty());
                        if apkt_tx.send(Pkt::Audio(p)).is_err() {
                            return Ok(false);
                        }
                    }
                };
                while pos < total_samples {
                    if cancel.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    let n = AUDIO_CHUNK.min((total_samples - pos) as usize);
                    let mix = jb_media::mixer::mix(project, seq, first + pos, n, media)?;
                    let f = AvFrame::alloc();
                    unsafe {
                        let fr = &mut *f.0;
                        fr.format = ffi::AVSampleFormat::AV_SAMPLE_FMT_FLTP as i32;
                        fr.sample_rate = sample_rate as i32;
                        ffi::av_channel_layout_default(&mut fr.ch_layout, 2);
                        fr.nb_samples = n as i32;
                        check(ffi::av_frame_get_buffer(f.0, 0), "audio frame")?;
                        let l = std::slice::from_raw_parts_mut(fr.data[0] as *mut f32, n);
                        let r = std::slice::from_raw_parts_mut(fr.data[1] as *mut f32, n);
                        for k in 0..n {
                            l[k] = mix[k * 2];
                            r[k] = mix[k * 2 + 1];
                        }
                        fr.pts = pos;
                        check(ffi::avcodec_send_frame(aenc.as_mut_ptr(), f.0), "send audio frame")?;
                    }
                    pos += n as i64;
                    if !drain(&mut aenc, &mut pkt)? {
                        return Ok(());
                    }
                }
                unsafe { ffi::avcodec_send_frame(aenc.as_mut_ptr(), std::ptr::null()) };
                drain(&mut aenc, &mut pkt)?;
                Ok(())
            })
        });
        drop(apkt_tx);

        // 5. Muxing, interleaving the two packet streams by time.
        let muxer = s.spawn(move || -> Result<()> {
            let res = mux(&mut out, vpkt_rx, apkt_rx, [v_tb_enc, a_tb_enc], [v_tb_out, a_tb_out]);
            if res.is_err() {
                cancel.store(true, Ordering::Relaxed);
            }
            res?;
            out.write_trailer()?;
            Ok(())
        });

        // 2. Rendering on the caller's thread (the GPU context lives here).
        let mut rendered = 0u64;
        let mut render_err = None;
        for (i, plan) in plan_rx {
            let frame = renderer(&plan);
            rendered += 1;
            if frame_tx.send((i, frame)).is_err() {
                break;
            }
            let elapsed = started.elapsed();
            let p = ExportProgress {
                frame: rendered,
                total_frames: total,
                elapsed,
                fps: rendered as f64 / elapsed.as_secs_f64().max(1e-9),
            };
            if !progress(&p) {
                cancel.store(true, Ordering::Relaxed);
                render_err = Some(anyhow::anyhow!("export cancelled"));
                break;
            }
            if cancel.load(Ordering::Relaxed) {
                break;
            }
        }
        drop(frame_tx);
        let join = |r: std::thread::Result<Result<()>>| r.map_err(|_| anyhow::anyhow!("export thread panicked"))?;
        planner.join().map_err(|_| anyhow::anyhow!("planner panicked"))?;
        let frames = video.join().map_err(|_| anyhow::anyhow!("video encoder panicked"))?;
        let audio_res = audio.map(|a| join(a.join()));
        let mux_res = join(muxer.join());
        if let Some(e) = render_err {
            return Err(e);
        }
        let frames = frames?;
        if let Some(r) = audio_res {
            r?;
        }
        mux_res?;
        anyhow::ensure!(frames == total, "encoded {frames} of {total} frames");
        Ok(frames)
    });
    let frames = match result {
        Ok(f) => f,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    };
    std::fs::rename(&tmp, &settings.path)?;
    let elapsed = started.elapsed();
    Ok(ExportReport {
        path: settings.path.clone(),
        encoder: encoder_name,
        hardware_encoder,
        width,
        height,
        frames,
        duration,
        elapsed,
        fps: frames as f64 / elapsed.as_secs_f64().max(1e-9),
    })
}

fn check(r: i32, what: &str) -> Result<i32> {
    if r < 0 {
        anyhow::bail!("{what}: {}", ff::Error::from(r));
    }
    Ok(r)
}

fn seconds(ts: i64, tb: ffi::AVRational) -> f64 {
    ts as f64 * tb.num as f64 / tb.den.max(1) as f64
}

fn mux(
    out: &mut ff::format::context::Output,
    video: Receiver<Pkt>,
    audio: Receiver<Pkt>,
    enc_tb: [ffi::AVRational; 2],
    out_tb: [ffi::AVRational; 2],
) -> Result<()> {
    let mut pending: [Option<ff::Packet>; 2] = [None, None];
    let mut done = [false, false];
    let rx = [video, audio];
    loop {
        for k in 0..2 {
            if pending[k].is_none() && !done[k] {
                match rx[k].recv() {
                    Ok(Pkt::Video(p)) | Ok(Pkt::Audio(p)) => pending[k] = Some(p),
                    Err(_) => done[k] = true,
                }
            }
        }
        let key = |k: usize| pending[k].as_ref().map(|p| seconds(p.dts().or(p.pts()).unwrap_or(0), enc_tb[k]));
        let pick = match (key(0), key(1)) {
            (Some(a), Some(b)) => Some(if b < a { 1 } else { 0 }),
            (Some(_), None) => Some(0),
            (None, Some(_)) => Some(1),
            (None, None) => None,
        };
        let Some(k) = pick else { break };
        let mut p = pending[k].take().unwrap();
        p.set_stream(k);
        p.rescale_ts(ff::Rational::from(enc_tb[k]), ff::Rational::from(out_tb[k]));
        p.write_interleaved(out)?;
    }
    Ok(())
}

/// Converts a rendered frame to what the encoder takes (BT.709 limited range).
fn to_encoder_frame(
    scaler: &mut Scaler,
    vf: &VideoFrame,
    width: u32,
    height: u32,
    pix_fmt: ffi::AVPixelFormat,
) -> Result<AvFrame> {
    let src = wrap_video_frame(vf);
    let same = vf.width == width
        && vf.height == height
        && jb_media::convert::av_format(vf.format) == pix_fmt
        && vf.format != PixelFormat::Rgba8
        && !vf.full_range
        && vf.color_space == jb_core::frame::ColorSpace::Bt709
        && vf.stride == width as usize * bytes_per_pixel(vf.format);
    if same {
        return Ok(src);
    }
    let dst = AvFrame::alloc();
    unsafe {
        let d = &mut *dst.0;
        d.width = width as i32;
        d.height = height as i32;
        d.format = pix_fmt as i32;
        d.colorspace = ffi::AVColorSpace::AVCOL_SPC_BT709;
        d.color_primaries = ffi::AVColorPrimaries::AVCOL_PRI_BT709;
        d.color_trc = ffi::AVColorTransferCharacteristic::AVCOL_TRC_BT709;
        d.color_range = ffi::AVColorRange::AVCOL_RANGE_MPEG;
        (*src.0).color_primaries = d.color_primaries;
        (*src.0).color_trc = d.color_trc;
        scaler.run(dst.0, src.0)?;
    }
    Ok(dst)
}

/// Seconds to ticks helper for callers building ranges.
pub fn seconds_to_ticks(s: f64) -> Ticks {
    (s * TICKS_PER_SECOND as f64).round() as Ticks
}

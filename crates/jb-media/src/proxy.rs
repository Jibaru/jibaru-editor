//! Proxy generation: a low-resolution copy with exactly the same frame
//! timing as the source (every frame keeps its original timestamp, the
//! equivalent of `-fps_mode passthrough`), short GOPs for fast scrubbing,
//! and the original audio stream copied untouched.

use crate::convert::{AvFrame, Scaler};
use crate::encode::{open_video_encoder, EncoderConfig, VideoCodec};
use crate::util::*;
use anyhow::{Context as _, Result};
use ffmpeg_next as ff;
use ff::ffi;
use ff::packet::{Mut as _, Ref as _};
use std::path::{Path, PathBuf};
use std::sync::mpsc::sync_channel;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ProxyOptions {
    /// Output height in pixels (coded orientation); width follows the aspect.
    pub height: u32,
    /// Bit rate; 0 picks one from the size and frame rate.
    pub bit_rate: u64,
    pub hardware: bool,
    /// Keyframe interval in seconds.
    pub gop_seconds: f64,
}

impl Default for ProxyOptions {
    fn default() -> Self {
        Self { height: 540, bit_rate: 0, hardware: true, gop_seconds: 0.5 }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyReport {
    pub path: PathBuf,
    pub encoder: String,
    pub hardware_decode: Option<String>,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
    pub seconds: f64,
}

enum Msg {
    Video(AvFrame),
    Audio(ff::Packet),
}

/// Writes a proxy of `src` to `dst` (MP4). `progress` receives 0..1 and
/// returns false to cancel (the partial file is removed).
pub fn generate_proxy(
    src: &Path,
    dst: &Path,
    opts: &ProxyOptions,
    progress: &mut dyn FnMut(f32) -> bool,
) -> Result<ProxyReport> {
    crate::init();
    let started = Instant::now();
    let tmp = dst.with_extension("partial.mp4");
    let result = run(src, &tmp, opts, progress);
    match result {
        Ok(mut r) => {
            std::fs::rename(&tmp, dst)?;
            r.path = dst.to_path_buf();
            r.seconds = started.elapsed().as_secs_f64();
            Ok(r)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn run(src: &Path, dst: &Path, opts: &ProxyOptions, progress: &mut dyn FnMut(f32) -> bool) -> Result<ProxyReport> {
    let dec = crate::video::VideoDecoder::open(src, opts.hardware)?;
    let info = dec.info().clone();
    let hw = dec.hardware().map(str::to_string);
    let (cw, ch) = (info.coded_width, info.coded_height);
    let h = opts.height.min(ch).max(2) & !1;
    let w = (((cw as f64 * h as f64 / ch.max(1) as f64).round() as u32).max(2)) & !1;

    let mut input = ff::format::input(&src)?;
    let vstream = input.streams().best(ff::media::Type::Video).context("no video")?;
    let (v_index, v_tb, v_rate, v_st) = unsafe {
        let p = vstream.as_ptr();
        let afr = (*p).avg_frame_rate;
        (vstream.index(), (*p).time_base, if afr.num > 0 { afr } else { (*p).r_frame_rate }, p)
    };
    let a_index = input.streams().best(ff::media::Type::Audio).map(|s| s.index());
    let duration = info.duration.max(1);

    let mut out = ff::format::output(&dst)?;
    let fps = v_rate.num as f64 / v_rate.den.max(1) as f64;
    let mut cfg = EncoderConfig::new(VideoCodec::H264, w, h, v_rate);
    cfg.time_base = v_tb;
    cfg.gop = ((fps * opts.gop_seconds).round() as u32).max(1);
    cfg.hardware = opts.hardware;
    if opts.bit_rate > 0 {
        cfg.bit_rate = opts.bit_rate;
    }
    unsafe {
        let par = &*(*v_st).codecpar;
        cfg.color_space = if par.color_space == ffi::AVColorSpace::AVCOL_SPC_UNSPECIFIED {
            ffi::AVColorSpace::AVCOL_SPC_BT709
        } else {
            par.color_space
        };
        cfg.color_primaries = par.color_primaries;
        cfg.color_trc = par.color_trc;
    }
    let mut venc = open_video_encoder(&cfg)?;
    let enc_name = venc.name.clone();
    let enc_tb = venc.time_base;
    let pix_fmt = venc.pix_fmt;

    // Output streams: 0 = video, 1 = copied audio.
    {
        let mut ost = out.add_stream(ff::encoder::find(ff::codec::Id::None))?;
        unsafe {
            let s = ost.as_mut_ptr();
            check(ffi::avcodec_parameters_from_context((*s).codecpar, venc.encoder.as_ptr()), "video params")?;
            (*(*s).codecpar).codec_tag = venc.codec_tag();
            (*s).time_base = enc_tb;
            (*s).avg_frame_rate = v_rate;
            // Keep the display rotation of the source.
            let ipar = (*v_st).codecpar;
            let sd = ffi::av_packet_side_data_get(
                (*ipar).coded_side_data,
                (*ipar).nb_coded_side_data,
                ffi::AVPacketSideDataType::AV_PKT_DATA_DISPLAYMATRIX,
            );
            if !sd.is_null() {
                let nsd = ffi::av_packet_side_data_new(
                    &mut (*(*s).codecpar).coded_side_data,
                    &mut (*(*s).codecpar).nb_coded_side_data,
                    ffi::AVPacketSideDataType::AV_PKT_DATA_DISPLAYMATRIX,
                    (*sd).size,
                    0,
                );
                if !nsd.is_null() {
                    std::ptr::copy_nonoverlapping((*sd).data, (*nsd).data, (*sd).size);
                }
            }
        }
    }
    let mut a_tb_in = ffi::AVRational { num: 1, den: 1 };
    if let Some(ai) = a_index {
        let ist = input.stream(ai).unwrap();
        a_tb_in = unsafe { (*ist.as_ptr()).time_base };
        let mut ost = out.add_stream(ff::encoder::find(ff::codec::Id::None))?;
        unsafe {
            let s = ost.as_mut_ptr();
            check(ffi::avcodec_parameters_copy((*s).codecpar, (*ist.as_ptr()).codecpar), "audio params")?;
            (*(*s).codecpar).codec_tag = 0;
            (*s).time_base = a_tb_in;
        }
    }
    let mut mux_opts = ff::Dictionary::new();
    mux_opts.set("movflags", "+faststart");
    out.write_header_with(mux_opts)?;
    let v_tb_out = unsafe { (**(*out.as_ptr()).streams.add(0)).time_base };
    let a_tb_out = if a_index.is_some() { unsafe { (**(*out.as_ptr()).streams.add(1)).time_base } } else { a_tb_in };

    // Only demux what we use.
    unsafe {
        let fmt = input.as_ptr();
        for i in 0..(*fmt).nb_streams as usize {
            if i != v_index && Some(i) != a_index {
                (**(*fmt).streams.add(i)).discard = ffi::AVDiscard::AVDISCARD_ALL;
            }
        }
    }
    drop(dec);
    let start = format_start_ticks(unsafe { input.as_ptr() });

    let (tx, rx) = sync_channel::<Msg>(8);
    let mut frames = 0u64;
    let encode_result = std::thread::scope(|scope| -> Result<()> {
        // Encoder + muxer thread.
        let enc_thread = scope.spawn(move || -> Result<u64> {
            let mut pkt = ff::Packet::empty();
            let mut n = 0u64;
            let drain = |venc: &mut crate::encode::VideoEncoder, out: &mut ff::format::context::Output, pkt: &mut ff::Packet| -> Result<()> {
                loop {
                    let r = unsafe { ffi::avcodec_receive_packet(venc.encoder.as_mut_ptr(), pkt.as_mut_ptr()) };
                    if r == EAGAIN || r == ffi::AVERROR_EOF {
                        return Ok(());
                    }
                    check(r, "encode")?;
                    pkt.set_stream(0);
                    pkt.rescale_ts(ff::Rational::from(enc_tb), ff::Rational::from(v_tb_out));
                    pkt.write_interleaved(out)?;
                }
            };
            for msg in rx {
                match msg {
                    Msg::Video(f) => {
                        check(unsafe { ffi::avcodec_send_frame(venc.encoder.as_mut_ptr(), f.0) }, "send frame")?;
                        n += 1;
                        drain(&mut venc, &mut out, &mut pkt)?;
                    }
                    Msg::Audio(mut p) => {
                        p.set_stream(1);
                        p.rescale_ts(ff::Rational::from(a_tb_in), ff::Rational::from(a_tb_out));
                        p.set_position(-1);
                        p.write_interleaved(&mut out)?;
                    }
                }
            }
            unsafe { ffi::avcodec_send_frame(venc.encoder.as_mut_ptr(), std::ptr::null()) };
            drain(&mut venc, &mut out, &mut pkt)?;
            out.write_trailer()?;
            Ok(n)
        });

        // Decode + scale on this thread.
        let res = (|| -> Result<()> {
            let params = input.stream(v_index).unwrap().parameters();
            let mut ctx = ff::codec::context::Context::from_parameters(params)?;
            let codec = ff::decoder::find(ctx.id()).context("no decoder")?;
            let hwfmt = setup_hw(&mut ctx, codec, opts.hardware);
            unsafe {
                (*ctx.as_mut_ptr()).pkt_timebase = v_tb;
                (*ctx.as_mut_ptr()).thread_count = if hwfmt.is_some() { 1 } else { 0 };
            }
            let mut vdec = ctx.decoder().open_as(codec)?.video()?;
            let mut scaler = Scaler::bilinear();
            let raw = AvFrame::alloc();
            let sw = AvFrame::alloc();
            let mut pkt = ff::Packet::empty();
            let mut eof = false;
            let mut last_report = Instant::now();
            loop {
                // Drain decoded frames.
                loop {
                    let r = unsafe { ffi::avcodec_receive_frame(vdec.as_mut_ptr(), raw.0) };
                    if r == EAGAIN || r == ffi::AVERROR_EOF {
                        break;
                    }
                    check(r, "decode")?;
                    let src = unsafe {
                        if hwfmt.is_some_and(|f| (*raw.0).format == f as i32) {
                            ffi::av_frame_unref(sw.0);
                            check(ffi::av_hwframe_transfer_data(sw.0, raw.0, 0), "hw transfer")?;
                            ffi::av_frame_copy_props(sw.0, raw.0);
                            sw.0
                        } else {
                            raw.0
                        }
                    };
                    let outf = AvFrame::alloc();
                    let pts = unsafe {
                        let f = &mut *src;
                        let fmt: ffi::AVPixelFormat = std::mem::transmute::<i32, ffi::AVPixelFormat>(f.format);
                        if is_full_range_format(fmt) {
                            f.format = dejpeg(fmt) as i32;
                            f.color_range = ffi::AVColorRange::AVCOL_RANGE_JPEG;
                        }
                        let o = &mut *outf.0;
                        o.width = w as i32;
                        o.height = h as i32;
                        o.format = pix_fmt as i32;
                        o.colorspace = cfg.color_space;
                        o.color_primaries = f.color_primaries;
                        o.color_trc = f.color_trc;
                        o.color_range = ffi::AVColorRange::AVCOL_RANGE_MPEG;
                        let ts = if f.best_effort_timestamp != ffi::AV_NOPTS_VALUE { f.best_effort_timestamp } else { f.pts };
                        scaler.run(outf.0, src)?;
                        o.pts = ts;
                        o.duration = f.duration;
                        o.pict_type = ffi::AVPictureType::AV_PICTURE_TYPE_NONE;
                        ts
                    };
                    frames += 1;
                    if tx.send(Msg::Video(outf)).is_err() {
                        anyhow::bail!("encoder stopped");
                    }
                    if last_report.elapsed().as_millis() > 100 {
                        last_report = Instant::now();
                        let t = ts_to_ticks(pts, v_tb) - start;
                        if !progress((t as f64 / duration as f64).clamp(0.0, 1.0) as f32) {
                            anyhow::bail!("cancelled");
                        }
                    }
                }
                if eof {
                    break;
                }
                match pkt.read(&mut input) {
                    Ok(()) => {
                        let si = pkt.stream();
                        if si == v_index {
                            let r = unsafe { ffi::avcodec_send_packet(vdec.as_mut_ptr(), pkt.as_ptr()) };
                            if r < 0 && r != EAGAIN && r != ffi::AVERROR_INVALIDDATA {
                                check(r, "send packet")?;
                            }
                        } else if Some(si) == a_index {
                            let p = std::mem::replace(&mut pkt, ff::Packet::empty());
                            if tx.send(Msg::Audio(p)).is_err() {
                                anyhow::bail!("encoder stopped");
                            }
                        }
                    }
                    Err(ff::Error::Eof) => {
                        unsafe { ffi::avcodec_send_packet(vdec.as_mut_ptr(), std::ptr::null()) };
                        eof = true;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            Ok(())
        })();
        drop(tx);
        let enc = enc_thread.join().map_err(|_| anyhow::anyhow!("encoder thread panicked"))?;
        res?;
        enc?;
        Ok(())
    });
    encode_result?;
    progress(1.0);
    Ok(ProxyReport {
        path: dst.to_path_buf(),
        encoder: enc_name,
        hardware_decode: hw,
        frames,
        width: w,
        height: h,
        seconds: 0.0,
    })
}

fn setup_hw(ctx: &mut ff::codec::context::Context, codec: ff::Codec, hardware: bool) -> Option<ffi::AVPixelFormat> {
    if !hardware || std::env::var_os("JB_NO_HWDEC").is_some() {
        return None;
    }
    let (t, pix) = crate::hw::find_config(unsafe { codec.as_ptr() })?;
    let dev = crate::hw::device(t)?;
    unsafe {
        let c = ctx.as_mut_ptr();
        (*c).hw_device_ctx = dev;
        (*c).opaque = pix as i32 as isize as *mut libc::c_void;
        (*c).get_format = Some(crate::hw::get_format);
    }
    Some(pix)
}

//! Video / audio encoder selection. Hardware encoders are preferred
//! (VideoToolbox on macOS; NVENC, Quick Sync, AMF or Media Foundation on
//! Windows). The software fallbacks are LGPL-compatible: libopenh264 when
//! FFmpeg was built with it, then FFmpeg's native MPEG-4 Part 2 encoder.
//! GPL encoders (libx264 / libx265) are never used.

use anyhow::{Context as _, Result};
use ffmpeg_next as ff;
use ff::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    Hevc,
}

#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub codec: VideoCodec,
    pub width: u32,
    pub height: u32,
    /// Time base of the frames' pts.
    pub time_base: ffi::AVRational,
    /// Nominal frame rate (rate control and container metadata).
    pub frame_rate: ffi::AVRational,
    /// Target bit rate in bits per second.
    pub bit_rate: u64,
    /// Keyframe interval in frames.
    pub gop: u32,
    /// Allow hardware encoders.
    pub hardware: bool,
    /// Force a specific FFmpeg encoder by name.
    pub encoder: Option<String>,
    /// Set when the output container needs global headers (MP4 does).
    pub global_header: bool,
    /// Color tagging of the encoded stream.
    pub color_space: ffi::AVColorSpace,
    pub color_primaries: ffi::AVColorPrimaries,
    pub color_trc: ffi::AVColorTransferCharacteristic,
    pub full_range: bool,
}

impl EncoderConfig {
    pub fn new(codec: VideoCodec, width: u32, height: u32, frame_rate: ffi::AVRational) -> Self {
        Self {
            codec,
            width,
            height,
            time_base: ffi::AVRational { num: frame_rate.den, den: frame_rate.num },
            frame_rate,
            bit_rate: default_bit_rate(width, height, frame_rate.num as f64 / frame_rate.den.max(1) as f64),
            gop: ((frame_rate.num as f64 / frame_rate.den.max(1) as f64) * 2.0).round().max(1.0) as u32,
            hardware: true,
            encoder: None,
            global_header: true,
            color_space: ffi::AVColorSpace::AVCOL_SPC_BT709,
            color_primaries: ffi::AVColorPrimaries::AVCOL_PRI_BT709,
            color_trc: ffi::AVColorTransferCharacteristic::AVCOL_TRC_BT709,
            full_range: false,
        }
    }
}

/// A sensible bit rate: 0.2 bits per pixel per frame (about 12 Mbps for
/// 1080p30, 50 Mbps for 4K30), similar to common editor defaults.
pub fn default_bit_rate(w: u32, h: u32, fps: f64) -> u64 {
    ((w as f64 * h as f64 * fps.clamp(1.0, 240.0) * 0.2) as u64).clamp(500_000, 200_000_000)
}

struct Candidate {
    name: &'static str,
    codec: VideoCodec,
    hardware: bool,
    pix_fmt: ffi::AVPixelFormat,
}

fn candidates(codec: VideoCodec, hardware: bool) -> Vec<Candidate> {
    use ffi::AVPixelFormat::{AV_PIX_FMT_NV12 as NV12, AV_PIX_FMT_YUV420P as YUV};
    let mut v = Vec::new();
    let hw_names: &[(&str, VideoCodec)] = if cfg!(target_os = "macos") {
        &[("hevc_videotoolbox", VideoCodec::Hevc), ("h264_videotoolbox", VideoCodec::H264)]
    } else if cfg!(target_os = "windows") {
        &[
            ("hevc_nvenc", VideoCodec::Hevc),
            ("hevc_qsv", VideoCodec::Hevc),
            ("hevc_amf", VideoCodec::Hevc),
            ("hevc_mf", VideoCodec::Hevc),
            ("h264_nvenc", VideoCodec::H264),
            ("h264_qsv", VideoCodec::H264),
            ("h264_amf", VideoCodec::H264),
            ("h264_mf", VideoCodec::H264),
        ]
    } else {
        &[("hevc_vaapi", VideoCodec::Hevc), ("hevc_nvenc", VideoCodec::Hevc), ("h264_nvenc", VideoCodec::H264)]
    };
    if hardware && std::env::var_os("JB_NO_HWENC").is_none() {
        // The requested codec first; HEVC falls back to hardware H.264
        // before any software encoder.
        for (n, c) in hw_names.iter().filter(|(_, c)| *c == codec) {
            v.push(Candidate { name: n, codec: *c, hardware: true, pix_fmt: NV12 });
        }
        if codec == VideoCodec::Hevc {
            for (n, c) in hw_names.iter().filter(|(_, c)| *c == VideoCodec::H264) {
                v.push(Candidate { name: n, codec: *c, hardware: true, pix_fmt: NV12 });
            }
        }
    }
    v.push(Candidate { name: "libopenh264", codec: VideoCodec::H264, hardware: false, pix_fmt: YUV });
    v
}

/// An opened video encoder.
pub struct VideoEncoder {
    pub encoder: ff::encoder::Video,
    /// FFmpeg encoder name, e.g. "hevc_videotoolbox".
    pub name: String,
    /// Codec actually produced (an HEVC request may fall back to H.264).
    pub codec_id: ffi::AVCodecID,
    pub hardware: bool,
    /// Pixel format the frames must be in.
    pub pix_fmt: ffi::AVPixelFormat,
    pub time_base: ffi::AVRational,
}

impl VideoEncoder {
    /// MP4 codec tag to use (hvc1 for HEVC so Apple players accept it).
    pub fn codec_tag(&self) -> u32 {
        if self.codec_id == ffi::AVCodecID::AV_CODEC_ID_HEVC {
            u32::from_le_bytes(*b"hvc1")
        } else {
            0
        }
    }
}

/// Opens the best available encoder for `cfg`.
pub fn open_video_encoder(cfg: &EncoderConfig) -> Result<VideoEncoder> {
    crate::init();
    let mut list = candidates(cfg.codec, cfg.hardware);
    if let Some(name) = &cfg.encoder {
        let pix = if name.contains("openh264") || name == "mpeg4" {
            ffi::AVPixelFormat::AV_PIX_FMT_YUV420P
        } else {
            ffi::AVPixelFormat::AV_PIX_FMT_NV12
        };
        let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
        list = vec![Candidate {
            name: leaked,
            codec: if name.contains("hevc") { VideoCodec::Hevc } else { VideoCodec::H264 },
            hardware: !(name.contains("openh264") || name == "mpeg4"),
            pix_fmt: pix,
        }];
    } else {
        list.push(Candidate {
            name: "mpeg4",
            codec: VideoCodec::H264,
            hardware: false,
            pix_fmt: ffi::AVPixelFormat::AV_PIX_FMT_YUV420P,
        });
    }
    let mut errors = Vec::new();
    for c in list {
        match try_open(cfg, &c) {
            Ok(e) => return Ok(e),
            Err(e) => errors.push(format!("{}: {e}", c.name)),
        }
    }
    anyhow::bail!("no usable video encoder ({})", errors.join("; "))
}

fn try_open(cfg: &EncoderConfig, c: &Candidate) -> Result<VideoEncoder> {
    let codec = ff::encoder::find_by_name(c.name).context("not built into FFmpeg")?;
    let ctx = ff::codec::context::Context::new_with_codec(codec);
    let mut enc = ctx.encoder().video()?;
    enc.set_width(cfg.width);
    enc.set_height(cfg.height);
    enc.set_time_base(ff::Rational::from(cfg.time_base));
    enc.set_frame_rate(Some(ff::Rational::from(cfg.frame_rate)));
    enc.set_gop(cfg.gop);
    enc.set_bit_rate(cfg.bit_rate as usize);
    enc.set_max_bit_rate((cfg.bit_rate * 3 / 2) as usize);
    unsafe {
        let p = enc.as_mut_ptr();
        (*p).pix_fmt = c.pix_fmt;
        (*p).colorspace = cfg.color_space;
        (*p).color_primaries = cfg.color_primaries;
        (*p).color_trc = cfg.color_trc;
        (*p).color_range =
            if cfg.full_range { ffi::AVColorRange::AVCOL_RANGE_JPEG } else { ffi::AVColorRange::AVCOL_RANGE_MPEG };
        (*p).sample_aspect_ratio = ffi::AVRational { num: 1, den: 1 };
        if cfg.global_header {
            (*p).flags |= ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        }
        if !c.hardware {
            (*p).thread_count = 0;
        }
        if c.name == "mpeg4" {
            // MPEG-4 Part 2 at a constant quantizer looks far better than its
            // bit rate mode; q=3 is visually close to transparent.
            (*p).flags |= ffi::AV_CODEC_FLAG_QSCALE as i32;
            (*p).global_quality = 3 * ffi::FF_QP2LAMBDA as i32;
            (*p).max_b_frames = 2;
        }
    }
    let mut opts = ff::Dictionary::new();
    match c.name {
        n if n.ends_with("_videotoolbox") => {
            opts.set("allow_sw", "0");
            opts.set("realtime", "0");
            opts.set("prio_speed", "1");
        }
        n if n.ends_with("_nvenc") => {
            opts.set("preset", "p4");
            opts.set("rc", "vbr");
        }
        n if n.ends_with("_qsv") => {
            opts.set("preset", "medium");
        }
        n if n.ends_with("_amf") => {
            opts.set("quality", "balanced");
        }
        n if n.ends_with("_mf") => {
            opts.set("hw_encoding", "1");
            opts.set("rate_control", "cbr");
        }
        _ => {}
    }
    let opened = enc.open_with(opts)?;
    let codec_id = unsafe { (*opened.as_ptr()).codec_id };
    let tb = unsafe { (*opened.as_ptr()).time_base };
    let _ = c.codec;
    Ok(VideoEncoder {
        encoder: opened,
        name: c.name.to_string(),
        codec_id,
        hardware: c.hardware,
        pix_fmt: c.pix_fmt,
        time_base: tb,
    })
}

/// Opens FFmpeg's native AAC encoder (LGPL) for interleaved stereo at `rate`.
pub fn open_aac_encoder(rate: u32, bit_rate: u64, global_header: bool) -> Result<ff::encoder::Audio> {
    crate::init();
    let codec = ff::encoder::find_by_name("aac").context("native aac encoder missing")?;
    let ctx = ff::codec::context::Context::new_with_codec(codec);
    let mut enc = ctx.encoder().audio()?;
    unsafe {
        let p = enc.as_mut_ptr();
        (*p).sample_rate = rate as i32;
        (*p).sample_fmt = ffi::AVSampleFormat::AV_SAMPLE_FMT_FLTP;
        ffi::av_channel_layout_default(&mut (*p).ch_layout, 2);
        (*p).bit_rate = bit_rate as i64;
        (*p).time_base = ffi::AVRational { num: 1, den: rate as i32 };
        if global_header {
            (*p).flags |= ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        }
    }
    Ok(enc.open()?)
}

/// Names of the video encoders that can actually be opened here, in
/// preference order (diagnostics).
pub fn available_video_encoders() -> Vec<String> {
    crate::init();
    let mut out = Vec::new();
    for codec in [VideoCodec::Hevc, VideoCodec::H264] {
        let cfg = EncoderConfig::new(codec, 640, 360, ffi::AVRational { num: 30, den: 1 });
        for c in candidates(codec, true).into_iter().chain(std::iter::once(Candidate {
            name: "mpeg4",
            codec: VideoCodec::H264,
            hardware: false,
            pix_fmt: ffi::AVPixelFormat::AV_PIX_FMT_YUV420P,
        })) {
            if !out.iter().any(|n| n == c.name) && try_open(&cfg, &c).is_ok() {
                out.push(c.name.to_string());
            }
        }
    }
    out
}

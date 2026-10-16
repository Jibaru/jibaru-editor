//! A single video decoder with hardware acceleration, efficient sequential
//! access and frame-accurate seeking.

use crate::convert::{decoded_to_video_frame, AvFrame, Scaler};
use crate::probe::{probe_input, ProbeInfo};
use crate::util::*;
use anyhow::{Context as _, Result};
use ff::ffi;
use ff::packet::Ref as _;
use ffmpeg_next as ff;
use jb_core::frame::VideoFrame;
use jb_core::time::{Ticks, TICKS_PER_SECOND};
use std::path::Path;

/// How a requested time is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeMode {
    /// Decode up to the exact frame displayed at the requested time.
    Accurate,
    /// Return the first frame after seeking to the preceding keyframe
    /// (thumbnails, fast scrubbing).
    Keyframe,
}

/// Forward distance under which decoding on is preferred to seeking, when
/// the container index cannot tell where the next keyframe is.
const MAX_DECODE_AHEAD: Ticks = 2 * TICKS_PER_SECOND;

/// Source times within this of a frame start select that frame. Project
/// files store in points truncated to whole ticks (Filmora writes frame 223
/// at 29.97 fps as 74407666, 0.67 tick before the frame), and Filmora shows
/// the frame that starts there, so tick rounding must not fall back one frame.
pub(crate) const FRAME_SNAP: Ticks = TICKS_PER_SECOND / 1000;

#[derive(Debug, Default, Clone, Copy)]
pub struct DecodeStats {
    pub frames_decoded: u64,
    pub frames_output: u64,
    pub seeks: u64,
}

pub struct VideoDecoder {
    input: ff::format::context::Input,
    stream_index: usize,
    time_base: ffi::AVRational,
    start: Ticks,
    decoder: ff::decoder::Video,
    hw_pix_fmt: Option<ffi::AVPixelFormat>,
    hw_name: Option<String>,
    frame_duration: Ticks,
    info: ProbeInfo,
    scaler: Scaler,
    packet: ff::Packet,
    raw: AvFrame,
    next_raw: AvFrame,
    sw: AvFrame,
    /// Source time of the most recently decoded frame.
    position: Option<Ticks>,
    eof: bool,
    last_output: Option<VideoFrame>,
    last_output_size: Option<(u32, u32)>,
    last_duration: Ticks,
    since_seek: u64,
    pub stats: DecodeStats,
}

impl VideoDecoder {
    /// Opens the best video stream of `path`. With `hardware`, a platform
    /// hardware decoder is used when the codec supports it, falling back to
    /// software otherwise.
    pub fn open(path: &Path, hardware: bool) -> Result<Self> {
        crate::init();
        let input = ff::format::input(&path).with_context(|| format!("cannot open {}", path.display()))?;
        let info = probe_input(path, &input)?;
        let stream = input.streams().best(ff::media::Type::Video).context("no video stream")?;
        let stream_index = stream.index();
        let time_base = unsafe { (*stream.as_ptr()).time_base };
        let start = format_start_ticks(unsafe { input.as_ptr() });
        let params = stream.parameters();
        let fr = info.frame_rate.map(|r| r.frame_duration()).unwrap_or(TICKS_PER_SECOND / 30);
        let (decoder, hw_pix_fmt, hw_name) = open_codec(params, time_base, hardware)?;
        // Only the selected stream is demuxed.
        unsafe {
            let fmt = input.as_ptr();
            for i in 0..(*fmt).nb_streams as usize {
                let st = *(*fmt).streams.add(i);
                if i != stream_index {
                    (*st).discard = ffi::AVDiscard::AVDISCARD_ALL;
                }
            }
        }
        Ok(Self {
            input,
            stream_index,
            time_base,
            start,
            decoder,
            hw_pix_fmt,
            hw_name,
            frame_duration: fr,
            info,
            scaler: Scaler::bilinear(),
            packet: ff::Packet::empty(),
            raw: AvFrame::alloc(),
            next_raw: AvFrame::alloc(),
            sw: AvFrame::alloc(),
            position: None,
            eof: false,
            last_output: None,
            last_output_size: None,
            last_duration: fr,
            since_seek: 0,
            stats: DecodeStats::default(),
        })
    }

    pub fn info(&self) -> &ProbeInfo {
        &self.info
    }

    /// Name of the hardware device in use ("videotoolbox", "d3d11va"...),
    /// or `None` for software decoding.
    pub fn hardware(&self) -> Option<&str> {
        self.hw_name.as_deref()
    }

    /// Source time of the last decoded frame.
    pub fn position(&self) -> Option<Ticks> {
        self.position
    }

    /// Nominal frame duration from the stream frame rate.
    pub fn frame_duration(&self) -> Ticks {
        self.frame_duration
    }

    /// Display duration of the last returned frame.
    pub fn last_duration(&self) -> Ticks {
        self.last_duration
    }

    /// Whether the decoder reached the end of the stream (the last frame is
    /// being held).
    pub fn at_end(&self) -> bool {
        self.eof
    }

    /// Frame displayed at `t` (source time), scaled to fit `max_size`.
    /// Past the end of the stream the last frame is held.
    pub fn frame_at(&mut self, t: Ticks, max_size: Option<(u32, u32)>, mode: DecodeMode) -> Result<Option<VideoFrame>> {
        match self.frame_at_inner(t, max_size, mode) {
            Err(e) if self.hw_name.is_some() => {
                // Hardware decoders can fail mid-stream (unsupported profile,
                // lost device): continue in software.
                let path = self.info.path.clone();
                let stats = self.stats;
                *self = Self::open(&path, false).with_context(|| format!("software fallback after: {e}"))?;
                self.stats = stats;
                self.frame_at_inner(t, max_size, mode)
            }
            r => r,
        }
    }

    fn frame_at_inner(&mut self, t: Ticks, max_size: Option<(u32, u32)>, mode: DecodeMode) -> Result<Option<VideoFrame>> {
        let t = t.max(0) + FRAME_SNAP;
        if let Some(last) = &self.last_output {
            let same_size = self.last_output_size == max_size;
            if same_size && t >= last.pts && (t < last.pts + self.last_duration || self.eof) {
                return Ok(Some(last.clone()));
            }
        }
        let behind = self.position.is_none_or(|p| t < p);
        let need_seek = match mode {
            DecodeMode::Keyframe => true,
            DecodeMode::Accurate => behind || self.position.is_some_and(|p| self.seek_helps(p, t)),
        };
        let found = if need_seek { self.seek_and_decode(t, mode)? } else { self.decode_until(t, mode)? };
        match found {
            true => {
                self.last_duration = self.raw_duration();
                self.output(max_size)?;
            }
            false => {
                // End of stream: hold the last frame, re-converting if needed.
                let stale = self.last_output.as_ref().is_none_or(|f| Some(f.pts) != self.position);
                if (stale || self.last_output_size != max_size) && unsafe { !(*self.raw.0).buf[0].is_null() } {
                    self.output(max_size)?;
                }
            }
        }
        Ok(self.last_output.clone())
    }

    /// Seeks before `t` and decodes up to it. Containers seek by decode
    /// timestamp, so with B-frames the keyframe found can still be after
    /// `t`; in that case seek further back.
    fn seek_and_decode(&mut self, t: Ticks, mode: DecodeMode) -> Result<bool> {
        let mut back = 0;
        loop {
            self.seek(t - back)?;
            let found = self.decode_until(t, mode)?;
            let overshoot = found && self.since_seek == 1 && self.position.is_some_and(|p| p > t);
            if !overshoot || t - back <= 0 || back >= 8 * TICKS_PER_SECOND {
                return Ok(found);
            }
            back = if back == 0 { TICKS_PER_SECOND / 2 } else { back * 2 };
        }
    }

    /// Whether seeking to `t` gets closer than decoding forward from `pos`.
    fn seek_helps(&self, pos: Ticks, t: Ticks) -> bool {
        if t <= pos + self.frame_duration * 2 {
            return false;
        }
        let st = unsafe { *(*self.input.as_ptr()).streams.add(self.stream_index) };
        let target = ticks_to_ts(t + self.start, self.time_base);
        let idx = unsafe { ffi::av_index_search_timestamp(st, target, ffi::AVSEEK_FLAG_BACKWARD) };
        if idx >= 0 {
            let e = unsafe { ffi::avformat_index_get_entry(st, idx) };
            if !e.is_null() {
                let key = ts_to_ticks(unsafe { (*e).timestamp }, self.time_base) - self.start;
                // Seek only if the keyframe before `t` lies ahead of us.
                return key > pos + self.frame_duration;
            }
        }
        t - pos > MAX_DECODE_AHEAD
    }

    fn seek(&mut self, t: Ticks) -> Result<()> {
        let ts = ticks_to_ts(t + self.start, self.time_base);
        let ret = unsafe {
            ffi::av_seek_frame(self.input.as_mut_ptr(), self.stream_index as i32, ts, ffi::AVSEEK_FLAG_BACKWARD)
        };
        if ret < 0 {
            // Some files only seek in byte or global time; try the generic path.
            let us = (t + self.start) / (TICKS_PER_SECOND / ffi::AV_TIME_BASE as i64);
            check(unsafe { ffi::avformat_seek_file(self.input.as_mut_ptr(), -1, i64::MIN, us, us, 0) }, "seek")?;
        }
        self.decoder.flush();
        self.position = None;
        self.eof = false;
        self.stats.seeks += 1;
        self.since_seek = 0;
        Ok(())
    }

    /// Decodes frames until one covering `t` is in `self.raw`. Returns false
    /// at end of stream.
    fn decode_until(&mut self, t: Ticks, mode: DecodeMode) -> Result<bool> {
        loop {
            match self.receive()? {
                Some(pts) => {
                    self.position = Some(pts);
                    self.stats.frames_decoded += 1;
                    self.since_seek += 1;
                    let dur = self.raw_duration();
                    if mode == DecodeMode::Keyframe || pts + dur > t {
                        return Ok(true);
                    }
                }
                None => {
                    if self.eof {
                        return Ok(false);
                    }
                    self.feed()?;
                }
            }
        }
    }

    fn raw_duration(&self) -> Ticks {
        let d = unsafe { (*self.raw.0).duration };
        if d > 0 {
            ts_to_ticks(d, self.time_base)
        } else {
            self.frame_duration
        }
    }

    /// Receives one decoded frame into `self.raw`, returning its source time.
    fn receive(&mut self) -> Result<Option<Ticks>> {
        // Receive into a scratch frame so `raw` keeps the last good frame
        // when the decoder reports end of stream.
        let ret = unsafe { ffi::avcodec_receive_frame(self.decoder.as_mut_ptr(), self.next_raw.0) };
        if ret == EAGAIN {
            return Ok(None);
        }
        if ret == ffi::AVERROR_EOF {
            self.eof = true;
            return Ok(None);
        }
        check(ret, "decode")?;
        unsafe {
            ffi::av_frame_unref(self.raw.0);
            ffi::av_frame_move_ref(self.raw.0, self.next_raw.0);
        }
        let ts = unsafe {
            let f = &*self.raw.0;
            if f.best_effort_timestamp != ffi::AV_NOPTS_VALUE {
                f.best_effort_timestamp
            } else {
                f.pts
            }
        };
        let pts = if ts == ffi::AV_NOPTS_VALUE {
            self.position.map_or(0, |p| p + self.frame_duration)
        } else {
            ts_to_ticks(ts, self.time_base) - self.start
        };
        Ok(Some(pts))
    }

    /// Sends the next packet of our stream to the decoder (or EOF).
    fn feed(&mut self) -> Result<()> {
        loop {
            match self.packet.read(&mut self.input) {
                Ok(()) => {
                    if self.packet.stream() != self.stream_index {
                        continue;
                    }
                    let ret = unsafe { ffi::avcodec_send_packet(self.decoder.as_mut_ptr(), self.packet.as_ptr()) };
                    if ret < 0 && ret != EAGAIN && ret != ffi::AVERROR_INVALIDDATA {
                        check(ret, "send packet")?;
                    }
                    return Ok(());
                }
                Err(ff::Error::Eof) => {
                    unsafe { ffi::avcodec_send_packet(self.decoder.as_mut_ptr(), std::ptr::null()) };
                    return Ok(());
                }
                Err(ff::Error::Other { errno }) if errno == libc::EAGAIN => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Converts the frame in `self.raw` into `self.last_output`.
    fn output(&mut self, max_size: Option<(u32, u32)>) -> Result<()> {
        let pts = self.position.unwrap_or(0);
        let frame = unsafe {
            let is_hw = self.hw_pix_fmt.is_some_and(|f| (*self.raw.0).format == f as i32);
            let src = if is_hw {
                ffi::av_frame_unref(self.sw.0);
                // Map the hardware surface (zero copy where supported, e.g.
                // VideoToolbox); fall back to a download.
                if std::env::var_os("JB_HW_TRANSFER").is_some()
                    || ffi::av_hwframe_map(self.sw.0, self.raw.0, ffi::AV_HWFRAME_MAP_READ as i32) < 0
                {
                    ffi::av_frame_unref(self.sw.0);
                    check(ffi::av_hwframe_transfer_data(self.sw.0, self.raw.0, 0), "hw transfer")?;
                }
                ffi::av_frame_copy_props(self.sw.0, self.raw.0);
                self.sw.0
            } else {
                self.raw.0
            };
            decoded_to_video_frame(&mut self.scaler, src, self.info.rotation, max_size, pts)?
        };
        self.stats.frames_output += 1;
        self.last_output = Some(frame);
        self.last_output_size = max_size;
        Ok(())
    }
}

fn open_codec(
    params: ff::codec::Parameters,
    time_base: ffi::AVRational,
    hardware: bool,
) -> Result<(ff::decoder::Video, Option<ffi::AVPixelFormat>, Option<String>)> {
    let mut ctx = ff::codec::context::Context::from_parameters(params)?;
    let codec_ref = ff::decoder::find(ctx.id()).context("no decoder for this codec")?;
    let codec = unsafe { codec_ref.as_ptr() };
    let mut hw = None;
    if hardware && std::env::var_os("JB_NO_HWDEC").is_none() {
        if let Some((dev_type, pix)) = crate::hw::find_config(codec) {
            if let Some(dev) = crate::hw::device(dev_type) {
                unsafe {
                    let c = ctx.as_mut_ptr();
                    (*c).hw_device_ctx = dev;
                    (*c).opaque = pix as i32 as isize as *mut libc::c_void;
                    (*c).get_format = Some(crate::hw::get_format);
                }
                hw = Some((pix, crate::hw::device_type_name(dev_type)));
            }
        }
    }
    unsafe {
        let c = ctx.as_mut_ptr();
        // Frame threading for software decoding; hardware decoders need none.
        (*c).thread_count = if hw.is_some() { 1 } else { 0 };
        (*c).thread_type = ffi::FF_THREAD_FRAME as i32 | ffi::FF_THREAD_SLICE as i32;
        (*c).pkt_timebase = time_base;
    }
    let opened = ctx.decoder().open_as(codec_ref)?.video()?;
    Ok((opened, hw.as_ref().map(|h| h.0), hw.map(|h| h.1)))
}

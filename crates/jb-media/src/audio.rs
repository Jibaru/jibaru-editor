//! Audio decoding with resampling to interleaved f32 stereo at a fixed
//! output rate, with sample-accurate random access.

use crate::convert::AvFrame;
use crate::util::*;
use anyhow::{Context as _, Result};
use ffmpeg_next as ff;
use ff::ffi;
use ff::packet::Ref as _;
use jb_core::time::{Ticks, TICKS_PER_SECOND};
use std::collections::VecDeque;
use std::path::Path;

/// Output channel count (interleaved stereo).
pub const CHANNELS: usize = 2;

/// Forward gap under which decoding on is preferred to seeking.
const MAX_DECODE_AHEAD: Ticks = TICKS_PER_SECOND;
/// Decoded audio kept behind the read position, so small rewinds (and
/// overlapping reads from the mixer) do not seek.
const KEEP_BEHIND: Ticks = TICKS_PER_SECOND / 2;
/// Pre-roll before the target on seek so codec priming settles.
const SEEK_PREROLL: Ticks = TICKS_PER_SECOND / 10;

struct Resampler(*mut ffi::SwrContext);
unsafe impl Send for Resampler {}
impl Drop for Resampler {
    fn drop(&mut self) {
        unsafe { ffi::swr_free(&mut self.0) };
    }
}

/// Converts a sample index at `rate` to ticks.
pub fn samples_to_ticks(s: i64, rate: u32) -> Ticks {
    ((s as i128 * TICKS_PER_SECOND as i128) / rate as i128) as Ticks
}

/// Converts ticks to the nearest sample index at `rate`.
pub fn ticks_to_samples(t: Ticks, rate: u32) -> i64 {
    let v = t as i128 * rate as i128;
    let d = TICKS_PER_SECOND as i128;
    (if v >= 0 { (v + d / 2) / d } else { (v - d / 2) / d }) as i64
}

pub struct AudioDecoder {
    input: ff::format::context::Input,
    stream_index: usize,
    time_base: ffi::AVRational,
    start: Ticks,
    decoder: ff::decoder::Audio,
    rate: u32,
    swr: Option<Resampler>,
    swr_key: (i32, i32, u64),
    out_channels: usize,
    packet: ff::Packet,
    frame: AvFrame,
    out: AvFrame,
    /// Decoded interleaved samples; `buf_start` is the sample index of the first.
    buf: VecDeque<f32>,
    buf_start: i64,
    /// Sample index where the next decoded sample lands, once known.
    next: Option<i64>,
    eof: bool,
    pub seeks: u64,
}

impl AudioDecoder {
    /// Opens the best audio stream of `path`; output is resampled to `rate`.
    pub fn open(path: &Path, rate: u32) -> Result<Self> {
        crate::init();
        let input = ff::format::input(&path).with_context(|| format!("cannot open {}", path.display()))?;
        let stream = input.streams().best(ff::media::Type::Audio).context("no audio stream")?;
        let stream_index = stream.index();
        let time_base = unsafe { (*stream.as_ptr()).time_base };
        let start = format_start_ticks(unsafe { input.as_ptr() });
        let mut ctx = ff::codec::context::Context::from_parameters(stream.parameters())?;
        unsafe { (*ctx.as_mut_ptr()).pkt_timebase = time_base };
        let codec = ff::decoder::find(ctx.id()).context("no decoder for this audio codec")?;
        let decoder = ctx.decoder().open_as(codec)?.audio()?;
        unsafe {
            let fmt = input.as_ptr();
            for i in 0..(*fmt).nb_streams as usize {
                if i != stream_index {
                    (**(*fmt).streams.add(i)).discard = ffi::AVDiscard::AVDISCARD_ALL;
                }
            }
        }
        Ok(Self {
            input,
            stream_index,
            time_base,
            start,
            decoder,
            rate,
            swr: None,
            swr_key: (0, 0, 0),
            out_channels: CHANNELS,
            packet: ff::Packet::empty(),
            frame: AvFrame::alloc(),
            out: AvFrame::alloc(),
            buf: VecDeque::new(),
            buf_start: 0,
            next: None,
            eof: false,
            seeks: 0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Position (source time) up to which audio has been decoded.
    pub fn position(&self) -> Option<Ticks> {
        self.next.map(|n| samples_to_ticks(n, self.rate))
    }

    /// Reads `frames` stereo frames starting at source time `start`.
    /// Returns `frames * 2` interleaved samples; silence where the source
    /// has no audio (before its start or after its end).
    pub fn read(&mut self, start: Ticks, frames: usize) -> Result<Vec<f32>> {
        let s0 = ticks_to_samples(start, self.rate);
        self.read_samples(s0, frames)
    }

    /// Like [`read`](Self::read) with the start given as a sample index.
    pub fn read_samples(&mut self, s0: i64, frames: usize) -> Result<Vec<f32>> {
        let s1 = s0 + frames as i64;
        let buf_end = self.buf_start + (self.buf.len() / CHANNELS) as i64;
        let behind = s0 < self.buf_start;
        let ahead = self
            .next
            .is_none_or(|n| samples_to_ticks(s0 - n, self.rate) > MAX_DECODE_AHEAD);
        if (behind || ahead) && !(s0 >= self.buf_start && s1 <= buf_end) {
            self.seek(samples_to_ticks(s0, self.rate))?;
        }
        while !self.eof && self.next.is_none_or(|n| n < s1) {
            self.decode_some()?;
        }
        let mut out = vec![0f32; frames * CHANNELS];
        let have = (self.buf.len() / CHANNELS) as i64;
        let from = (s0 - self.buf_start).max(0);
        let to = (s1 - self.buf_start).min(have);
        for i in from..to {
            let o = ((self.buf_start + i - s0) as usize) * CHANNELS;
            let b = i as usize * CHANNELS;
            out[o] = self.buf[b];
            out[o + 1] = self.buf[b + 1];
        }
        // Drop what is far behind the read position.
        let keep_from = s0 - ticks_to_samples(KEEP_BEHIND, self.rate);
        if keep_from > self.buf_start {
            let n = ((keep_from - self.buf_start) as usize).min(self.buf.len() / CHANNELS);
            self.buf.drain(..n * CHANNELS);
            self.buf_start += n as i64;
        }
        Ok(out)
    }

    fn seek(&mut self, t: Ticks) -> Result<()> {
        let target = (t - SEEK_PREROLL).max(0);
        let ts = ticks_to_ts(target + self.start, self.time_base);
        let ret = unsafe {
            ffi::av_seek_frame(self.input.as_mut_ptr(), self.stream_index as i32, ts, ffi::AVSEEK_FLAG_BACKWARD)
        };
        if ret < 0 {
            let us = (target + self.start) / (TICKS_PER_SECOND / ffi::AV_TIME_BASE as i64);
            check(
                unsafe { ffi::avformat_seek_file(self.input.as_mut_ptr(), -1, i64::MIN, us, us, 0) },
                "audio seek",
            )?;
        }
        self.decoder.flush();
        self.swr = None;
        self.buf.clear();
        self.next = None;
        self.eof = false;
        self.seeks += 1;
        Ok(())
    }

    /// Decodes one packet's worth of audio into the buffer.
    fn decode_some(&mut self) -> Result<()> {
        loop {
            let ret = unsafe { ffi::avcodec_receive_frame(self.decoder.as_mut_ptr(), self.frame.0) };
            if ret == 0 {
                self.push_frame()?;
                return Ok(());
            }
            if ret == ffi::AVERROR_EOF {
                self.eof = true;
                self.flush_resampler()?;
                return Ok(());
            }
            if ret != EAGAIN {
                check(ret, "audio decode")?;
            }
            match self.packet.read(&mut self.input) {
                Ok(()) => {
                    if self.packet.stream() == self.stream_index {
                        let r = unsafe { ffi::avcodec_send_packet(self.decoder.as_mut_ptr(), self.packet.as_ptr()) };
                        if r < 0 && r != EAGAIN && r != ffi::AVERROR_INVALIDDATA {
                            check(r, "audio send packet")?;
                        }
                    }
                }
                Err(ff::Error::Eof) => unsafe {
                    ffi::avcodec_send_packet(self.decoder.as_mut_ptr(), std::ptr::null());
                },
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn resampler(&mut self) -> Result<*mut ffi::SwrContext> {
        let f = unsafe { &*self.frame.0 };
        let key = (f.format, f.sample_rate, f.ch_layout.nb_channels as u64 | ((f.ch_layout.order as u64) << 32));
        if self.swr.is_none() || self.swr_key != key {
            self.flush_resampler()?;
            let mut in_layout: ffi::AVChannelLayout = unsafe { std::mem::zeroed() };
            unsafe {
                if f.ch_layout.order == ffi::AVChannelOrder::AV_CHANNEL_ORDER_UNSPEC || f.ch_layout.nb_channels == 0 {
                    ffi::av_channel_layout_default(&mut in_layout, f.ch_layout.nb_channels.max(1));
                } else {
                    ffi::av_channel_layout_copy(&mut in_layout, &f.ch_layout);
                }
            }
            // Mono is resampled as mono and duplicated, so it plays at full
            // level on both sides (swresample's upmix would attenuate it).
            self.out_channels = if in_layout.nb_channels == 1 { 1 } else { CHANNELS };
            let mut out_layout: ffi::AVChannelLayout = unsafe { std::mem::zeroed() };
            unsafe { ffi::av_channel_layout_default(&mut out_layout, self.out_channels as i32) };
            let mut swr: *mut ffi::SwrContext = std::ptr::null_mut();
            unsafe {
                check(
                    ffi::swr_alloc_set_opts2(
                        &mut swr,
                        &out_layout,
                        ffi::AVSampleFormat::AV_SAMPLE_FMT_FLT,
                        self.rate as i32,
                        &in_layout,
                        std::mem::transmute::<i32, ffi::AVSampleFormat>(f.format),
                        f.sample_rate,
                        0,
                        std::ptr::null_mut(),
                    ),
                    "swr alloc",
                )?;
                ffi::av_channel_layout_uninit(&mut in_layout);
                let r = ffi::swr_init(swr);
                if r < 0 {
                    ffi::swr_free(&mut swr);
                    check(r, "swr init")?;
                }
            }
            self.swr = Some(Resampler(swr));
            self.swr_key = key;
        }
        Ok(self.swr.as_ref().unwrap().0)
    }

    fn push_frame(&mut self) -> Result<()> {
        if self.next.is_none() {
            let f = unsafe { &*self.frame.0 };
            let ts = if f.best_effort_timestamp != ffi::AV_NOPTS_VALUE { f.best_effort_timestamp } else { f.pts };
            let t = if ts == ffi::AV_NOPTS_VALUE { 0 } else { ts_to_ticks(ts, self.time_base) - self.start };
            let n = ticks_to_samples(t, self.rate);
            self.next = Some(n);
            self.buf.clear();
            self.buf_start = n;
        }
        let swr = self.resampler()?;
        self.convert(swr, self.frame.0)
    }

    fn convert(&mut self, swr: *mut ffi::SwrContext, input: *const ffi::AVFrame) -> Result<()> {
        unsafe {
            let out = self.out.0;
            ffi::av_frame_unref(out);
            ffi::av_channel_layout_default(&mut (*out).ch_layout, self.out_channels as i32);
            (*out).format = ffi::AVSampleFormat::AV_SAMPLE_FMT_FLT as i32;
            (*out).sample_rate = self.rate as i32;
            let in_samples = if input.is_null() { 0 } else { (*input).nb_samples as i64 };
            (*out).nb_samples = (ffi::swr_get_delay(swr, self.rate as i64)
                + ffi::av_rescale_rnd(
                    in_samples,
                    self.rate as i64,
                    if input.is_null() { self.rate as i64 } else { (*input).sample_rate as i64 },
                    ffi::AVRounding::AV_ROUND_UP,
                )
                + 16) as i32;
            check(ffi::av_frame_get_buffer(out, 0), "audio buffer")?;
            let n = ffi::swr_convert(
                swr,
                (*out).data.as_mut_ptr(),
                (*out).nb_samples,
                if input.is_null() { std::ptr::null() } else { (*input).data.as_ptr() as *const *const u8 },
                in_samples as i32,
            );
            check(n, "resample")?;
            let samples = std::slice::from_raw_parts((*out).data[0] as *const f32, n as usize * self.out_channels);
            if self.out_channels == 1 {
                for &v in samples {
                    self.buf.push_back(v);
                    self.buf.push_back(v);
                }
            } else {
                self.buf.extend(samples.iter().copied());
            }
            if let Some(next) = &mut self.next {
                *next += n as i64;
            }
        }
        Ok(())
    }

    fn flush_resampler(&mut self) -> Result<()> {
        if let Some(swr) = self.swr.as_ref().map(|s| s.0) {
            if self.next.is_some() {
                self.convert(swr, std::ptr::null())?;
            }
        }
        Ok(())
    }
}

/// Decodes the whole audio track of `path` at `rate` (tests, short clips).
pub fn decode_all(path: &Path, rate: u32) -> Result<Vec<f32>> {
    let mut d = AudioDecoder::open(path, rate)?;
    let info = crate::probe_info(path)?;
    let frames = ticks_to_samples(info.duration, rate) as usize;
    d.read_samples(0, frames)
}

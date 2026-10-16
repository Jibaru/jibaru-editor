//! Media inspection.

use crate::util::*;
use anyhow::{Context as _, Result};
use ffmpeg_next as ff;
use ff::ffi;
use jb_core::frame::ColorSpace;
use jb_core::model::{Extra, MediaAsset, MediaKind};
use jb_core::time::{Rational, Ticks, TICKS_PER_SECOND};
use std::path::{Path, PathBuf};

/// File extensions decoded with the `image` crate instead of FFmpeg.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif", "tga", "qoi"];

/// Default duration given to still images when placed on a timeline.
pub const IMAGE_DEFAULT_DURATION: Ticks = 5 * TICKS_PER_SECOND;

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorInfo {
    /// YUV matrix / transfer family used by the renderer.
    pub space: ColorSpace,
    pub full_range: bool,
    /// FFmpeg names, e.g. "bt709", "smpte2084", "bt2020nc".
    pub primaries: String,
    pub transfer: String,
    pub matrix: String,
    pub bit_depth: u32,
    pub hdr: bool,
}

impl Default for ColorInfo {
    fn default() -> Self {
        Self {
            space: ColorSpace::Bt709,
            full_range: true,
            primaries: "bt709".into(),
            transfer: "iec61966-2-1".into(),
            matrix: "rgb".into(),
            bit_depth: 8,
            hdr: false,
        }
    }
}

/// Everything known about a media file.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeInfo {
    pub path: PathBuf,
    pub kind: MediaKind,
    pub duration: Ticks,
    /// Display size, after applying `rotation`.
    pub width: u32,
    pub height: u32,
    /// Size of the coded frames, before rotation.
    pub coded_width: u32,
    pub coded_height: u32,
    pub frame_rate: Option<Rational>,
    /// Clockwise rotation to apply for display: 0, 90, 180 or 270.
    pub rotation: u32,
    pub color: ColorInfo,
    pub video_codec: Option<String>,
    pub pixel_format: Option<String>,
    pub has_audio: bool,
    pub audio_codec: Option<String>,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_rate: u64,
    pub container: String,
}

impl ProbeInfo {
    /// Converts to the core model's asset description.
    pub fn to_asset(&self, id: impl Into<String>) -> MediaAsset {
        MediaAsset {
            id: id.into(),
            name: self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            path: self.path.to_string_lossy().into_owned(),
            kind: self.kind,
            duration: self.duration,
            width: self.width,
            height: self.height,
            frame_rate: self.frame_rate,
            has_audio: self.has_audio,
            extra: Extra::Null,
        }
    }
}

/// Probes `path` and returns a new [`MediaAsset`] with a fresh id.
pub fn probe(path: impl AsRef<Path>) -> Result<MediaAsset> {
    Ok(probe_info(path)?.to_asset(jb_core::model::new_id()))
}

/// Probes `path` and returns the full technical description.
pub fn probe_info(path: impl AsRef<Path>) -> Result<ProbeInfo> {
    let path = path.as_ref();
    if is_image_path(path) {
        return probe_image(path);
    }
    crate::init();
    let ictx = ff::format::input(&path).with_context(|| format!("cannot open {}", path.display()))?;
    probe_input(path, &ictx)
}

pub(crate) fn probe_input(path: &Path, ictx: &ff::format::context::Input) -> Result<ProbeInfo> {
    let fmt = unsafe { ictx.as_ptr() };
    let container = unsafe { cstr_name((*(*fmt).iformat).name) };
    let video = ictx.streams().best(ff::media::Type::Video).filter(|s| unsafe {
        // Cover art in audio files is not a video track.
        (*s.as_ptr()).disposition & ffi::AV_DISPOSITION_ATTACHED_PIC as i32 == 0
    });
    let audio = ictx.streams().best(ff::media::Type::Audio);
    let fmt_duration = unsafe { (*fmt).duration };
    let mut duration =
        if fmt_duration > 0 { fmt_duration * (TICKS_PER_SECOND / ffi::AV_TIME_BASE as i64) } else { 0 };

    let mut info = ProbeInfo {
        path: path.to_path_buf(),
        kind: MediaKind::Audio,
        duration: 0,
        width: 0,
        height: 0,
        coded_width: 0,
        coded_height: 0,
        frame_rate: None,
        rotation: 0,
        color: ColorInfo::default(),
        video_codec: None,
        pixel_format: None,
        has_audio: audio.is_some(),
        audio_codec: None,
        sample_rate: 0,
        channels: 0,
        bit_rate: unsafe { (*fmt).bit_rate.max(0) as u64 },
        container,
    };

    if let Some(st) = &video {
        let sp = unsafe { st.as_ptr() };
        let par = unsafe { &*(*sp).codecpar };
        info.kind = MediaKind::Video;
        info.coded_width = par.width as u32;
        info.coded_height = par.height as u32;
        info.rotation = stream_rotation(sp);
        let (w, h) = (par.width as u32, par.height as u32);
        (info.width, info.height) = if info.rotation % 180 == 90 { (h, w) } else { (w, h) };
        let afr = unsafe { (*sp).avg_frame_rate };
        let rfr = unsafe { (*sp).r_frame_rate };
        let fr = if afr.num > 0 && afr.den > 0 { afr } else { rfr };
        if fr.num > 0 && fr.den > 0 {
            info.frame_rate = Some(Rational::new(fr.num as i64, fr.den as i64));
        }
        let codec_id: ffi::AVCodecID = par.codec_id;
        info.video_codec = Some(unsafe { cstr_name(ffi::avcodec_get_name(codec_id)) });
        let pix: ffi::AVPixelFormat = unsafe { std::mem::transmute::<i32, ffi::AVPixelFormat>(par.format) };
        info.pixel_format = Some(pix_name(pix));
        let pi = pix_info(pix);
        let trc = par.color_trc;
        info.color = ColorInfo {
            space: map_color_space(par.color_space, trc, h),
            full_range: par.color_range == ffi::AVColorRange::AVCOL_RANGE_JPEG || is_full_range_format(pix),
            primaries: unsafe { cstr_name(ffi::av_color_primaries_name(par.color_primaries)) },
            transfer: unsafe { cstr_name(ffi::av_color_transfer_name(trc)) },
            matrix: unsafe { cstr_name(ffi::av_color_space_name(par.color_space)) },
            bit_depth: pi.depth,
            hdr: matches!(
                trc,
                ffi::AVColorTransferCharacteristic::AVCOL_TRC_SMPTE2084
                    | ffi::AVColorTransferCharacteristic::AVCOL_TRC_ARIB_STD_B67
            ),
        };
        if duration == 0 {
            let d = unsafe { (*sp).duration };
            if d > 0 {
                duration = ts_to_ticks(d, unsafe { (*sp).time_base });
            }
        }
        // A single-frame "video" (e.g. a lone image read by FFmpeg).
        let frames = unsafe { (*sp).nb_frames };
        if frames == 1 && !info.has_audio {
            info.kind = MediaKind::Image;
            duration = IMAGE_DEFAULT_DURATION;
        }
    }
    if let Some(st) = &audio {
        let par = unsafe { &*(*st.as_ptr()).codecpar };
        info.audio_codec = Some(unsafe { cstr_name(ffi::avcodec_get_name(par.codec_id)) });
        info.sample_rate = par.sample_rate as u32;
        info.channels = par.ch_layout.nb_channels as u32;
        if duration == 0 {
            let d = unsafe { (*st.as_ptr()).duration };
            if d > 0 {
                duration = ts_to_ticks(d, unsafe { (*st.as_ptr()).time_base });
            }
        }
    }
    if video.is_none() && audio.is_none() {
        anyhow::bail!("{} has no audio or video streams", path.display());
    }
    info.duration = duration;
    Ok(info)
}

fn probe_image(path: &Path) -> Result<ProbeInfo> {
    let (w, h, rotation) = crate::image::image_dimensions(path)?;
    Ok(ProbeInfo {
        path: path.to_path_buf(),
        kind: MediaKind::Image,
        duration: IMAGE_DEFAULT_DURATION,
        width: w,
        height: h,
        coded_width: w,
        coded_height: h,
        frame_rate: None,
        rotation,
        color: ColorInfo::default(),
        video_codec: None,
        pixel_format: Some("rgba".into()),
        has_audio: false,
        audio_codec: None,
        sample_rate: 0,
        channels: 0,
        bit_rate: 0,
        container: "image".into(),
    })
}

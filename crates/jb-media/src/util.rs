//! Small FFmpeg helpers shared by the modules of this crate.

use ffmpeg_next::ffi;
use jb_core::frame::ColorSpace;
use jb_core::time::{Ticks, TICKS_PER_SECOND};

pub(crate) const EAGAIN: i32 = ffi::AVERROR(libc::EAGAIN);

/// Turns a negative FFmpeg return code into an error.
pub(crate) fn check(ret: i32, what: &str) -> anyhow::Result<i32> {
    if ret < 0 {
        anyhow::bail!("{what}: {}", ffmpeg_next::Error::from(ret));
    }
    Ok(ret)
}

/// Converts a timestamp in `tb` units to ticks.
pub(crate) fn ts_to_ticks(ts: i64, tb: ffi::AVRational) -> Ticks {
    if tb.den == 0 {
        return 0;
    }
    let v = ts as i128 * tb.num as i128 * TICKS_PER_SECOND as i128;
    let d = tb.den as i128;
    // Round to nearest, symmetric around zero.
    let r = if v >= 0 { (v + d / 2) / d } else { (v - d / 2) / d };
    r as Ticks
}

/// Converts ticks to a timestamp in `tb` units (rounded to nearest).
pub(crate) fn ticks_to_ts(t: Ticks, tb: ffi::AVRational) -> i64 {
    if tb.num == 0 {
        return 0;
    }
    let v = t as i128 * tb.den as i128;
    let d = tb.num as i128 * TICKS_PER_SECOND as i128;
    let r = if v >= 0 { (v + d / 2) / d } else { (v - d / 2) / d };
    r as i64
}

/// Container start time in ticks (0 when unknown).
pub(crate) fn format_start_ticks(fmt: *const ffi::AVFormatContext) -> Ticks {
    let st = unsafe { (*fmt).start_time };
    if st == ffi::AV_NOPTS_VALUE {
        0
    } else {
        st * (TICKS_PER_SECOND / ffi::AV_TIME_BASE as i64)
    }
}

/// Clockwise display rotation (0, 90, 180 or 270) from a stream's display
/// matrix side data.
pub(crate) fn stream_rotation(st: *const ffi::AVStream) -> u32 {
    unsafe {
        let par = (*st).codecpar;
        let sd = ffi::av_packet_side_data_get(
            (*par).coded_side_data,
            (*par).nb_coded_side_data,
            ffi::AVPacketSideDataType::AV_PKT_DATA_DISPLAYMATRIX,
        );
        if sd.is_null() || (*sd).size < 36 {
            return 0;
        }
        let ccw = ffi::av_display_rotation_get((*sd).data as *const i32);
        if ccw.is_nan() {
            return 0;
        }
        let cw = (-ccw).round() as i64;
        let snapped = ((cw as f64 / 90.0).round() as i64 * 90).rem_euclid(360);
        snapped as u32
    }
}

pub(crate) fn map_color_space(
    space: ffi::AVColorSpace,
    trc: ffi::AVColorTransferCharacteristic,
    height: u32,
) -> ColorSpace {
    use ffi::AVColorSpace as S;
    use ffi::AVColorTransferCharacteristic as T;
    match space {
        S::AVCOL_SPC_BT2020_NCL | S::AVCOL_SPC_BT2020_CL => match trc {
            T::AVCOL_TRC_ARIB_STD_B67 => ColorSpace::Bt2020Hlg,
            _ => ColorSpace::Bt2020Pq,
        },
        S::AVCOL_SPC_BT470BG | S::AVCOL_SPC_SMPTE170M | S::AVCOL_SPC_FCC => ColorSpace::Bt601,
        S::AVCOL_SPC_BT709 => ColorSpace::Bt709,
        // Unspecified: the usual convention is BT.601 for SD, BT.709 above.
        _ => {
            if height >= 720 || height == 0 {
                ColorSpace::Bt709
            } else {
                ColorSpace::Bt601
            }
        }
    }
}

pub(crate) fn is_full_range_format(fmt: ffi::AVPixelFormat) -> bool {
    use ffi::AVPixelFormat as P;
    matches!(fmt, P::AV_PIX_FMT_YUVJ420P | P::AV_PIX_FMT_YUVJ422P | P::AV_PIX_FMT_YUVJ444P | P::AV_PIX_FMT_YUVJ440P)
}

/// Maps the deprecated `yuvj*` formats to their plain equivalents.
pub(crate) fn dejpeg(fmt: ffi::AVPixelFormat) -> ffi::AVPixelFormat {
    use ffi::AVPixelFormat as P;
    match fmt {
        P::AV_PIX_FMT_YUVJ420P => P::AV_PIX_FMT_YUV420P,
        P::AV_PIX_FMT_YUVJ422P => P::AV_PIX_FMT_YUV422P,
        P::AV_PIX_FMT_YUVJ444P => P::AV_PIX_FMT_YUV444P,
        P::AV_PIX_FMT_YUVJ440P => P::AV_PIX_FMT_YUV440P,
        f => f,
    }
}

pub(crate) struct PixInfo {
    pub depth: u32,
    pub alpha: bool,
    pub hw: bool,
}

pub(crate) fn pix_info(fmt: ffi::AVPixelFormat) -> PixInfo {
    unsafe {
        let d = ffi::av_pix_fmt_desc_get(fmt);
        if d.is_null() {
            return PixInfo { depth: 8, alpha: false, hw: false };
        }
        let flags = (*d).flags;
        PixInfo {
            depth: (*d).comp[0].depth as u32,
            alpha: flags & ffi::AV_PIX_FMT_FLAG_ALPHA as u64 != 0,
            hw: flags & ffi::AV_PIX_FMT_FLAG_HWACCEL as u64 != 0,
        }
    }
}

pub(crate) fn pix_name(fmt: ffi::AVPixelFormat) -> String {
    unsafe {
        let p = ffi::av_get_pix_fmt_name(fmt);
        if p.is_null() {
            "none".into()
        } else {
            std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

pub(crate) fn cstr_name(p: *const libc::c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
    }
}

/// Stable 64-bit FNV-1a hash, used for cache keys on disk.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Largest even size that fits `(w, h)` inside `max` preserving aspect
/// ratio. Never upscales.
pub(crate) fn fit_within(w: u32, h: u32, max: Option<(u32, u32)>) -> (u32, u32) {
    let (mut tw, mut th) = (w, h);
    if let Some((mw, mh)) = max {
        let (mw, mh) = (mw.max(2), mh.max(2));
        if w > mw || h > mh {
            let k = (mw as f64 / w as f64).min(mh as f64 / h as f64);
            tw = ((w as f64 * k).round() as u32).max(2);
            th = ((h as f64 * k).round() as u32).max(2);
        }
    }
    (tw & !1, th & !1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_conversions_round_trip() {
        let tb = ffi::AVRational { num: 1, den: 30000 };
        for ts in [0i64, 1001, 2002, 123_123, -1001] {
            assert_eq!(ticks_to_ts(ts_to_ticks(ts, tb), tb), ts);
        }
    }

    #[test]
    fn fit_keeps_aspect_and_even() {
        assert_eq!(fit_within(3840, 2160, Some((960, 540))), (960, 540));
        assert_eq!(fit_within(5312, 2988, Some((960, 960))), (960, 540));
        assert_eq!(fit_within(640, 480, Some((1920, 1080))), (640, 480));
        assert_eq!(fit_within(641, 481, None), (640, 480));
    }
}

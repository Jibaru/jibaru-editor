//! Pixel format conversion, scaling and rotation between FFmpeg frames and
//! [`VideoFrame`]s. Output frames always use a tight layout: the first plane
//! has `stride` bytes per row and, for NV12 / P010, the interleaved chroma
//! plane follows immediately with the same stride and half the rows.

use crate::util::*;
use anyhow::Result;
use ffmpeg_next::ffi;
use jb_core::frame::{ColorSpace, PixelFormat, VideoFrame};
use jb_core::time::Ticks;
use std::sync::Arc;

pub fn av_format(f: PixelFormat) -> ffi::AVPixelFormat {
    match f {
        PixelFormat::Rgba8 => ffi::AVPixelFormat::AV_PIX_FMT_RGBA,
        PixelFormat::Nv12 => ffi::AVPixelFormat::AV_PIX_FMT_NV12,
        PixelFormat::P010 => ffi::AVPixelFormat::AV_PIX_FMT_P010LE,
    }
}

/// Bytes per sample of the luma (or RGBA pixel) plane.
pub fn bytes_per_pixel(f: PixelFormat) -> usize {
    match f {
        PixelFormat::Rgba8 => 4,
        PixelFormat::Nv12 => 1,
        PixelFormat::P010 => 2,
    }
}

/// Total buffer size of a tightly packed frame.
pub fn frame_size(f: PixelFormat, w: u32, h: u32) -> usize {
    let stride = w as usize * bytes_per_pixel(f);
    match f {
        PixelFormat::Rgba8 => stride * h as usize,
        _ => stride * h as usize + stride * (h as usize).div_ceil(2),
    }
}

fn color_props(cs: ColorSpace) -> (ffi::AVColorSpace, ffi::AVColorPrimaries, ffi::AVColorTransferCharacteristic) {
    use ffi::AVColorPrimaries as P;
    use ffi::AVColorSpace as S;
    use ffi::AVColorTransferCharacteristic as T;
    match cs {
        ColorSpace::Bt709 => (S::AVCOL_SPC_BT709, P::AVCOL_PRI_BT709, T::AVCOL_TRC_BT709),
        ColorSpace::Bt601 => (S::AVCOL_SPC_SMPTE170M, P::AVCOL_PRI_SMPTE170M, T::AVCOL_TRC_SMPTE170M),
        ColorSpace::Bt2020Pq => (S::AVCOL_SPC_BT2020_NCL, P::AVCOL_PRI_BT2020, T::AVCOL_TRC_SMPTE2084),
        ColorSpace::Bt2020Hlg => (S::AVCOL_SPC_BT2020_NCL, P::AVCOL_PRI_BT2020, T::AVCOL_TRC_ARIB_STD_B67),
    }
}

/// A reusable swscale context. Re-initializes itself when frame properties
/// change between calls.
pub struct Scaler {
    ctx: *mut ffi::SwsContext,
}

// SAFETY: the context is only used through `&mut self`.
unsafe impl Send for Scaler {}

impl Scaler {
    /// `threads == 0` picks a count automatically.
    pub fn new(threads: i32, flags: ffi::SwsFlags) -> Self {
        let ctx = unsafe { ffi::sws_alloc_context() };
        assert!(!ctx.is_null(), "sws_alloc_context failed");
        unsafe {
            (*ctx).threads = threads;
            (*ctx).flags = flags as u32;
        }
        Self { ctx }
    }

    /// Bilinear scaler with automatic threading, good for previews and export.
    pub fn bilinear() -> Self {
        Self::new(0, ffi::SwsFlags::SWS_BILINEAR)
    }

    /// Area-averaging scaler with automatic threading, for decoded video:
    /// a downscale averages the source pixels each output pixel covers (a 2×2
    /// box at half size), as an editor's full-resolution render downscaled
    /// for viewing would. Bilinear's wider tent filter blurred test-card
    /// edges and checkers measurably against Filmora's 1080p exports viewed
    /// at 960×540 (docs/fidelity.md, "Synthetic test media"). Upscales fall
    /// back to bilinear inside swscale.
    pub fn area() -> Self {
        Self::new(0, ffi::SwsFlags::SWS_AREA)
    }

    /// Converts `src` into `dst`. `dst` must have width, height, format and
    /// color properties set; its buffers are allocated if missing.
    ///
    /// # Safety
    /// Both pointers must be valid frames.
    pub unsafe fn run(&mut self, dst: *mut ffi::AVFrame, src: *const ffi::AVFrame) -> Result<()> {
        check(ffi::sws_scale_frame(self.ctx, dst, src), "sws_scale_frame")?;
        Ok(())
    }
}

impl Drop for Scaler {
    fn drop(&mut self) {
        unsafe { ffi::sws_free_context(&mut self.ctx) };
    }
}

unsafe extern "C" fn drop_arc_vec(opaque: *mut libc::c_void, _data: *mut u8) {
    drop(Box::from_raw(opaque as *mut Arc<Vec<u8>>));
}

/// An owned `AVFrame` pointer.
pub struct AvFrame(pub *mut ffi::AVFrame);

unsafe impl Send for AvFrame {}

impl AvFrame {
    pub fn alloc() -> Self {
        let f = unsafe { ffi::av_frame_alloc() };
        assert!(!f.is_null(), "av_frame_alloc failed");
        Self(f)
    }
}

impl Drop for AvFrame {
    fn drop(&mut self) {
        unsafe { ffi::av_frame_free(&mut self.0) };
    }
}

/// Wraps a [`VideoFrame`] in an `AVFrame` without copying; the frame keeps
/// the pixel data alive through a reference to its `Arc`.
pub fn wrap_video_frame(vf: &VideoFrame) -> AvFrame {
    let f = AvFrame::alloc();
    unsafe {
        let opaque = Box::into_raw(Box::new(vf.data.clone())) as *mut libc::c_void;
        let buf = ffi::av_buffer_create(
            vf.data.as_ptr() as *mut u8,
            vf.data.len(),
            Some(drop_arc_vec),
            opaque,
            ffi::AV_BUFFER_FLAG_READONLY as i32,
        );
        assert!(!buf.is_null(), "av_buffer_create failed");
        (*f.0).buf[0] = buf;
        (*f.0).width = vf.width as i32;
        (*f.0).height = vf.height as i32;
        (*f.0).format = av_format(vf.format) as i32;
        let base = vf.data.as_ptr() as *mut u8;
        (*f.0).data[0] = base;
        (*f.0).linesize[0] = vf.stride as i32;
        if vf.format != PixelFormat::Rgba8 {
            (*f.0).data[1] = base.add(vf.stride * vf.height as usize);
            (*f.0).linesize[1] = vf.stride as i32;
        }
        apply_color(f.0, vf.format, vf.color_space, vf.full_range);
    }
    f
}

unsafe fn apply_color(f: *mut ffi::AVFrame, fmt: PixelFormat, cs: ColorSpace, full_range: bool) {
    let (spc, pri, trc) = color_props(cs);
    (*f).color_primaries = pri;
    (*f).color_trc = trc;
    if fmt == PixelFormat::Rgba8 {
        (*f).colorspace = ffi::AVColorSpace::AVCOL_SPC_RGB;
        (*f).color_range = ffi::AVColorRange::AVCOL_RANGE_JPEG;
    } else {
        (*f).colorspace = spc;
        (*f).color_range =
            if full_range { ffi::AVColorRange::AVCOL_RANGE_JPEG } else { ffi::AVColorRange::AVCOL_RANGE_MPEG };
    }
}

/// Runs `scaler` from `src` into a newly allocated tight buffer.
unsafe fn scale_into_vec(
    scaler: &mut Scaler,
    src: *const ffi::AVFrame,
    fmt: PixelFormat,
    w: u32,
    h: u32,
    cs: ColorSpace,
    full_range: bool,
) -> Result<Vec<u8>> {
    // swscale allocates the destination itself: its SIMD kernels write whole
    // blocks past the end of each row, so they need FFmpeg's aligned,
    // padded buffers. Scaling straight into a tight Vec overran it and
    // corrupted the heap (crashes at a later free, often at process exit).
    let dst = AvFrame::alloc();
    (*dst.0).width = w as i32;
    (*dst.0).height = h as i32;
    (*dst.0).format = av_format(fmt) as i32;
    apply_color(dst.0, fmt, cs, full_range);
    // Keep the source transfer/primaries so swscale never tries gamut mapping.
    if fmt != PixelFormat::Rgba8 || (*src).colorspace != ffi::AVColorSpace::AVCOL_SPC_RGB {
        (*dst.0).color_primaries = (*src).color_primaries;
        (*dst.0).color_trc = (*src).color_trc;
    }
    scaler.run(dst.0, src)?;
    if (*dst.0).data[0].is_null() {
        anyhow::bail!("sws_scale_frame produced no buffer");
    }
    Ok(copy_tight(dst.0, fmt, w, h))
}

/// Converts any [`VideoFrame`] to another size / format / color encoding.
pub fn convert_frame(
    scaler: &mut Scaler,
    src: &VideoFrame,
    width: u32,
    height: u32,
    format: PixelFormat,
    color_space: ColorSpace,
    full_range: bool,
) -> Result<VideoFrame> {
    if src.width == width
        && src.height == height
        && src.format == format
        && src.stride == width as usize * bytes_per_pixel(format)
        && (format == PixelFormat::Rgba8 || (src.color_space == color_space && src.full_range == full_range))
    {
        return Ok(src.clone());
    }
    let wrapped = wrap_video_frame(src);
    unsafe {
        // RGB input carries no matrix; give it the destination's primaries.
        if src.format == PixelFormat::Rgba8 {
            let (_, pri, trc) = color_props(color_space);
            (*wrapped.0).color_primaries = pri;
            (*wrapped.0).color_trc = trc;
        }
    }
    let data = unsafe { scale_into_vec(scaler, wrapped.0, format, width, height, color_space, full_range)? };
    Ok(VideoFrame {
        width,
        height,
        format,
        color_space,
        full_range,
        stride: width as usize * bytes_per_pixel(format),
        data: Arc::new(data),
        pts: src.pts,
    })
}

/// Output format chosen for a decoded software pixel format.
pub(crate) fn output_format(src: ffi::AVPixelFormat) -> PixelFormat {
    let pi = pix_info(src);
    if pi.alpha {
        PixelFormat::Rgba8
    } else if pi.depth > 8 {
        PixelFormat::P010
    } else {
        PixelFormat::Nv12
    }
}

/// Converts a decoded software frame into a [`VideoFrame`], scaling it to
/// fit `max_size` (applied to display dimensions) and rotating it upright.
///
/// # Safety
/// `src` must be a valid software frame; its format field may be rewritten
/// from a deprecated `yuvj*` value to the equivalent plain format.
pub(crate) unsafe fn decoded_to_video_frame(
    scaler: &mut Scaler,
    src: *mut ffi::AVFrame,
    rotation: u32,
    max_size: Option<(u32, u32)>,
    pts: Ticks,
) -> Result<VideoFrame> {
    let mut sfmt: ffi::AVPixelFormat = std::mem::transmute::<i32, ffi::AVPixelFormat>((*src).format);
    if is_full_range_format(sfmt) {
        sfmt = dejpeg(sfmt);
        (*src).format = sfmt as i32;
        (*src).color_range = ffi::AVColorRange::AVCOL_RANGE_JPEG;
    }
    let fmt = output_format(sfmt);
    let (sw, sh) = ((*src).width as u32, (*src).height as u32);
    let swapped = rotation % 180 == 90;
    let (dw, dh) = if swapped { (sh, sw) } else { (sw, sh) };
    let (tw, th) = fit_within(dw, dh, max_size);
    let (cw, ch) = if swapped { (th, tw) } else { (tw, th) };
    let full_range = (*src).color_range == ffi::AVColorRange::AVCOL_RANGE_JPEG;
    let cs = map_color_space((*src).colorspace, (*src).color_trc, sh);

    let data = if av_format(fmt) == sfmt && (cw, ch) == (sw, sh) {
        copy_tight(src, fmt, cw, ch)
    } else {
        scale_into_vec(scaler, src, fmt, cw, ch, cs, full_range || fmt == PixelFormat::Rgba8)?
    };
    let (data, w, h) = if rotation != 0 { (rotate(&data, fmt, cw, ch, rotation), tw, th) } else { (data, cw, ch) };
    Ok(VideoFrame {
        width: w,
        height: h,
        format: fmt,
        color_space: cs,
        full_range: full_range || fmt == PixelFormat::Rgba8,
        stride: w as usize * bytes_per_pixel(fmt),
        data: Arc::new(data),
        pts,
    })
}

unsafe fn copy_tight(src: *const ffi::AVFrame, fmt: PixelFormat, w: u32, h: u32) -> Vec<u8> {
    let stride = w as usize * bytes_per_pixel(fmt);
    let mut out = vec![0u8; frame_size(fmt, w, h)];
    let planes: &[(usize, usize)] =
        if fmt == PixelFormat::Rgba8 { &[(0, h as usize)] } else { &[(0, h as usize), (1, (h as usize).div_ceil(2))] };
    let mut off = 0;
    for &(p, rows) in planes {
        let ls = (*src).linesize[p] as isize;
        let base = (*src).data[p];
        for r in 0..rows {
            let row = std::slice::from_raw_parts(base.offset(ls * r as isize), stride);
            out[off..off + stride].copy_from_slice(row);
            off += stride;
        }
    }
    out
}

/// Rotates a tightly packed frame clockwise by 90, 180 or 270 degrees.
pub fn rotate(data: &[u8], fmt: PixelFormat, w: u32, h: u32, rotation: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; data.len()];
    match fmt {
        PixelFormat::Rgba8 => rotate_plane(data, &mut out, w, h, 4, rotation),
        PixelFormat::Nv12 | PixelFormat::P010 => {
            let b = bytes_per_pixel(fmt);
            let ysize = w * h * b;
            rotate_plane(&data[..ysize], &mut out[..ysize], w, h, b, rotation);
            rotate_plane(&data[ysize..], &mut out[ysize..], w / 2, h / 2, 2 * b, rotation);
        }
    }
    out
}

fn rotate_plane(src: &[u8], dst: &mut [u8], w: usize, h: usize, e: usize, rotation: u32) {
    for y in 0..h {
        for x in 0..w {
            let (nx, ny, nw) = match rotation {
                90 => (h - 1 - y, x, h),
                180 => (w - 1 - x, h - 1 - y, w),
                270 => (y, w - 1 - x, h),
                _ => (x, y, w),
            };
            let s = (y * w + x) * e;
            let d = (ny * nw + nx) * e;
            dst[d..d + e].copy_from_slice(&src[s..s + e]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_rgba_90() {
        // 2x1 image: [A, B] -> rotated clockwise becomes 1x2: [A; B].
        let a = [1u8, 2, 3, 4];
        let b = [5u8, 6, 7, 8];
        let src: Vec<u8> = a.iter().chain(b.iter()).copied().collect();
        let out = rotate(&src, PixelFormat::Rgba8, 2, 1, 90);
        assert_eq!(&out[0..4], &a);
        assert_eq!(&out[4..8], &b);
        let back = rotate(&rotate(&out, PixelFormat::Rgba8, 1, 2, 90), PixelFormat::Rgba8, 2, 1, 180);
        assert_eq!(back, src);
    }

    #[test]
    fn convert_rgba_to_nv12_and_back() {
        let (w, h) = (64u32, 32u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for p in px.chunks_mut(4) {
            p.copy_from_slice(&[200, 100, 50, 255]);
        }
        let src = VideoFrame {
            width: w,
            height: h,
            format: PixelFormat::Rgba8,
            color_space: ColorSpace::Bt709,
            full_range: true,
            stride: (w * 4) as usize,
            data: Arc::new(px),
            pts: 0,
        };
        crate::init();
        let mut s = Scaler::bilinear();
        let nv = convert_frame(&mut s, &src, 32, 16, PixelFormat::Nv12, ColorSpace::Bt709, false).unwrap();
        assert_eq!(nv.data.len(), frame_size(PixelFormat::Nv12, 32, 16));
        let back = convert_frame(&mut s, &nv, 32, 16, PixelFormat::Rgba8, ColorSpace::Bt709, true).unwrap();
        let p = &back.data[(8 * 32 + 8) * 4..(8 * 32 + 8) * 4 + 3];
        assert!(
            (p[0] as i32 - 200).abs() <= 3 && (p[1] as i32 - 100).abs() <= 3 && (p[2] as i32 - 50).abs() <= 3,
            "{p:?}"
        );
    }

    /// Regression: swscale's SIMD kernels write past the end of rows, which
    /// overran tight output buffers and corrupted the heap (intermittent
    /// SIGABRT / SIGTRAP at exit on macOS CI). Sizes that are not multiples
    /// of the kernel block width hit it; CI also runs this test under Guard
    /// Malloc, which faults on the first byte written out of bounds.
    #[test]
    fn scaling_never_writes_outside_tight_frames() {
        crate::init();
        let mut s = Scaler::bilinear();
        for (w, h) in [(2u32, 2u32), (6, 4), (18, 10), (32, 16), (34, 18), (62, 36), (66, 2), (130, 6)] {
            let px: Vec<u8> = (0..w * h * 4).map(|i| (i * 7 % 251) as u8).collect();
            let src = VideoFrame {
                width: w,
                height: h,
                format: PixelFormat::Rgba8,
                color_space: ColorSpace::Bt709,
                full_range: true,
                stride: (w * 4) as usize,
                data: Arc::new(px),
                pts: 0,
            };
            for (fmt, tw, th) in
                [(PixelFormat::Nv12, w, h), (PixelFormat::P010, w + 2, h), (PixelFormat::Rgba8, w + 2, h + 2)]
            {
                let out = convert_frame(&mut s, &src, tw, th, fmt, ColorSpace::Bt709, false).unwrap();
                assert_eq!(out.data.len(), frame_size(fmt, tw, th), "{w}x{h} -> {fmt:?} {tw}x{th}");
                let back = convert_frame(&mut s, &out, w, h, PixelFormat::Rgba8, ColorSpace::Bt709, true).unwrap();
                assert_eq!(back.data.len(), frame_size(PixelFormat::Rgba8, w, h));
            }
        }
    }
}

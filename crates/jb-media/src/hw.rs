//! Hardware decoding setup: VideoToolbox on macOS, D3D11VA / DXVA2 on
//! Windows, VAAPI / CUDA on Linux. Devices are created once per process and
//! shared by all decoders, which keeps GPU memory use low.

use ffmpeg_next::ffi;
use std::collections::HashMap;
use std::sync::Mutex;

/// Device types to try, in order of preference, for this platform.
pub fn preferred_device_types() -> &'static [ffi::AVHWDeviceType] {
    use ffi::AVHWDeviceType as T;
    #[cfg(target_os = "macos")]
    {
        &[T::AV_HWDEVICE_TYPE_VIDEOTOOLBOX]
    }
    #[cfg(target_os = "windows")]
    {
        &[T::AV_HWDEVICE_TYPE_D3D11VA, T::AV_HWDEVICE_TYPE_DXVA2]
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        &[T::AV_HWDEVICE_TYPE_VAAPI, T::AV_HWDEVICE_TYPE_CUDA]
    }
}

pub fn device_type_name(t: ffi::AVHWDeviceType) -> String {
    crate::util::cstr_name(unsafe { ffi::av_hwdevice_get_type_name(t) })
}

struct DeviceRef(*mut ffi::AVBufferRef);
// SAFETY: FFmpeg hardware device contexts are internally synchronized and
// designed to be shared between decoders on different threads.
unsafe impl Send for DeviceRef {}

static DEVICES: Mutex<Option<HashMap<i32, DeviceRef>>> = Mutex::new(None);

/// Returns a new reference to the shared device of type `t`, creating it on
/// first use. `None` when the device cannot be created on this machine.
pub(crate) fn device(t: ffi::AVHWDeviceType) -> Option<*mut ffi::AVBufferRef> {
    let mut guard = DEVICES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let key = t as i32;
    if !map.contains_key(&key) {
        let mut ctx: *mut ffi::AVBufferRef = std::ptr::null_mut();
        let ret = unsafe { ffi::av_hwdevice_ctx_create(&mut ctx, t, std::ptr::null(), std::ptr::null_mut(), 0) };
        // Remember failures too (null) so we do not retry on every open.
        map.insert(key, DeviceRef(if ret < 0 { std::ptr::null_mut() } else { ctx }));
    }
    let d = map.get(&key)?.0;
    if d.is_null() {
        None
    } else {
        Some(unsafe { ffi::av_buffer_ref(d) })
    }
}

/// Codecs worth decoding in hardware. Others (MPEG-4 Part 2, ProRes on
/// most GPUs, intra codecs) are fast in software and often fail hwaccel setup.
fn worth_hw(id: ffi::AVCodecID) -> bool {
    use ffi::AVCodecID as C;
    matches!(id, C::AV_CODEC_ID_H264 | C::AV_CODEC_ID_HEVC | C::AV_CODEC_ID_VP9 | C::AV_CODEC_ID_AV1)
}

/// Finds a hardware configuration of `codec` usable on this platform.
/// Returns the device type and the hardware pixel format.
pub(crate) fn find_config(codec: *const ffi::AVCodec) -> Option<(ffi::AVHWDeviceType, ffi::AVPixelFormat)> {
    if !worth_hw(unsafe { (*codec).id }) {
        return None;
    }
    for &want in preferred_device_types() {
        let mut i = 0;
        loop {
            let cfg = unsafe { ffi::avcodec_get_hw_config(codec, i) };
            if cfg.is_null() {
                break;
            }
            let cfg = unsafe { &*cfg };
            if cfg.methods & ffi::AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX as i32 != 0 && cfg.device_type == want {
                return Some((want, cfg.pix_fmt));
            }
            i += 1;
        }
    }
    None
}

/// `get_format` callback: picks the hardware format stored in `opaque` when
/// offered, otherwise falls back to the first software format.
pub(crate) unsafe extern "C" fn get_format(
    ctx: *mut ffi::AVCodecContext,
    fmts: *const ffi::AVPixelFormat,
) -> ffi::AVPixelFormat {
    let want = (*ctx).opaque as isize as i32;
    let mut p = fmts;
    let mut first_sw = ffi::AVPixelFormat::AV_PIX_FMT_NONE;
    while *p != ffi::AVPixelFormat::AV_PIX_FMT_NONE {
        if *p as i32 == want {
            return *p;
        }
        if first_sw == ffi::AVPixelFormat::AV_PIX_FMT_NONE && !crate::util::pix_info(*p).hw {
            first_sw = *p;
        }
        p = p.add(1);
    }
    first_sw
}

/// Hardware decode device types that can be created on this machine.
pub fn available_devices() -> Vec<String> {
    preferred_device_types()
        .iter()
        .filter(|t| device(**t).map(|d| unsafe { unref(d) }).is_some())
        .map(|t| device_type_name(*t))
        .collect()
}

unsafe fn unref(mut d: *mut ffi::AVBufferRef) {
    ffi::av_buffer_unref(&mut d);
}

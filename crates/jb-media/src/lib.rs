//! Media probing and hardware-accelerated decoding on top of FFmpeg.
//!
//! - [`probe`] / [`probe_info`] inspect a file.
//! - [`MediaPool`] implements [`jb_core::FrameSource`] with cached decoders,
//!   hardware decoding, accurate seeking and audio access.
//! - [`MediaCache`] serves thumbnails, waveform peaks and proxies from an
//!   on-disk cache for the UI.
//! - [`encode`] picks a hardware video encoder with LGPL-safe fallbacks; it
//!   is shared by proxy generation and `jb-export`.
//!
//! FFmpeg is linked dynamically and only LGPL components are used (no
//! libx264/libx265).

pub mod audio;
pub mod cache;
pub mod convert;
pub mod encode;
pub mod hw;
pub mod image;
pub mod mixer;
pub mod pool;
pub mod probe;
pub mod proxy;
pub mod video;
pub mod waveform;

mod util;

pub use cache::{MediaCache, Thumbnail};
pub use encode::{EncoderConfig, VideoCodec, VideoEncoder};
pub use pool::{MediaPool, PoolConfig};
pub use probe::{probe, probe_info, ColorInfo, ProbeInfo};
pub use proxy::{generate_proxy, ProxyOptions, ProxyReport};
pub use video::{DecodeMode, VideoDecoder};
pub use waveform::Waveform;

/// Re-export of the FFmpeg binding so dependants use the exact same version.
pub use ffmpeg_next as ff;

use std::sync::Once;

/// Initializes FFmpeg once. Every public entry point calls this, so callers
/// normally never need to.
pub fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = ffmpeg_next::init();
        let level = match std::env::var("JB_FFMPEG_LOG").as_deref() {
            Ok("debug") => ffmpeg_next::util::log::Level::Debug,
            Ok("verbose") => ffmpeg_next::util::log::Level::Verbose,
            Ok("info") => ffmpeg_next::util::log::Level::Info,
            Ok("warning") => ffmpeg_next::util::log::Level::Warning,
            _ => ffmpeg_next::util::log::Level::Error,
        };
        ffmpeg_next::util::log::set_level(level);
    });
}

/// Version string of the FFmpeg libraries in use (e.g. "libavcodec 62.1.102").
pub fn ffmpeg_version() -> String {
    let v = ffmpeg_next::codec::version();
    format!("libavcodec {}.{}.{}", v >> 16, (v >> 8) & 0xff, v & 0xff)
}

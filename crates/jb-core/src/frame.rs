//! Decoded frame types and the trait that connects decoders to the renderer.
//! Living in the core lets `jb-media` and `jb-render` stay independent.

use crate::time::Ticks;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8-bit RGBA, straight alpha, sRGB transfer.
    Rgba8,
    /// 8-bit 4:2:0, full Y plane followed by interleaved UV plane.
    Nv12,
    /// 10-bit 4:2:0 in 16-bit words (HDR / 10-bit camera footage).
    P010,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Bt709,
    Bt601,
    Bt2020Pq,
    Bt2020Hlg,
}

/// A decoded video frame in CPU memory.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub color_space: ColorSpace,
    /// Full range (0-255) vs limited (16-235) for YUV formats.
    pub full_range: bool,
    /// Bytes per row of the first plane.
    pub stride: usize,
    pub data: Arc<Vec<u8>>,
    /// Presentation time of this frame in source time.
    pub pts: Ticks,
}

/// Supplies decoded frames for media assets. Implementations are expected to
/// cache decoders and serve sequential access efficiently.
pub trait FrameSource: Send + Sync {
    /// Frame of `media_id` displayed at `source_time`. `max_size` lets the
    /// source decode or scale at a reduced resolution for previews.
    fn frame(
        &self,
        media_id: &str,
        source_time: Ticks,
        max_size: Option<(u32, u32)>,
    ) -> anyhow::Result<Option<VideoFrame>>;
}

//! A minimal CPU compositor: draws the top-most media (or color) layer
//! scaled to fit the canvas. It exists so `export` can be tested end to end
//! without a GPU; real exports use the `jb-render` compositor.

use anyhow::Result;
use jb_core::eval::{FramePlan, Layer, LayerContent};
use jb_core::frame::{ColorSpace, FrameSource, PixelFormat, VideoFrame};
use jb_core::model::FitMode;
use jb_media::convert::{convert_frame, frame_size, Scaler};
use std::sync::Arc;

pub struct CpuCompositor<'a> {
    source: &'a dyn FrameSource,
    width: u32,
    height: u32,
    background: [f64; 4],
    max_size: Option<(u32, u32)>,
    scaler: Scaler,
}

enum Top<'p> {
    Media(&'p Layer, &'p str, i64),
    Color([f64; 4]),
}

fn top_layer(layers: &[Layer]) -> Option<Top<'_>> {
    for l in layers.iter().rev() {
        match &l.content {
            LayerContent::Media { media_id, source_time } => return Some(Top::Media(l, media_id, *source_time)),
            LayerContent::Color(c) => return Some(Top::Color(*c)),
            LayerContent::Nested(p) => {
                if let Some(t) = top_layer(&p.layers) {
                    return Some(t);
                }
            }
            _ => {}
        }
    }
    None
}

/// BT.709 limited-range YUV of a linear RGB color.
fn yuv709(c: [f64; 4]) -> [u8; 3] {
    let enc = |v: f64| v.clamp(0.0, 1.0).powf(1.0 / 2.2);
    let (r, g, b) = (enc(c[0]), enc(c[1]), enc(c[2]));
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let u = (b - y) / 1.8556;
    let v = (r - y) / 1.5748;
    [(16.0 + 219.0 * y).round() as u8, (128.0 + 224.0 * u).round() as u8, (128.0 + 224.0 * v).round() as u8]
}

impl<'a> CpuCompositor<'a> {
    pub fn new(source: &'a dyn FrameSource, width: u32, height: u32) -> Self {
        Self {
            source,
            width: width & !1,
            height: height & !1,
            background: [0.0, 0.0, 0.0, 1.0],
            max_size: None,
            scaler: Scaler::bilinear(),
        }
    }

    pub fn with_background(mut self, c: [f64; 4]) -> Self {
        self.background = c;
        self
    }

    /// `max_size` passed to the frame source (match the export's prefetch).
    pub fn with_max_size(mut self, max: Option<(u32, u32)>) -> Self {
        self.max_size = max;
        self
    }

    fn canvas(&self, color: [f64; 4]) -> Vec<u8> {
        let (w, h) = (self.width as usize, self.height as usize);
        let [y, u, v] = yuv709(color);
        let mut data = vec![y; frame_size(PixelFormat::Nv12, self.width, self.height)];
        for px in data[w * h..].as_chunks_mut::<2>().0 {
            px[0] = u;
            px[1] = v;
        }
        data
    }

    fn frame(&self, data: Vec<u8>, pts: i64) -> VideoFrame {
        VideoFrame {
            width: self.width,
            height: self.height,
            format: PixelFormat::Nv12,
            color_space: ColorSpace::Bt709,
            full_range: false,
            stride: self.width as usize,
            data: Arc::new(data),
            pts,
        }
    }

    /// Renders `plan` into a BT.709 limited-range NV12 frame.
    pub fn render(&mut self, plan: &FramePlan) -> Result<VideoFrame> {
        let Some(top) = top_layer(&plan.layers) else {
            return Ok(self.frame(self.canvas(self.background), plan.time));
        };
        let (layer, media_id, st) = match top {
            Top::Color(c) => return Ok(self.frame(self.canvas(c), plan.time)),
            Top::Media(l, m, t) => (l, m, t),
        };
        let mut data = self.canvas(self.background);
        let Some(src) = self.source.frame(media_id, st, self.max_size)? else {
            return Ok(self.frame(data, plan.time));
        };
        let (cw, ch) = (self.width as f64, self.height as f64);
        let (sw, sh) = (src.width as f64, src.height as f64);
        let (fw, fh) = match layer.transform.fit {
            FitMode::Stretch => (cw, ch),
            FitMode::None => (sw.min(cw), sh.min(ch)),
            // Fill is approximated by Fit in this reference compositor.
            _ => {
                let k = (cw / sw).min(ch / sh);
                (sw * k, sh * k)
            }
        };
        let (fw, fh) = (((fw.round() as u32).max(2)) & !1, ((fh.round() as u32).max(2)) & !1);
        let scaled = convert_frame(&mut self.scaler, &src, fw, fh, PixelFormat::Nv12, ColorSpace::Bt709, false)?;
        let (x0, y0) = (((self.width - fw) / 2) & !1, ((self.height - fh) / 2) & !1);
        let (w, h) = (self.width as usize, self.height as usize);
        let (fwu, fhu) = (fw as usize, fh as usize);
        for row in 0..fhu {
            let d = (y0 as usize + row) * w + x0 as usize;
            data[d..d + fwu].copy_from_slice(&scaled.data[row * scaled.stride..row * scaled.stride + fwu]);
        }
        let uv_src = scaled.stride * fhu;
        for row in 0..fhu / 2 {
            let d = w * h + (y0 as usize / 2 + row) * w + x0 as usize;
            let s = uv_src + row * scaled.stride;
            data[d..d + fwu].copy_from_slice(&scaled.data[s..s + fwu]);
        }
        Ok(self.frame(data, plan.time))
    }
}

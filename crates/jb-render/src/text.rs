//! Title rasterization. Text is shaped with cosmic-text (system fonts,
//! fallback, bidi), rasterized to a coverage mask on the CPU and converted
//! to a signed distance field. The GPU title pass draws fill, stroke, shadow
//! and background from that field, so all four stay crisp at any size and the
//! CPU work is cached while the title does not change.
//!
//! Geometric title values (`font_size`, stroke width, shadow offset/blur) are
//! fractions of the canvas height.

use cosmic_text::{Align, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, SwashCache, Weight};
use jb_core::TitleData;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Distance field of a laid-out title plus where it sits on the canvas.
pub struct TitleRaster {
    /// Top-left of the field in canvas pixels.
    pub origin: [f32; 2],
    pub width: u32,
    pub height: u32,
    /// Signed distance in pixels (negative inside glyphs), as f16 bits.
    pub sdf: Vec<u16>,
    /// Text bounding box in canvas pixels (x0, y0, x1, y1).
    pub text_box: [f32; 4],
}

pub struct TextEngine {
    fonts: FontSystem,
    swash: SwashCache,
    cache: HashMap<u64, std::sync::Arc<TitleRaster>>,
}

impl TextEngine {
    pub fn new() -> Self {
        Self { fonts: FontSystem::new(), swash: SwashCache::new(), cache: HashMap::new() }
    }

    pub fn raster(&mut self, t: &TitleData, canvas: [u32; 2]) -> std::sync::Arc<TitleRaster> {
        let key = title_key(t, canvas);
        if let Some(r) = self.cache.get(&key) {
            return r.clone();
        }
        if self.cache.len() > 256 {
            self.cache.clear();
        }
        let r = std::sync::Arc::new(self.rasterize(t, canvas));
        self.cache.insert(key, r.clone());
        r
    }

    fn rasterize(&mut self, t: &TitleData, canvas: [u32; 2]) -> TitleRaster {
        let ch = canvas[1] as f32;
        let font_px = (t.font_size as f32 * ch).clamp(1.0, 2000.0);
        let line_h = font_px * 1.2;
        let mut attrs = Attrs::new();
        if !t.font_family.is_empty() {
            attrs = attrs.family(Family::Name(&t.font_family));
        }
        if t.bold {
            attrs = attrs.weight(Weight::BOLD);
        }
        if t.italic {
            attrs = attrs.style(Style::Italic);
        }
        let mut buf = Buffer::new(&mut self.fonts, Metrics::new(font_px, line_h));
        buf.set_size(None, None);
        buf.set_text(&t.text, &attrs, Shaping::Advanced, Some(Align::Center));
        buf.shape_until_scroll(&mut self.fonts, false);
        // Measure so we can centre lines ourselves (no wrapping width).
        let mut text_w = 0.0f32;
        let mut text_h = 0.0f32;
        for run in buf.layout_runs() {
            text_w = text_w.max(run.line_w);
            text_h = text_h.max(run.line_top + run.line_height);
        }
        let stroke = t.stroke.as_ref().map_or(0.0, |s| s.width as f32 * ch);
        let (sh_off, sh_blur) = t
            .shadow
            .as_ref()
            .map_or(([0.0, 0.0], 0.0), |s| ([s.offset[0] as f32 * ch, s.offset[1] as f32 * ch], s.blur as f32 * ch));
        let bg_pad = if t.background.is_some() { font_px * 0.3 } else { 0.0 };
        let spread = (stroke + sh_blur * 2.0 + sh_off[0].abs().max(sh_off[1].abs()) + bg_pad + 4.0).ceil();
        let pad = spread as i32;
        let w = (text_w.ceil() as i32 + 2 * pad).max(1) as u32;
        let h = (text_h.ceil() as i32 + 2 * pad).max(1) as u32;
        let (w, h) = (w.min(8192), h.min(8192));
        let mut cover = vec![0f32; (w * h) as usize];
        for run in buf.layout_runs() {
            let dx = (text_w - run.line_w) * 0.5;
            for g in run.glyphs {
                let pg = g.physical((dx + pad as f32, run.line_y + pad as f32), 1.0);
                let Some(img) = self.swash.get_image(&mut self.fonts, pg.cache_key).as_ref() else { continue };
                let p = img.placement;
                let bpp = match img.content {
                    cosmic_text::SwashContent::Mask => 1,
                    _ => 4,
                };
                for yy in 0..p.height as i32 {
                    let y = pg.y - p.top + yy;
                    if y < 0 || y >= h as i32 {
                        continue;
                    }
                    for xx in 0..p.width as i32 {
                        let x = pg.x + p.left + xx;
                        if x < 0 || x >= w as i32 {
                            continue;
                        }
                        let i = (yy * p.width as i32 + xx) as usize * bpp;
                        let a = if bpp == 1 { img.data[i] } else { img.data[i + 3] } as f32 / 255.0;
                        let c = &mut cover[(y as u32 * w + x as u32) as usize];
                        *c = (*c + a).min(1.0);
                    }
                }
            }
        }
        let sdf = coverage_to_sdf(&cover, w as usize, h as usize);
        let cx = t.position[0] as f32 * canvas[0] as f32;
        let cy = t.position[1] as f32 * canvas[1] as f32;
        let origin = [(cx - w as f32 * 0.5).round(), (cy - h as f32 * 0.5).round()];
        let text_box = [
            origin[0] + pad as f32,
            origin[1] + pad as f32,
            origin[0] + pad as f32 + text_w,
            origin[1] + pad as f32 + text_h,
        ];
        TitleRaster { origin, width: w, height: h, sdf: sdf.iter().map(|d| f32_to_f16(*d)).collect(), text_box }
    }
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn title_key(t: &TitleData, canvas: [u32; 2]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    t.text.hash(&mut h);
    t.font_family.hash(&mut h);
    t.font_size.to_bits().hash(&mut h);
    t.bold.hash(&mut h);
    t.italic.hash(&mut h);
    t.position.map(f64::to_bits).hash(&mut h);
    if let Some(s) = &t.stroke {
        s.width.to_bits().hash(&mut h);
    }
    if let Some(s) = &t.shadow {
        s.offset.map(f64::to_bits).hash(&mut h);
        s.blur.to_bits().hash(&mut h);
    }
    t.background.is_some().hash(&mut h);
    canvas.hash(&mut h);
    h.finish()
}

/// 1D squared Euclidean distance transform (Felzenszwalb & Huttenlocher).
fn edt_1d(f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    for q in 1..n {
        let fq = f[q] + (q * q) as f32;
        let mut s;
        loop {
            let p = v[k];
            s = (fq - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            // z[0] is -inf, so this never underflows k.
            if s <= z[k] {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f32::INFINITY;
    }
    k = 0;
    for (q, dq) in d.iter_mut().enumerate().take(n) {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let p = v[k];
        *dq = (q as f32 - p as f32).powi(2) + f[p];
    }
}

fn edt_2d(grid: &mut [f32], w: usize, h: usize) {
    let n = w.max(h);
    let (mut f, mut d, mut v, mut z) = (vec![0f32; n], vec![0f32; n], vec![0usize; n], vec![0f32; n + 1]);
    for x in 0..w {
        for y in 0..h {
            f[y] = grid[y * w + x];
        }
        edt_1d(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        f[..w].copy_from_slice(&grid[y * w..y * w + w]);
        edt_1d(&f[..w], &mut d[..w], &mut v, &mut z);
        grid[y * w..y * w + w].copy_from_slice(&d[..w]);
    }
}

/// Signed distance (pixels, negative inside) from an anti-aliased coverage mask.
pub fn coverage_to_sdf(cover: &[f32], w: usize, h: usize) -> Vec<f32> {
    const INF: f32 = 1e20;
    let mut outside: Vec<f32> = cover.iter().map(|&c| if c >= 0.5 { 0.0 } else { INF }).collect();
    let mut inside: Vec<f32> = cover.iter().map(|&c| if c < 0.5 { 0.0 } else { INF }).collect();
    edt_2d(&mut outside, w, h);
    edt_2d(&mut inside, w, h);
    cover
        .iter()
        .zip(outside.iter().zip(inside.iter()))
        .map(|(&c, (&o, &i))| {
            // Edge pixels: use coverage for sub-pixel accuracy.
            if c > 0.0 && c < 1.0 {
                0.5 - c
            } else if c >= 0.5 {
                -(i.sqrt() - 0.5)
            } else {
                o.sqrt() - 0.5
            }
        })
        .collect()
}

/// IEEE 754 half-precision conversion (round to nearest).
pub fn f32_to_f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xff) as i32;
    let mant = x & 0x7f_ffff;
    if exp == 255 {
        return sign | 0x7c00 | if mant != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 31 {
        return sign | 0x7c00;
    }
    if e <= 0 {
        if e < -10 {
            return sign;
        }
        let m = (mant | 0x80_0000) >> (1 - e);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    let r = sign as u32 | ((e as u32) << 10) | (mant >> 13);
    // Round half up on the dropped bits.
    (r + ((mant >> 12) & 1)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_conversion() {
        assert_eq!(f32_to_f16(0.0), 0);
        assert_eq!(f32_to_f16(1.0), 0x3c00);
        assert_eq!(f32_to_f16(-2.0), 0xc000);
        assert_eq!(f32_to_f16(0.5), 0x3800);
        assert_eq!(f32_to_f16(65504.0), 0x7bff);
    }

    #[test]
    fn sdf_of_square() {
        let (w, h) = (20, 20);
        let mut c = vec![0f32; w * h];
        for y in 5..15 {
            for x in 5..15 {
                c[y * w + x] = 1.0;
            }
        }
        let d = coverage_to_sdf(&c, w, h);
        assert!(d[10 * w + 10] < -4.0);
        assert!((d[10 * w + 2] - 2.5).abs() < 0.01);
    }
}

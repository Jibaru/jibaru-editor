//! Still images, decoded with the `image` crate into RGBA.

use anyhow::{Context as _, Result};
use image::{DynamicImage, ImageDecoder, ImageReader};
use jb_core::frame::{ColorSpace, PixelFormat, VideoFrame};
use std::path::Path;
use std::sync::Arc;

fn open(path: &Path) -> Result<(DynamicImage, image::metadata::Orientation)> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let img = DynamicImage::from_decoder(decoder)?;
    Ok((img, orientation))
}

/// Display dimensions (EXIF orientation applied) without decoding pixels
/// when the format allows it.
pub fn image_dimensions(path: &Path) -> Result<(u32, u32, u32)> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let (w, h) = decoder.dimensions();
    use image::metadata::Orientation as O;
    let swapped = matches!(
        decoder.orientation().unwrap_or(O::NoTransforms),
        O::Rotate90 | O::Rotate270 | O::Rotate90FlipH | O::Rotate270FlipH
    );
    Ok(if swapped { (h, w, 0) } else { (w, h, 0) })
}

/// Loads an image as an upright RGBA frame no larger than `max_size`.
pub fn load_image(path: &Path, max_size: Option<(u32, u32)>) -> Result<VideoFrame> {
    let (mut img, orientation) = open(path)?;
    img.apply_orientation(orientation);
    if let Some((mw, mh)) = max_size {
        if img.width() > mw || img.height() > mh {
            img = img.resize(mw, mh, image::imageops::FilterType::Triangle);
        }
    }
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(VideoFrame {
        width: w,
        height: h,
        format: PixelFormat::Rgba8,
        color_space: ColorSpace::Bt709,
        full_range: true,
        stride: w as usize * 4,
        data: Arc::new(rgba.into_raw()),
        pts: 0,
    })
}

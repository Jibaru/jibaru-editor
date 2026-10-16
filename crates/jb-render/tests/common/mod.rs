#![allow(dead_code)]

use jb_render::Renderer;

/// A renderer, or None (test skipped) when no GPU adapter exists.
pub fn renderer(w: u32, h: u32) -> Option<Renderer> {
    match Renderer::new(w, h) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    }
}

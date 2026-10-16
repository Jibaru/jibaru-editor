#![allow(dead_code)]

use jb_render::Renderer;

/// A renderer with exact output levels (no Filmora output bias), or None
/// (test skipped) when no GPU adapter exists.
pub fn renderer(w: u32, h: u32) -> Option<Renderer> {
    match Renderer::new(w, h) {
        Ok(mut r) => {
            r.options.output_bias = [0.0; 3];
            Some(r)
        }
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    }
}

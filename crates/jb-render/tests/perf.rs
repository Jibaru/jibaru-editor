//! Throughput of a typical 4K frame: two 4K NV12 video layers (one as a
//! picture-in-picture with a blend mode), a title, plus two effects.
//! Prints ms/frame; run with `--release -- --nocapture` for real numbers.
//! The budget assertion only applies to release builds on hardware GPUs.

mod common;

use jb_core::eval::{FramePlan, LayerContent};
use jb_core::frame::{FrameSource, VideoFrame};
use jb_core::param::Value;
use jb_core::{BlendMode, Ticks, TitleData};
use jb_render::testing::*;
use std::time::Instant;

/// Serves two different frames per media, alternating with time, so every
/// rendered frame needs a fresh upload like real playback.
struct Alternating {
    frames: Vec<(String, [VideoFrame; 2])>,
}

impl FrameSource for Alternating {
    fn frame(&self, id: &str, t: Ticks, _: Option<(u32, u32)>) -> anyhow::Result<Option<VideoFrame>> {
        let f = self.frames.iter().find(|(m, _)| m == id).map(|(_, f)| {
            let mut v = f[(t / 333_667) as usize % 2].clone();
            v.pts = t;
            v
        });
        Ok(f)
    }
}

fn plan(t: Ticks) -> FramePlan {
    let mut base = media_layer("a");
    base.content = LayerContent::Media { media_id: "a".into(), source_time: t };
    base.effects.push(effect("jb.color.adjust", 0.5, &[("contrast", Value::Float(20.0)), ("saturation", Value::Float(15.0))]));
    let mut pip = media_layer("b");
    pip.content = LayerContent::Media { media_id: "b".into(), source_time: t };
    pip.transform.scale = [0.45, 0.45];
    pip.transform.position = [0.22, -0.2];
    pip.transform.rotation = 4.0;
    pip.blend_mode = BlendMode::Screen;
    pip.effects.push(effect("jb.gaussian_blur", 0.5, &[("intensity", Value::Float(30.0))]));
    let mut title = media_layer("t");
    title.content = LayerContent::Title(TitleData {
        text: "Jibaru 4K".into(),
        font_family: String::new(),
        font_size: 0.1,
        color: [1.0, 1.0, 1.0, 1.0],
        position: [0.5, 0.82],
        bold: true,
        italic: false,
        stroke: Some(jb_core::Stroke { color: [0.0, 0.0, 0.0, 1.0], width: 0.004 }),
        shadow: Some(jb_core::Shadow { color: [0.0, 0.0, 0.0, 0.6], offset: [0.004, 0.004], blur: 0.004 }),
        background: None,
    });
    FramePlan { time: t, layers: vec![base, pip, title] }
}

#[test]
fn compose_4k_three_layers_two_effects() {
    let (w, h) = (3840, 2160);
    let Some(mut r) = common::renderer(w, h) else { return };
    let mk = |f: fn(u32, u32) -> VideoFrame| {
        let a = to_nv12(&f(w, h), false);
        let mut b = a.clone();
        let mut d = (*b.data).clone();
        d.iter_mut().take(w as usize * 64).for_each(|v| *v = v.wrapping_add(7));
        b.data = std::sync::Arc::new(d);
        [a, b]
    };
    let src = Alternating { frames: vec![("a".into(), mk(pattern_a)), ("b".into(), mk(pattern_b))] };
    let project = project(w, h);
    let step = 333_667; // one 29.97 fps frame
    // Warm up (pipeline compilation, pool allocation, font loading).
    for i in 0..3 {
        r.render_to_texture(&project, &plan(i * step), &src);
    }
    r.wait_idle();
    let report = r.last_report().clone();
    let n = 60;
    let t0 = Instant::now();
    for i in 0..n {
        r.render_to_texture(&project, &plan((i + 3) * step), &src);
    }
    r.wait_idle();
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let t1 = Instant::now();
    let m = 10;
    for i in 0..m {
        let f = r.render(&project, &plan(i * step), &src);
        assert_eq!(f.width, w);
    }
    let ms_readback = t1.elapsed().as_secs_f64() * 1000.0 / m as f64;
    let adapter = r.gpu().adapter_info.name.clone();
    println!("4K 3 layers + 2 effects on {adapter}: {ms:.2} ms/frame (preview path), {ms_readback:.2} ms/frame with RGBA readback");
    assert!(report.unknown_kinds.is_empty() && report.errors.is_empty(), "{report:?}");
    if !cfg!(debug_assertions) && !r.gpu().is_software() {
        assert!(ms < 16.0, "4K frame took {ms:.2} ms");
    }
}

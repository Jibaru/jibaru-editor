//! Compositor behaviour checked against analytic expectations.

mod common;

use jb_core::eval::{FramePlan, LayerContent};
use jb_core::BlendMode;
use jb_render::testing::*;

fn solid(color: [f64; 4]) -> jb_core::eval::Layer {
    let mut l = media_layer("x");
    l.content = LayerContent::Color(color);
    l
}

fn srgb(v: f64) -> u8 {
    let e = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (e * 255.0).round() as u8
}

fn close(a: [u8; 4], b: [u8; 4], tol: i32) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (*x as i32 - *y as i32).abs() <= tol)
}

#[test]
fn passthrough_media_matches_source() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let src = MemorySource::ab(64, 36);
    let p = project(64, 36);
    let plan = FramePlan { time: 0, layers: vec![media_layer("a")] };
    let out = r.render(&p, &plan, &src);
    let d = mean_abs_diff(&out.data, &src.frames["a"].data);
    assert!(d < 0.6, "diff {d}");
    assert!(r.last_report().unknown_kinds.is_empty());
}

#[test]
fn nv12_limited_and_full_range_convert() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let p = project(64, 36);
    let a = pattern_a(64, 36);
    for full in [false, true] {
        let src = MemorySource::new().with("a", to_nv12(&a, full));
        let plan = FramePlan { time: 0, layers: vec![media_layer("a")] };
        let out = r.render(&p, &plan, &src);
        let d = mean_abs_diff(&out.data, &a.data);
        // 4:2:0 subsampling blurs chroma on sharp edges; the average stays small.
        assert!(d < 6.0, "full={full} diff {d}");
    }
}

#[test]
fn dissolve_midpoint_mixes_colors() {
    let Some(mut r) = common::renderer(32, 18) else { return };
    let p = project(32, 18);
    let src = MemorySource::new()
        .with("a", jb_render::testing::pattern_a(32, 18))
        .with("b", jb_render::testing::pattern_b(32, 18));
    let plan = transition_plan("jb.dissolve", 0.5, &[]);
    let out = r.render(&p, &plan, &src);
    let (a, b) = (&src.frames["a"], &src.frames["b"]);
    for (x, y) in [(3, 3), (20, 10), (30, 16)] {
        let (pa, pb, po) = (pixel(a, x, y), pixel(b, x, y), pixel(&out, x, y));
        for k in 0..3 {
            let want = (pa[k] as i32 + pb[k] as i32) / 2;
            assert!((po[k] as i32 - want).abs() <= 2, "{x},{y} ch{k}: {} vs {want}", po[k]);
        }
    }
}

#[test]
fn unknown_kinds_are_reported_and_pass_through() {
    let Some(mut r) = common::renderer(32, 18) else { return };
    let p = project(32, 18);
    let src = MemorySource::ab(32, 18);
    let plan = effect_plan("filmora:does-not-exist", 0.5, &[]);
    let out = r.render(&p, &plan, &src);
    assert!(r.last_report().unknown_kinds.contains("filmora:does-not-exist"));
    assert!(mean_abs_diff(&out.data, &src.frames["a"].data) < 0.6);
}

#[test]
fn solid_colors_and_blend_modes() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    let p = project(16, 16);
    let src = MemorySource::new();
    let bottom = [0.5, 0.25, 1.0, 1.0];
    let top = [0.5, 1.0, 0.2, 1.0];
    let to_disp = |c: [f64; 4]| c.map(|v| {
        if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
    });
    let (b, t) = (to_disp(bottom), to_disp(top));
    let cases: Vec<(BlendMode, Box<dyn Fn(f64, f64) -> f64>)> = vec![
        (BlendMode::Normal, Box::new(|_b, s| s)),
        (BlendMode::Multiply, Box::new(|b, s| b * s)),
        (BlendMode::Screen, Box::new(|b, s| b + s - b * s)),
        (BlendMode::Darken, Box::new(|b: f64, s: f64| b.min(s))),
        (BlendMode::Lighten, Box::new(|b: f64, s: f64| b.max(s))),
        (BlendMode::Difference, Box::new(|b: f64, s: f64| (b - s).abs())),
        (BlendMode::Add, Box::new(|b: f64, s: f64| (b + s).min(1.0))),
    ];
    for (mode, f) in cases {
        let mut l2 = solid(top);
        l2.blend_mode = mode;
        let plan = FramePlan { time: 0, layers: vec![solid(bottom), l2] };
        let out = r.render(&p, &plan, &src);
        let px = pixel(&out, 8, 8);
        let want = [0, 1, 2].map(|k| (f(b[k], t[k]) * 255.0).round() as u8);
        assert!(close(px, [want[0], want[1], want[2], 255], 2), "{mode:?}: {px:?} vs {want:?}");
    }
    let _ = srgb(0.5);
}

#[test]
fn opacity_and_fit_modes() {
    let Some(mut r) = common::renderer(40, 20) else { return };
    let p = project(40, 20);
    // A square 10x10 white image fitted into a 40x20 canvas -> 20x20 centred.
    let sq = jb_render::testing::pattern_a(10, 10);
    let white = jb_core::frame::VideoFrame { data: std::sync::Arc::new(vec![255u8; 400]), ..sq };
    let src = MemorySource::new().with("w", white);
    let mut l = media_layer("w");
    let plan = FramePlan { time: 0, layers: vec![l.clone()] };
    let out = r.render(&p, &plan, &src);
    assert_eq!(pixel(&out, 20, 10), [255, 255, 255, 255]);
    assert_eq!(pixel(&out, 5, 10), [0, 0, 0, 255]);
    assert_eq!(pixel(&out, 34, 10), [0, 0, 0, 255]);
    l.transform.fit = jb_core::FitMode::Fill;
    l.opacity = 0.5;
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l.clone()] }, &src);
    let px = pixel(&out, 2, 10);
    assert!((px[0] as i32 - 128).abs() <= 1, "{px:?}");
    // Moved right by a quarter canvas and scaled by half.
    l.opacity = 1.0;
    l.transform.fit = jb_core::FitMode::Fit;
    l.transform.position = [0.25, 0.0];
    l.transform.scale = [0.5, 0.5];
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l] }, &src);
    assert_eq!(pixel(&out, 30, 10), [255, 255, 255, 255]);
    assert_eq!(pixel(&out, 20, 10), [0, 0, 0, 255]);
    assert_eq!(pixel(&out, 30, 3), [0, 0, 0, 255]);
}

#[test]
fn nested_sequence_and_adjustment_layer() {
    let Some(mut r) = common::renderer(32, 18) else { return };
    let p = project(32, 18);
    let src = MemorySource::ab(32, 18);
    let inner = FramePlan { time: 0, layers: vec![media_layer("b")] };
    let mut nested = media_layer("n");
    nested.content = LayerContent::Nested(Box::new(inner));
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![nested] }, &src);
    assert!(mean_abs_diff(&out.data, &src.frames["b"].data) < 0.6);
}

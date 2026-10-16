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
    let to_disp = |c: [f64; 4]| c.map(|v| if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 });
    let (b, t) = (to_disp(bottom), to_disp(top));
    type BlendFn = Box<dyn Fn(f64, f64) -> f64>;
    let cases: Vec<(BlendMode, BlendFn)> = vec![
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

fn title(text: &str) -> jb_core::TitleData {
    jb_core::TitleData {
        text: text.into(),
        font_family: String::new(),
        font_size: 0.25,
        color: [1.0, 0.0, 0.0, 1.0],
        position: [0.5, 0.5],
        bold: true,
        italic: false,
        stroke: None,
        shadow: None,
        background: None,
    }
}

#[test]
fn titles_render_fill_stroke_shadow_background() {
    let Some(mut r) = common::renderer(320, 180) else { return };
    let p = project(320, 180);
    let src = MemorySource::new();
    let mut l = media_layer("t");
    l.content = LayerContent::Title(title("HI"));
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l.clone()] }, &src);
    let reds =
        (0..180).flat_map(|y| (0..320).map(move |x| (x, y))).filter(|&(x, y)| pixel(&out, x, y)[0] > 200).count();
    assert!(reds > 500, "fill pixels: {reds}");
    // Text is centred: nothing in the left and right margins.
    assert!((0..180).all(|y| pixel(&out, 5, y)[0] == 0 && pixel(&out, 314, y)[0] == 0));
    // Stroke + background.
    let mut t = title("HI");
    t.stroke = Some(jb_core::Stroke { color: [0.0, 1.0, 0.0, 1.0], width: 0.02 });
    t.background = Some([0.0, 0.0, 1.0, 1.0]);
    t.shadow = Some(jb_core::Shadow { color: [1.0, 1.0, 1.0, 1.0], offset: [0.03, 0.03], blur: 0.0 });
    l.content = LayerContent::Title(t);
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l] }, &src);
    let count = |f: &dyn Fn([u8; 4]) -> bool| {
        (0..180).flat_map(|y| (0..320).map(move |x| (x, y))).filter(|&(x, y)| f(pixel(&out, x, y))).count()
    };
    assert!(count(&|c| c[1] > 200 && c[0] < 50) > 200, "stroke");
    assert!(count(&|c| c[2] > 200 && c[0] < 50 && c[1] < 50) > 1000, "background");
    assert!(count(&|c| c[0] > 200 && c[1] > 200 && c[2] > 200) > 50, "shadow");
    assert!(count(&|c| c[0] > 200 && c[1] < 50) > 300, "fill");
}

#[test]
fn crop_rotation_and_flip() {
    let Some(mut r) = common::renderer(40, 40) else { return };
    let p = project(40, 40);
    // Left half red, right half blue.
    let mut data = vec![0u8; 40 * 40 * 4];
    for y in 0..40 {
        for x in 0..40 {
            let i = (y * 40 + x) * 4;
            data[i..i + 4].copy_from_slice(if x < 20 { &[255, 0, 0, 255] } else { &[0, 0, 255, 255] });
        }
    }
    let f = jb_core::frame::VideoFrame { data: std::sync::Arc::new(data), ..pattern_a(40, 40) };
    let src = MemorySource::new().with("s", f);
    let mut l = media_layer("s");
    // Crop away the right half: the red half is stretched over the canvas (fit = stretch).
    l.transform.crop = [0.0, 0.0, 0.5, 0.0];
    l.transform.fit = jb_core::FitMode::Stretch;
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l.clone()] }, &src);
    assert_eq!(pixel(&out, 35, 20), [255, 0, 0, 255]);
    // Flip horizontally without crop: blue on the left.
    l.transform.crop = [0.0; 4];
    l.transform.flip_h = true;
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l.clone()] }, &src);
    assert_eq!(pixel(&out, 5, 20), [0, 0, 255, 255]);
    // Rotate 90 degrees clockwise (unflipped): red (left) ends up on top.
    l.transform.flip_h = false;
    l.transform.rotation = 90.0;
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![l] }, &src);
    assert_eq!(pixel(&out, 20, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&out, 20, 35), [0, 0, 255, 255]);
}

#[test]
fn linear_light_mode_blends_in_linear() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    r.options.linear_light = true;
    let p = project(16, 16);
    let src = MemorySource::new();
    let mut top = solid([1.0, 1.0, 1.0, 1.0]);
    top.opacity = 0.5;
    let out = r.render(&p, &FramePlan { time: 0, layers: vec![solid([0.0, 0.0, 0.0, 1.0]), top] }, &src);
    // 50% linear grey encodes to ~188 in sRGB (display-space mixing would give 128).
    let v = pixel(&out, 8, 8)[0];
    assert!((v as i32 - 188).abs() <= 2, "{v}");
}

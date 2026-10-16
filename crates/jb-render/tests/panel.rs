//! Properties-panel effects (corner radius, chroma key, masks, denoise, lens
//! correction, stabilization, color wheels, background and shadow).

mod common;

use jb_core::eval::{FramePlan, Layer, LayerContent};
use jb_core::param::Value;
use jb_render::testing::*;

const W: u32 = 64;
const H: u32 = 36;

fn render(plan: &FramePlan, w: u32, h: u32) -> Option<jb_core::frame::VideoFrame> {
    let mut r = common::renderer(w, h)?;
    let src = MemorySource::ab(w, h);
    let out = r.render(&project(w, h), plan, &src);
    assert!(r.last_report().unknown_kinds.is_empty(), "{:?}", r.last_report().unknown_kinds);
    assert!(r.last_report().errors.is_empty(), "{:?}", r.last_report().errors);
    Some(out)
}

fn plan(kind: &str, params: &[(&str, Value)]) -> FramePlan {
    effect_plan(kind, 0.5, params)
}

/// Mean difference against the untouched source.
fn diff(kind: &str, params: &[(&str, Value)]) -> Option<f64> {
    let out = render(&plan(kind, params), W, H)?;
    Some(mean_abs_diff(&out.data, &pattern_a(W, H).data))
}

fn f(v: f64) -> Value {
    Value::Float(v)
}

#[test]
fn neutral_defaults_are_identity() {
    for (kind, params) in [
        ("jb.corner_radius", vec![]),
        ("jb.denoise", vec![("strength", f(0.0))]),
        ("jb.lens_correction", vec![]),
        ("jb.stabilize", vec![]),
        ("jb.color.wheels", vec![]),
        ("jb.background", vec![]),
        ("jb.shadow", vec![]),
        ("jb.mask.rectangle", vec![("opacity", f(0.0))]),
    ] {
        let Some(d) = diff(kind, &params) else { return };
        assert!(d < 0.8, "{kind}: diff {d}");
    }
}

#[test]
fn non_default_params_change_pixels() {
    for (kind, params) in [
        ("jb.corner_radius", vec![("radius", f(100.0))]),
        ("jb.denoise", vec![("strength", f(100.0))]),
        ("jb.lens_correction", vec![("level", f(80.0))]),
        ("jb.lens_correction", vec![("level", f(-80.0))]),
        ("jb.stabilize", vec![("data", Value::Text("30|0.1,0.05;0.1,0.05".into()))]),
        ("jb.color.wheels", vec![("shadows_x", f(80.0)), ("highlights_lum", f(-50.0))]),
        ("jb.chroma_key", vec![("color", Value::Color([0.05, 0.3, 0.6, 1.0])), ("tolerance", f(60.0))]),
    ] {
        let Some(d) = diff(kind, &params) else { return };
        assert!(d > 1.0, "{kind} {params:?}: diff {d}");
    }
}

#[test]
fn corner_radius_clears_corners() {
    let Some(out) = render(&plan("jb.corner_radius", &[("radius", f(100.0))]), W, H) else { return };
    assert_eq!(&pixel(&out, 0, 0)[..3], &[0, 0, 0]);
    assert_eq!(pixel(&out, 32, 18), pixel(&pattern_a(W, H), 32, 18));
}

fn solid(color: [f64; 4], kind: &str, params: &[(&str, Value)]) -> FramePlan {
    let mut l = media_layer("x");
    l.content = LayerContent::Color(color);
    l.effects.push(effect(kind, 0.5, params));
    FramePlan { time: 0, layers: vec![l] }
}

#[test]
fn chroma_key_removes_green_keeps_red() {
    let Some(g) = render(&solid([0.0, 1.0, 0.0, 1.0], "jb.chroma_key", &[]), 16, 16) else { return };
    assert_eq!(&pixel(&g, 8, 8)[..3], &[0, 0, 0], "green keyed onto the black background");
    let Some(r) = render(&solid([1.0, 0.0, 0.0, 1.0], "jb.chroma_key", &[]), 16, 16) else { return };
    let p = pixel(&r, 8, 8);
    assert!(p[0] > 240 && p[1] < 10, "{p:?}");
    let Some(a) =
        render(&solid([1.0, 0.0, 0.0, 1.0], "jb.chroma_key", &[("alpha_channel", Value::Bool(true))]), 16, 16)
    else {
        return;
    };
    assert!(pixel(&a, 8, 8)[..3].iter().all(|v| *v > 245), "matte shown white where kept");
    // Edge controls run without errors.
    let Some(e) = render(
        &solid([0.0, 1.0, 0.0, 1.0], "jb.chroma_key", &[("edge_thickness", f(3.0)), ("edge_feather", f(4.0))]),
        16,
        16,
    ) else {
        return;
    };
    assert_eq!(&pixel(&e, 8, 8)[..3], &[0, 0, 0]);
}

#[test]
fn masks_keep_the_inside() {
    let src = pattern_a(W, H);
    for shape in ["rectangle", "ellipse", "heart", "star", "double_line"] {
        let kind = format!("jb.mask.{shape}");
        let Some(out) = render(&plan(&kind, &[]), W, H) else { return };
        assert_eq!(&pixel(&out, 0, 0)[..3], &[0, 0, 0], "{shape}: corner masked");
        let c = if shape == "heart" { (32, 20) } else { (32, 18) };
        assert_eq!(pixel(&out, c.0, c.1), pixel(&src, c.0, c.1), "{shape}: centre kept");
        let Some(inv) = render(&plan(&kind, &[("invert", Value::Bool(true))]), W, H) else { return };
        assert_eq!(pixel(&inv, 0, 0), pixel(&src, 0, 0), "{shape}: inverted keeps the corner");
    }
    // Single line keeps the upper half.
    let Some(out) = render(&plan("jb.mask.single_line", &[]), W, H) else { return };
    assert_eq!(pixel(&out, 10, 5), pixel(&src, 10, 5));
    assert_eq!(&pixel(&out, 10, 30)[..3], &[0, 0, 0]);
}

/// Pattern A shrunk to half size in the middle of a 192x108 canvas.
fn pip(effect_kind: &str, params: &[(&str, Value)]) -> Layer {
    let mut l = media_layer("a");
    l.transform.scale = [0.5, 0.5];
    l.effects.push(effect(effect_kind, 0.5, params));
    l
}

#[test]
fn background_fills_around_a_pip_clip() {
    let (w, h) = (192, 108);
    let Some(blur) = render(&FramePlan { time: 0, layers: vec![pip("jb.background", &[])] }, w, h) else { return };
    let p = pixel(&blur, 4, 4);
    assert!(p[..3].iter().any(|v| *v > 20), "blurred fill visible in the corner: {p:?}");
    let Some(color) = render(
        &FramePlan {
            time: 0,
            layers: vec![pip("jb.background", &[("mode", f(1.0)), ("color", Value::Color([1.0, 0.0, 0.0, 1.0]))])],
        },
        w,
        h,
    ) else {
        return;
    };
    assert_eq!(&pixel(&color, 4, 4)[..3], &[255, 0, 0]);
    // The clip itself stays on top.
    let none = render(&FramePlan { time: 0, layers: vec![pip("jb.corner_radius", &[])] }, w, h).unwrap();
    assert_eq!(pixel(&color, 96, 54), pixel(&none, 96, 54));
}

#[test]
fn shadow_is_drawn_outside_the_clip() {
    let (w, h) = (192, 108);
    let white = Value::Color([1.0, 1.0, 1.0, 1.0]);
    let params =
        [("distance", f(100.0)), ("angle", f(45.0)), ("blur", f(0.0)), ("opacity", f(100.0)), ("color", white)];
    let Some(out) = render(&FramePlan { time: 0, layers: vec![pip("jb.shadow", &params)] }, w, h) else { return };
    // Clip covers x 48..144, y 27..81; the shadow is offset ~7 px down-right.
    let p = pixel(&out, 147, 84);
    assert!(p[..3].iter().all(|v| *v > 200), "shadow below-right of the clip: {p:?}");
    assert_eq!(&pixel(&out, 40, 20)[..3], &[0, 0, 0], "nothing up-left");
    let none = render(&FramePlan { time: 0, layers: vec![pip("jb.corner_radius", &[])] }, w, h).unwrap();
    assert_eq!(pixel(&out, 96, 54), pixel(&none, 96, 54), "clip drawn over its shadow");
}

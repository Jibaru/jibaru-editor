//! Color tools and looks checked numerically.

mod common;

use jb_core::eval::{FramePlan, LayerContent};
use jb_core::param::Value;
use jb_render::testing::*;

fn srgb(v: f64) -> f64 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn solid_plan(color: [f64; 4], kind: &str, params: &[(&str, Value)]) -> FramePlan {
    let mut l = media_layer("x");
    l.content = LayerContent::Color(color);
    l.effects.push(effect(kind, 0.5, params));
    FramePlan { time: 0, layers: vec![l] }
}

fn px(r: &mut jb_render::Renderer, plan: &FramePlan) -> [u8; 4] {
    let src = MemorySource::ab(16, 16);
    let f = r.render(&project(16, 16), plan, &src);
    pixel(&f, 8, 8)
}

#[test]
fn exposure_one_stop_doubles_linear_light() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    let p = px(&mut r, &solid_plan([0.2, 0.1, 0.05, 1.0], "jb.color.exposure", &[("stops", Value::Float(1.0))]));
    for (k, v) in [0.4, 0.2, 0.1].iter().enumerate() {
        let want = (srgb(*v) * 255.0).round() as i32;
        assert!((p[k] as i32 - want).abs() <= 2, "ch{k} {} vs {want}", p[k]);
    }
}

#[test]
fn saturation_minus_100_is_grey() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    let p = px(&mut r, &solid_plan([0.8, 0.2, 0.05, 1.0], "jb.color.saturation", &[("amount", Value::Float(-100.0))]));
    assert!((p[0] as i32 - p[1] as i32).abs() <= 1 && (p[1] as i32 - p[2] as i32).abs() <= 1, "{p:?}");
}

#[test]
fn neutral_defaults_are_no_ops() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    let c = [0.3, 0.5, 0.1, 1.0];
    let base = px(
        &mut r,
        &FramePlan {
            time: 0,
            layers: vec![{
                let mut l = media_layer("x");
                l.content = LayerContent::Color(c);
                l
            }],
        },
    );
    for kind in [
        "jb.color.exposure",
        "jb.color.brightness",
        "jb.color.contrast",
        "jb.color.saturation",
        "jb.color.vibrance",
        "jb.color.temperature",
        "jb.color.tint",
        "jb.color.highlights_shadows",
        "jb.color.hsl",
        "jb.color.curves",
        "jb.color.lut",
        "jb.color.adjust",
    ] {
        let p = px(&mut r, &solid_plan(c, kind, &[]));
        assert!(
            base.iter().zip(p.iter()).all(|(a, b)| (*a as i32 - *b as i32).abs() <= 1),
            "{kind}: {p:?} vs {base:?}"
        );
        assert!(r.last_report().unknown_kinds.is_empty() && r.last_report().errors.is_empty(), "{kind}");
    }
    // Looks at zero intensity.
    let p = px(&mut r, &solid_plan(c, "jb.teal_orange", &[("intensidad", Value::Float(0.0))]));
    assert!(base.iter().zip(p.iter()).all(|(a, b)| (*a as i32 - *b as i32).abs() <= 1), "look: {p:?} vs {base:?}");
}

#[test]
fn curves_and_lut_files() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    let c = [0.3, 0.5, 0.1, 1.0];
    let want = c.map(srgb);
    // Inverting master curve.
    let p = px(&mut r, &solid_plan(c, "jb.color.curves", &[("master", Value::Text("0/1 1/0".into()))]));
    for k in 0..3 {
        let w = ((1.0 - want[k]) * 255.0).round() as i32;
        assert!((p[k] as i32 - w).abs() <= 2, "curves ch{k}: {} vs {w}", p[k]);
    }
    // Identity LUT file is a no-op; an inverting one inverts.
    let dir = std::env::temp_dir().join(format!("jb-lut-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, inv) in [("id.cube", false), ("inv.cube", true)] {
        let mut s = String::from("LUT_3D_SIZE 17\n");
        for b in 0..17 {
            for g in 0..17 {
                for rr in 0..17 {
                    let v = [rr, g, b].map(|x| x as f64 / 16.0).map(|x| if inv { 1.0 - x } else { x });
                    s += &format!("{} {} {}\n", v[0], v[1], v[2]);
                }
            }
        }
        let path = dir.join(name);
        std::fs::write(&path, s).unwrap();
        let p = px(&mut r, &solid_plan(c, "jb.color.lut", &[("path", Value::Text(path.to_string_lossy().into()))]));
        for k in 0..3 {
            let w = ((if inv { 1.0 - want[k] } else { want[k] }) * 255.0).round() as i32;
            assert!((p[k] as i32 - w).abs() <= 2, "{name} ch{k}: {} vs {w}", p[k]);
        }
    }
    let _ = px(&mut r, &solid_plan(c, "jb.color.lut", &[("path", Value::Text("/nonexistent.cube".into()))]));
    assert!(!r.last_report().errors.is_empty());
}

#[test]
fn black_and_white_look_is_grey() {
    let Some(mut r) = common::renderer(16, 16) else { return };
    let p =
        px(&mut r, &solid_plan([0.8, 0.2, 0.05, 1.0], "jb.bn_alto_contraste", &[("intensidad", Value::Float(100.0))]));
    assert!((p[0] as i32 - p[1] as i32).abs() <= 1 && (p[1] as i32 - p[2] as i32).abs() <= 1, "{p:?}");
}

//! Lane 3 transitions: keyframe tables interpolate between rows, and every
//! fitted Filmora GUID resolves to a registered transition.

mod common;

use jb_render::testing::*;
use jb_render::{FxKind, Registry};

fn mean(data: &[u8]) -> f64 {
    let rgb = data.iter().enumerate().filter(|(i, _)| i % 4 != 3);
    rgb.map(|(_, v)| *v as f64).sum::<f64>() / (data.len() / 4 * 3) as f64
}

#[test]
fn keyframes_interpolate_between_rows() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let src = MemorySource::ab(64, 36);
    let p = project(64, 36);
    // Wash to white brightens steadily over the first half, between and on rows.
    let mut last = 0.0;
    for k in 0..12 {
        let t = 0.02 + k as f64 * 0.035;
        let out = r.render(&p, &transition_plan("jb.wash_white", t, &[]), &src);
        let m = mean(&out.data);
        assert!(m > last + 0.5, "progress {t}: mean {m} after {last}");
        last = m;
    }
}

#[test]
fn lane3_aliases_resolve_to_transitions() {
    let reg = Registry::builtin();
    for (fid, to, name) in jb_render::aliases_lane3::FILMORA_LANE3 {
        let fx = reg.get(&format!("filmora:{fid}")).unwrap_or_else(|| panic!("{name}: {to} not registered"));
        assert_eq!(fx.info().kind, FxKind::Transition, "{name}");
    }
}

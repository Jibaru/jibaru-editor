//! Effects and transitions used by the user's Filmora projects
//! (docs/filmora-coverage.md) resolve and render.

mod common;

use jb_core::param::Value;
use jb_render::testing::*;

#[test]
fn filmora_coverage_ids_resolve() {
    let reg = jb_render::Registry::builtin();
    for (fid, to) in jb_render::aliases::FILMORA {
        assert_eq!(reg.canonical(&format!("filmora:{fid}")), Some(*to), "{fid}");
    }
}

#[test]
fn filmora_adjust_color_exposure_brightens() {
    let Some(mut r) = common::renderer(32, 18) else { return };
    let p = project(32, 18);
    let src = MemorySource::ab(32, 18);
    let plan = effect_plan("filmora:662E16ED-4524-4D13-AAE9-11DBA0C63E17", 0.5, &[("u_exposure", Value::Float(60.0))]);
    let out = r.render(&p, &plan, &src);
    assert!(r.last_report().unknown_kinds.is_empty());
    let sum = |d: &[u8]| d.iter().map(|v| *v as u64).sum::<u64>();
    assert!(sum(&out.data) > sum(&src.frames["a"].data) + 1000);
}

#[test]
fn filmora_transitions_go_from_a_to_b() {
    let Some(mut r) = common::renderer(48, 27) else { return };
    let p = project(48, 27);
    let src = MemorySource::ab(48, 27);
    for (fid, _) in jb_render::aliases::FILMORA.iter().take(6) {
        let kind = format!("filmora:{fid}");
        let start = r.render(&p, &transition_plan(&kind, 0.0, &[]), &src);
        assert!(r.last_report().unknown_kinds.is_empty(), "{kind}");
        assert!(mean_abs_diff(&start.data, &src.frames["a"].data) < 2.0, "{kind} start");
        let end = r.render(&p, &transition_plan(&kind, 1.0, &[]), &src);
        assert!(mean_abs_diff(&end.data, &src.frames["b"].data) < 2.0, "{kind} end");
    }
}

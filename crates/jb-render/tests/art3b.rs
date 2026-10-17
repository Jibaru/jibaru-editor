//! Art3b motion-graphics transitions: every mapped Filmora GUID resolves to
//! a registered art3b transition, and the scenes render something between
//! the two clips mid-way.

mod common;

use jb_render::testing::*;
use jb_render::{FxKind, Registry};

#[test]
fn art3b_aliases_resolve_to_transitions() {
    let reg = Registry::builtin();
    for (fid, to) in jb_render::aliases_art3b::FILMORA_ART3B {
        let fx = reg.get(&format!("filmora:{fid}")).unwrap_or_else(|| panic!("{fid}: {to} not registered"));
        assert_eq!(fx.info().id, *to, "{fid} maps to {} instead of {to}", fx.info().id);
        assert_eq!(fx.info().kind, FxKind::Transition, "{fid}");
    }
}

#[test]
fn art3b_items_start_on_a_and_end_on_b() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let src = MemorySource::ab(64, 36);
    let p = project(64, 36);
    let a = r.render(&p, &transition_plan("jb.dissolve", 0.0, &[]), &src).data;
    let b = r.render(&p, &transition_plan("jb.dissolve", 1.0, &[]), &src).data;
    for id in jb_render::fx::art3b::items::LIB_IDS {
        let s = r.render(&p, &transition_plan(id, 0.0, &[]), &src).data;
        let e = r.render(&p, &transition_plan(id, 1.0, &[]), &src).data;
        assert!(mean_abs_diff(&s, &a) < 2.0, "{id} does not start on the outgoing clip");
        assert!(mean_abs_diff(&e, &b) < 2.0, "{id} does not end on the incoming clip");
    }
}

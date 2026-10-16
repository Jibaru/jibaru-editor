//! Lane-1 Filmora transitions (fitted tables and parametric wipes) resolve by
//! their exact-case GUID and run from the outgoing to the incoming clip.

mod common;

use jb_render::testing::*;

#[test]
fn lane1_ids_resolve() {
    let reg = jb_render::Registry::builtin();
    for (fid, to) in jb_render::aliases_lane1::FILMORA_LANE1 {
        assert_eq!(reg.canonical(&format!("filmora:{fid}")), Some(*to), "{fid}");
        assert_eq!(reg.canonical(&format!("filmora:{}", fid.to_ascii_uppercase())), Some(*to), "{fid}");
    }
}

#[test]
fn lane1_transitions_end_on_b() {
    let Some(mut r) = common::renderer(48, 27) else { return };
    let p = project(48, 27);
    let src = MemorySource::ab(48, 27);
    for (fid, _) in jb_render::aliases_lane1::FILMORA_LANE1 {
        let kind = format!("filmora:{fid}");
        let end = r.render(&p, &transition_plan(&kind, 1.0, &[]), &src);
        assert!(r.last_report().unknown_kinds.is_empty(), "{kind}");
        assert!(mean_abs_diff(&end.data, &src.frames["b"].data) < 2.0, "{kind} end");
    }
}

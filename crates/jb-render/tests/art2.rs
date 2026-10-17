//! Art 2 (procedural paint / ink / paper transitions): every recreated
//! Filmora GUID resolves to exactly one art 2 item, every item is listed in
//! the library, starts on the outgoing clip and ends on the incoming one.

mod common;

use jb_render::testing::*;
use jb_render::{FxKind, Registry};
use std::collections::HashSet;

#[test]
fn art2_ids_resolve_once() {
    let reg = Registry::builtin();
    let mut seen = HashSet::new();
    for (fid, to, name) in jb_render::aliases_art2::FILMORA_ART2 {
        assert!(seen.insert(fid.to_ascii_uppercase()), "{name}: GUID mapped twice");
        assert_eq!(reg.canonical(&format!("filmora:{fid}")), Some(*to), "{name}");
        assert_eq!(reg.get(to).map(|f| f.info().kind), Some(FxKind::Transition), "{name}");
        // No older lane table maps the same GUID.
        let up = fid.to_ascii_uppercase();
        assert!(
            !jb_render::aliases_lane1::FILMORA_LANE1.iter().any(|(f, _)| f.to_ascii_uppercase() == up),
            "{name}: still in lane 1"
        );
        assert!(
            !jb_render::aliases_lane3::FILMORA_LANE3.iter().any(|(f, _, _)| f.to_ascii_uppercase() == up),
            "{name}: still in lane 3"
        );
    }
}

#[test]
fn art2_items_are_listed() {
    let ids: Vec<&str> = jb_render::fx::art2_items::items().iter().map(|i| i.id).collect();
    assert_eq!(ids, jb_render::fx::art2_items::LIB_IDS);
    let ids: Vec<&str> = jb_render::fx::art2_gold::items().iter().map(|i| i.id).collect();
    assert_eq!(ids, jb_render::fx::art2_gold::LIB_IDS);
}

#[test]
fn art2_runs_from_a_to_b() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let p = project(64, 36);
    let src = MemorySource::ab(64, 36);
    for id in jb_render::fx::art2_items::LIB_IDS.iter().chain(jb_render::fx::art2_gold::LIB_IDS) {
        let start = r.render(&p, &transition_plan(id, 0.0, &[]), &src);
        assert!(r.last_report().errors.is_empty(), "{id}: {:?}", r.last_report().errors);
        assert!(mean_abs_diff(&start.data, &src.frames["a"].data) < 2.0, "{id} start");
        let end = r.render(&p, &transition_plan(id, 1.0, &[]), &src);
        assert!(mean_abs_diff(&end.data, &src.frames["b"].data) < 2.0, "{id} end");
        let mid = r.render(&p, &transition_plan(id, 0.5, &[]), &src);
        let da = mean_abs_diff(&mid.data, &src.frames["a"].data);
        let db = mean_abs_diff(&mid.data, &src.frames["b"].data);
        assert!(da > 1.0 || db > 1.0, "{id} mid");
    }
}

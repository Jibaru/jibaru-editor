//! Art-4 procedural items: every fitted Filmora GUID resolves to a
//! registered item of the same kind, and seeded randomness is reproducible.

mod common;

use jb_render::testing::*;
use jb_render::Registry;

#[test]
fn art4_aliases_resolve() {
    let reg = Registry::builtin();
    for (fid, to, name) in jb_render::aliases_art4::FILMORA_ART4 {
        let fx = reg.get(&format!("filmora:{fid}")).unwrap_or_else(|| panic!("{name}: {to} not registered"));
        assert_eq!(fx.info().id, *to, "{name}");
    }
}

#[test]
fn art4_items_are_deterministic() {
    let Some(mut r) = common::renderer(64, 36) else { return };
    let src = MemorySource::ab(64, 36);
    let p = project(64, 36);
    for s in jb_render::fx::art4_items::SPECS {
        let plan = match s.kind {
            jb_render::FxKind::Transition => transition_plan(s.id, 0.4, &[]),
            _ => effect_plan(s.id, 0.4, &[]),
        };
        let a = r.render(&p, &plan, &src).data.to_vec();
        let b = r.render(&p, &plan, &src).data.to_vec();
        assert_eq!(a, b, "{}", s.id);
    }
}

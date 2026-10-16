//! Library groups (Filmora's sidebar categories) map onto the registry.

use jb_render::library::{group_names, places, GROUPS};
use jb_render::{FxKind, Registry};
use std::collections::HashSet;

/// Registry items the library does not list: properties-panel effects and
/// helpers for imported projects.
fn panel_only(id: &str, category: &str) -> bool {
    category == "Panel"
        || id.starts_with("jb.color.")
        || id.starts_with("jb.filmora.")
        || id.starts_with("jb.fast_wipe_")
        || ["jb.auto_enhance", "jb.pan_slide", "jb.film_burn_02"].contains(&id)
}

#[test]
fn groups_list_existing_items_of_their_tab() {
    let reg = Registry::builtin();
    let mut names = HashSet::new();
    for g in GROUPS {
        assert!(names.insert((g.kind as u8, g.name)), "duplicate group {}", g.name);
        assert!(!g.items.is_empty(), "empty group {}", g.name);
        let mut seen = HashSet::new();
        for id in g.items {
            let info = reg.iter().find(|i| i.id == *id).unwrap_or_else(|| panic!("{}: unknown id {id}", g.name));
            assert_eq!(info.kind, g.kind, "{}: {id} is in the wrong tab", g.name);
            assert!(!panel_only(id, info.category), "{}: {id} belongs to the properties panel", g.name);
            assert!(seen.insert(*id), "{}: {id} listed twice", g.name);
        }
    }
}

#[test]
fn every_library_item_is_in_a_group() {
    let reg = Registry::builtin();
    for info in reg.iter() {
        if info.kind == FxKind::Animation || panel_only(info.id, info.category) {
            continue;
        }
        assert!(places(info.id).next().is_some(), "{} ({}) is in no library group", info.id, info.name);
    }
    // Filmora's first groups.
    assert_eq!(group_names(FxKind::Transition).take(3).collect::<Vec<_>>(), ["Trending", "Fade&Dissolve", "Zoom"]);
    assert_eq!(group_names(FxKind::Effect).next(), Some("Trending"));
    assert_eq!(group_names(FxKind::Filter).next(), Some("Basic"));
}

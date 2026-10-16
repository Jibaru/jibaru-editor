//! Alias table: alternative names and imported identifiers
//! (`filmora:<id>`, prototype `ae:<id>`) mapped onto built-in effects.

use crate::registry::Registry;

/// Short/legacy names for built-ins.
const NAMES: &[(&str, &str)] = &[("jb.cross_fade", "jb.dissolve"), ("jb.crossfade", "jb.dissolve")];

pub fn register(r: &mut Registry) {
    for (from, to) in NAMES {
        if r.get(to).is_some() {
            r.alias(*from, to);
        }
    }
    // Every built-in ported from the agent-editor prototype is reachable by
    // its prototype id as `ae:<id>`.
    let ids: Vec<&'static str> = r.iter().map(|i| i.id).collect();
    for id in ids {
        if let Some(short) = id.strip_prefix("jb.") {
            r.alias(format!("ae:{short}"), id);
        }
    }
}

//! Filmora store transitions of lane 1 (zoom / spin / blur / camera moves and
//! parametric wipes) mapped to their fitted `jb.*` implementations. GUIDs keep
//! their exact case (Filmora resolves them case-sensitively).

use crate::registry::Registry;

/// (Filmora GUID, jb id).
pub const FILMORA_LANE1: &[(&str, &str)] = &[
    ("F9A10F75-ECFC-4DEC-B7B8-BDFB457FBCA0", "jb.slow_bounce"),
    ("FD4A4891-4E1C-4e05-B290-8195CC55E8C5", "jb.soft_wipe_diagonal"),
    ("C456899A-C306-4f88-ACD9-5B158A5B1F48", "jb.soft_wipe_down"),
    ("6675F015-A329-4ee9-9A89-E5427799D0BD", "jb.soft_wipe_right"),
    ("ABBD7AED-1384-4fd2-862B-E955E5F9C3AE", "jb.soft_iris_close"),
    ("74876B99-7AD0-49b3-91D4-D3136E9A850A", "jb.soft_clock_sweep"),
    ("836D2042-F8B9-4E20-96E2-CBD409DB3EDA", "jb.star_reveal"),
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE1 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

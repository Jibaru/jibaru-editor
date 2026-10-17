//! Filmora store transitions (art3b: graphic, cartoon and themed motion
//! transitions) mapped onto our own-art recreations in `fx/art3b_items.rs`.
//! GUIDs keep Filmora's exact case (`register` also adds the upper-case
//! form). Comments give the Filmora item each GUID was measured on
//! (`docs/fidelity-art3b.md`).

use crate::registry::Registry;

pub const FILMORA_ART3B: &[(&str, &str)] = &[
    ("1E059E6B-7E12-4DC1-A5ED-982CF52EDD8F", "jb.glossy_slab_sweep"), // Influencer Toolkit Vol 1 Transition 01
    ("1C10FA86-5D69-4E77-9F5C-C99D2097D86F", "jb.chevron_trio_right"), // Linear 14
    ("C2A21964-21C9-48B3-8E17-3A78E97EC290", "jb.ring_burst_center"), // Linear 19
    ("2A59A5A5-A492-425C-904E-D7703218AC16", "jb.spinning_square_frames"), // Linear 4
    ("3FC5FC6D-C669-40FA-B1AF-308BED370634", "jb.square_frames_zoom"), // Plain Shape 17
    ("CEC1827B-745E-4EB4-AD1B-A1EBEE58FF0D", "jb.hex_windows"), // Plain Shape 20
    ("554FEC3F-E022-46A6-B763-67E2E9D6BA7B", "jb.white_ring_iris"), // Plain Shape 22
    ("33745F8B-AE5A-4DC2-8C1B-1C61735B5A85", "jb.orange_ring_rise"), // Plain Shape 8
    ("1E111A76-BC1D-4BE8-9315-79CB49CC8C36", "jb.red_ring_iris"), // Plain Shape 9
    ("5DC77869-75E6-45F6-9BCA-9B96CBA83DEC", "jb.red_x_panels"), // Scoreboard Transition 01
    ("108B1920-9128-4983-BAC9-FB4100603A8E", "jb.diagonal_streak_sweep"), // Shape Gradient Transitions Transition 25
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_ART3B {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

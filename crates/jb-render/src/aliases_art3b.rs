//! Filmora store transitions (art3b: graphic, cartoon and themed motion
//! transitions) mapped onto our own-art recreations in `fx/art3b_items.rs`.
//! GUIDs keep Filmora's exact case (`register` also adds the upper-case
//! form). Comments give the Filmora item each GUID was measured on
//! (`docs/fidelity-art3b.md`).

use crate::registry::Registry;

pub const FILMORA_ART3B: &[(&str, &str)] = &[
    ("1E059E6B-7E12-4DC1-A5ED-982CF52EDD8F", "jb.glossy_slab_sweep"), // Influencer Toolkit Vol 1 Transition 01
    ("0647A4ED-769A-494E-92EF-34984F81DD18", "jb.card_ring_turn"), // Layered Vision
    ("1C10FA86-5D69-4E77-9F5C-C99D2097D86F", "jb.chevron_trio_right"), // Linear 14
    ("C2A21964-21C9-48B3-8E17-3A78E97EC290", "jb.ring_burst_center"), // Linear 19
    ("2A59A5A5-A492-425C-904E-D7703218AC16", "jb.spinning_square_frames"), // Linear 4
    ("B4DBCD45-650F-4E17-8FDF-7C13C72932EE", "jb.sparkle_heart_swell"), // Love Daily Record Transition 03
    ("DD751802-4E0F-42F2-AA81-600665DE369C", "jb.memphis_disc_roll"), // Memphis Art Transition 06
    ("C9CA2925-ECC7-421C-8E35-60C9984C1EAA", "jb.corner_wedge_close"), // Modern Transparent Transition 10
    ("08C887B6-5597-4A52-BE2B-133E6B6EC9BD", "jb.capsule_column_run"), // Motion Transitions Pack Vol 4 Transition 14
    ("FDFA48E8-C537-46AE-9200-3B952A2030FE", "jb.arrowhead_slots"), // Movie Atmosphere CN Transition 07
    ("9c12f97c-2022-6562-8868-6beeecfc32d0", "jb.chevron_band_sweep"), // News Set Channel 7 Transition 2
    ("EF9351C0-EA39-4190-93FA-E6633052C8E1", "jb.card_carousel"), // Perspective Break
    ("668D3D30-43B8-41ef-A222-86E7912A497E", "jb.pinwheel_wedges"), // Pinwheel
    ("3FC5FC6D-C669-40FA-B1AF-308BED370634", "jb.square_frames_zoom"), // Plain Shape 17
    ("CEC1827B-745E-4EB4-AD1B-A1EBEE58FF0D", "jb.hex_windows"), // Plain Shape 20
    ("554FEC3F-E022-46A6-B763-67E2E9D6BA7B", "jb.white_ring_iris"), // Plain Shape 22
    ("33745F8B-AE5A-4DC2-8C1B-1C61735B5A85", "jb.orange_ring_rise"), // Plain Shape 8
    ("1E111A76-BC1D-4BE8-9315-79CB49CC8C36", "jb.red_ring_iris"), // Plain Shape 9
    ("CE1BB859-98C2-4E9F-90D0-DE880C931F12", "jb.red_streak_lanes"), // Powerful Sports Team Transition 02
    ("CAD04901-7A9A-4172-A68F-82F3E1781E52", "jb.photo_desk_pan"), // Profile Movie Transition 01
    ("5DC77869-75E6-45F6-9BCA-9B96CBA83DEC", "jb.red_x_panels"), // Scoreboard Transition 01
    ("108B1920-9128-4983-BAC9-FB4100603A8E", "jb.diagonal_streak_sweep"), // Shape Gradient Transitions Transition 25
    ("401962DA-6B63-4EB0-95C5-5C43A04601DE", "jb.panel_ring_spin"), // Space Rotation Left
    ("E943F100-16E7-4F2D-8821-A26E4C94A201", "jb.fireball_swirl"), // Special Effects Skill Pack Transition 01
    ("D9AEBB98-B58B-4EC1-9750-F17BC822E7CB", "jb.cloud_puff_cover"), // Special Effects Skill Transition 03
    ("F2F3C70C-0ADF-41BE-B4EB-B2DDC98CDFD9", "jb.vertical_cutouts"), // Vertical Cutout 1
    ("CA0C9A79-A8F7-4B25-AC48-942275E756A9", "jb.rising_thumb_row"), // Vision Blocks
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_ART3B {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

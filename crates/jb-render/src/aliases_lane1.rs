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
    ("A6E8974E-227F-4F80-8EC0-8F88DC0B8309", "jb.fast_bounce"),
    ("108D84A2-AE20-4A99-A4FD-1EA595812893", "jb.fast_spin_blur"),
    ("71520D52-8F60-4B84-9C43-3C6F34BA37BE", "jb.pink_flash_cut"),
    ("B02B7916-3BC6-454F-B737-856E162130C0", "jb.soft_blur_dissolve"),
    ("BC3CFF30-291A-4995-852D-689CE6A1E1CF", "jb.scanline_smear"),
    ("fa7ae951-4577-ef11-8851-f44efc291984", "jb.ending_fade_black"),
    ("B9DE5D35-4C24-489A-A656-822DB775B1AF", "jb.tinted_dissolve"),
    ("B8437D46-123B-425E-B6E6-C091353AE98D", "jb.warm_wash_dissolve"),
    ("D4A854F2-BCAF-49BA-A160-DDD46CA15655", "jb.card_slide_up"),
    ("47EEB6BF-8E99-488f-BCD8-ABD73A495CB4", "jb.band_close"),
    ("2D9EDF1E-BB2B-4BC1-B455-6FAE3B00BD6F", "jb.laser_band_blue"),
    ("70D8169D-7B37-4864-9E4B-7593C646ECD3", "jb.laser_band_red"),
    ("44ECCE7F-73FE-4815-8C3C-B1E14B73926F", "jb.torn_paper_rise"),
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE1 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

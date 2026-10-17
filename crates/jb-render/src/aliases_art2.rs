//! Filmora store transitions recreated with our own procedural paint, ink
//! and paper artwork (`fx/art2_*.rs`) mapped to their `jb.*` ids. GUIDs keep
//! their exact case (Filmora resolves them case-sensitively). Each GUID is
//! mapped here only (its stand-in alias was removed from the older lane
//! tables).

use crate::registry::Registry;

/// (Filmora GUID, jb id, Filmora name).
pub const FILMORA_ART2: &[(&str, &str, &str)] = &[
    ("3B7844DB-2D95-4136-BE2E-4D59D3FB0CF8", "jb.brush_columns", "Brush Art Design Pack Transition 08"),
    ("b20e8acf-35d7-4958-aebc-64d2358c6b97", "jb.dry_brush_sweeps", "Beauty Brush Transition 1"),
    ("7d98be69-834b-f111-9e87-50ebf65600b6", "jb.brush_drift_three", "Brush Drift Transition 01"),
    ("8298be69-834b-f111-9e87-50ebf65600b6", "jb.diagonal_brush_drift", "Brush Drift Transition 02"),
    ("8198be69-834b-f111-9e87-50ebf65600b6", "jb.rising_brush_drift", "Brush Drift Transition 03"),
    ("7f98be69-834b-f111-9e87-50ebf65600b6", "jb.crossing_brush_drift", "Brush Drift Transition 04"),
    ("7e98be69-834b-f111-9e87-50ebf65600b6", "jb.rounded_brush_drift", "Brush Drift Transition 05"),
    ("8398be69-834b-f111-9e87-50ebf65600b6", "jb.paused_diagonal_drift", "Brush Drift Transition 06"),
    ("8098be69-834b-f111-9e87-50ebf65600b6", "jb.closing_paint_edges", "Brush Drift Transition 07"),
    ("8498be69-834b-f111-9e87-50ebf65600b6", "jb.watercolor_strips", "Brush Drift Transition 08"),
    ("8598be69-834b-f111-9e87-50ebf65600b6", "jb.watercolor_bands", "Brush Drift Transition 09"),
    ("2fe80ebb-e89e-4de9-b669-c9de540d15ea", "jb.swelling_brush_stroke", "Beauty Brush Transition 2"),
    ("c826f80c-db96-f111-b463-d8bbc19410ee", "jb.gold_tide", "Golden Brush Transition 01"),
    ("de3499f9-d196-f111-b463-d8bbc19410ee", "jb.gold_curtain", "Golden Brush Transition 02"),
    ("db3499f9-d196-f111-b463-d8bbc19410ee", "jb.gilded_edge_fall", "Golden Brush Transition 04"),
    ("06868c04-ce96-f111-b463-d8bbc19410ee", "jb.gold_slash_run", "Golden Brush Transition 06"),
    ("05868c04-ce96-f111-b463-d8bbc19410ee", "jb.gilded_torn_edge", "Golden Brush Transition 07"),
    ("df3499f9-d196-f111-b463-d8bbc19410ee", "jb.gold_vortex", "Golden Brush Transition 08"),
    ("8698be69-834b-f111-9e87-50ebf65600b6", "jb.ink_blooms", "Brush Drift Transition 10"),
    ("c726f80c-db96-f111-b463-d8bbc19410ee", "jb.gold_cross_strokes", "Golden Brush Transition 03"),
    ("07868c04-ce96-f111-b463-d8bbc19410ee", "jb.gold_led_columns", "Golden Brush Transition 05"),
    ("9ACD289B-0E2C-4A81-8BFF-687BAEB6FC56", "jb.torn_paper_hole", "Paper Tear Transition 20"),
    ("0EB21BC5-6121-40A3-987A-217661816A98", "jb.torn_paper_rift", "Torn Paper Transition 09"),
    ("9C246287-A006-4F7D-A161-BAB117D39BAB", "jb.paper_roll_peel", "Grunge Scribble Vlog Transition 02"),
];

pub fn register(r: &mut Registry) {
    for (fid, to, _) in FILMORA_ART2 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

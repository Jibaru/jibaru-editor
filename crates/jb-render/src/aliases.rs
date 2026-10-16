//! Alias table: alternative names and imported identifiers
//! (`filmora:<id>`, prototype `ae:<id>`) mapped onto built-in effects.

use crate::registry::Registry;

/// Short/legacy names for built-ins.
const NAMES: &[(&str, &str)] = &[
    ("jb.cross_fade", "jb.dissolve"),
    ("jb.crossfade", "jb.dissolve"),
    ("jb.fade_white", "jb.dip_white"),
    ("jb.blur.gaussian", "jb.gaussian_blur"),
    ("jb.blur.motion", "jb.motion_blur"),
    ("jb.blur.directional", "jb.motion_blur"),
    ("jb.blur.radial", "jb.radial_blur"),
    ("jb.blur.zoom", "jb.radial_blur"),
    ("jb.vignette", "jb.vineta"),
    ("jb.film_grain", "jb.grano_pelicula"),
    ("jb.old_film", "jb.pelicula_antigua"),
    ("jb.shake", "jb.handheld_shake"),
    ("jb.invert", "jb.negativo"),
    ("jb.black_white", "jb.blanco_negro"),
];

/// Filmora resource ids found in the user's projects (docs/filmora-coverage.md)
/// mapped to our implementations. Kinds arrive as `filmora:<id>`.
pub const FILMORA: &[(&str, &str)] = &[
    // Transitions
    ("8D02CB83-6FC6-406B-8506-252F58A1D3B3", "jb.fast_zoom"),
    ("C8965C45-074B-4BF5-948E-D9373D10836C", "jb.wipe_left"),
    ("3B49DBEE-9A15-4844-B613-E2C2497EA236", "jb.fade_black"),
    ("2981D185-D52E-44f4-ABD5-3CE83890E32E", "jb.dissolve"),
    ("38B7DD62-E852-4B41-81E1-80901511721D", "jb.pan_slide"),
    ("521158CA-2709-4FB1-8C3C-29781494BE1F", "jb.film_burn_02"),
    // Other store transitions cached locally (calibrated on their previews).
    ("2DF0B554-780C-455C-921C-E83F62B59FD8", "jb.fast_wipe_right"),
    ("AF3B9CF3-CB2F-4FC4-A9C7-6A1A9CA8B5B7", "jb.blur_dissolve"),
    ("A679E80D-8647-4028-A366-6BFFEE34715A", "jb.cross_zoom"),
    // Audio cross-fade: if it ever reaches the video path, a dissolve is right.
    ("audio/blender/transition-fade", "jb.dissolve"),
    // Effects
    ("662E16ED-4524-4D13-AAE9-11DBA0C63E17", "jb.filmora.adjust_color"),
    ("45D9AD95-4F03-4B2F-A8C7-8F7817B73FA4", "jb.auto_enhance"),
];

pub fn register(r: &mut Registry) {
    for (from, to) in NAMES {
        if r.get(to).is_some() {
            r.alias(*from, to);
        }
    }
    for (fid, to) in FILMORA {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            // Filmora ids are case-insensitive GUIDs.
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
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

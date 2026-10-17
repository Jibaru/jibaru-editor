//! The art1 items (light leaks, flares, film burns, bokeh, sparkles, particles,
//! glows, fire and energy overlays): metadata of every item; the artwork is in
//! `shaders/art1/<file>.wgsl` (see `art1_kit.rs` and docs/fidelity-art1.md).

use super::art1_kit::{register_all, ArtSpec};
use crate::registry::{FxKind, Registry};

macro_rules! art {
    ($id:literal, $name:literal, $cat:literal, $kind:ident, $file:literal, $desc:literal) => {
        ArtSpec {
            id: $id,
            name: $name,
            category: $cat,
            kind: FxKind::$kind,
            description: $desc,
            file: $file,
            wgsl: include_str!(concat!("../shaders/art1/", $file, ".wgsl")),
        }
    };
}

/// Library group of the art1 transitions (`library.rs`).
pub const LIB_TRANSITIONS: &[&str] = &[
    "jb.cyan_comet_glitter",
    "jb.pink_sparkler_rise",
    "jb.glitter_swirl",
    "jb.golden_comet_arc",
    "jb.twin_comet_burst",
    "jb.cyan_splash_wipe",
    "jb.ember_flash_leak",
    "jb.peach_orb_leak",
    "jb.sunlit_haze_leak",
    "jb.golden_bokeh_band",
    "jb.glow_ink_rise",
    "jb.flaming_ball_dash_1",
    "jb.flaming_ball_dash_2",
    "jb.flaming_ball_dash_3",
    "jb.flaming_ball_dash_4",
    "jb.gold_bokeh_beam",
];

/// Library group of the art1 effects (`library.rs`).
pub const LIB_EFFECTS: &[&str] = &[
    "jb.fast_light_leaks",
    "jb.focus_pull_bokeh",
    "jb.cine_film_burn",
    "jb.cine_film_gate_sun",
    "jb.cine_light_bands",
    "jb.edge_glow_pulse",
    "jb.flash_tint_leak",
    "jb.cine_light_rainbow",
    "jb.cine_scribble_leak",
    "jb.cine_scribble_aurora",
    "jb.cine_paint_ring",
];

/// Every art1 item.
pub fn specs() -> Vec<ArtSpec> {
    vec![
        art!("jb.cyan_comet_glitter", "Cyan Comet Glitter", "Light", Transition, "cyan_comet_glitter", "A white-hot comet lays a cyan glow band across the frame; the new clip appears along it under drifting glitter."),
        art!("jb.pink_sparkler_rise", "Pink Sparkler Rise", "Light", Transition, "pink_sparkler_rise", "A sparkler zig-zags across the frame trailing white sparks and pink bokeh; the new clip sweeps in behind it."),
        art!("jb.glitter_swirl", "Glitter Swirl", "Light", Transition, "glitter_swirl", "A spark winds a widening vortex of white glitter; the new clip opens from its centre as the glitter scatters."),
        art!("jb.golden_comet_arc", "Golden Comet Arc", "Light", Transition, "golden_comet_arc", "A golden comet loops over the frame leaving a glitter wake that breaks into warm bokeh and reveals the new clip."),
        art!("jb.twin_comet_burst", "Twin Comet Burst", "Light", Transition, "twin_comet_burst", "Two golden comets race along the diagonal, cross and burst into amber bokeh that clears on the new clip."),
        art!("jb.cyan_splash_wipe", "Cyan Splash Wipe", "Creative", Transition, "cyan_splash_wipe", "Jagged strokes of ice-blue paint slash over the frame, then tear away diagonally to uncover the new clip."),
        art!("jb.ember_flash_leak", "Ember Flash Leak", "Light", Transition, "ember_flash_leak", "A warm leak bleaches the frame white, contracts to a hot spot and burns through ember red to the new clip."),
        art!("jb.peach_orb_leak", "Peach Orb Leak", "Light", Transition, "peach_orb_leak", "Big out-of-focus orange orbs drift in and melt into a cream flash that clears on the new clip."),
        art!("jb.sunlit_haze_leak", "Sunlit Haze Leak", "Light", Transition, "sunlit_haze_leak", "A low sun throws a diagonal shaft of light over the old clip while the new clip rises from below."),
        art!("jb.golden_bokeh_band", "Golden Bokeh Band", "Light", Transition, "golden_bokeh_band", "A band of golden light full of bokeh sweeps left to right, revealing the new clip."),
        art!("jb.fast_light_leaks", "Fast Light Leaks", "Light", Effect, "fast_light_leaks", "A short burst of burnt film light: churning cream flares and maroon smoke that bleach the picture and thin out."),
        art!("jb.focus_pull_bokeh", "Focus Pull Bokeh", "Light", Effect, "focus_pull_bokeh", "The lens racks out of focus and back while the highlights swell into glowing discs."),
        art!("jb.cine_film_burn", "Cine Film Burn", "Light", Effect, "cine_film_burn", "A reel of burning film flickers over the picture: orange and white burns, a red flash-back and sprocket holes."),
        art!("jb.cine_film_gate_sun", "Cine Film Gate Sun", "Light", Effect, "cine_film_gate_sun", "The picture becomes a film frame in a projector gate with a burning sun crescent, a swelling burn and a frame roll."),
        art!("jb.cine_light_bands", "Cine Light Bands", "Light", Effect, "cine_light_bands", "Strobing vertical bands of white, yellow and orange stage light flash across the picture."),
        art!("jb.edge_glow_pulse", "Edge Glow Pulse", "Light", Effect, "edge_glow_pulse", "A soft band sweeps across the picture again and again, lighting its edges up as glowing white lines."),
        art!("jb.flash_tint_leak", "Flash Tint Leak", "Light", Effect, "flash_tint_leak", "An aged, slightly darker print with a soft light leak washing over it: lavender, cream, peach and a red burn."),
        art!("jb.cine_light_rainbow", "Cine Light Rainbow", "Light", Effect, "cine_light_rainbow", "An old-cinema print look with a pink defocus bloom and a late streaky rainbow flare down the left of the frame."),
        art!("jb.cine_scribble_leak", "Cine Scribble Leak", "Light", Effect, "cine_scribble_leak", "Hand-drawn loops of cream light scrawl across the picture between pink washes, a dark smeared band and a rainbow flare."),
        art!("jb.cine_scribble_aurora", "Cine Scribble Aurora", "Light", Effect, "cine_scribble_aurora", "Pink light washes, glowing film-edge smears, big hand-drawn loops of cream light and rippling aurora curtains."),
        art!("jb.cine_paint_ring", "Cine Paint Ring", "Light", Effect, "cine_paint_ring", "A magenta flash, a splash of white paint, a bleached pastel print with a rough ring of dark paint, and a streaked white flare."),
        art!("jb.glow_ink_rise", "Glow Ink Rise", "Creative", Transition, "glow_ink_rise", "The new clip wells up from the bottom like soaking ink, its ragged front burning with a thin white glow."),
        art!("jb.flaming_ball_dash_1", "Flaming Ball Dash 1", "Creative", Transition, "flaming_ball_dash_1", "A ball trailing a long tail of fire swells towards the camera until it fills the frame, then whips away to the new clip."),
        art!("jb.flaming_ball_dash_2", "Flaming Ball Dash 2", "Creative", Transition, "flaming_ball_dash_2", "A ball bounces up from the lower left and rushes at the camera until it fills the frame, cutting to the new clip."),
        art!("jb.flaming_ball_dash_3", "Flaming Ball Dash 3", "Creative", Transition, "flaming_ball_dash_3", "A small ball wrapped in fire streaks in from the distance and grows until it fills the frame, cutting to the new clip."),
        art!("jb.flaming_ball_dash_4", "Flaming Ball Dash 4", "Creative", Transition, "flaming_ball_dash_4", "A ball flies straight at the camera in a ring of kicked-up dust until it fills the frame, then cuts to the new clip."),
        art!("jb.gold_bokeh_beam", "Gold Bokeh Beam", "Light", Transition, "gold_bokeh_beam", "A star flares and a thin beam drops through the frame, which floods with golden bokeh while a bright outline tumbles past."),
    ]
}

pub fn register(r: &mut Registry) {
    let mut all = specs();
    all.extend(super::art1_kit::dev_specs(&all));
    register_all(r, &all);
}

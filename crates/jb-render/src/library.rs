//! Where registry items sit in the app's library, Filmora style: the
//! sidebar groups of the Transitions, Effects and Filters tabs (Filmora's
//! group names) and the items listed under each, in display order.
//!
//! An item can be listed in several groups (Filmora's "Trending" repeats
//! items of other groups). Items of those kinds that are in no group are
//! not shown in the library: properties-panel effects (color, masks, ...)
//! and helpers for imported projects.

use crate::registry::FxKind;

/// One sidebar group of a library tab.
#[derive(Debug, Clone, Copy)]
pub struct LibraryGroup {
    pub kind: FxKind,
    pub name: &'static str,
    /// Registry ids, in display order.
    pub items: &'static [&'static str],
}

const fn g(kind: FxKind, name: &'static str, items: &'static [&'static str]) -> LibraryGroup {
    LibraryGroup { kind, name, items }
}

use FxKind::{Effect as E, Filter as F, Transition as T};

/// Every group, tab by tab, in sidebar order.
pub const GROUPS: &[LibraryGroup] = &[
    // Transitions
    g(
        T,
        "Trending",
        &[
            "jb.dissolve",
            "jb.blur_dissolve",
            "jb.fade_black",
            "jb.smooth_dissolve",
            "jb.cross_zoom",
            "jb.white_flash",
            "jb.push_left",
            "jb.whip_right",
            "jb.zoom_in",
            "jb.glitch_digital",
            "jb.light_leak",
            "jb.wipe_iris_in",
            "jb.spin_zoom",
            "jb.slide",
        ],
    ),
    g(
        T,
        "Fade&Dissolve",
        &[
            "jb.dissolve",
            "jb.smooth_dissolve",
            "jb.blur_dissolve",
            "jb.zoom_dissolve",
            "jb.luma_dissolve",
            "jb.additive_dissolve",
            "jb.fade_black",
            "jb.fade_color",
            "jb.dip_white",
            "jb.dip_gray",
        ],
    ),
    g(
        T,
        "Zoom",
        &[
            "jb.fast_zoom",
            "jb.zoom_in",
            "jb.zoom_out",
            "jb.cross_zoom",
            "jb.zoom_dissolve",
            "jb.radial_zoom_blur",
            "jb.spin_zoom",
        ],
    ),
    g(
        T,
        "Slides",
        &[
            "jb.slide",
            "jb.slide_from_left",
            "jb.slide_from_top",
            "jb.slide_from_bottom",
            "jb.push_left",
            "jb.push_right",
            "jb.push_up",
            "jb.push_down",
            "jb.wipe_left",
        ],
    ),
    g(
        T,
        "Premium",
        &[
            "jb.glitch_digital",
            "jb.rgb_split",
            "jb.vhs_rewind",
            "jb.film_burn",
            "jb.light_leak",
            "jb.ripple",
            "jb.spin_zoom",
            "jb.cross_zoom",
            "jb.whip_right",
            "jb.wipe_spiral",
        ],
    ),
    g(T, "3D", &["jb.spin", "jb.spin_zoom", "jb.squeeze_h", "jb.squeeze_v", "jb.bounce_in"]),
    g(
        T,
        "Film Burn",
        &["jb.film_burn", "jb.light_leak", "jb.warm_flash", "jb.overexposure", "jb.additive_dissolve", "jb.flicker"],
    ),
    g(
        T,
        "Speed Blur",
        &["jb.whip_right", "jb.whip_up", "jb.whip_down", "jb.radial_zoom_blur", "jb.fast_zoom", "jb.wipe_left"],
    ),
    g(T, "Camera Moves", crate::fx::lane1_tables::LIB_CAMERA_MOVES),
    g(T, "Soft Wipes", crate::fx::lane1_tables::LIB_SOFT_WIPES),
    g(T, "Flash & Fade", crate::fx::lane1_tables::LIB_FLASH_FADE),
    g(T, "Overlay Washes", crate::fx::lane1_tables::LIB_OVERLAY_WASHES),
    g(T, "3D Turns", crate::fx::lane2_layers::LIB_3D),
    g(T, "Slides & Rolls", crate::fx::lane2_layers::LIB_SLIDES),
    g(
        T,
        "Glitch",
        &[
            "jb.glitch_digital",
            "jb.rgb_split",
            "jb.pixelate",
            "jb.mosaic",
            "jb.vhs_rewind",
            "jb.negative_cut",
            "jb.camera_shake",
        ],
    ),
    g(
        T,
        "Light",
        &["jb.white_flash", "jb.flicker", "jb.strobe", "jb.warm_flash", "jb.overexposure", "jb.negative_cut"],
    ),
    g(T, "Warp", &["jb.ripple", "jb.pixelate", "jb.mosaic", "jb.squeeze_h", "jb.camera_shake"]),
    g(
        T,
        "Wipe",
        &[
            "jb.wipe_to_left",
            "jb.wipe_to_right",
            "jb.wipe_up",
            "jb.wipe_down",
            "jb.wipe_diagonal_down",
            "jb.wipe_diagonal_up",
            "jb.wipe_soft",
            "jb.wipe_barn_door_h",
            "jb.wipe_barn_door_v",
            "jb.wipe_blinds_h",
            "jb.wipe_blinds_v",
            "jb.wipe_clock",
            "jb.wipe_wavy",
            "jb.wipe_zigzag",
        ],
    ),
    g(
        T,
        "Shape",
        &[
            "jb.wipe_iris_in",
            "jb.wipe_iris_out",
            "jb.wipe_box",
            "jb.wipe_diamond",
            "jb.wipe_checkerboard",
            "jb.wipe_diamonds",
            "jb.wipe_heart",
            "jb.wipe_star",
            "jb.wipe_spiral",
        ],
    ),
    // Effects ("Video Effects")
    g(
        E,
        "Trending",
        &[
            "jb.pixel_mosaic",
            "jb.gaussian_blur",
            "jb.vhs",
            "jb.glitch",
            "jb.chromatic_aberration",
            "jb.handheld_shake",
            "jb.slow_zoom",
            "jb.pelicula_antigua",
            "jb.fuga_luz",
            "jb.barras_cine",
            "jb.kaleidoscope",
            "jb.tilt_shift",
        ],
    ),
    g(
        E,
        "Basic",
        &[
            "jb.gaussian_blur",
            "jb.blur.box",
            "jb.sharpen",
            "jb.pixel_mosaic",
            "jb.vineta",
            "jb.barras_cine",
            "jb.blanco_negro",
            "jb.negativo",
            "jb.mirror",
            "jb.flip_horizontal",
            "jb.flip_vertical",
            "jb.background_blur",
        ],
    ),
    g(E, "Zoom", &["jb.slow_zoom", "jb.zoom_pulse", "jb.ken_burns_pan", "jb.fisheye", "jb.radial_blur"]),
    g(
        E,
        "Shake",
        &["jb.handheld_shake", "jb.zoom_pulse", "jb.glitch", "jb.destello_estroboscopico", "jb.chromatic_aberration"],
    ),
    g(
        E,
        "Multi-Frame",
        &[
            "jb.mirror",
            "jb.mirror_horizontal",
            "jb.mirror_vertical",
            "jb.kaleidoscope",
            "jb.flip_horizontal",
            "jb.flip_vertical",
        ],
    ),
    g(
        E,
        "Blur",
        &["jb.gaussian_blur", "jb.blur.box", "jb.background_blur", "jb.tilt_shift", "jb.motion_blur", "jb.radial_blur"],
    ),
    g(E, "Distortion", &["jb.fisheye", "jb.water_ripple", "jb.pixel_mosaic", "jb.chromatic_aberration", "jb.emboss"]),
    g(E, "Glitch", &["jb.glitch", "jb.chromatic_aberration", "jb.pixel_retro", "jb.vhs", "jb.destello_estroboscopico"]),
    g(
        E,
        "Retro",
        &["jb.vhs", "jb.sepia", "jb.blanco_negro", "jb.negativo", "jb.pixel_retro", "jb.super8", "jb.posterizar"],
    ),
    g(E, "Film", &["jb.pelicula_antigua", "jb.grano_pelicula", "jb.polvo_rayones", "jb.super8", "jb.barras_cine"]),
    g(E, "Light", &["jb.vineta", "jb.fuga_luz", "jb.destello_estroboscopico", "jb.neon_bordes"]),
    g(
        E,
        "Artistic",
        &["jb.sketch", "jb.charcoal", "jb.emboss", "jb.posterizar", "jb.comic", "jb.neon_bordes", "jb.sharpen"],
    ),
    g(E, "Frame", &["jb.barras_cine", "jb.marco_polaroid", "jb.sombra_interior", "jb.marco_redondeado"]),
    // Filters
    g(
        F,
        "Basic",
        &[
            "jb.vivido",
            "jb.teal_orange",
            "jb.cine_calido",
            "jb.cine_frio",
            "jb.matte",
            "jb.pastel",
            "jb.bn_suave",
            "jb.atardecer",
            "jb.hora_azul",
            "jb.kodak",
        ],
    ),
    g(F, "Mono", &["jb.bn_alto_contraste", "jb.bn_suave", "jb.noir", "jb.selenio"]),
    g(
        F,
        "Cinematic",
        &[
            "jb.teal_orange",
            "jb.cine_frio",
            "jb.cine_calido",
            "jb.moody",
            "jb.matte",
            "jb.bleach_bypass",
            "jb.kodak",
            "jb.fuji",
        ],
    ),
    g(F, "Portrait", &["jb.retrato_suave", "jb.pastel", "jb.vivido", "jb.sepia_suave"]),
    g(F, "Summer", &["jb.atardecer", "jb.dorado", "jb.tropical", "jb.playa"]),
    g(F, "Lifestyle", &["jb.bosque", "jb.neon_noche", "jb.hora_azul", "jb.nordico", "jb.hielo", "jb.moody"]),
    g(F, "Retro", &["jb.vintage_70s", "jb.retro_90s", "jb.polaroid", "jb.sepia_suave"]),
    g(F, "Travel", &["jb.tropical", "jb.bosque", "jb.playa", "jb.neon_noche", "jb.nordico"]),
    // Lane 4: fitted looks, camera effects and reveal transitions.
    g(F, "Fitted Looks", crate::fx::lane4_all::looks::LIB_IDS),
    g(T, "Ink & Brush", crate::fx::lane4_all::reveal::LIB_IDS),
    g(T, "Zoom Warps", crate::fx::lane4_all::camtr::LIB_IDS),
    g(E, "Camera Motion", crate::fx::lane4_all::camera::LIB_IDS),
    g(E, "Stylized", crate::fx::lane4_all::LIB_STYLIZED),
    g(E, "Fitted Effects", crate::fx::lane4_all::looks::LIB_FX),
];

/// [`GROUPS`] followed by the groups contributed by fitted item sets.
pub fn all_groups() -> impl Iterator<Item = &'static LibraryGroup> {
    GROUPS.iter().chain(crate::fx::lane3::LIBRARY.iter())
}

/// The groups listing `id`, with its position in each.
pub fn places(id: &str) -> impl Iterator<Item = (&'static LibraryGroup, usize)> + '_ {
    all_groups().filter_map(move |g| g.items.iter().position(|i| *i == id).map(|p| (g, p)))
}

/// Group names of a tab (`kind`), in sidebar order.
pub fn group_names(kind: FxKind) -> impl Iterator<Item = &'static str> {
    all_groups().filter(move |g| g.kind == kind).map(|g| g.name)
}

//! Comic and cartoon transitions: original comic-page layouts and cartoon
//! artwork, keyframed on camera moves and shapes measured on Filmora's
//! exports.

use super::{art_item, art_wgsl, at_frames, tab};
use crate::fx::lane3::kit::Item;

/// Comic Style 3 (Filmora): camera over our comic page per reference frame
/// (`[frame, zoom, offset x, offset y]`), measured from the panel rectangles.
const COMIC_PAGE: &[[f32; 4]] = &[
    [0.0, 1.0, 0.0, 0.0],
    [2.0, 1.0, 0.0, 0.0],
    [3.0, 0.989, 0.0105, 0.0035],
    [4.0, 0.962, 0.031, 0.015],
    [5.0, 0.926, 0.054, 0.0295],
    [6.0, 0.89, 0.077, 0.044],
    [7.0, 0.8385, 0.112, 0.0635],
    [8.0, 0.787, 0.145, 0.085],
    [9.0, 0.7365, 0.177, 0.1085],
    [10.0, 0.69, 0.207, 0.128],
    [11.0, 0.6495, 0.2345, 0.1425],
    [12.0, 0.621, 0.252, 0.159],
    [13.0, 0.5965, 0.265, 0.171],
    [14.0, 0.582, 0.276, 0.177],
    [15.0, 0.581, 0.2705, 0.1785],
    [16.0, 0.584, 0.247, 0.178],
    [17.0, 0.588, 0.207, 0.1765],
    [18.0, 0.59, 0.157, 0.175],
    [19.0, 0.59, 0.096, 0.1755],
    [20.0, 0.596, 0.025, 0.176],
    [21.0, 0.5975, -0.0475, 0.176],
    [22.0, 0.601, -0.126, 0.174],
    [23.0, 0.6065, -0.212, 0.173],
    [24.0, 0.59, -0.28, 0.17],
    [26.0, 0.59, -0.43, 0.17],
    [28.0, 0.59, -0.57, 0.17],
    [30.0, 0.59, -0.63, 0.17],
    [32.0, 0.59, -0.63, 0.19],
    [34.0, 0.59, -0.63, 0.26],
    [36.0, 0.59, -0.63, 0.35],
    [38.0, 0.59, -0.63, 0.45],
    [40.0, 0.59, -0.63, 0.56],
    [42.0, 0.59, -0.63, 0.68],
    [44.0, 0.59, -0.63, 0.76],
    [46.0, 0.59, -0.63, 0.82],
    [48.0, 0.59, -0.63, 0.85],
    [49.0, 0.6205, -0.6845, 0.874],
    [50.0, 0.659, -0.755, 0.895],
    [51.0, 0.705, -0.844, 0.9235],
    [52.0, 0.756, -0.946, 0.955],
    [53.0, 0.8045, -1.0445, 0.9885],
    [54.0, 0.851, -1.141, 1.019],
    [55.0, 0.886, -1.2085, 1.045],
    [56.0, 0.916, -1.267, 1.06],
    [57.0, 0.9226, -1.2803, 1.0632],
    [57.5, 0.92593, -1.287, 1.0648],
    [58.0, 0.9259, -1.287, 1.0648],
    [60.0, 0.92593, -1.287, 1.0648],
];

/// Cartoony Vlog Pack Transition 01 (Filmora): radii measured on the
/// coverage of the spiral around the centre (`[frame, cover, hole, outer,
/// spin | gem, blob x, blob y, blob scale]`).
const CANDY_SPIRAL: &[[f32; 9]] = &[
    [0.0, 0.0, 0.0, 2.0, 0.68, 0.0, 0.1, 1.2, 0.0],
    [3.0, 0.0, 0.0, 2.0, 0.68, 0.0, 0.1, 1.2, 0.0],
    [3.5, 0.05, 0.0, 2.0, 0.68, 0.05, 0.1, 1.2, 0.3],
    [4.0, 0.14, 0.0, 2.0, 0.68, 0.07, 0.15, 1.05, 0.35],
    [5.0, 0.2, 0.0, 2.0, 0.68, 0.09, 0.18, 0.9, 0.4],
    [6.0, 0.55, 0.0, 2.0, 0.68, 0.1, 0.12, 0.85, 0.45],
    [7.0, 0.6, 0.0, 2.0, 0.76, 0.1, 0.05, 0.8, 0.5],
    [8.0, 0.75, 0.0, 2.0, 0.88, 0.11, 0.05, 0.55, 0.5],
    [10.0, 1.15, 0.0, 2.0, 1.08, 0.12, 0.25, 0.05, 0.45],
    [14.0, 1.15, 0.0, 2.0, 1.32, 0.12, 0.45, 0.05, 0.4],
    [16.0, 1.15, 0.0, 2.0, 1.28, 0.13, 0.85, 0.5, 0.6],
    [17.0, 1.15, 0.0, 2.0, 1.32, 0.15, 0.95, 0.6, 0.6],
    [18.0, 1.15, 0.2, 2.0, 1.28, 0.18, 1.2, 0.6, 0.6],
    [19.0, 1.15, 0.36, 2.0, 1.32, 0.2, 1.3, 0.6, 0.0],
    [21.0, 1.15, 0.36, 2.0, 1.36, 0.24, 1.3, 0.6, 0.0],
    [22.0, 1.15, 0.42, 2.0, 1.28, 0.26, 1.3, 0.6, 0.0],
    [23.0, 1.15, 0.5, 2.0, 1.48, 0.0, 1.3, 0.6, 0.0],
    [24.0, 1.15, 0.6, 1.3, 1.4, 0.0, 1.3, 0.6, 0.0],
    [26.0, 1.15, 0.64, 0.94, 1.64, 0.0, 1.3, 0.6, 0.0],
    [28.0, 1.15, 0.64, 0.86, 1.52, 0.0, 1.3, 0.6, 0.0],
    [29.0, 1.15, 0.63, 0.8, 1.6, 0.0, 1.3, 0.6, 0.0],
    [30.0, 1.15, 0.63, 0.74, 1.96, 0.0, 1.3, 0.6, 0.0],
    [31.0, 1.15, 0.63, 0.7, 2.0, 0.0, 1.3, 0.6, 0.0],
    [32.0, 1.15, 0.63, 0.67, 2.04, 0.0, 1.3, 0.6, 0.0],
    [34.0, 1.15, 0.63, 0.65, 2.2, 0.0, 1.3, 0.6, 0.0],
    [36.0, 1.15, 0.63, 0.62, 2.12, 0.0, 1.3, 0.6, 0.0],
    [60.0, 1.15, 0.63, 0.62, 2.12, 0.0, 1.3, 0.6, 0.0],
];

/// Animate Effect Transition 03 (Filmora): blob radius and splat size per
/// reference frame, the splat fitted to the uncovered area of each frame
/// (`[frame, blob, hole]`).
const COMIC_BURST: &[[f32; 3]] = &[
    [0.0, 0.0, 0.0],
    [16.5, 0.0, 0.0],
    [17.0, 0.75, 0.0],
    [17.5, 1.3, 0.0],
    [18.5, 1.3, 0.0],
    [18.5, 1.3, 0.06],
    [21.0, 1.3, 0.09],
    [22.0, 1.3, 0.3],
    [23.0, 1.3, 0.94],
    [24.0, 1.3, 1.21],
    [25.0, 1.3, 1.69],
    [26.0, 1.3, 2.15],
    [27.5, 1.3, 2.3],
    [28.0, 1.3, 2.73],
    [29.0, 1.3, 3.17],
    [30.0, 1.3, 3.39],
    [32.0, 1.3, 4.5],
    [60.0, 1.3, 8.0],
];

pub fn items() -> Vec<Item> {
    vec![
        art_item(
            "jb.comic_burst",
            "Comic Burst",
            "Comic",
            "A pale paper blob balloons over the clip and fills the frame, then the next clip bursts through it as a spiky splat that blows the paper apart.",
            art_wgsl("", &[tab("cb", &at_frames(COMIC_BURST))], include_str!("../shaders/art3a/comic_burst.wgsl")),
        ),
        art_item(
            "jb.candy_swirl",
            "Candy Swirl",
            "Cartoon",
            "A striped candy spiral winds out over the clip around a spinning mint gem while a spotted cartoon blob bounces past, then unwinds from the centre onto the next clip.",
            art_wgsl("", &[tab("cs", &at_frames(CANDY_SPIRAL))], include_str!("../shaders/art3a/candy_spiral.wgsl")),
        ),
        art_item(
        "jb.comic_page",
        "Comic Page",
        "Comic",
        "The camera pulls back until the clip is one panel of a comic page, pans across the page to the panel holding the next clip and pushes into it.",
        art_wgsl("", &[tab("cp", &at_frames(COMIC_PAGE))], include_str!("../shaders/art3a/comic_page.wgsl")),
        ),
    ]
}

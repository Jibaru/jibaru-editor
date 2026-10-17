//! Halloween cartoon transitions drawn as original vector artwork:
//! jack-o'-lanterns, witch hats, skulls, bats and ghosts. Timing, motion
//! paths and coverage follow Filmora's exports of the matching items.

use super::{art_item, art_wgsl, at_frames, tab};
use crate::fx::lane3::kit::Item;

/// Halloween Colorful Transition 04 (Filmora): the camera pulls back out of
/// a jack-o'-lantern's mouth (frames 10-24), holds, the jaw shuts (36-40)
/// and the head flies up (46-55) over the incoming clip, which shows at
/// 87 % brightness until the end. Rows `[frame, scale, dy, jaw, side, gain B, -, -]`.
const PUMPKIN_GRIN: &[[f32; 8]] = &[
    [0.0, 8.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [10.0, 8.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [11.0, 7.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [12.0, 6.1, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [13.0, 5.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [14.0, 4.2, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [15.0, 4.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [16.0, 3.2, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [17.0, 2.2, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [18.0, 1.24, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [20.0, 1.06, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [22.0, 1.02, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [24.0, 1.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [36.0, 1.0, 0.0, 0.0, 0.0, 0.87, 0.0, 0.0],
    [39.5, 1.0, 0.05, 0.9, 0.0, 0.87, 0.0, 0.0],
    [39.5, 1.0, 0.05, 0.9, 1.0, 0.87, 0.0, 0.0],
    [40.0, 1.0, 0.06, 1.0, 1.0, 0.87, 0.0, 0.0],
    [46.0, 1.0, 0.06, 1.0, 1.0, 0.87, 0.0, 0.0],
    [47.0, 1.0, -0.11, 1.0, 1.0, 0.87, 0.0, 0.0],
    [48.0, 1.0, -0.26, 1.0, 1.0, 0.87, 0.0, 0.0],
    [49.0, 1.0, -0.51, 1.0, 1.0, 0.87, 0.0, 0.0],
    [50.0, 1.0, -0.73, 1.0, 1.0, 0.87, 0.0, 0.0],
    [51.0, 1.0, -0.75, 1.0, 1.0, 0.87, 0.0, 0.0],
    [52.0, 1.0, -0.86, 1.0, 1.0, 0.87, 0.0, 0.0],
    [53.0, 1.0, -0.94, 1.0, 1.0, 0.87, 0.0, 0.0],
    [54.0, 1.0, -1.01, 1.0, 1.0, 0.87, 0.0, 0.0],
    [55.0, 1.0, -1.4, 1.0, 1.0, 0.87, 0.0, 0.0],
    [60.0, 1.0, -1.4, 1.0, 1.0, 0.87, 0.0, 0.0],
];

const SPRITES: &str = include_str!("../shaders/art3a/sprites.wgsl");

/// Halloween Colorful Transition 09 (Filmora): the hat drops to the centre
/// (frames 5-13), bobs there, slides off to the bottom-left corner (30-38)
/// and returns at frame 43 riding a wavy flood of the incoming clip that
/// covers the frame by frame 52; B shows at 87 %. Rows
/// `[frame, x, y, rot, scale, flood right, flood top, gain B, -]`.
const HAT_WAVE: &[[f32; 9]] = &[
    [0.0, 0.52, -0.25, 0.0, 0.32, -0.3, 1.2, 0.87, 0.0],
    [5.0, 0.52, -0.2, 0.0, 0.32, -0.3, 1.2, 0.87, 0.0],
    [9.0, 0.52, 0.3, -0.1, 0.32, -0.3, 1.2, 0.87, 0.0],
    [13.0, 0.52, 0.52, -0.2, 0.32, -0.3, 1.2, 0.87, 0.0],
    [16.0, 0.52, 0.5, -0.2, 0.32, -0.3, 1.2, 0.87, 0.0],
    [22.0, 0.52, 0.51, -0.3, 0.32, -0.3, 1.2, 0.87, 0.0],
    [30.0, 0.52, 0.52, -0.25, 0.32, -0.3, 1.2, 0.87, 0.0],
    [34.0, 0.36, 0.66, -0.35, 0.32, -0.3, 1.2, 0.87, 0.0],
    [38.0, 0.08, 0.93, -0.5, 0.32, -0.3, 1.2, 0.87, 0.0],
    [42.0, 0.06, 0.94, -0.5, 0.32, -0.3, 1.2, 0.87, 0.0],
    [43.0, 0.08, 0.92, -0.4, 0.32, 0.1, 0.92, 0.87, 0.0],
    [44.0, 0.16, 0.86, -0.3, 0.32, 0.2, 0.85, 0.87, 0.0],
    [46.0, 0.27, 0.74, -0.2, 0.32, 0.27, 0.74, 0.87, 0.0],
    [47.0, 0.38, 0.62, -0.1, 0.32, 0.38, 0.6, 0.87, 0.0],
    [48.0, 0.56, 0.45, 0.0, 0.32, 0.57, 0.45, 0.87, 0.0],
    [49.0, 0.66, 0.36, 0.1, 0.32, 0.68, 0.35, 0.87, 0.0],
    [50.0, 0.9, 0.12, 0.2, 0.32, 0.95, 0.07, 0.87, 0.0],
    [51.0, 0.97, 0.06, 0.2, 0.32, 0.97, 0.06, 0.87, 0.0],
    [52.0, 1.3, -0.3, 0.3, 0.32, 1.3, -0.3, 0.87, 0.0],
    [60.0, 1.3, -0.3, 0.3, 0.32, 1.3, -0.3, 0.87, 0.0],
];

/// Halloween Colorful Transition 21 (Filmora): the bat's wing tips enter
/// at the bottom on frame 10 and it rises at a steady 0.059 frame heights
/// per frame until it has left (frame 40); B shows at 87 %. Rows
/// `[frame, tip v, gain B, -, -]`.
const BAT_RISE: &[[f32; 5]] = &[
    [0.0, 1.55, 0.87, 0.0, 0.0],
    [10.0, 0.96, 0.87, 0.0, 0.0],
    [26.0, 0.02, 0.87, 0.0, 0.0],
    [38.0, -0.82, 0.87, 0.0, 0.0],
    [60.0, -0.82, 0.87, 0.0, 0.0],
];

/// Halloween Colorful Transition 17 (Filmora): the skull peeks in at the
/// left (frames 5-17), then sweeps right along the lower half (to frame 46)
/// while the hat crosses the upper half from the right (frames 11-44); the
/// incoming clip (at 87 %) follows each. Rows
/// `[frame, skull x, skull y, hat x, hat y, lower front, upper front, gain B, hat rot]`.
const SKULL_HAT: &[[f32; 9]] = &[
    [0.0, -0.2, 0.6, 1.4, 0.15, -0.1, 1.1, 0.87, -0.3],
    [6.0, -0.03, 0.58, 1.35, 0.15, -0.1, 1.1, 0.87, -0.3],
    [14.0, 0.05, 0.62, 1.1, 0.18, -0.1, 1.1, 0.87, -0.25],
    [17.0, 0.07, 0.63, 0.98, 0.2, 0.0, 1.0, 0.87, -0.2],
    [18.0, 0.16, 0.63, 0.92, 0.22, 0.06, 1.0, 0.87, -0.2],
    [20.0, 0.24, 0.64, 0.79, 0.25, 0.14, 0.91, 0.87, -0.15],
    [22.0, 0.28, 0.64, 0.73, 0.26, 0.18, 0.85, 0.87, -0.12],
    [24.0, 0.37, 0.65, 0.63, 0.27, 0.27, 0.75, 0.87, -0.1],
    [26.0, 0.47, 0.67, 0.52, 0.27, 0.37, 0.64, 0.87, -0.1],
    [28.0, 0.52, 0.68, 0.46, 0.27, 0.42, 0.58, 0.87, -0.1],
    [30.0, 0.64, 0.68, 0.35, 0.26, 0.54, 0.47, 0.87, -0.12],
    [32.0, 0.71, 0.67, 0.24, 0.25, 0.61, 0.36, 0.87, -0.15],
    [34.0, 0.75, 0.66, 0.18, 0.24, 0.65, 0.3, 0.87, -0.18],
    [36.0, 0.83, 0.65, 0.09, 0.22, 0.73, 0.21, 0.87, -0.2],
    [38.0, 0.9, 0.65, -0.01, 0.2, 0.8, 0.11, 0.87, -0.25],
    [40.0, 0.94, 0.65, -0.05, 0.18, 0.84, 0.07, 0.87, -0.3],
    [42.0, 1.0, 0.64, -0.12, 0.16, 0.9, -0.01, 0.87, -0.3],
    [44.0, 1.04, 0.64, -0.2, 0.15, 0.94, -0.05, 0.87, -0.3],
    [46.0, 1.08, 0.64, -0.3, 0.15, 0.96, -0.1, 0.87, -0.3],
    [48.0, 1.14, 0.64, -0.4, 0.15, 1.05, -0.1, 0.87, -0.3],
    [50.0, 1.25, 0.64, -0.5, 0.15, 1.1, -0.1, 0.87, -0.3],
    [60.0, 1.4, 0.64, -0.6, 0.15, 1.1, -0.1, 0.87, -0.3],
];

/// Horrible Colorful Transition 02 (Filmora): the ghost peeks in at the
/// left (frames 14-22) and sweeps right (to frame 43) just ahead of a
/// rounded front of the incoming clip (87 %). Rows
/// `[frame, ghost x, ghost y, front, gain B, tilt, -, -, -]`.
const GHOST_SWEEP: &[[f32; 9]] = &[
    [0.0, -0.3, 0.45, -0.2, 0.87, 0.2, 0.0, 0.0, 0.0],
    [14.0, -0.15, 0.45, -0.2, 0.87, 0.2, 0.0, 0.0, 0.0],
    [18.0, -0.02, 0.44, -0.15, 0.87, 0.1, 0.0, 0.0, 0.0],
    [22.0, 0.11, 0.44, 0.03, 0.87, 0.0, 0.0, 0.0, 0.0],
    [24.0, 0.23, 0.45, 0.13, 0.87, 0.0, 0.0, 0.0, 0.0],
    [26.0, 0.34, 0.45, 0.24, 0.87, 0.05, 0.0, 0.0, 0.0],
    [28.0, 0.41, 0.46, 0.31, 0.87, 0.05, 0.0, 0.0, 0.0],
    [30.0, 0.53, 0.46, 0.43, 0.87, 0.0, 0.0, 0.0, 0.0],
    [32.0, 0.66, 0.46, 0.56, 0.87, -0.05, 0.0, 0.0, 0.0],
    [34.0, 0.72, 0.47, 0.62, 0.87, -0.05, 0.0, 0.0, 0.0],
    [36.0, 0.84, 0.47, 0.74, 0.87, 0.0, 0.0, 0.0, 0.0],
    [38.0, 0.95, 0.47, 0.85, 0.87, 0.05, 0.0, 0.0, 0.0],
    [40.0, 1.0, 0.47, 0.9, 0.87, 0.05, 0.0, 0.0, 0.0],
    [42.0, 1.1, 0.47, 0.99, 0.87, 0.1, 0.0, 0.0, 0.0],
    [44.0, 1.25, 0.47, 1.2, 0.87, 0.1, 0.0, 0.0, 0.0],
    [60.0, 1.4, 0.47, 1.3, 0.87, 0.1, 0.0, 0.0, 0.0],
];

/// Horrible Colorful Transition 13 (Filmora): the ghost rises from the
/// bottom (frames 16-24) and holds while little ghosts circle it (to 42),
/// the camera dives into its mouth (43-48), white at 49, and the white
/// closes into a wedge at the top centre by frame 55 over the incoming clip
/// (87 %). Rows `[frame, y, scale, orbit, zoom, side, white, wedge, gain]`.
const BOO_ZOOM: &[[f32; 9]] = &[
    [0.0, 1.6, 1.05, 0.0, 1.0, 0.0, 0.0, 0.0, 0.87],
    [15.0, 1.6, 1.05, 0.0, 1.0, 0.0, 0.0, 0.0, 0.87],
    [18.0, 1.0, 1.05, 0.0, 1.0, 0.0, 0.0, 0.0, 0.87],
    [20.0, 0.75, 1.05, 0.3, 1.0, 0.0, 0.0, 0.0, 0.87],
    [22.0, 0.6, 1.05, 0.6, 1.0, 0.0, 0.0, 0.0, 0.87],
    [24.0, 0.53, 1.05, 0.8, 1.0, 0.0, 0.0, 0.0, 0.87],
    [42.0, 0.52, 1.05, 1.0, 1.0, 0.0, 0.0, 0.0, 0.87],
    [42.5, 0.52, 1.05, 1.0, 1.0, 0.0, 0.0, 0.0, 0.87],
    [42.5, 0.52, 1.05, 0.0, 2.5, 0.0, 0.0, 0.0, 0.87],
    [45.0, 0.52, 1.05, 0.0, 3.2, 0.0, 0.0, 0.0, 0.87],
    [47.0, 0.52, 1.05, 0.0, 5.0, 0.0, 0.2, 0.0, 0.87],
    [48.0, 0.52, 1.05, 0.0, 8.0, 0.0, 0.7, 0.0, 0.87],
    [48.5, 0.52, 1.05, 0.0, 12.0, 0.0, 1.0, 0.0, 0.87],
    [48.5, 0.52, 1.05, 0.0, 12.0, 1.0, 1.0, 0.0, 0.87],
    [49.5, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 1.05, 0.87],
    [51.0, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 1.05, 0.87],
    [52.0, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 0.8, 0.87],
    [53.0, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 0.55, 0.87],
    [54.0, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 0.2, 0.87],
    [55.0, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 0.0, 0.87],
    [60.0, 0.52, 1.05, 0.0, 12.0, 1.0, 0.0, 0.0, 0.87],
];

/// Halloween Horrors Pack Transition 02 (Filmora): the crowd's front rises
/// from the bottom (frames 12-30) and fills the frame; its hems lift off
/// the incoming clip from frame 33 to 50. Rows `[frame, top, hem, gain B, -]`.
const GHOST_CROWD: &[[f32; 5]] = &[
    [0.0, 1.20, 1.60, 1.00, 0.00],
    [12.0, 1.00, 1.60, 1.00, 0.00],
    [14.0, 0.93, 1.60, 1.00, 0.00],
    [18.0, 0.74, 1.60, 1.00, 0.00],
    [22.0, 0.49, 1.60, 1.00, 0.00],
    [26.0, 0.21, 1.60, 1.00, 0.00],
    [30.0, -0.08, 1.60, 1.00, 0.00],
    [33.0, -0.30, 1.14, 1.00, 0.00],
    [34.0, -0.35, 0.99, 1.00, 0.00],
    [36.0, -0.45, 0.83, 1.00, 0.00],
    [38.0, -0.55, 0.72, 1.00, 0.00],
    [40.0, -0.65, 0.48, 1.00, 0.00],
    [42.0, -0.75, 0.36, 1.00, 0.00],
    [44.0, -0.85, 0.19, 1.00, 0.00],
    [46.0, -0.95, 0.12, 1.00, 0.00],
    [48.0, -1.05, 0.05, 1.00, 0.00],
    [50.0, -1.15, -0.01, 1.00, 0.00],
    [53.0, -1.30, -0.25, 1.00, 0.00],
    [60.0, -1.60, -0.60, 1.00, 0.00],
];

/// Halloween Party Transition 03 (Filmora): bats gather over the outgoing
/// clip (frames 5-23), one flies at the camera (23-28), the camera dives
/// into its glowing eyes (28-31) and the incoming clip cuts in at frame 32.
/// Rows `[frame, swarm, big bat scale, eye zoom, side, glow, -, -, -]`.
const BAT_SWARM: &[[f32; 9]] = &[
    [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [5.0, 2.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [12.0, 20.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [22.0, 46.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [22.5, 46.0, 0.35, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [24.0, 46.0, 0.66, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [25.0, 46.0, 1.25, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [26.0, 46.0, 1.7, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [28.0, 46.0, 2.6, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [29.0, 46.0, 2.6, 1.6, 0.0, 0.1, 0.0, 0.0, 0.0],
    [30.0, 46.0, 2.6, 3.0, 0.0, 0.3, 0.0, 0.0, 0.0],
    [31.0, 46.0, 2.6, 3.2, 0.0, 0.55, 0.0, 0.0, 0.0],
    [31.5, 46.0, 2.6, 3.3, 0.0, 0.6, 0.0, 0.0, 0.0],
    [31.5, 46.0, 2.6, 3.3, 1.0, 0.6, 0.0, 0.0, 0.0],
    [60.0, 46.0, 2.6, 3.3, 1.0, 0.6, 0.0, 0.0, 0.0],
];

/// Halloween Party Transition 02 (Filmora): the camera pulls out of the
/// lantern's nose hole (frames 1-8), the eyes glow, the grin lights up
/// (25-31), the camera dives into it (31-38) and finds the outgoing clip,
/// then the incoming one, through the light. Rows
/// `[frame, zoom, pivot, mouth, eyes, wash, B mix, through, tint]`.
const LANTERN_FACE: &[[f32; 9]] = &[
    [0.0, 60.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.5, 60.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.5, 17.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0, 14.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [4.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [5.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [6.0, 4.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0],
    [7.0, 2.6, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0],
    [8.0, 1.6, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.0],
    [9.0, 1.2, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [10.0, 1.06, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [12.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [25.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [27.0, 1.0, 0.3, 0.3, 1.0, 0.0, 0.0, 0.0, 0.0],
    [29.0, 1.0, 0.7, 0.6, 1.0, 0.0, 0.0, 0.0, 0.0],
    [31.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [32.0, 1.7, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [33.0, 2.1, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [33.5, 2.4, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.6],
    [33.5, 2.4, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.6],
    [35.0, 3.2, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.5],
    [35.5, 3.4, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.45],
    [35.5, 3.4, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.45],
    [36.0, 6.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.3],
    [37.0, 12.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.1],
    [37.5, 16.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.05],
    [37.5, 16.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.05],
    [38.0, 20.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
    [60.0, 20.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
];

/// Creepy Pack Transition 03 (Filmora): centre of the rolling lantern per
/// reference frame (from its edges on the centre row), clip switch at frame
/// 29.5 while it covers the frame (`[frame, x, switch, roll, -]`).
const LANTERN_ROLL: &[[f32; 5]] = &[
    [0.0, 2.2, 0.0, 0.480, 0.0],
    [4.0, 1.79, 0.0, 0.357, 0.0],
    [6.0, 1.69, 0.0, 0.327, 0.0],
    [8.0, 1.57, 0.0, 0.291, 0.0],
    [10.0, 1.49, 0.0, 0.267, 0.0],
    [12.0, 1.32, 0.0, 0.216, 0.0],
    [14.0, 1.13, 0.0, 0.159, 0.0],
    [16.0, 1.04, 0.0, 0.132, 0.0],
    [18.0, 0.82, 0.0, 0.066, 0.0],
    [20.0, 0.62, 0.0, 0.006, 0.0],
    [22.0, 0.53, 0.0, -0.021, 0.0],
    [24.0, 0.35, 0.0, -0.075, 0.0],
    [26.0, 0.19, 0.0, -0.123, 0.0],
    [28.0, 0.12, 0.0, -0.144, 0.0],
    [29.5, 0.06, 0.0, -0.162, 0.0],
    [29.5, 0.06, 1.0, -0.162, 0.0],
    [30.0, 0.0, 1.0, -0.180, 0.0],
    [32.0, -0.12, 1.0, -0.216, 0.0],
    [34.0, -0.18, 1.0, -0.234, 0.0],
    [36.0, -0.33, 1.0, -0.279, 0.0],
    [38.0, -0.56, 1.0, -0.348, 0.0],
    [40.0, -0.67, 1.0, -0.381, 0.0],
    [42.0, -0.87, 1.0, -0.441, 0.0],
    [44.0, -1.09, 1.0, -0.507, 0.0],
    [46.0, -1.19, 1.0, -0.537, 0.0],
    [48.0, -1.38, 1.0, -0.594, 0.0],
    [50.0, -1.56, 1.0, -0.648, 0.0],
    [52.0, -1.64, 1.0, -0.672, 0.0],
    [54.0, -1.77, 1.0, -0.711, 0.0],
    [56.0, -2.0, 1.0, -0.780, 0.0],
    [60.0, -2.3, 1.0, -0.870, 0.0],
];

pub fn items() -> Vec<Item> {
    vec![
        art_item(
            "jb.lantern_roll",
            "Lantern Roll",
            "Halloween",
            "A huge dark jack-o'-lantern with a glowing red grin rolls across the frame from right to left, switching to the next clip as it passes.",
            art_wgsl("", &[tab("lr", &at_frames(LANTERN_ROLL))], include_str!("../shaders/art3a/lantern_roll.wgsl")),
        ),
        art_item(
            "jb.lantern_face",
            "Lantern Face",
            "Halloween",
            "The camera pulls out of a glowing jack-o'-lantern's nose, its grin lights up and the camera dives into the light to reveal the next clip.",
            art_wgsl("", &[tab("lf", &at_frames(LANTERN_FACE))], include_str!("../shaders/art3a/lantern_face.wgsl")),
        ),
        art_item(
            "jb.bat_swarm_dive",
            "Bat Swarm Dive",
            "Halloween",
            "Bats swarm over the clip, one flies at the camera and its glowing eyes fill the frame before the next clip cuts in.",
            art_wgsl(SPRITES, &[tab("bs", &at_frames(BAT_SWARM))], include_str!("../shaders/art3a/bat_swarm.wgsl")),
        ),
        art_item(
            "jb.ghost_crowd",
            "Ghost Crowd",
            "Halloween",
            "A crowd of sheet ghosts rises up and fills the frame, then drifts away upward to uncover the next clip.",
            art_wgsl(SPRITES, &[tab("gc", &at_frames(GHOST_CROWD))], include_str!("../shaders/art3a/ghost_crowd.wgsl")),
        ),
        art_item(
            "jb.boo_dive",
            "Boo Dive",
            "Halloween",
            "A big cheeky ghost pops up with little ghosts circling it, the camera dives into its mouth and a white flash opens on the next clip.",
            art_wgsl(SPRITES, &[tab("bz", &at_frames(BOO_ZOOM))], include_str!("../shaders/art3a/boo_zoom.wgsl")),
        ),
        art_item(
            "jb.ghost_glide",
            "Ghost Glide",
            "Halloween",
            "A friendly sheet ghost floats across the frame, drawing the next clip in behind a rounded front.",
            art_wgsl(SPRITES, &[tab("gs", &at_frames(GHOST_SWEEP))], include_str!("../shaders/art3a/ghost_sweep.wgsl")),
        ),
        art_item(
            "jb.skull_and_hat",
            "Skull and Hat Sweep",
            "Halloween",
            "A rolling skull and a gliding witch hat cross the frame in opposite directions, each pulling the next clip in behind it.",
            art_wgsl(SPRITES, &[tab("sk", &at_frames(SKULL_HAT))], include_str!("../shaders/art3a/skull_hat.wgsl")),
        ),
        art_item(
            "jb.bat_rise",
            "Bat Rise",
            "Halloween",
            "A giant bat with raised wings flies up through the frame and the next clip rises in behind it.",
            art_wgsl(SPRITES, &[tab("bt", &at_frames(BAT_RISE))], include_str!("../shaders/art3a/bat_rise.wgsl")),
        ),
        art_item(
            "jb.witch_hat_wave",
            "Witch Hat Wave",
            "Halloween",
            "A witch hat drops onto the clip, slides away and returns riding a wavy flood that brings in the next clip.",
            art_wgsl(SPRITES, &[tab("hw", &at_frames(HAT_WAVE))], include_str!("../shaders/art3a/hat_wave.wgsl")),
        ),
        art_item(
        "jb.pumpkin_grin",
        "Pumpkin Grin",
        "Halloween",
        "The camera pulls out of a jack-o'-lantern's grin showing the outgoing clip; the jaw snaps shut and the head flies up off the incoming clip.",
        art_wgsl("", &[tab("pk", &at_frames(PUMPKIN_GRIN))], include_str!("../shaders/art3a/pumpkin_grin.wgsl")),
        ),
    ]
}

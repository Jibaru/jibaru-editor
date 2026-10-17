//! The art-4 models: WGSL pass structures shared by the items in
//! `art4_data.rs`. Each model documents what its constants `c(i)` and
//! keyframed values `k(i)` mean. Lengths are in uv units unless noted;
//! "1080p px" values scale with the render height.

use crate::fx::art4_kit::{encode_single, Art4Fx, Model};
use crate::fx::{FxCx, Uniforms};
use crate::registry::FxIo;

/// Swirl: the frame is twisted about the centre by an angle that falls off
/// with the distance, `θ(r) = k0 · max(0, 1 − r / k1)^k2 + k5`, zoomed by
/// `k3`, and the outgoing and incoming clips are mixed by `k4`. Distances are
/// measured in a metric stretched by `c0` horizontally (1: uv space, so the
/// swirl is elliptical like the frame). Outside the frame is black.
pub const SWIRL: Model = Model { name: "swirl", bodies: &[include_str!("../shaders/art4/swirl.wgsl")], encode: encode_single };

/// Ripple: water-drop bumps (gradient of a seeded two-octave noise of
/// frequency `c0` cells per frame height, seed `c1`, drifting with phase
/// `k2`) displace the outgoing clip by `k0`, plus an optional radial ring
/// (radius `k4`, amplitude `k5`, width `k6`, wavenumber `c3`, centre
/// `c4, c5`); zoom `k3`; `c2 > 0.5` warps the incoming clip too; `k1`
/// mixes to the incoming clip.
pub const RIPPLE: Model = Model { name: "ripple", bodies: &[include_str!("../shaders/art4/ripple.wgsl")], encode: encode_single };

/// Ring wave: a front of radius `k0` (soft edge `k5`, metric stretched by
/// `c0`) spreads from the centre; behind it concentric rings of wavenumber
/// `c1` and phase `k1` displace the image radially by `k2` and carry the
/// incoming clip with weight `k3 + k4 · cos(phase + c2)`; zoom `k6`.
pub const RING_WAVE: Model =
    Model { name: "ring_wave", bodies: &[include_str!("../shaders/art4/ring_wave.wgsl")], encode: encode_single };

/// Wave split: horizontal lines bend into sine waves (`k0`, `c0` cycles
/// across, phase `k1`, second harmonic `k6` at `c2` cycles), columns sway
/// (`k2`, `c1` cycles down, phase `k3`), red and blue are split
/// horizontally by `±k4`, and `k5` mixes to the incoming clip.
pub const WAVE_SPLIT: Model =
    Model { name: "wave_split", bodies: &[include_str!("../shaders/art4/wave_split.wgsl")], encode: encode_single };

/// Unsharp mask (strong sharpening): gaussian of `c0` 1080p px, amount `c1`,
/// red/blue fringes from taps shifted `±(c2, c6)` px blended by `c3`,
/// saturation `1 + c7`, scanlines (amplitude `c8`, period `c9` px, phase
/// `c10`), then contrast `c4`, lift `c5` and per-channel lift `c11..c13`.
pub const UNSHARP: Model = Model { name: "unsharp", bodies: &[include_str!("../shaders/art4/unsharp.wgsl")], encode: encode_unsharp };

/// Glow: the clip zoomed by `k5` is mixed with a gaussian copy (sigma `k0`
/// 1080p px) by `k1`, the copy is screen-blended back as a glow (`k4`), then
/// gain `k2`, lift `k3`, tint `k6..k8` and saturation `1 + k9`.
pub const GLOW: Model = Model { name: "glow", bodies: &[include_str!("../shaders/art4/glow.wgsl")], encode: encode_glow };

fn encode_unsharp(fx: &Art4Fx, cx: &mut FxCx<'_>, io: &FxIo<'_>, u: Uniforms) {
    let px = io.a.height as f32 / 1080.0;
    let s = u.p[0][0] * px;
    let bl = cx.blur(io.a, s, s);
    fx.pass(cx, 0, io.a, None, Some(&bl), io.out, u);
}

fn encode_glow(fx: &Art4Fx, cx: &mut FxCx<'_>, io: &FxIo<'_>, u: Uniforms) {
    let px = io.a.height as f32 / 1080.0;
    let s = u.p[16][0].max(0.0) * px;
    let bl = cx.blur(io.a, s, s);
    fx.pass(cx, 0, io.a, None, Some(&bl), io.out, u);
}

/// Warped prism: a four-sided box carrying the outgoing clip on even faces
/// and the incoming clip on odd ones, ray-cast in perspective while it
/// turns about the vertical (`c0 = 0`) or horizontal (`c0 = 1`) axis.
/// Camera distance `c1` (half-heights), `c2 > 0.5` mirrors the face
/// coordinate, `c3 > 0.5` puts the incoming clip on the back face too.
/// Keyframed: turn angle `k0` (radians), twist along the axis `k1`, face
/// bulge `k2` (sagitta / half-width; curved faces), zoom `k3`, tilt `k4`,
/// bend `k5` (turn ∝ axis position²), offset `k6, k7` (half-heights),
/// screen radial warp `k8` (barrel > 0) and cask `k9` (box wider in the
/// middle of its axis by that fraction), camera dolly `k10` (half-heights).
pub const PRISM: Model = Model { name: "prism", bodies: &[include_str!("../shaders/art4/prism.wgsl")], encode: encode_single };

/// Mosaic: the frame is cut into `k0` columns and `k0 · k1` rows (`k1 = 0`:
/// as many rows as columns) with a cell edge on the centre (offset `k2, k3`
/// cells),
/// each filled with the clip point-sampled at `c0, c1` inside the cell (mixed
/// with the cell average by `c2`); `k4`
/// mixes outgoing to incoming, then gain `k5` and lift `k6`.
pub const MOSAIC: Model = Model { name: "mosaic", bodies: &[include_str!("../shaders/art4/mosaic.wgsl")], encode: encode_single };

/// Channel split: red, green and blue are read from the clip shifted by
/// `(k0, k1)`, `(k2, k3)`, `(k4, k5)` (uv) and smeared horizontally over
/// `k6` 1080p px; `k9` blends the split over the clip (0: fully), then gain
/// `1 + k7` and lift `k8`. Outside the frame is mirrored.
pub const RGB_SPLIT: Model = Model { name: "rgb_split", bodies: &[include_str!("../shaders/art4/rgb_split.wgsl")], encode: encode_single };

/// Banknote engraving: the luminance (gain `c8`, lift `c9`, gamma `c11`)
/// is drawn as ink lines of two screens, concentric rings about `c0, c1`
/// (uv) of period `c2` 1080p px and straight lines at angle `c3` of period
/// `c4` waving by `c5` px at `c6` cycles per 1000 px (weight `c12`); a line
/// is inked where the screen falls below the darkness (sharpness `c7`).
/// `c10` blends the engraving over the plain grey.
pub const ENGRAVE: Model = Model { name: "engrave", bodies: &[include_str!("../shaders/art4/engrave.wgsl")], encode: encode_single };

/// Shatter: the clip is a jittered Voronoi mosaic of cells `c0` 1080p px
/// (seed `c16`). A cell breaks off at progress `c1 + c2 · x + c3 · rnd`
/// (`x`: horizontal position, or with `c7 > 0.5` the distance from `c8, c9`
/// in frame heights); `c4` before that black cracks of `c5` cells outline
/// it. A released shard shrinks to `c6` over `c10`, drifts `c11` px per
/// unit progress in a random direction plus `c12` downwards, is lit by
/// `c15` and vanishes after `c13 + c14 · rnd`. The rest is black.
pub const SHATTER: Model = Model { name: "shatter", bodies: &[include_str!("../shaders/art4/shatter.wgsl")], encode: encode_single };

/// Colour glitch: the clip (outgoing mixed to incoming by `k14`; rows waved
/// sideways by `k15` uv, `c1` cycles down, phase `c2 + c3 · progress`) with
/// red/blue split `k11` 1080p px, smeared
/// horizontally over `k5` px, posterized to `k8` levels (< 2: off), mixed
/// by `k6` with a rainbow of its luminance (`c0` cycles, or `k12` when set,
/// phase `k7`) where the luminance exceeds `k13`,
/// inverted by `k0`, desaturated by `k1`, channel gains `1 + k2..k4`, then
/// gain `1 + k9` and lift `k10`.
pub const PSY: Model = Model { name: "psy", bodies: &[include_str!("../shaders/art4/psy.wgsl")], encode: encode_single };

/// Signal glitch: the frame jitters by `k0, k1` 1080p px with a red/blue
/// split of `k2` px; a fraction `k9` of horizontal bands (`c0` px high) is
/// shifted by up to `±k8` px; `k3` seeded bars (height `c1 + c2 · rnd` px,
/// width `c3 + c4 · rnd` of the frame, cyan with probability `c5`, else
/// white) are drawn at opacity `k7`. The random layout changes with the
/// integer beat `k4`. Then gain `1 + k5` and lift `k6`.
pub const JITTER_BARS: Model =
    Model { name: "jitter_bars", bodies: &[include_str!("../shaders/art4/jitter_bars.wgsl")], encode: encode_single };

/// Block dissolve: square cells (`c0` across, seed `c12`) of the outgoing
/// clip drop out to a flat colour `c2..c4` at progress `c5 + c6 · rnd +
/// c7 · bias`, and cells of the incoming clip appear at `c8 + c9 · rnd' +
/// c10 · bias`; bias is the horizontal position, or with `c11 > 0.5` the
/// closeness to the vertical line at `x = c13` (0: centre; edges go first).
pub const BLOCK_DISSOLVE: Model =
    Model { name: "block_dissolve", bodies: &[include_str!("../shaders/art4/block_dissolve.wgsl")], encode: encode_single };

pub static MODELS: &[&Model] = &[&BLOCK_DISSOLVE, &JITTER_BARS, &PSY, &SHATTER, &ENGRAVE, &RGB_SPLIT, &MOSAIC, &PRISM, &SWIRL, &RIPPLE, &RING_WAVE, &WAVE_SPLIT, &UNSHARP, &GLOW];

pub fn model(name: &str) -> Option<&'static Model> {
    MODELS.iter().copied().find(|m| m.name == name)
}

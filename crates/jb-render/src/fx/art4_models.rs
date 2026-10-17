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
/// red/blue fringes from taps shifted `±c2` px blended by `c3`, then
/// contrast `c4` and lift `c5`.
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

pub static MODELS: &[&Model] = &[&PRISM, &SWIRL, &RIPPLE, &RING_WAVE, &WAVE_SPLIT, &UNSHARP, &GLOW];

pub fn model(name: &str) -> Option<&'static Model> {
    MODELS.iter().copied().find(|m| m.name == name)
}

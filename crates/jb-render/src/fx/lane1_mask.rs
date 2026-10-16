//! Parametric soft-mask wipes fitted frame by frame on Filmora exports
//! (linear, circular, clock sweep, pinwheel and star masks). The mask
//! position `c`, edge width `w` and rotation `r` are tabulated at progress
//! keyframes (`shaders/lane1/mask.wgsl` has the mask definition); the
//! shape constants are per item.

use super::{info, FxCx, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};

/// Mask family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Linear = 0,
    Circle = 1,
    Sweep = 2,
    Pinwheel = 3,
    Star = 4,
}

pub struct MaskSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub shape: Shape,
    /// +1: B where `d < c`; -1: B where `d > c`.
    pub sgn: f32,
    /// Mask centre in height units from the frame centre.
    pub centre: [f32; 2],
    pub blades: f32,
    /// Smoothstep the edge ramp.
    pub smooth: bool,
    /// `[p, c, w, r]` keyframes.
    pub rows: &'static [[f32; 4]],
}

struct MaskFx {
    info: FxInfo,
    spec: &'static MaskSpec,
    shader: ShaderSrc,
}

fn sample(rows: &[[f32; 4]], p: f32) -> [f32; 4] {
    if rows.is_empty() {
        return [0.0, 1.0, 0.0, 0.0];
    }
    if p <= rows[0][0] {
        return rows[0];
    }
    for w in rows.windows(2) {
        if p < w[1][0] {
            let k = (p - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
            return std::array::from_fn(|i| w[0][i] + (w[1][i] - w[0][i]) * k);
        }
    }
    rows[rows.len() - 1]
}

impl Effect for MaskFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let s = self.spec;
        let r = sample(s.rows, io.progress);
        let mut u = Uniforms { progress: io.progress, ..Default::default() };
        u.p[0] = [r[1], r[2], r[3], 0.0];
        u.p[1] = [s.shape as u32 as f32, s.sgn, s.centre[0], s.centre[1]];
        u.p[2] = [s.blades, if s.smooth { 1.0 } else { 0.0 }, 0.0, 0.0];
        cx.run(&self.shader, io.a, io.b, io.out, u);
    }
}

pub fn register(r: &mut Registry) {
    let body = include_str!("../shaders/lane1/mask.wgsl");
    for spec in super::lane1_tables::MASKS {
        r.register(MaskFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Transition, spec.description, &[]),
            spec,
            shader: ShaderSrc::new("jb.lane1.mask", body),
        });
    }
}

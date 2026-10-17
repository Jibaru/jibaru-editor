//! Table-driven camera transitions fitted frame by frame on Filmora exports.
//!
//! One shader (`shaders/lane1/table.wgsl`) renders every item: each side is
//! the clip seen through a camera (shift, zoom, rotation, optional
//! perspective, radial bulge and swirl) with spin / zoom / motion blur, and the frame is a
//! per-channel mix `al * A' + be * B' + ga` in display space (dissolves,
//! flashes, fades and tints). Each item is a table of those parameters at
//! progress keyframes, fitted on Filmora's export of the item (2.002 s,
//! centred on the cut) and simplified to the keyframes that linear
//! interpolation needs. Other durations scale in progress.
//!
//! Row layout ([`ROW`] floats): `p`, A `[tx, ty, k, phi, bx, by, zb, rb, px, py, dk, sw]`,
//! B the same, `al.rgb`, `be.rgb`, `ga.rgb`, `av.rgb`, `bv.rgb` (vignette terms: the
//! weights become `al + av * q` and `be + bv * q` with `q = |2 (uv - 0.5)|^4`).

use super::{info, FxCx, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};

pub const ROW: usize = 40;

/// Edge handling of a side's source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Transparent = 0,
    Clamp = 1,
    Mirror = 2,
}

/// One fitted item.
pub struct TableSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub edge: Edge,
    /// Layer order: 0 sum, 1 A over B, 2 B over A (coverage of transparent edges).
    pub order: u8,
    pub rows: &'static [[f32; ROW]],
}

pub struct TableFx {
    info: FxInfo,
    spec: &'static TableSpec,
    shader: ShaderSrc,
}

/// Linear interpolation of the rows at progress `p` (held outside).
pub fn sample_rows(rows: &[[f32; ROW]], p: f32) -> [f32; ROW] {
    let identity = {
        let mut r = [0.0; ROW];
        r[3] = 1.0;
        r[15] = 1.0;
        r
    };
    if rows.is_empty() {
        return identity;
    }
    if p <= rows[0][0] {
        return rows[0];
    }
    for w in rows.windows(2) {
        if p < w[1][0] {
            let k = (p - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
            let mut out = [0.0; ROW];
            for (i, o) in out.iter_mut().enumerate() {
                *o = w[0][i] + (w[1][i] - w[0][i]) * k;
            }
            return out;
        }
    }
    rows[rows.len() - 1]
}

impl Effect for TableFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let r = sample_rows(self.spec.rows, io.progress);
        let mut u = Uniforms { progress: io.progress, time: io.time, ..Default::default() };
        for side in 0..2 {
            let o = 1 + side * 12;
            u.p[side * 3] = [r[o], r[o + 1], r[o + 2], r[o + 3]];
            u.p[side * 3 + 1] = [r[o + 4], r[o + 5], r[o + 6], r[o + 7]];
            u.p[side * 3 + 2] = [r[o + 8], r[o + 9], r[o + 10], r[o + 11]];
        }
        u.p[6] = [r[25], r[26], r[27], 0.0];
        u.p[7] = [r[28], r[29], r[30], 0.0];
        u.p[8] = [r[31], r[32], r[33], 0.0];
        u.p[10] = [r[34], r[35], r[36], 0.0];
        u.p[11] = [r[37], r[38], r[39], 0.0];
        u.p[9] = [self.spec.edge as u32 as f32, self.spec.order as f32, 0.0, 0.0];
        cx.run(&self.shader, io.a, io.b, io.out, u);
    }
}

pub fn register(r: &mut Registry) {
    let body = include_str!("../shaders/lane1/table.wgsl");
    for spec in super::lane1_tables::TABLES {
        r.register(TableFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Transition, spec.description, &[]),
            spec,
            shader: ShaderSrc::new("jb.lane1.table", body),
        });
    }
}

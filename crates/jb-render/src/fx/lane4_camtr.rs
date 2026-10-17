//! Camera transitions (zoom-blur warps, twists, whips): each side goes
//! through its own per-frame camera (similarity, motion blur, zoom blur,
//! lens distortion; see `lane4_camera.rs`), the two are mixed with a
//! per-frame weight and a per-channel display-space gain/offset is applied:
//!
//! ```text
//! out = g · ((1 − a) · camA(A) + a · camB(B)) + o
//! ```
//!
//! Rows were fitted frame by frame on the reference export with both
//! clips known (`docs/fidelity-lane4.md`).

use crate::fx::lane4_all::camera::{encode_cam, Edge, CAM_ID};
use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};

/// `[p, A: s rot tx ty mx my z k1, B: s rot tx ty mx my z k1, a, gr gg gb, or og ob]`.
pub type CamTrRow = [f32; 24];

pub struct CamTrSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub edge: Edge,
    pub rows: &'static [CamTrRow],
}

fn row_at(rows: &[CamTrRow], p: f32) -> CamTrRow {
    let mut id = [0.0f32; 24];
    id[1] = 1.0;
    id[9] = 1.0;
    id[18] = 1.0;
    id[19] = 1.0;
    id[20] = 1.0;
    if rows.is_empty() {
        id[17] = if p < 0.5 { 0.0 } else { 1.0 };
        return id;
    }
    if p <= rows[0][0] {
        return rows[0];
    }
    for w in rows.windows(2) {
        if p < w[1][0] {
            let k = (p - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
            let mut r = w[0];
            for i in 1..24 {
                r[i] = w[0][i] + (w[1][i] - w[0][i]) * k;
            }
            return r;
        }
    }
    rows[rows.len() - 1]
}

fn side_row(r: &CamTrRow, o: usize) -> crate::fx::lane4_all::camera::CamRow {
    let mut c = CAM_ID;
    c[1..8].copy_from_slice(&r[o..o + 7]);
    c[12] = r[o + 7];
    c
}

const MIX: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let a = to_disp(sa(in.uv));
    let b = to_disp(sb(in.uv));
    let k = u.e0.x;
    let m = mix(a, b, k);
    let c = m.rgb * u.e1.rgb + u.e2.rgb / 255.0;
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), m.a));
}
"#;

fn mix_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.l4.camtr_mix", MIX))
}

pub struct CamTrFx {
    info: FxInfo,
    spec: &'static CamTrSpec,
}

impl Effect for CamTrFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let r = row_at(self.spec.rows, io.progress);
        let b_in = io.b.unwrap_or(io.a);
        let ta = cx.temp_like(io.out);
        let tb = cx.temp_like(io.out);
        encode_cam(cx, &side_row(&r, 1), self.spec.edge, io.a, &ta, 1.0);
        encode_cam(cx, &side_row(&r, 9), self.spec.edge, b_in, &tb, 1.0);
        let mut u = Uniforms::default();
        u.e[0] = [r[17].clamp(0.0, 1.0), 0.0, 0.0, 0.0];
        u.e[1] = [r[18], r[19], r[20], 0.0];
        u.e[2] = [r[21], r[22], r[23], 0.0];
        let mut pass = Pass::full(mix_src(), io.out, u);
        pass.a = Bind::Tex(&ta);
        pass.b = Bind::Tex(&tb);
        cx.draw(pass);
    }
}

include!("lane4/camtr_table.rs");

pub fn register(r: &mut Registry) {
    for spec in CAMTRS {
        r.register(CamTrFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Transition, spec.description, &[]),
            spec,
        });
    }
}

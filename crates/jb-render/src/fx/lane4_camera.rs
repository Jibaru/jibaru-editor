//! Tabulated camera effects (shakes, zoom punches, flashes): every frame is
//! the clip through a similarity transform about the centre, smeared by a
//! motion blur (a vector in centred units, y spanning −0.5..0.5) and a zoom
//! blur, with the red and blue channels sampled at their own offsets (RGB
//! split), then a per-channel display-space gain and offset:
//!
//! ```text
//! q    = R(−rot) (p − t) / s                 p = centred, aspect-correct coords
//! out  = g · mean_k src((q + d_c + k m)(1 + k z)) + o     k ∈ [−½, ½]
//! ```
//!
//! The rows were fitted frame by frame to reference exports of the effect at
//! its default settings (`docs/fidelity-lane4.md`); the table is indexed by
//! clip time and repeats with the fitted period.

use crate::fx::Bind;
use crate::fx::{info, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec as P, Registry};

/// One fitted frame:
/// `[t, s, rot, tx, ty, mx, my, z, rdx, rdy, bdx, bdy, gr, gg, gb, or, og, ob]`
/// (time in seconds, color offsets in 0..255 code values).
pub type CamRow = [f32; 18];

/// How samples outside the frame are filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Clamp,
    Mirror,
    Black,
}

pub struct CamSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub edge: Edge,
    /// Table period in seconds (time wraps around it).
    pub period: f32,
    pub rows: &'static [CamRow],
}

/// The identity row (no transform, no blur, no color change).
pub const CAM_ID: CamRow = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

/// Linear interpolation of the rows at time `t` (seconds, wrapped by the period).
pub fn cam_at(spec: &CamSpec, t: f32) -> CamRow {
    let rows = spec.rows;
    if rows.is_empty() {
        return CAM_ID;
    }
    let t = if spec.period > 0.0 { t.rem_euclid(spec.period) } else { t };
    if t <= rows[0][0] {
        return rows[0];
    }
    for w in rows.windows(2) {
        if t < w[1][0] {
            let k = (t - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
            let mut r = w[0];
            for i in 1..18 {
                r[i] = w[0][i] + (w[1][i] - w[0][i]) * k;
            }
            return r;
        }
    }
    rows[rows.len() - 1]
}

const BODY: &str = r#"
fn cam_sample(q: vec2f) -> vec4f {
    var uv = from_c(q);
    let mode = u.e3.x;
    if (mode > 1.5) { return select(vec4f(0.0), sa(uv), in01(uv)); }
    if (mode > 0.5) {
        let m = abs(fract(uv * 0.5) * 2.0 - 1.0);
        uv = 1.0 - m;
    }
    return sa(clamp(uv, vec2f(0.0), vec2f(1.0)));
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let s = u.e0.x;
    let rot = u.e0.y;
    let t = u.e0.zw;
    let m = u.e1.xy;
    let z = u.e1.z;
    let q = rot2(to_c(in.uv) - t, -rot) / s;
    let dr = u.e3.zw;
    let db = u.p[2].xy;
    let split = abs(dr.x) + abs(dr.y) + abs(db.x) + abs(db.y) > 1e-6;
    let blur = abs(m.x) + abs(m.y) + abs(z) > 1e-5;
    let n = select(1, 25, blur);
    var acc = vec4f(0.0);
    for (var i = 0; i < n; i = i + 1) {
        let k = select(0.0, f32(i) / f32(n - 1) - 0.5, blur);
        let f = 1.0 + k * z;
        let g = cam_sample((q + k * m) * f);
        if (split) {
            let r = cam_sample((q + dr + k * m) * f);
            let b = cam_sample((q + db + k * m) * f);
            acc = acc + vec4f(r.r, g.g, b.b, g.a);
        } else {
            acc = acc + g;
        }
    }
    let d = to_disp(acc / f32(n));
    let c = d.rgb * u.p[0].rgb + vec3f(u.p[1].rgb) / 255.0;
    return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), d.a));
}
"#;

fn shader_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.l4.camera", BODY))
}

const PARAMS: &[P] = &[P::float("intensity", "Intensity", 0.0, 100.0, 100.0).unit("%")];

/// Records one camera pass for `row` (intensity 0..1 blends toward identity).
pub fn encode_cam(cx: &mut FxCx<'_>, row: &CamRow, edge: Edge, a: &crate::pool::Tex, out: &crate::pool::Tex, k: f32) {
    let mut r = *row;
    for i in 1..18 {
        r[i] = CAM_ID[i] + (r[i] - CAM_ID[i]) * k;
    }
    let mut u = Uniforms::default();
    u.e[0] = [r[1], r[2], r[3], r[4]];
    u.e[1] = [r[5], r[6], r[7], 0.0];
    let mode = match edge {
        Edge::Clamp => 0.0,
        Edge::Mirror => 1.0,
        Edge::Black => 2.0,
    };
    u.e[3] = [mode, 0.0, r[8], r[9]];
    u.p[0] = [r[12], r[13], r[14], 0.0];
    u.p[1] = [r[15], r[16], r[17], 0.0];
    u.p[2] = [r[10], r[11], 0.0, 0.0];
    let mut pass = Pass::full(shader_src(), out, u);
    pass.a = Bind::Tex(a);
    cx.draw(pass);
}

pub struct CamFx {
    info: FxInfo,
    spec: &'static CamSpec,
}

impl Effect for CamFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let row = cam_at(self.spec, io.time);
        let k = (io.params.get("intensity") / 100.0).clamp(0.0, 1.0);
        encode_cam(cx, &row, self.spec.edge, io.a, io.out, k);
    }
}

include!("lane4/camera_table.rs");

pub fn register(r: &mut Registry) {
    for spec in CAMS {
        r.register(CamFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Effect, spec.description, PARAMS),
            spec,
        });
    }
}

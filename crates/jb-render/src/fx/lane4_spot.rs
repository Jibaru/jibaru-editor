//! Spot color: the frame goes through one fitted look (a 3D LUT, here a
//! dark monochrome grade) except inside a rectangle that jumps to a new
//! place on a fixed beat, where the original colors are kept through a
//! per-rectangle affine color matrix (each rectangle has its own tint).
//! Rectangles, beat and matrices were measured on the reference export
//! (`docs/fidelity-lane4.md`).

use crate::fx::lane4_all::looks::{lut_from_bytes, LUT_N};
use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec as P, Registry};

pub struct SpotRect {
    /// Start time in seconds (within the period).
    pub start: f32,
    /// uv rectangle `[x0, y0, x1, y1]`.
    pub rect: [f32; 4],
    /// Rows of `out = M · [r, g, b, 1]` in 0..255 code values.
    pub m: [[f32; 4]; 3],
}

pub struct SpotSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub period: f32,
    pub lut: &'static [u8],
    pub rects: &'static [SpotRect],
}

const BODY: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let d = to_disp(sa(in.uv));
    let c = clamp(d.rgb, vec3f(0.0), vec3f(1.0));
    let n = u.e0.x;
    let g = textureSampleLevel(tlut, smp, c * ((n - 1.0) / n) + 0.5 / n, 0.0).rgb;
    let r = u.e1;
    let inside = all(in.uv >= r.xy) && all(in.uv < r.zw);
    var o = g;
    if (inside) {
        let x = vec4f(c * 255.0, 1.0);
        o = clamp(vec3f(dot(u.e2, x), dot(u.e3, x), dot(u.p[1], x)) / 255.0, vec3f(0.0), vec3f(1.0));
    }
    let k = clamp(u.p[0].x / 100.0, 0.0, 1.0);
    return from_disp(vec4f(mix(d.rgb, o, k), d.a));
}
"#;

fn shader_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.l4.spot", BODY))
}

const PARAMS: &[P] = &[P::float("intensity", "Intensity", 0.0, 100.0, 100.0).unit("%")];

pub struct SpotFx {
    info: FxInfo,
    spec: &'static SpotSpec,
}

impl Effect for SpotFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let s = self.spec;
        let t = if s.period > 0.0 { io.time.rem_euclid(s.period) } else { io.time };
        let rect = s.rects.iter().rev().find(|r| t >= r.start - 1e-4).or(s.rects.first());
        let view = cx.aux_texture(&format!("l4lut:{}", s.id), || lut_from_bytes(s.lut).to_aux());
        let mut u = Uniforms::default();
        u.p[0][0] = io.params.get("intensity");
        u.e[0] = [LUT_N as f32, 0.0, 0.0, 0.0];
        if let Some(r) = rect {
            u.e[1] = r.rect;
            u.e[2] = r.m[0];
            u.e[3] = r.m[1];
            u.p[1] = r.m[2];
        } else {
            u.e[1] = [2.0, 2.0, 2.0, 2.0];
        }
        let mut pass = Pass::full(shader_src(), io.out, u);
        pass.a = Bind::Tex(io.a);
        pass.lut = Some(&view);
        cx.draw(pass);
    }
}

include!("lane4/spot_table.rs");

pub fn register(r: &mut Registry) {
    for spec in SPOTS {
        r.register(SpotFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Effect, spec.description, PARAMS),
            spec,
        });
    }
}

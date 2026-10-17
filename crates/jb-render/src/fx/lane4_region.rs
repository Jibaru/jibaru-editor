//! Region blur: a fixed rectangle of the frame is smeared by a separable
//! gaussian (strongly horizontal by default), the rest is untouched. The
//! rectangle and the blur widths were fitted on the reference export
//! (`docs/fidelity-lane4.md`).

use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec as P, Registry};

pub struct RegionSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    /// Default uv rectangle `[x0, y0, x1, y1]`.
    pub rect: [f32; 4],
    /// Gaussian sigmas in 1080p pixels.
    pub sigma: [f32; 2],
}

const BODY: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let r = u.e0;
    let inside = all(in.uv >= r.xy) && all(in.uv < r.zw);
    return select(sa(in.uv), sb(in.uv), inside);
}
"#;

fn shader_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.l4.region", BODY))
}

const PARAMS: &[P] = &[
    P::float("strength", "Strength", 0.0, 400.0, 100.0).unit("%"),
    P::vec2("center", "Center", 0.0, 1.0, [0.5, 0.5]),
    P::vec2("size", "Size", 0.0, 1.0, [0.0, 0.0]),
];

pub struct RegionFx {
    info: FxInfo,
    spec: &'static RegionSpec,
}

impl Effect for RegionFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let s = self.spec;
        let k = io.params.get("strength") / 100.0;
        let px = io.a.height as f32 / 1080.0;
        let blurred = cx.blur(io.a, s.sigma[0] * px * k, s.sigma[1] * px * k);
        let mut rect = s.rect;
        let size = io.params.slot("size").unwrap_or([0.0; 4]);
        let c = io.params.slot("center").unwrap_or([0.5, 0.5, 0.0, 0.0]);
        if size[0] > 0.0 && size[1] > 0.0 {
            rect = [c[0] - size[0] / 2.0, c[1] - size[1] / 2.0, c[0] + size[0] / 2.0, c[1] + size[1] / 2.0];
        }
        let mut u = Uniforms::default();
        u.e[0] = rect;
        let mut pass = Pass::full(shader_src(), io.out, u);
        pass.a = Bind::Tex(io.a);
        pass.b = Bind::Tex(&blurred);
        cx.draw(pass);
    }
}

pub static REGIONS: &[RegionSpec] = &[RegionSpec {
    id: "jb.l4.region_smear",
    name: "Region Smear",
    category: "Stylized",
    description: "Horizontal smear inside a centred rectangle (privacy blur).",
    rect: [0.3521, 0.35, 0.6479, 0.6463],
    sigma: [90.0, 0.0],
}];

pub fn register(r: &mut Registry) {
    for spec in REGIONS {
        r.register(RegionFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Effect, spec.description, PARAMS),
            spec,
        });
    }
}

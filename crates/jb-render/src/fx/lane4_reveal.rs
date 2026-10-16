//! Procedural reveal transitions (ink, brush, liquid and mask wipes): the
//! incoming clip shows where a reveal map falls below a threshold that
//! follows the transition's measured coverage curve.
//!
//! ```text
//! m(p) = (1 − k) · dot(p, (cos θ, sin θ)) + k · |p − c| + amp · (fbm(p · f + seed) − ½)
//! B where m(p) < thr(progress)            p = centred, aspect-correct coords
//! ```
//!
//! `θ, k, c, amp, f, seed` were searched per item to best match the mask
//! sequence measured on Filmora's export (alpha of every frame recovered
//! from the known outgoing / incoming frames); `thr` is tabulated per
//! frame so the revealed area matches the export frame by frame. The
//! shapes are our own noise fields, not Filmora's mattes, so edges differ
//! in detail (see `docs/fidelity-lane4.md`).

use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};

pub struct RevealSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    /// `[θ, k, cx, cy, amp, f, seed]`.
    pub map: [f32; 7],
    /// `(progress, threshold)` pairs, progress ascending.
    pub thr: &'static [[f32; 2]],
}

const BODY: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let p = to_c(in.uv);
    let th = u.e0.x;
    let k = u.e0.y;
    let c = u.e0.zw;
    let amp = u.e1.x;
    let f = u.e1.y;
    let seed = u.e1.z;
    let thr = u.e1.w;
    let d = cos(th) * p.x + sin(th) * p.y;
    let r = length(p - c);
    var m = (1.0 - k) * d + k * r;
    if (amp > 0.0) {
        m = m + amp * (fbm(p * f + vec2f(seed * 13.1, seed * 7.7)) - 0.5);
    }
    // Anti-aliased edge about one output pixel wide.
    let w = 0.75 * u.inv_size.y;
    let b = 1.0 - smoothstep(thr - w, thr + w, m);
    return mix(sa(in.uv), sb(in.uv), b);
}
"#;

fn shader_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.l4.reveal", BODY))
}

/// Threshold at `p` (linear between the tabulated pairs).
pub fn thr_at(tab: &[[f32; 2]], p: f32) -> f32 {
    if tab.is_empty() {
        return if p < 0.5 { -10.0 } else { 10.0 };
    }
    if p <= tab[0][0] {
        return tab[0][1];
    }
    for w in tab.windows(2) {
        if p < w[1][0] {
            let k = (p - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
            return w[0][1] + (w[1][1] - w[0][1]) * k;
        }
    }
    tab[tab.len() - 1][1]
}

pub struct RevealFx {
    info: FxInfo,
    spec: &'static RevealSpec,
}

impl Effect for RevealFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let m = self.spec.map;
        let mut u = Uniforms::default();
        u.e[0] = [m[0], m[1], m[2], m[3]];
        u.e[1] = [m[4], m[5], m[6], thr_at(self.spec.thr, io.progress)];
        let mut pass = Pass::full(shader_src(), io.out, u);
        pass.a = Bind::Tex(io.a);
        pass.b = io.b.map_or(Bind::Tex(io.a), Bind::Tex);
        cx.draw(pass);
    }
}

include!("lane4/reveal_table.rs");

pub fn register(r: &mut Registry) {
    for spec in REVEALS {
        r.register(RevealFx {
            info: info(spec.id, spec.name, spec.category, FxKind::Transition, spec.description, &[]),
            spec,
        });
    }
}

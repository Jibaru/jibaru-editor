//! Shared machinery of the art-4 items (procedural recreations of Filmora
//! glitch, warp, 3D-surface and stylized transitions/effects, drawn with our
//! own parametric WGSL; `docs/fidelity-art4.md`).
//!
//! Every item is data: a [`Model`] (one or more WGSL passes and the code
//! that records them), a constant vector `c` and a table of keyframe rows
//! `keys` interpolated over progress. Shaders read them through the
//! `c(i)` / `k(i)` accessors of `shaders/art4/common.wgsl`:
//!
//! ```text
//! u.p[0..16]   c[0..64]     constants of the item
//! u.p[16..28]  k[0..48]     keyframe row at the current progress (linear)
//! u.p[28..32]               free for the model's passes
//! u.e0..e3                  free for the model's passes
//! ```
//!
//! Randomness is seeded and deterministic (integer hashes of block indices
//! and of the frame "beat"), so a render is reproducible frame by frame.

use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::pool::Tex;
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Number of constants of an item.
pub const NC: usize = 64;
/// Number of keyframed values of an item.
pub const NK: usize = 48;

/// One keyframe row: values at progress `p`.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub p: f32,
    pub v: [f32; NK],
}

/// A keyframe row from a short value list (missing values are 0).
pub const fn row(p: f32, v: &[f32]) -> Key {
    let mut o = [0.0; NK];
    let mut i = 0;
    while i < v.len() && i < NK {
        o[i] = v[i];
        i += 1;
    }
    Key { p, v: o }
}

/// Records the passes of one model.
pub type EncodeFn = fn(&Art4Fx, &mut FxCx<'_>, &FxIo<'_>, Uniforms);

/// A family of looks sharing shaders and pass structure.
pub struct Model {
    pub name: &'static str,
    /// WGSL bodies (each defines `fs_main`, optionally `vs_mesh`).
    pub bodies: &'static [&'static str],
    pub encode: EncodeFn,
}

/// One registered item.
pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub kind: FxKind,
    pub description: &'static str,
    pub model: &'static str,
    pub c: &'static [f32],
    pub keys: &'static [Key],
}

pub const COMMON: &str = include_str!("../shaders/art4/common.wgsl");

/// Shader sources of a model (built once, then shared).
pub fn shaders(m: &'static Model) -> &'static [ShaderSrc] {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static [ShaderSrc]>>> = OnceLock::new();
    let mut map = CACHE.get_or_init(Default::default).lock().unwrap();
    map.entry(m.name).or_insert_with(|| {
        let v: Vec<ShaderSrc> = m
            .bodies
            .iter()
            .enumerate()
            .map(|(i, b)| ShaderSrc::new(format!("jb.art4.{}#{i}", m.name), format!("{COMMON}\n{b}")))
            .collect();
        Box::leak(v.into_boxed_slice())
    })
}

/// Keyframe row at progress `p` (linear between rows, held at the ends).
pub fn key_at(keys: &[Key], p: f32) -> [f32; NK] {
    match keys {
        [] => [0.0; NK],
        [k] => k.v,
        _ => {
            if p <= keys[0].p {
                return keys[0].v;
            }
            for w in keys.windows(2) {
                if p < w[1].p {
                    let t = (p - w[0].p) / (w[1].p - w[0].p).max(1e-6);
                    let mut o = [0.0; NK];
                    for i in 0..NK {
                        o[i] = w[0].v[i] + (w[1].v[i] - w[0].v[i]) * t;
                    }
                    return o;
                }
            }
            keys[keys.len() - 1].v
        }
    }
}

pub struct Art4Fx {
    pub info: FxInfo,
    pub spec: &'static Spec,
    pub model: &'static Model,
}

impl Art4Fx {
    pub fn new(spec: &'static Spec) -> Self {
        let model = crate::fx::art4_models::model(spec.model).unwrap_or_else(|| panic!("art4 model {}", spec.model));
        Self { info: info(spec.id, spec.name, spec.category, spec.kind, spec.description, &[]), spec, model }
    }

    /// Shader `i` of the model.
    pub fn shader(&self, i: usize) -> &'static ShaderSrc {
        &shaders(self.model)[i]
    }

    /// One full-screen pass of shader `i` reading `a` (and `b`) into `out`.
    pub fn pass(&self, cx: &mut FxCx<'_>, i: usize, a: &Tex, b: Option<&Tex>, aux: Option<&Tex>, out: &Tex, u: Uniforms) {
        let mut p = Pass::full(self.shader(i), out, u);
        p.a = Bind::Tex(a);
        p.b = Bind::Tex(b.unwrap_or(a));
        p.aux = aux.map_or(Bind::None, Bind::Tex);
        cx.draw(p);
    }
}

impl Effect for Art4Fx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let mut u = Uniforms::default();
        u.progress = io.progress;
        u.time = io.time;
        for (i, v) in self.spec.c.iter().take(NC).enumerate() {
            u.p[i / 4][i % 4] = *v;
        }
        let k = key_at(self.spec.keys, io.progress);
        for (i, v) in k.iter().enumerate() {
            u.p[16 + i / 4][i % 4] = *v;
        }
        (self.model.encode)(self, cx, io, u)
    }
}

/// The default encoder: one full-screen pass of shader 0 over A (and B).
pub fn encode_single(fx: &Art4Fx, cx: &mut FxCx<'_>, io: &FxIo<'_>, u: Uniforms) {
    fx.pass(cx, 0, io.a, io.b, None, io.out, u);
}

pub fn register(r: &mut Registry, specs: &'static [Spec]) {
    for s in specs {
        r.register(Art4Fx::new(s));
    }
}

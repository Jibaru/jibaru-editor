//! Fitted color looks: 3D LUTs (17³) measured from input → output pixel
//! pairs of reference exports (plain source frames against the same frames
//! with the look applied), optionally followed by a low-resolution spatial
//! gain/offset field (vignettes, light maps).
//!
//! The LUT data lives in `lane4/luts/<name>.lut` (17³ RGB triplets, u8,
//! red varying fastest) and the optional field in `lane4/luts/<name>.field`
//! (16×9 nodes by default, or a `u32` width/height header, at cell centres,
//! gain block then offset block, RGB f32 little
//! endian, bilinear in between: `out = gain(uv) · lut(in) + offset(uv)`).
//! The LUT was solved by regularized least squares over flat-region pixel
//! pairs of a test card and a natural scene, with a second-difference
//! smoothness prior so colors outside the measured gamut extrapolate
//! linearly. See `docs/fidelity-lane4.md` for the method and the measured
//! error of every look.

use crate::fx::Bind;
use crate::fx::{info, param_accessors, AuxImage, FxCx, Pass, ShaderSrc, Uniforms};
use crate::lut::Lut3d;
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec as P, Registry};
use crate::text::f32_to_f16;

/// LUT resolution of every fitted look.
pub const LUT_N: u32 = 17;
/// Default node counts of the spatial gain/offset field.
pub const FIELD_W: u32 = 16;
pub const FIELD_H: u32 = 9;

/// One fitted look.
pub struct LookSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: FxKind,
    pub category: &'static str,
    pub description: &'static str,
    /// 17³ RGB u8 triplets, red fastest (None: identity).
    pub lut: Option<&'static [u8]>,
    /// Spatial gain/offset field blob (see the module docs).
    pub field: Option<&'static [u8]>,
    /// The frame repeated `tiles × tiles` times (video wall), 1 = off.
    pub tiles: u32,
    /// Horizontal line pattern: period in 1080p rows and 16 gain samples
    /// over one period (multiplies the look's output).
    pub lines: Option<(f32, [f32; 16])>,
}

const BODY: &str = r#"
// Bilinear lookup of the field block starting at row `row0` (gain 0,
// offset at row `fh`); the field has `fw × fh` nodes (u.e1.xy).
fn l4_field(uv: vec2f, row0: i32) -> vec3f {
    let fs = u.e1.xy;
    let p = clamp(uv * fs - 0.5, vec2f(0.0), fs - 1.0);
    let i = min(vec2i(floor(p)), vec2i(fs) - 2);
    let f = p - vec2f(i);
    let a = textureLoad(tx, vec2i(i.x, row0 + i.y), 0).rgb;
    let b = textureLoad(tx, vec2i(i.x + 1, row0 + i.y), 0).rgb;
    let c = textureLoad(tx, vec2i(i.x, row0 + i.y + 1), 0).rgb;
    let d = textureLoad(tx, vec2i(i.x + 1, row0 + i.y + 1), 0).rgb;
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
fn fx(uv: vec2f) -> vec4f {
    var d = to_disp(sa(uv));
    if (u.e0.z > 1.5) {
        d = to_disp(sa(fract(uv * u.e0.z)));
    }
    var c = clamp(d.rgb, vec3f(0.0), vec3f(1.0));
    let n = u.e0.x;
    if (n > 1.5) {
        c = textureSampleLevel(tlut, smp, c * ((n - 1.0) / n) + 0.5 / n, 0.0).rgb;
    }
    if (u.e0.y > 0.5) {
        c = clamp(c * l4_field(uv, 0) + l4_field(uv, i32(u.e1.y)), vec3f(0.0), vec3f(1.0));
    }
    if (u.e1.z > 0.0) {
        let x = fract(uv.y * 1080.0 / u.e1.z) * 16.0 - 0.5;
        let i0 = i32(floor(x));
        let f = x - f32(i0);
        let a = u.p[16 + ((i0 + 16) % 16)].x;
        let b = u.p[16 + ((i0 + 17) % 16)].x;
        c = clamp(c * mix(a, b, f), vec3f(0.0), vec3f(1.0));
    }
    let k = clamp(p_intensity() / 100.0, 0.0, 1.0);
    return from_disp(vec4f(mix(d.rgb, c, k), d.a));
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f { return fx(in.uv); }
"#;

const PARAMS: &[P] = &[P::float("intensity", "Intensity", 0.0, 100.0, 100.0).unit("%")];

fn shader_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.l4.look", format!("{}\n{BODY}", param_accessors(PARAMS))))
}

/// Expands a u8 LUT blob into a [`Lut3d`].
pub fn lut_from_bytes(bytes: &[u8]) -> Lut3d {
    let n = LUT_N as usize;
    assert_eq!(bytes.len(), n * n * n * 3, "fitted LUT size");
    let data = bytes.chunks_exact(3).map(|c| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]).collect();
    Lut3d { size: LUT_N, data }
}

/// Node counts of a field blob: 16 × 9 without a header, otherwise two
/// little-endian u32 (width, height) before the data.
pub fn field_dims(bytes: &[u8]) -> (u32, u32, &[u8]) {
    if bytes.len() == (FIELD_W * FIELD_H * 2 * 12) as usize {
        return (FIELD_W, FIELD_H, bytes);
    }
    let w = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let h = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    (w, h, &bytes[8..])
}

/// Field blob (gain block then offset block, RGB f32) -> RGBA16F image of
/// `w × 2h` texels.
pub fn field_image(bytes: &[u8]) -> AuxImage {
    let (fw, fh, bytes) = field_dims(bytes);
    let n = (fw * fh * 2) as usize;
    assert_eq!(bytes.len(), n * 12, "fitted field size");
    let mut data = Vec::with_capacity(n * 8);
    for t in bytes.chunks_exact(12) {
        for k in 0..4 {
            let v = if k < 3 { f32::from_le_bytes([t[k * 4], t[k * 4 + 1], t[k * 4 + 2], t[k * 4 + 3]]) } else { 1.0 };
            data.extend_from_slice(&f32_to_f16(v).to_le_bytes());
        }
    }
    AuxImage { width: fw, height: 2 * fh, depth: 1, format: wgpu::TextureFormat::Rgba16Float, data }
}

/// Records the look pass of `spec` from `a` into `out` with intensity 0..100.
pub fn encode_look(cx: &mut FxCx<'_>, spec: &LookSpec, a: &crate::pool::Tex, out: &crate::pool::Tex, intensity: f32) {
    let mut u = Uniforms::default();
    u.p[0][0] = intensity;
    let lut = spec.lut.map(|bytes| cx.aux_texture(&format!("l4lut:{}", spec.id), || lut_from_bytes(bytes).to_aux()));
    u.e[0][0] = if lut.is_some() { LUT_N as f32 } else { 0.0 };
    let field = spec.field.map(|bytes| cx.aux_texture(&format!("l4field:{}", spec.id), || field_image(bytes)));
    u.e[0][1] = if field.is_some() { 1.0 } else { 0.0 };
    let (fw, fh) = spec.field.map_or((FIELD_W, FIELD_H), |b| {
        let (w, h, _) = field_dims(b);
        (w, h)
    });
    u.e[1] = [fw as f32, fh as f32, 0.0, 0.0];
    if let Some((period, g)) = spec.lines {
        u.e[1][2] = period;
        for (i, v) in g.iter().enumerate() {
            u.p[16 + i] = [*v, 0.0, 0.0, 0.0];
        }
    }
    let small;
    let mut src = a;
    if spec.tiles > 1 {
        // Box-filtered downscale first, so the tiles do not alias.
        u.e[0][2] = spec.tiles as f32;
        let n = spec.tiles;
        let (w, h) = ((a.width / n).max(1), (a.height / n).max(1));
        let mut cur = a.clone();
        while cur.width / 2 >= w {
            let half = cx.temp((cur.width / 2).max(1), (cur.height / 2).max(1));
            cx.copy(&cur, &half);
            cur = half;
        }
        small = cx.temp(w, h);
        cx.copy(&cur, &small);
        src = &small;
    }
    let mut pass = Pass::full(shader_src(), out, u);
    pass.a = Bind::Tex(src);
    pass.lut = lut.as_ref();
    if let Some(v) = &field {
        pass.aux = Bind::View(v, [fw, 2 * fh]);
    }
    cx.draw(pass);
}

pub struct LookFx {
    info: FxInfo,
    spec: &'static LookSpec,
}

impl LookFx {
    pub fn new(spec: &'static LookSpec) -> Self {
        Self { info: info(spec.id, spec.name, spec.category, spec.kind, spec.description, PARAMS), spec }
    }
}

impl Effect for LookFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        encode_look(cx, self.spec, io.a, io.out, io.params.get("intensity"));
    }
}

include!("lane4/looks_table.rs");

pub fn register(r: &mut Registry) {
    for spec in LOOKS {
        r.register(LookFx::new(spec));
    }
}

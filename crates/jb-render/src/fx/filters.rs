//! Color looks (Filmora "Filters" tab, kind = Filter).
//!
//! Every look is a declarative recipe (white balance, channel mixer, color
//! balance, curves, eq, vibrance, vignette) evaluated by one shared shader.
//! The single `intensidad` slider (0..100) scales every adjustment toward
//! neutral, so 0 is the untouched clip and 100 the full look. The recipe
//! semantics follow the equivalent ffmpeg filters (colortemperature,
//! colorchannelmixer, colorbalance, curves, eq, vibrance, vignette) so looks
//! match the prototype renderer.

use super::{param_accessors, FxCx, ShaderSrc, Uniforms};
use crate::fx::{AuxImage, Bind, Pass};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec, Registry};
use crate::text::f32_to_f16;

const IDENT_MIX: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
const MIX_KEYS: [&str; 9] = ["rr", "rg", "rb", "gr", "gg", "gb", "br", "bg", "bb"];
const BAL_KEYS: [&str; 9] = ["rs", "gs", "bs", "rm", "gm", "bm", "rh", "gh", "bh"];

/// One look recipe (ported from the prototype's `_Look`).
#[derive(Clone)]
pub struct Look {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    /// Kelvin (6500 = neutral), None = no white-balance step.
    temp: Option<f32>,
    mixer: Option<[f32; 9]>,
    balance: [f32; 9],
    /// Curves per channel: index 0 = r, 1 = g, 2 = b, 3 = master.
    curves: [Option<Vec<(f32, f32)>>; 4],
    contrast: f32,
    brightness: f32,
    saturation: f32,
    gamma: f32,
    vibrance: f32,
    vignette: f32,
}

impl Look {
    fn new(id: &'static str, name: &'static str, category: &'static str, description: &'static str) -> Self {
        Self {
            id,
            name,
            category,
            description,
            temp: None,
            mixer: None,
            balance: [0.0; 9],
            curves: [None, None, None, None],
            contrast: 1.0,
            brightness: 0.0,
            saturation: 1.0,
            gamma: 1.0,
            vibrance: 0.0,
            vignette: 0.0,
        }
    }
    fn temp(mut self, k: f32) -> Self {
        self.temp = Some(k);
        self
    }
    fn mixer(mut self, kv: &[(&str, f32)]) -> Self {
        let mut m = IDENT_MIX;
        for (k, v) in kv {
            m[MIX_KEYS.iter().position(|x| x == k).expect("mixer key")] = *v;
        }
        self.mixer = Some(m);
        self
    }
    fn balance(mut self, kv: &[(&str, f32)]) -> Self {
        for (k, v) in kv {
            self.balance[BAL_KEYS.iter().position(|x| x == k).expect("balance key")] = *v;
        }
        self
    }
    fn curve(mut self, ch: char, pts: &[(f32, f32)]) -> Self {
        let i = match ch {
            'r' => 0,
            'g' => 1,
            'b' => 2,
            _ => 3,
        };
        self.curves[i] = Some(pts.to_vec());
        self
    }
    fn contrast(mut self, v: f32) -> Self {
        self.contrast = v;
        self
    }
    fn bright(mut self, v: f32) -> Self {
        self.brightness = v;
        self
    }
    fn sat(mut self, v: f32) -> Self {
        self.saturation = v;
        self
    }
    fn gamma(mut self, v: f32) -> Self {
        self.gamma = v;
        self
    }
    fn vib(mut self, v: f32) -> Self {
        self.vibrance = v;
        self
    }
    fn vig(mut self, v: f32) -> Self {
        self.vignette = v;
        self
    }
}

/// Blackbody color of a temperature in Kelvin (Tanner Helland fit, as ffmpeg's colortemperature).
pub(crate) fn kelvin_rgb(k: f32) -> [f32; 3] {
    let t = k / 100.0;
    let r = if t <= 66.0 { 1.0 } else { 1.292_936_2 * (t - 60.0).powf(-0.133_204_76) };
    let g = if t <= 66.0 { 0.390_081_6 * t.ln() - 0.631_841_4 } else { 1.129_890_9 * (t - 60.0).powf(-0.075_514_85) };
    let b = if t >= 66.0 {
        1.0
    } else if t <= 19.0 {
        0.0
    } else {
        0.543_206_8 * (t - 10.0).ln() - 1.196_254_1
    };
    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Natural cubic spline through `pts` (sorted by x), flat outside, sampled at `n` points over 0..1.
pub(crate) fn natural_spline(pts: &[(f32, f32)], n: usize) -> Vec<f32> {
    let mut p: Vec<(f64, f64)> = pts.iter().map(|&(x, y)| (x as f64, y as f64)).collect();
    p.sort_by(|a, b| a.0.total_cmp(&b.0));
    p.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
    let sample = |i: usize| i as f64 / (n - 1) as f64;
    if p.is_empty() {
        return (0..n).map(|i| sample(i) as f32).collect();
    }
    if p.len() == 1 {
        return vec![p[0].1 as f32; n];
    }
    let m = p.len();
    let h: Vec<f64> = (0..m - 1).map(|i| p[i + 1].0 - p[i].0).collect();
    // Second derivatives (natural boundary) via the tridiagonal algorithm.
    let mut sd = vec![0f64; m];
    if m > 2 {
        let k = m - 2;
        let (mut a, mut b, mut c, mut d) = (vec![0f64; k], vec![0f64; k], vec![0f64; k], vec![0f64; k]);
        for j in 0..k {
            let i = j + 1;
            a[j] = h[i - 1];
            b[j] = 2.0 * (h[i - 1] + h[i]);
            c[j] = h[i];
            d[j] = 6.0 * ((p[i + 1].1 - p[i].1) / h[i] - (p[i].1 - p[i - 1].1) / h[i - 1]);
        }
        for j in 1..k {
            let w = a[j] / b[j - 1];
            b[j] -= w * c[j - 1];
            d[j] -= w * d[j - 1];
        }
        let mut x = vec![0f64; k];
        x[k - 1] = d[k - 1] / b[k - 1];
        for j in (0..k - 1).rev() {
            x[j] = (d[j] - c[j] * x[j + 1]) / b[j];
        }
        sd[1..(k + 1)].copy_from_slice(&x[..k]);
    }
    (0..n)
        .map(|s| {
            let x = sample(s);
            let y = if x <= p[0].0 {
                p[0].1
            } else if x >= p[m - 1].0 {
                p[m - 1].1
            } else {
                let i = (0..m - 1).find(|&i| x < p[i + 1].0).unwrap_or(m - 2);
                let (x0, x1, y0, y1) = (p[i].0, p[i + 1].0, p[i].1, p[i + 1].1);
                let hh = h[i];
                let (ta, tb) = ((x1 - x) / hh, (x - x0) / hh);
                ta * y0 + tb * y1 + ((ta.powi(3) - ta) * sd[i] + (tb.powi(3) - tb) * sd[i + 1]) * hh * hh / 6.0
            };
            y.clamp(0.0, 1.0) as f32
        })
        .collect()
}

/// 256x1 RGBA16F curve texture: r, g, b channel curves and master in alpha.
pub(crate) fn curve_image(curves: [&[f32]; 4]) -> AuxImage {
    let mut data = Vec::with_capacity(256 * 8);
    for i in 0..256 {
        for c in curves {
            data.extend_from_slice(&f32_to_f16(c[i]).to_le_bytes());
        }
    }
    AuxImage { width: 256, height: 1, depth: 1, format: wgpu::TextureFormat::Rgba16Float, data }
}

pub(crate) fn identity_curve() -> Vec<f32> {
    (0..256).map(|i| i as f32 / 255.0).collect()
}

const LOOK_WGSL: &str = r#"
// p[1] = (temp r, g, b, has temp)  p[2..5] = mixer rows  p[5..8] = balance shadows/mid/highlights
// p[8] = eq (contrast, brightness, saturation, gamma)  p[9] = (vibrance, vignette, has curves, has mixer)
fn curve(v: vec3f) -> vec4f {
    let n = 256.0;
    let c = clamp(v, vec3f(0.0), vec3f(1.0)) * ((n - 1.0) / n) + 0.5 / n;
    return vec4f(sx(vec2f(c.r, 0.5)).r, sx(vec2f(c.g, 0.5)).g, sx(vec2f(c.b, 0.5)).b, 0.0);
}
fn master(v: vec3f) -> vec3f {
    let n = 256.0;
    let c = clamp(v, vec3f(0.0), vec3f(1.0)) * ((n - 1.0) / n) + 0.5 / n;
    return vec3f(sx(vec2f(c.r, 0.5)).a, sx(vec2f(c.g, 0.5)).a, sx(vec2f(c.b, 0.5)).a);
}
fn bal_comp(v: f32, l: f32, s: f32, m: f32, h: f32) -> f32 {
    let a = 4.0;
    let b = 0.333;
    let scale = 0.7;
    let ss = s * clamp((b - l) * a + 0.5, 0.0, 1.0) * scale;
    let mm = m * clamp((l - b) * a + 0.5, 0.0, 1.0) * clamp((1.0 - l - b) * a + 0.5, 0.0, 1.0) * scale;
    let hh = h * clamp((l + b - 1.0) * a + 0.5, 0.0, 1.0) * scale;
    return clamp(v + ss + mm + hh, 0.0, 1.0);
}
fn fx(uv: vec2f) -> vec4f {
    let k = clamp(p_intensidad() / 100.0, 0.0, 1.0);
    let d = to_disp(sa(uv));
    var c = clamp(d.rgb, vec3f(0.0), vec3f(1.0));
    if (k <= 0.0) { return sa(uv); }
    if (u.p[1].w > 0.5) {
        var n = c * u.p[1].rgb;
        let l0 = max(c.r, max(c.g, c.b)) + min(c.r, min(c.g, c.b));
        let l1 = max(n.r, max(n.g, n.b)) + min(n.r, min(n.g, n.b));
        let np = n * (l0 / max(l1, 1e-6));
        n = mix(n, np, 0.6);
        c = mix(c, n, min(1.0, k));
    }
    if (u.p[9].w > 0.5) {
        let ident = mat3x3f(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0));
        let rows = mat3x3f(u.p[2].xyz, u.p[3].xyz, u.p[4].xyz);
        let m = transpose(ident + (rows - ident) * k);
        c = clamp(m * c, vec3f(0.0), vec3f(1.0));
    }
    let s = clamp(u.p[5].xyz * k, vec3f(-1.0), vec3f(1.0));
    let mi = clamp(u.p[6].xyz * k, vec3f(-1.0), vec3f(1.0));
    let hi = clamp(u.p[7].xyz * k, vec3f(-1.0), vec3f(1.0));
    if (any(s != vec3f(0.0)) || any(mi != vec3f(0.0)) || any(hi != vec3f(0.0))) {
        let l = 0.5 * (max(c.r, max(c.g, c.b)) + min(c.r, min(c.g, c.b)));
        c = vec3f(bal_comp(c.r, l, s.r, mi.r, hi.r), bal_comp(c.g, l, s.g, mi.g, hi.g), bal_comp(c.b, l, s.b, mi.b, hi.b));
    }
    if (u.p[9].z > 0.5) {
        c = mix(c, curve(c).rgb, k);
        c = mix(c, master(c), k);
    }
    let eq = vec4f(1.0) + (u.p[8] - vec4f(1.0, 0.0, 1.0, 1.0)) * k - vec4f(0.0, 1.0, 0.0, 0.0);
    // eq works on luma (contrast/brightness/gamma) and chroma (saturation).
    let y = luma(c);
    var y2 = (y - 0.5) * eq.x + 0.5 + eq.y;
    y2 = pow(clamp(y2, 0.0, 1.0), 1.0 / max(eq.w, 1e-3));
    c = vec3f(y2) + (c - vec3f(y)) * eq.z;
    c = clamp(c, vec3f(0.0), vec3f(1.0));
    let vib = clamp(u.p[9].x * k, -2.0, 2.0);
    if (vib != 0.0) {
        let sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
        let l = luma(c);
        let sc = 1.0 + vib * (1.0 - sign(vib) * sat);
        c = clamp(mix(vec3f(l), c, sc), vec3f(0.0), vec3f(1.0));
    }
    if (u.p[9].y > 0.0) {
        let ang = 0.15 + 0.5 * u.p[9].y * k;
        let dn = length(to_c(uv)) / length(vec2f(0.5 * aspect(), 0.5));
        let f = cos(ang * dn);
        c = c * (f * f * f * f);
    }
    return from_disp(vec4f(c, d.a));
}
"#;

const PARAMS: &[ParamSpec] = &[ParamSpec::float("intensidad", "Intensity", 0.0, 100.0, 70.0).unit("%")];

fn shader_src() -> &'static ShaderSrc {
    static S: std::sync::OnceLock<ShaderSrc> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        ShaderSrc::new(
            "jb.filter.look",
            format!(
                "{}\n{LOOK_WGSL}\n@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return fx(in.uv); }}\n",
                param_accessors(PARAMS)
            ),
        )
    })
}

/// A look effect: shared shader + per-look uniforms and curve texture.
pub struct LookFx {
    info: FxInfo,
    look: Look,
}

impl Effect for LookFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let l = &self.look;
        let mut u = Uniforms::with_params(io.params);
        u.progress = io.progress;
        u.time = io.time;
        if let Some(t) = l.temp {
            let c = kelvin_rgb(t);
            u.p[1] = [c[0], c[1], c[2], 1.0];
        }
        let m = l.mixer.unwrap_or(IDENT_MIX);
        for row in 0..3 {
            u.p[2 + row] = [m[row * 3], m[row * 3 + 1], m[row * 3 + 2], 0.0];
        }
        let b = l.balance;
        // Rows: shadows, midtones, highlights; columns r, g, b.
        u.p[5] = [b[0], b[1], b[2], 0.0];
        u.p[6] = [b[3], b[4], b[5], 0.0];
        u.p[7] = [b[6], b[7], b[8], 0.0];
        u.p[8] = [l.contrast, l.brightness, l.saturation, l.gamma];
        let has_curves = l.curves.iter().any(|c| c.is_some());
        u.p[9] = [l.vibrance, l.vignette, has_curves as u32 as f32, l.mixer.is_some() as u32 as f32];
        let aux = if has_curves {
            Some(cx.aux_texture(&format!("look-curves:{}", l.id), || {
                let id = identity_curve();
                let ch: Vec<Vec<f32>> = l
                    .curves
                    .iter()
                    .map(|c| c.as_ref().map_or_else(|| id.clone(), |p| natural_spline(p, 256)))
                    .collect();
                curve_image([&ch[0], &ch[1], &ch[2], &ch[3]])
            }))
        } else {
            None
        };
        let mut p = Pass::full(shader_src(), io.out, u);
        p.a = Bind::Tex(io.a);
        if let Some(v) = &aux {
            p.aux = Bind::View(v, [256, 1]);
        }
        cx.draw(p);
    }
}

fn looks() -> Vec<Look> {
    const BW: &[(&str, f32)] = &[
        ("rr", 0.3),
        ("rg", 0.59),
        ("rb", 0.11),
        ("gr", 0.3),
        ("gg", 0.59),
        ("gb", 0.11),
        ("br", 0.3),
        ("bg", 0.59),
        ("bb", 0.11),
    ];
    vec![
        // ── Cinema
        Look::new(
            "jb.teal_orange",
            "Teal & Orange",
            "Cinema",
            "Teal shadows and orange skin tones, the blockbuster look.",
        )
        .balance(&[
            ("rs", -0.25),
            ("gs", 0.04),
            ("bs", 0.22),
            ("rm", 0.1),
            ("gm", -0.02),
            ("bm", -0.1),
            ("rh", 0.18),
            ("gh", -0.02),
            ("bh", -0.2),
        ])
        .curve('m', &[(0.0, 0.02), (0.25, 0.2), (0.75, 0.8), (1.0, 1.0)])
        .sat(1.2)
        .vib(0.3),
        Look::new(
            "jb.cine_frio",
            "Cinematic Cold",
            "Cinema",
            "Thriller look: cold tones, strong contrast, less saturation.",
        )
        .temp(10500.0)
        .balance(&[("bs", 0.2), ("gs", 0.05), ("bm", 0.08)])
        .curve('m', &[(0.0, 0.0), (0.25, 0.19), (0.75, 0.82), (1.0, 1.0)])
        .sat(0.75)
        .vig(0.4),
        Look::new("jb.cine_calido", "Cinematic Warm", "Cinema", "Warm golden cinematic look with deep blacks.")
            .temp(3600.0)
            .balance(&[("rs", 0.05), ("bs", -0.1), ("rh", 0.1), ("bh", -0.12)])
            .curve('m', &[(0.0, 0.0), (0.25, 0.2), (0.75, 0.8), (1.0, 1.0)])
            .sat(1.1)
            .vig(0.4),
        Look::new("jb.moody", "Moody", "Cinema", "Dark and desaturated, with muted greenish shadows.")
            .balance(&[("rs", -0.04), ("gs", 0.1), ("bs", 0.1), ("gm", 0.03), ("rh", 0.06), ("bh", -0.04)])
            .curve('m', &[(0.0, 0.04), (0.3, 0.2), (0.7, 0.6), (1.0, 0.88)])
            .sat(0.55)
            .gamma(0.9)
            .vig(0.6),
        Look::new("jb.matte", "Faded Matte", "Cinema", "Lifted blacks and dimmed whites, a matte film finish.")
            .curve('m', &[(0.0, 0.16), (0.3, 0.33), (0.7, 0.7), (1.0, 0.9)])
            .balance(&[("bs", 0.05), ("rh", 0.03)])
            .sat(0.8),
        Look::new(
            "jb.bleach_bypass",
            "Bleach Bypass",
            "Cinema",
            "Hard contrast and washed-out colors, like skip-bleach film.",
        )
        .curve('m', &[(0.0, 0.0), (0.2, 0.1), (0.5, 0.52), (0.8, 0.92), (1.0, 1.0)])
        .sat(0.4)
        .contrast(1.1)
        .balance(&[("bs", 0.04)]),
        Look::new("jb.kodak", "Kodak Film", "Cinema", "Kodak film emulation: warm, rich reds, soft shadows.")
            .temp(5200.0)
            .balance(&[
                ("rs", 0.05),
                ("bs", -0.05),
                ("rm", 0.06),
                ("gm", 0.02),
                ("bm", -0.06),
                ("rh", 0.06),
                ("bh", -0.08),
            ])
            .curve('m', &[(0.0, 0.05), (0.25, 0.22), (0.75, 0.8), (1.0, 0.97)])
            .curve('r', &[(0.0, 0.0), (0.5, 0.54), (1.0, 1.0)])
            .sat(1.15),
        Look::new(
            "jb.fuji",
            "Fuji Film",
            "Cinema",
            "Fuji film emulation: strong greens and cyans, neutral skin, cool shadows.",
        )
        .balance(&[("rs", -0.06), ("gs", 0.05), ("bs", 0.08), ("gm", 0.05), ("bm", 0.02), ("rh", 0.02), ("gh", 0.02)])
        .curve('m', &[(0.0, 0.04), (0.25, 0.21), (0.75, 0.79), (1.0, 0.98)])
        .curve('g', &[(0.0, 0.0), (0.5, 0.53), (1.0, 1.0)])
        .sat(1.1)
        .vib(0.25),
        // ── Vintage
        Look::new(
            "jb.vintage_70s",
            "Vintage 70s",
            "Vintage",
            "Warm yellows, olive greens and faded blacks of the 70s.",
        )
        .temp(4600.0)
        .mixer(&[("rr", 0.95), ("rg", 0.1), ("gg", 0.92), ("gr", 0.08), ("bb", 0.75), ("bg", 0.05)])
        .balance(&[("gs", 0.05), ("bs", -0.04), ("rh", 0.05), ("gh", 0.06), ("bh", -0.1)])
        .curve('m', &[(0.0, 0.12), (0.5, 0.5), (1.0, 0.92)])
        .curve('b', &[(0.0, 0.06), (1.0, 0.82)])
        .sat(0.85)
        .vig(0.4),
        Look::new("jb.retro_90s", "Retro 90s", "Vintage", "90s camcorder colors: magentas, cyans and high contrast.")
            .balance(&[("rs", 0.08), ("bs", 0.12), ("gm", -0.08), ("rh", 0.06), ("bh", 0.06)])
            .curve('m', &[(0.0, 0.06), (0.25, 0.2), (0.75, 0.82), (1.0, 0.96)])
            .sat(1.35)
            .contrast(1.05),
        Look::new(
            "jb.polaroid",
            "Polaroid",
            "Vintage",
            "Instant photo: creamy highlights, cyan shadows and pastel colors.",
        )
        .curve('m', &[(0.0, 0.1), (0.5, 0.55), (1.0, 0.95)])
        .curve('r', &[(0.0, 0.04), (1.0, 1.0)])
        .curve('g', &[(0.0, 0.06), (1.0, 0.97)])
        .curve('b', &[(0.0, 0.16), (1.0, 0.82)])
        .balance(&[("rs", -0.15), ("gs", 0.04), ("bs", 0.12), ("rh", 0.12), ("gh", 0.08), ("bh", -0.06)])
        .sat(0.85)
        .bright(0.02),
        Look::new("jb.sepia_suave", "Soft Sepia", "Vintage", "Delicate sepia tone of an old photograph.")
            .mixer(&[
                ("rr", 0.33),
                ("rg", 0.65),
                ("rb", 0.13),
                ("gr", 0.26),
                ("gg", 0.52),
                ("gb", 0.1),
                ("br", 0.16),
                ("bg", 0.32),
                ("bb", 0.06),
            ])
            .curve('m', &[(0.0, 0.06), (0.5, 0.56), (1.0, 1.0)]),
        // ── Black & White
        Look::new(
            "jb.bn_alto_contraste",
            "B&W High Contrast",
            "Black & White",
            "Black and white with pure blacks and bright whites.",
        )
        .mixer(BW)
        .curve('m', &[(0.0, 0.0), (0.25, 0.12), (0.5, 0.5), (0.75, 0.88), (1.0, 1.0)])
        .contrast(1.1),
        Look::new("jb.bn_suave", "B&W Soft", "Black & White", "Black and white with soft greys and lifted blacks.")
            .mixer(BW)
            .curve('m', &[(0.0, 0.1), (0.5, 0.52), (1.0, 0.92)]),
        Look::new("jb.noir", "Noir", "Black & White", "Film noir: hard, slightly cold monochrome with a vignette.")
            .mixer(&[
                ("rr", 0.2),
                ("rg", 0.7),
                ("rb", 0.1),
                ("gr", 0.2),
                ("gg", 0.7),
                ("gb", 0.1),
                ("br", 0.22),
                ("bg", 0.72),
                ("bb", 0.12),
            ])
            .curve('m', &[(0.0, 0.0), (0.3, 0.12), (0.6, 0.6), (0.85, 0.95), (1.0, 1.0)])
            .gamma(0.9)
            .vig(0.9),
        Look::new("jb.selenio", "Selenium", "Black & White", "Monochrome toned cool purple, like a selenium print.")
            .mixer(&[
                ("rr", 0.3),
                ("rg", 0.59),
                ("rb", 0.11),
                ("gr", 0.27),
                ("gg", 0.55),
                ("gb", 0.1),
                ("br", 0.33),
                ("bg", 0.62),
                ("bb", 0.14),
            ])
            .curve('m', &[(0.0, 0.03), (0.5, 0.48), (1.0, 0.97)]),
        // ── Warm
        Look::new("jb.atardecer", "Warm Sunset", "Warm", "Intense sunset oranges and pinks.")
            .temp(3000.0)
            .balance(&[("rs", 0.1), ("bs", 0.06), ("rm", 0.1), ("gm", -0.03), ("rh", 0.12), ("gh", 0.02), ("bh", -0.1)])
            .sat(1.25)
            .vib(0.3),
        Look::new("jb.dorado", "Golden Hour", "Warm", "Soft, bright golden light of the magic hour.")
            .temp(4200.0)
            .balance(&[("rm", 0.06), ("gm", 0.04), ("bm", -0.12), ("rh", 0.08), ("gh", 0.06), ("bh", -0.15)])
            .curve('m', &[(0.0, 0.03), (0.5, 0.56), (1.0, 1.0)])
            .sat(1.12),
        // ── Cool
        Look::new("jb.hora_azul", "Blue Hour", "Cool", "Deep blue-hour blues and cold shadows.")
            .temp(14000.0)
            .balance(&[("rs", -0.08), ("bs", 0.18), ("bm", 0.08), ("bh", 0.05)])
            .curve('m', &[(0.0, 0.02), (0.5, 0.46), (1.0, 0.97)])
            .sat(1.05),
        Look::new("jb.nordico", "Nordic", "Cool", "Scandinavian: cold, clean, bright and low saturation.")
            .temp(8500.0)
            .curve('m', &[(0.0, 0.1), (0.5, 0.66), (1.0, 1.0)])
            .balance(&[("bs", 0.06), ("gs", 0.02)])
            .sat(0.55)
            .bright(0.05),
        Look::new("jb.hielo", "Ice", "Cool", "Icy cyan and bright whites.")
            .balance(&[
                ("rs", -0.3),
                ("gs", 0.08),
                ("bs", 0.3),
                ("rm", -0.25),
                ("gm", 0.05),
                ("bm", 0.15),
                ("rh", -0.12),
                ("gh", 0.02),
                ("bh", 0.08),
            ])
            .curve('m', &[(0.0, 0.05), (0.5, 0.58), (1.0, 1.0)])
            .sat(1.1),
        // ── Travel
        Look::new("jb.tropical", "Tropical Vivid", "Travel", "Turquoise cyan, intense greens and tanned skin.")
            .balance(&[("rs", -0.05), ("gs", 0.03), ("bs", 0.08), ("rh", 0.06), ("bh", -0.04)])
            .curve('m', &[(0.0, 0.0), (0.25, 0.22), (0.75, 0.8), (1.0, 1.0)])
            .sat(1.4)
            .vib(0.6),
        Look::new("jb.bosque", "Forest Green", "Travel", "Deep greens and moss, dark shadows.")
            .mixer(&[("rr", 0.9), ("gg", 1.08), ("gr", 0.04), ("bb", 0.88), ("bg", 0.08)])
            .balance(&[("rs", -0.05), ("gs", 0.08), ("gm", 0.06), ("bm", -0.04), ("rh", 0.04)])
            .curve('m', &[(0.0, 0.02), (0.3, 0.24), (0.7, 0.68), (1.0, 0.97)])
            .sat(1.1),
        Look::new("jb.playa", "Beach", "Travel", "Bright and airy: warm sand and turquoise sea.")
            .temp(5200.0)
            .balance(&[("rs", -0.08), ("gs", 0.06), ("bs", 0.14), ("rh", 0.1), ("gh", 0.08), ("bh", -0.04)])
            .curve('m', &[(0.0, 0.06), (0.5, 0.66), (1.0, 1.0)])
            .sat(1.35)
            .vib(0.5)
            .bright(0.03),
        Look::new("jb.neon_noche", "Neon City", "Travel", "Urban neon night: magentas and cyans, deep blacks.")
            .balance(&[
                ("rs", 0.12),
                ("gs", -0.1),
                ("bs", 0.25),
                ("rm", 0.1),
                ("gm", -0.12),
                ("bm", 0.12),
                ("rh", -0.08),
                ("gh", 0.04),
                ("bh", 0.1),
            ])
            .curve('m', &[(0.0, 0.0), (0.3, 0.2), (0.75, 0.8), (1.0, 1.0)])
            .sat(1.3)
            .vig(0.5),
        // ── Portrait
        Look::new("jb.retrato_suave", "Soft Portrait", "Portrait", "Warm, luminous skin and lower contrast.")
            .temp(5600.0)
            .balance(&[("rm", 0.05), ("gm", 0.01), ("bm", -0.03), ("rh", 0.03)])
            .curve('m', &[(0.0, 0.06), (0.5, 0.56), (1.0, 0.98)])
            .contrast(0.9)
            .sat(0.95),
        Look::new("jb.pastel", "Pastel", "Portrait", "Soft washed colors, high key, pinkish shadows.")
            .curve('m', &[(0.0, 0.2), (0.5, 0.62), (1.0, 0.98)])
            .balance(&[("rs", 0.08), ("bs", 0.08), ("gh", 0.03), ("bh", 0.02)])
            .sat(0.7)
            .contrast(0.85),
        Look::new("jb.vivido", "Vivid", "Portrait", "Saturated colors and clean contrast that make the image pop.")
            .curve('m', &[(0.0, 0.0), (0.25, 0.22), (0.75, 0.82), (1.0, 1.0)])
            .sat(1.7)
            .vib(0.8)
            .contrast(1.05),
    ]
}

pub fn register(r: &mut Registry) {
    for look in looks() {
        let info = FxInfo {
            id: look.id,
            name: look.name,
            category: look.category,
            kind: FxKind::Filter,
            description: look.description,
            params: PARAMS.to_vec(),
        };
        r.register(LookFx { info, look });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_identity_and_shape() {
        let id = natural_spline(&[(0.0, 0.0), (1.0, 1.0)], 256);
        assert!(id.iter().enumerate().all(|(i, v)| (v - i as f32 / 255.0).abs() < 1e-5));
        let s = natural_spline(&[(0.0, 0.0), (0.25, 0.2), (0.75, 0.8), (1.0, 1.0)], 256);
        assert!((s[64] - 0.2).abs() < 0.01 && (s[191] - 0.8).abs() < 0.01);
        assert!(s[128] > 0.49 && s[128] < 0.51);
    }

    #[test]
    fn kelvin_neutral() {
        let c = kelvin_rgb(6600.0);
        assert!(c.iter().all(|v| *v > 0.95));
        assert_eq!(looks().len(), 28);
    }
}

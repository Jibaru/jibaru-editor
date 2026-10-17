//! Shared machinery of the transitions fitted on Filmora's exports in
//! `lane3_*.rs`: keyframe tables (piecewise-linear curves over progress,
//! emitted as WGSL arrays) and WGSL helpers for masks, grading, transforms
//! and procedural light fields.
//!
//! Every table row is `[p, v0, v1, ...]` with `p` ascending in 0..1. Two rows
//! with the same `p` make a jump; jumps are placed half a frame before the
//! frame where they take effect so that frame-time rounding never picks the
//! wrong side. Values are display-space unless noted.

use crate::fx::{info, shader, FxCx, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec, Registry};

/// Emits a keyframe table as WGSL: `fn kf_<name>(p) -> vec4f` (values 0..3)
/// and, for wider rows, `fn kf_<name>_<j>(p) -> vec4f` (values 4j..4j+3).
/// Missing values read as 0.
pub fn table<const K: usize>(name: &str, rows: &[[f32; K]]) -> String {
    assert!(K >= 2 && !rows.is_empty(), "table {name}: at least one value per row");
    let n = rows.len();
    let v4 = |r: &[f32; K], o: usize| {
        let g = |i: usize| r.get(1 + o + i).copied().unwrap_or(0.0);
        format!("vec4f({:?}, {:?}, {:?}, {:?})", g(0), g(1), g(2), g(3))
    };
    // Times are stored as vec4 (p in .x): a `var<private> array<f32, N>`
    // read back stale values on Metal (interpolation stuck on the row).
    let ps: Vec<String> = rows.iter().map(|r| format!("vec4f({:?}, 0.0, 0.0, 0.0)", r[0])).collect();
    let mut s = format!("var<private> KP_{name}: array<vec4f, {n}> = array<vec4f, {n}>({});\n", ps.join(", "));
    let halves = (K - 1).div_ceil(4);
    for h in 0..halves {
        let vals: Vec<String> = rows.iter().map(|r| v4(r, 4 * h)).collect();
        s.push_str(&format!(
            "var<private> KV{h}_{name}: array<vec4f, {n}> = array<vec4f, {n}>({});\n",
            vals.join(", ")
        ));
    }
    s.push_str(&format!(
        r#"fn kfi_{name}(p: f32) -> vec2f {{
    if (p <= KP_{name}[0].x) {{ return vec2f(0.0, 0.0); }}
    for (var i = 0; i < {last}; i = i + 1) {{
        if (p < KP_{name}[i + 1].x) {{
            return vec2f(f32(i), (p - KP_{name}[i].x) / max(KP_{name}[i + 1].x - KP_{name}[i].x, 1e-6));
        }}
    }}
    return vec2f(f32({last}), 0.0);
}}
fn kf_{name}(p: f32) -> vec4f {{
    let r = kfi_{name}(p);
    let i = i32(r.x);
    return mix(KV0_{name}[i], KV0_{name}[min(i + 1, {last})], r.y);
}}
"#,
        last = n - 1
    ));
    for h in 1..halves {
        s.push_str(&format!(
            r#"fn kf_{name}_{h}(p: f32) -> vec4f {{
    let r = kfi_{name}(p);
    let i = i32(r.x);
    return mix(KV{h}_{name}[i], KV{h}_{name}[min(i + 1, {last})], r.y);
}}
"#,
            last = n - 1
        ));
    }
    s
}

/// WGSL helpers shared by every lane 3 transition.
pub const KIT: &str = r#"
// Linear ramp of a threshold: 1 where m < t - s/2, 0 where m > t + s/2.
fn l3_ramp(m: f32, t: f32, s: f32) -> f32 {
    if (s <= 1e-5) { return select(0.0, 1.0, m < t); }
    return clamp(0.5 + (t - m) / s, 0.0, 1.0);
}
// Smooth ramp (smoothstep edge of total width s centred on t).
fn l3_sramp(m: f32, t: f32, s: f32) -> f32 {
    if (s <= 1e-5) { return select(0.0, 1.0, m < t); }
    return 1.0 - smoothstep(t - 0.5 * s, t + 0.5 * s, m);
}
fn l3_mirror(q: vec2f) -> vec2f {
    let f = q - 2.0 * floor(q * 0.5);
    return select(f, 2.0 - f, f > vec2f(1.0));
}
// Sample uv of an image scaled by s, rotated by r (radians, clockwise on
// screen) and shifted by d (uv) about the frame centre plus c (uv).
fn l3_xf(uv: vec2f, s: f32, r: f32, d: vec2f, c: vec2f) -> vec2f {
    let q = to_c(uv - d - c) ;
    return from_c(rot2(q, -r) / max(s, 1e-4)) + c;
}
fn l3_side(use_b: bool, q: vec2f) -> vec4f { return select(sa(q), sb(q), use_b); }
// Display-space straight colour of A or B at q.
fn l3_da(q: vec2f) -> vec3f { return to_disp(sa(q)).rgb; }
fn l3_db(q: vec2f) -> vec3f { return to_disp(sb(q)).rgb; }
// Display-space mix of two working-space colours.
fn l3_mix(a: vec4f, b: vec4f, w: f32) -> vec4f { return from_disp(mix(to_disp(a), to_disp(b), w)); }
fn l3_out(c: vec3f) -> vec4f { return from_disp(vec4f(clamp(c, vec3f(0.0), vec3f(1.0)), 1.0)); }
fn l3_luma(c: vec3f) -> f32 { return dot(c, vec3f(0.2126, 0.7152, 0.0722)); }
// Screen blend of a light colour (display space).
fn l3_screen(base: vec3f, light: vec3f) -> vec3f { return 1.0 - (1.0 - base) * (1.0 - clamp(light, vec3f(0.0), vec3f(1.0))); }
// Soft round glow of radius r (fraction of height) at c (uv), 1 at the centre.
fn l3_glow(uv: vec2f, c: vec2f, r: f32) -> f32 {
    let d = to_c(uv) - to_c(c);
    return exp(-dot(d, d) / max(r * r, 1e-6));
}
// Smooth noise field in 0..1 (fbm), at scale k (cells per frame height), drifting with t.
fn l3_noise(uv: vec2f, k: f32, t: vec2f) -> f32 {
    return fbm(to_c(uv) * k + t);
}
"#;

/// One transition of this lane: id, display name, category, description,
/// parameters and its WGSL (`fx(uv)` plus any tables).
pub struct Item {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub params: Vec<ParamSpec>,
    pub wgsl: String,
}

impl Item {
    pub fn new(
        id: &'static str,
        name: &'static str,
        category: &'static str,
        description: &'static str,
        wgsl: String,
    ) -> Self {
        Self { id, name, category, description, params: Vec::new(), wgsl }
    }
}

/// WGSL of the global-mix recipe: rows `[p, aA, aL, bB, bL, or, og, ob]`,
/// `out = aA A + aL luma(A) + bB B + bL luma(B) + o` in display space. It
/// covers dissolves, dips to a colour, grey/desaturated fades and uniform
/// flashes; `extra` is WGSL inserted before `fx` (helpers of a variant) and
/// `post` a statement run on `c` (display rgb) before output.
pub fn global_mix(rows: &[[f32; 8]], extra: &str, post: &str) -> String {
    format!(
        r#"{}
{extra}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let k0 = kf_g(p);
    let k1 = kf_g_1(p);
    let a = l3_da(uv);
    let b = l3_db(uv);
    var c = k0.x * a + k0.y * l3_luma(a) + k0.z * b + k0.w * l3_luma(b) + k1.xyz;
    {post}
    return l3_out(c);
}}
"#,
        table("g", rows)
    )
}

/// Global mix plus a smooth light field: rows `[p, aA, aL, bB, bL, c...]`
/// where `c` holds, per channel (r, g, b in turn), the coefficients of a
/// polynomial of degree `deg` in `x = u - 1/2`, `y = v - 1/2` (terms
/// `x^i y^j`, `i` outer, `j` inner, `i + j <= deg`) added to the global mix.
/// The field reproduces the broad shape of glows, leaks and colour washes.
pub fn field_mix<const K: usize>(rows: &[[f32; K]], deg: usize) -> String {
    format!(
        r#"{}
{}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let k0 = kf_g(p);
    let a = l3_da(uv);
    let bb = l3_db(uv);
    let c = k0.x * a + k0.y * l3_luma(a) + k0.z * bb + k0.w * l3_luma(bb) + l3_field(uv, p);
    return l3_out(c);
}}
"#,
        table("g", rows),
        field_fn("g", K, 4, deg)
    )
}

/// WGSL `fn l3_field(uv, p) -> vec3f`: the polynomial light field whose
/// coefficients are values `first..` of table `name` (rows `K` wide):
/// per channel (r, g, b in turn) the coefficients of the terms `x^i y^j`,
/// `x = u - 1/2`, `y = v - 1/2`, `i` outer, `j` inner, `i + j <= deg`.
pub fn field_fn(name: &str, k: usize, first: usize, deg: usize) -> String {
    let nb = (deg + 1) * (deg + 2) / 2;
    assert_eq!(k, 1 + first + 3 * nb, "field {name}: row width");
    let chunks = (k - 1).div_ceil(4);
    // Unrolled: value i of the row is component i % 4 of chunk i / 4.
    let val = |i: usize| format!("k{}.{}", i / 4, ["x", "y", "z", "w"][i % 4]);
    let mut body = String::new();
    for j in first / 4..chunks {
        let f = if j == 0 { format!("kf_{name}(p)") } else { format!("kf_{name}_{j}(p)") };
        body.push_str(&format!("    let k{j} = {f};\n"));
    }
    let mut t = 0;
    for i in 0..=deg {
        for j in 0..=(deg - i) {
            let (r, g, b) = (val(first + t), val(first + nb + t), val(first + 2 * nb + t));
            let term = (0..i).map(|_| "x").chain((0..j).map(|_| "y")).collect::<Vec<_>>().join(" * ");
            let term = if term.is_empty() { "1.0".to_string() } else { term };
            body.push_str(&format!("    f = f + ({term}) * vec3f({r}, {g}, {b});\n"));
            t += 1;
        }
    }
    format!(
        "fn l3_field(uv: vec2f, p: f32) -> vec3f {{\n    let x = uv.x - 0.5;\n    let y = uv.y - 0.5;\n    var f = vec3f(0.0);\n{body}    return f;\n}}\n"
    )
}

/// [`edge_wipe`] followed by a per-frame gain and light field: rows of
/// `field` are `[p, a, field.r.., field.g.., field.b..]` and
/// `out = a W + field(uv)` (display space). The field carries the broad
/// colour and brightness of overlay artwork that rides on the wipe.
pub fn edge_wipe_field<const K: usize>(mask: &str, rows: &[[f32; 3]], field: &[[f32; K]], deg: usize) -> String {
    format!(
        r#"{}
{}
{}
fn l3_mask(uv: vec2f) -> f32 {{
    let c = to_c(uv);
    let A = aspect();
    return {mask};
}}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let k = kf_g(p);
    let w = to_disp(l3_mix(sa(uv), sb(uv), l3_ramp(l3_mask(uv), k.x, k.y))).rgb;
    return l3_out(kf_h(p).x * w + l3_field(uv, p));
}}
"#,
        table("g", rows),
        table("h", field),
        field_fn("h", K, 1, deg)
    )
}

/// WGSL of the edge-wipe recipe: B shows where the scalar mask `mask`
/// (WGSL expression of `uv`, `c = to_c(uv)` and `A = aspect()`) is below the
/// threshold `t`, with a linear edge of width `s`; rows `[p, t, s]`.
pub fn edge_wipe(mask: &str, rows: &[[f32; 3]]) -> String {
    format!(
        r#"{}
fn l3_mask(uv: vec2f) -> f32 {{
    let c = to_c(uv);
    let A = aspect();
    return {mask};
}}
fn fx(uv: vec2f) -> vec4f {{
    let k = kf_g(prog());
    return l3_mix(sa(uv), sb(uv), l3_ramp(l3_mask(uv), k.x, k.y));
}}
"#,
        table("g", rows)
    )
}

/// [`edge_wipe`] followed by a per-frame grade `out = a W + o` (display
/// space) from rows `[p, a, or, og, ob]`: the tint or dimming of an overlay
/// layered over the whole frame.
pub fn edge_wipe_graded(mask: &str, rows: &[[f32; 3]], grade: &[[f32; 5]]) -> String {
    format!(
        r#"{}
{}
fn l3_mask(uv: vec2f) -> f32 {{
    let c = to_c(uv);
    let A = aspect();
    return {mask};
}}
fn fx(uv: vec2f) -> vec4f {{
    let p = prog();
    let k = kf_g(p);
    let gr = kf_h(p);
    let w = to_disp(l3_mix(sa(uv), sb(uv), l3_ramp(l3_mask(uv), k.x, k.y))).rgb;
    return l3_out(gr.x * w + gr.yzw);
}}
"#,
        table("g", rows),
        table("h", grade)
    )
}

/// WGSL of the ramp-wipe recipe: weight of B = clamp(a + b m) for the mask
/// expression `mask` (as in [`edge_wipe`]); rows `[p, a, b]`. Suits soft,
/// wide dissolve fronts whose width changes over time.
pub fn ramp_wipe(mask: &str, rows: &[[f32; 3]]) -> String {
    format!(
        r#"{}
fn l3_mask(uv: vec2f) -> f32 {{
    let c = to_c(uv);
    let A = aspect();
    return {mask};
}}
fn fx(uv: vec2f) -> vec4f {{
    let k = kf_g(prog());
    return l3_mix(sa(uv), sb(uv), clamp(k.x + k.y * l3_mask(uv), 0.0, 1.0));
}}
"#,
        table("g", rows)
    )
}

pub fn register_items(r: &mut Registry, items: Vec<Item>) {
    for it in items {
        shader(
            r,
            info(it.id, it.name, it.category, FxKind::Transition, it.description, &it.params),
            format!("{KIT}\n{}", it.wgsl),
        );
    }
}

/// Value of a keyframe table at progress `p` on the CPU (same interpolation
/// as the WGSL emitted by [`table`]).
pub fn eval_rows<const K: usize>(rows: &[[f32; K]], p: f32) -> [f32; K] {
    if p <= rows[0][0] {
        return rows[0];
    }
    for w in rows.windows(2) {
        if p < w[1][0] {
            let k = (p - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
            let mut out = w[0];
            for (o, (a, b)) in out.iter_mut().zip(w[0].iter().zip(&w[1])) {
                *o = a + (b - a) * k;
            }
            return out;
        }
    }
    rows[rows.len() - 1]
}

/// Flash recipe (two passes): the outgoing clip before `cut` (progress), the
/// incoming one from it, zoomed by `s` about the centre and lit as
/// `out = g Z + h blur(Z, sigma) + o` in display space (clipped). Rows
/// `[p, g, h, sigma (1080p px), s, or, og, ob]`. Covers flashes, glows and
/// bloom washes, with the punch-in zoom some of them add.
pub struct FlashFx {
    pub info: FxInfo,
    pub rows: &'static [[f32; 8]],
    pub cut: f32,
    pub shader: ShaderSrc,
}

const FLASH_WGSL: &str = r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let q = zoom_uv(in.uv, max(u.e0.z, 1e-3), vec2f(0.5));
    let x = to_disp(sa(q)).rgb;
    let bl = to_disp(sb(q)).rgb;
    return l3_out(u.e0.x * x + u.e0.y * bl + u.e1.xyz);
}
"#;

impl FlashFx {
    pub fn new(it: Item, rows: &'static [[f32; 8]], cut: f32) -> Self {
        let shader = ShaderSrc::new(it.id, format!("{KIT}\n{FLASH_WGSL}"));
        let info = info(it.id, it.name, it.category, FxKind::Transition, it.description, &it.params);
        Self { info, rows, cut, shader }
    }
}

impl Effect for FlashFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let r = eval_rows(self.rows, io.progress);
        let src = match io.b {
            Some(b) if io.progress >= self.cut => b,
            _ => io.a,
        };
        let sigma = r[3].max(0.0) * src.height as f32 / 1080.0;
        let bl = cx.blur(src, sigma, sigma);
        let mut e = [[0.0; 4]; 4];
        e[0] = [r[1], r[2], r[4], 0.0];
        e[1] = [r[5], r[6], r[7], 0.0];
        let u = Uniforms { progress: io.progress, e, ..Default::default() };
        cx.run(&self.shader, src, Some(&bl), io.out, u);
    }
}

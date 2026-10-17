//! Layered corner-pin transitions (lane 2: slides, pushes, rolls, cubes,
//! flips, doors, cards, folds, stacks and grids).
//!
//! Each item is a table of layers fitted frame by frame on Filmora's export
//! of the transition (between our own grid test cards; only the exported
//! output was measured). A frame is a background color plus layers drawn back
//! to front; a layer shows the outgoing clip (A), the incoming clip (B) or a
//! solid color through a *bilinear corner pin*: the four corners of its source
//! rect land on given screen points and the clip is interpolated bilinearly
//! between them. That is what Filmora's 3D transitions measure as: on Cube
//! Right the optical-flow residual of a face is 0.15 px for a bilinear pin and
//! 2.9 px for the best perspective homography. Shading is a per-channel
//! display-space gain field `g + gu (u - 0.5) + gv (v - 0.5)` over the face
//! plus an offset; motion blur averages the pin over a corner displacement
//! (the shutter); edges are either clipped (antialiased) or mirrored-repeat.
//!
//! Keys are tabulated in transition progress (one per fitted frame, keys that
//! linear interpolation reproduces dropped), so other durations scale.
//! A layer is shown from half a key spacing before its first key to half a
//! key spacing after its last one, and hidden otherwise. Tables live in
//! `lane2/tables.txt`.

use super::FxCx;
use crate::fx::{Bind, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};
use std::sync::OnceLock;

const SHADER: &str = include_str!("lane2/layers.wgsl");

// Library group lists (`LIB_3D`, `LIB_SLIDES`), generated with the tables.
include!("lane2/library_ids.rs");
const TABLES: &str = include_str!("lane2/tables.txt");

/// Layers per pass (9 uniform slots each, 32 available).
const PER_PASS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Src {
    A,
    B,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Edge {
    Clip,
    Mirror,
    Wrap,
}

/// One tabulated state of a layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub p: f32,
    /// Screen uv of the source rect's corners TL, TR, BR, BL.
    pub c: [f32; 8],
    pub g: [f32; 3],
    pub gu: [f32; 3],
    pub gv: [f32; 3],
    /// Display-space offset (0..1).
    pub o: [f32; 3],
    pub a: f32,
    /// Corner displacement over the shutter (motion blur), screen uv.
    pub bc: [f32; 8],
}

impl Key {
    fn lerp(&self, b: &Key, k: f32) -> Key {
        let m = |x: f32, y: f32| x + (y - x) * k;
        let m3 = |x: [f32; 3], y: [f32; 3]| [m(x[0], y[0]), m(x[1], y[1]), m(x[2], y[2])];
        let m8 = |x: [f32; 8], y: [f32; 8]| std::array::from_fn(|i| m(x[i], y[i]));
        Key {
            p: m(self.p, b.p),
            c: m8(self.c, b.c),
            g: m3(self.g, b.g),
            gu: m3(self.gu, b.gu),
            gv: m3(self.gv, b.gv),
            o: m3(self.o, b.o),
            a: m(self.a, b.a),
            bc: m8(self.bc, b.bc),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub src: Src,
    pub edge: Edge,
    /// Perspective (homography) pin instead of the bilinear one.
    pub perspective: bool,
    pub rect: [f32; 4],
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub bg: [f32; 3],
    /// Key spacing in progress (one fitted frame).
    pub dp: f32,
    pub layers: Vec<Layer>,
}

impl Table {
    /// The visible layers at progress `p`, back to front.
    pub fn eval(&self, p: f32) -> Vec<(&Layer, Key)> {
        let mut out = Vec::new();
        for l in &self.layers {
            let (Some(first), Some(last)) = (l.keys.first(), l.keys.last()) else { continue };
            let margin = 0.5 * self.dp;
            if p < first.p - margin || p >= last.p + margin {
                continue;
            }
            let i = l.keys.partition_point(|k| k.p <= p);
            let key = if i == 0 {
                *first
            } else if i >= l.keys.len() {
                *last
            } else {
                let (a, b) = (&l.keys[i - 1], &l.keys[i]);
                a.lerp(b, (p - a.p) / (b.p - a.p).max(1e-6))
            };
            out.push((l, key));
        }
        out
    }
}

/// Parses `lane2/tables.txt`: `item <id>` sections with `bg r g b`, `dp x`,
/// `layer <A|B|S> <clip|mirror|wrap> [homo] u0 v0 u1 v1` and `k` key lines
/// `p c0x c0y c1x c1y c2x c2y c3x c3y gr gg gb gur gug gub gvr gvg gvb or og ob a [bc x8]`
/// (gains and offsets in display 0..1).
pub fn parse_tables(text: &str) -> Result<Vec<Table>, String> {
    let mut out: Vec<Table> = Vec::new();
    let mut cur: Option<(String, Table)> = None;
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let head = it.next().unwrap_or("");
        let rest: Vec<&str> = it.collect();
        let nums = |r: &[&str]| -> Result<Vec<f32>, String> {
            r.iter().map(|s| s.parse::<f32>().map_err(|e| format!("line {}: {e}", n + 1))).collect()
        };
        match head {
            "item" => {
                if let Some((_, t)) = cur.take() {
                    out.push(t);
                }
                let id = rest.first().ok_or(format!("line {}: item without id", n + 1))?;
                cur = Some((
                    id.to_string(),
                    Table {
                        id: id.to_string(),
                        name: id.to_string(),
                        category: "3D".into(),
                        description: String::new(),
                        bg: [0.0; 3],
                        dp: 1.0 / 60.0,
                        layers: Vec::new(),
                    },
                ));
            }
            _ => {
                let (_, t) = cur.as_mut().ok_or(format!("line {}: outside an item", n + 1))?;
                let text = || line[head.len()..].trim().to_string();
                match head {
                    "name" => t.name = text(),
                    "category" => t.category = text(),
                    "desc" => {
                        if !t.description.is_empty() {
                            t.description.push(' ');
                        }
                        t.description.push_str(&text());
                    }
                    "bg" => {
                        let v = nums(&rest)?;
                        t.bg = [v[0], v[1], v[2]];
                    }
                    "dp" => t.dp = nums(&rest)?[0],
                    "layer" => {
                        let src = match rest.first().copied() {
                            Some("A") => Src::A,
                            Some("B") => Src::B,
                            _ => Src::Solid,
                        };
                        let edge = match rest.get(1).copied() {
                            Some("mirror") => Edge::Mirror,
                            Some("wrap") => Edge::Wrap,
                            _ => Edge::Clip,
                        };
                        let perspective = rest.get(2) == Some(&"homo");
                        let v = nums(&rest[if perspective { 3 } else { 2 }..])?;
                        let rect = if v.len() >= 4 { [v[0], v[1], v[2], v[3]] } else { [0.0, 0.0, 1.0, 1.0] };
                        t.layers.push(Layer { src, edge, perspective, rect, keys: Vec::new() });
                    }
                    "k" => {
                        let v = nums(&rest)?;
                        if v.len() < 22 {
                            return Err(format!("line {}: key needs 22 values", n + 1));
                        }
                        let a3 = |i: usize| [v[i], v[i + 1], v[i + 2]];
                        let key = Key {
                            p: v[0],
                            c: std::array::from_fn(|i| v[1 + i]),
                            g: a3(9),
                            gu: a3(12),
                            gv: a3(15),
                            o: a3(18),
                            a: v[21],
                            bc: std::array::from_fn(|i| v.get(22 + i).copied().unwrap_or(0.0)),
                        };
                        let key = if v.len() >= 30 { key } else { Key { bc: [0.0; 8], ..key } };
                        t.layers.last_mut().ok_or(format!("line {}: key before layer", n + 1))?.keys.push(key);
                    }
                    other => return Err(format!("line {}: unknown record {other:?}", n + 1)),
                }
            }
        }
    }
    if let Some((_, t)) = cur.take() {
        out.push(t);
    }
    Ok(out)
}

/// Every lane 2 table, in file order.
pub fn tables() -> &'static [Table] {
    static T: OnceLock<Vec<Table>> = OnceLock::new();
    T.get_or_init(|| parse_tables(TABLES).expect("lane2/tables.txt"))
}

pub fn table(id: &str) -> Option<&'static Table> {
    tables().iter().find(|t| t.id == id)
}

fn shader() -> &'static ShaderSrc {
    static S: OnceLock<ShaderSrc> = OnceLock::new();
    S.get_or_init(|| {
        ShaderSrc::new(
            "jb.lane2.layers",
            format!("{SHADER}\n@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return fx(in.uv); }}\n"),
        )
    })
}

/// Records the passes drawing `table` at `progress` into `io.out`.
pub fn encode_table(table: &Table, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
    let b = io.b.unwrap_or(io.a);
    let layers = table.eval(io.progress);
    let chunks: Vec<&[(&Layer, Key)]> = if layers.is_empty() { vec![&[]] } else { layers.chunks(PER_PASS).collect() };
    let mut prev: Option<crate::pool::Tex> = None;
    let last = chunks.len() - 1;
    for (ci, chunk) in chunks.iter().enumerate() {
        let mut u = Uniforms::default();
        u.progress = io.progress;
        u.e[0] = [table.bg[0], table.bg[1], table.bg[2], chunk.len() as f32];
        u.e[1] = [if prev.is_some() { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0];
        for (li, (l, k)) in chunk.iter().enumerate() {
            let s = li * 9;
            let taps = blur_taps(&k.bc);
            let src = match l.src {
                Src::A => 0.0,
                Src::B => 1.0,
                Src::Solid => 2.0,
            };
            let edge = match l.edge {
                Edge::Clip => 0.0,
                Edge::Mirror => 1.0,
                Edge::Wrap => 2.0,
            };
            u.p[s] = [k.c[0], k.c[1], k.c[2], k.c[3]];
            u.p[s + 1] = [k.c[4], k.c[5], k.c[6], k.c[7]];
            u.p[s + 2] = [k.bc[0], k.bc[1], k.bc[2], k.bc[3]];
            u.p[s + 3] = [k.bc[4], k.bc[5], k.bc[6], k.bc[7]];
            u.p[s + 4] = [k.g[0], k.g[1], k.g[2], k.a];
            u.p[s + 5] = [k.gu[0], k.gu[1], k.gu[2], src];
            u.p[s + 6] = [k.gv[0], k.gv[1], k.gv[2], edge + if l.perspective { 4.0 } else { 0.0 }];
            u.p[s + 7] = [k.o[0], k.o[1], k.o[2], taps as f32];
            u.p[s + 8] = l.rect;
        }
        let target = if ci == last { io.out.clone() } else { cx.temp_like(io.out) };
        let mut pass = Pass::full(shader(), &target, u);
        pass.a = Bind::Tex(io.a);
        pass.b = Bind::Tex(b);
        if let Some(p) = &prev {
            pass.aux = Bind::Tex(p);
        }
        cx.draw(pass);
        prev = Some(target);
    }
}

/// Motion-blur taps for a corner displacement: one per ~3 pixels of the
/// longest corner path at 960x540 (as fitted), 8 to 48; 1 without blur.
pub fn blur_taps(bc: &[f32; 8]) -> u32 {
    let mut n: f32 = 0.0;
    for i in 0..4 {
        n = n.max(((bc[2 * i] * 960.0).powi(2) + (bc[2 * i + 1] * 540.0).powi(2)).sqrt());
    }
    if n < 1e-3 {
        1
    } else {
        ((n / 3.0).ceil() as u32).clamp(8, 48)
    }
}

/// A transition rendered from its fitted table.
pub struct TableFx {
    pub info: FxInfo,
    pub table: &'static Table,
}

impl Effect for TableFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        encode_table(self.table, cx, io);
    }
}

pub fn register(r: &mut Registry) {
    for t in tables() {
        let info = super::info(&t.id, &t.name, &t.category, FxKind::Transition, &t.description, &[]);
        r.register(TableFx { info, table: t });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_parse_and_have_items() {
        for tab in tables() {
            let id = &tab.id;
            assert!(!tab.layers.is_empty(), "{id}");
            assert!(!tab.description.is_empty() && tab.name != tab.id, "{id}: name/desc");
            for l in &tab.layers {
                assert!(l.keys.windows(2).all(|w| w[0].p < w[1].p), "{id}: keys out of order");
            }
        }
    }

    #[test]
    fn eval_interpolates_and_hides() {
        let t = parse_tables(
            "item x\ndp 0.1\nlayer A clip 0 0 1 1\n\
             k 0.0 0 0 1 0 1 1 0 1 1 1 1 0 0 0 0 0 0 0 0 0 1\n\
             k 0.5 0 0 0.5 0 0.5 1 0 1 1 1 1 0 0 0 0 0 0 0 0 0 1\n",
        )
        .unwrap();
        let tab = &t[0];
        assert_eq!(tab.eval(0.25)[0].1.c[2], 0.75);
        assert!(tab.eval(0.56).is_empty());
        assert_eq!(tab.eval(-0.04).len(), 1);
    }
}

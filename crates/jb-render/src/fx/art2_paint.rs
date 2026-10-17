//! Art 2: paint, brush, ink and torn-paper transitions drawn procedurally
//! (our own artwork, no third-party assets).
//!
//! Every item is data: a list of [`Stroke`]s evaluated in order by one WGSL
//! kit (`shaders/art2/paint.wgsl`): brush strokes along quadratic paths with
//! bristle-ragged heads and sides, dry-brush streak gaps, ink / watercolour
//! blots with mottled washes, ragged sweeps and torn-paper edges with a
//! paper rim. A stroke reveals the incoming clip, restores the outgoing one
//! or lays a [`Paint`] (gold leaf, paper, ink) with streaks, sheen, wet-edge
//! darkening and grain. The stroke and paint tables are emitted as WGSL
//! arrays, one shader per item. Timing, motion paths, coverage over time
//! and palettes follow the reference behaviour documented in
//! `docs/fidelity-art2.md`; the shapes are drawn by these rules.

use crate::fx::{info, FxCx, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};

/// Frame aspect the items are laid out for (16:9).
pub const ASPECT: f32 = 16.0 / 9.0;

/// Motion easing of a stroke.
#[derive(Clone, Copy, Debug)]
pub enum Ease {
    Linear = 0,
    Smooth = 1,
    Out = 2,
    In = 3,
    OutQuad = 4,
}

/// Shape kind of a stroke.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// Brush along a quadratic path; the head travels from `a` to `b`.
    Brush = 0,
    /// Ink / watercolour blot around `a`, radius `w.0` to `w.1`; `c` stretches x / y.
    Blot = 1,
    /// A ragged edge moving from `a` to `b`, covering what it has passed.
    Sweep = 2,
    /// Brush whose tail also leaves, `tail` progress after the head arrives.
    Streak = 3,
}

/// Edge character.
#[derive(Clone, Copy, Debug)]
pub enum Style {
    Bristle = 0,
    Torn = 1,
    Clean = 2,
    Water = 3,
}

/// What a stroke lays down.
#[derive(Clone, Copy, Debug)]
pub enum Fill {
    /// The incoming clip.
    B,
    /// The outgoing clip (paints it back).
    A,
    /// Paint `k` of the item.
    Paint(u8),
}

/// One stroke (see the WGSL kit for the exact semantics).
#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub a: [f32; 2],
    pub c: Option<[f32; 2]>,
    pub b: [f32; 2],
    /// Half width at the start and the end (fraction of the frame height).
    pub w: [f32; 2],
    /// Progress window of the motion.
    pub t: [f32; 2],
    pub kind: Kind,
    pub ease: Ease,
    pub rag: f32,
    pub dry: f32,
    pub fill: Fill,
    pub seed: f32,
    pub style: Style,
    pub rim: f32,
    /// Opacity starts at `op.0` and reaches 1 at progress `op.1` (when > 0).
    pub op: [f32; 2],
    pub soft: f32,
    pub mottle: f32,
    /// Head raggedness (fraction of the width).
    pub jag: f32,
    /// Streak: progress span over which the tail follows the head.
    pub tail: f32,
    /// Optional head timing knots `(p1, h1, p2, h2)` (piecewise linear).
    pub knots: [f32; 4],
    /// Progress by which dry-brush gaps have closed (0: never).
    pub dry_close: f32,
    /// Width growth `(p0, p1, factor)`: the width scales by `factor` over `p0..p1`.
    pub grow: [f32; 3],
    /// Cover the outside of a blot / the side ahead of a sweep / outside a brush.
    pub invert: bool,
    /// Opacity falls from 1 to 0 over this progress window (when `.1 > .0`).
    pub fade_out: [f32; 2],
    /// Morph: `a`, `b`, `c` move to these points over `morph_t`.
    pub morph: Option<([f32; 2], [f32; 2], [f32; 2])>,
    pub morph_t: [f32; 2],
}

impl Stroke {
    /// A bristle brush stroke from `a` to `b`, half width `w`, during `t0..t1`.
    pub fn brush(a: [f32; 2], b: [f32; 2], w: f32, t0: f32, t1: f32) -> Self {
        Self {
            a,
            c: None,
            b,
            w: [w, w],
            t: [t0, t1],
            kind: Kind::Brush,
            ease: Ease::Smooth,
            rag: 0.12,
            dry: 0.0,
            fill: Fill::B,
            seed: (a[0] * 13.7 + a[1] * 7.1 + b[0] * 3.3 + b[1] * 5.9).fract() * 50.0,
            style: Style::Bristle,
            rim: 0.0,
            op: [1.0, 0.0],
            soft: 0.0,
            mottle: 0.0,
            jag: 1.2,
            tail: 0.0,
            knots: [0.0; 4],
            dry_close: 0.0,
            grow: [0.0; 3],
            invert: false,
            fade_out: [0.0; 2],
            morph: None,
            morph_t: [0.0; 2],
        }
    }
    /// A ragged edge moving along direction `deg` (degrees clockwise from +x,
    /// y down) whose line passes `d0` then `d1` (height units, signed, from
    /// the frame centre along that direction): it covers what it has passed.
    pub fn sweep_dir(deg: f32, d0: f32, d1: f32, t0: f32, t1: f32) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        let p = |d: f32| [(0.5 * ASPECT + c * d) / ASPECT, 0.5 + s * d];
        Self::sweep(p(d0), p(d1), t0, t1)
    }
    /// An ink blot at `a` growing from radius `r0` to `r1`.
    pub fn blot(a: [f32; 2], r0: f32, r1: f32, t0: f32, t1: f32) -> Self {
        Self { kind: Kind::Blot, c: Some([1.0, 1.0]), w: [r0, r1], style: Style::Water, rag: 0.25, ..Self::brush(a, a, 0.0, t0, t1) }
    }
    /// A ragged edge travelling from `a` to `b`.
    pub fn sweep(a: [f32; 2], b: [f32; 2], t0: f32, t1: f32) -> Self {
        Self { kind: Kind::Sweep, w: [0.05, 0.05], style: Style::Torn, rag: 0.03, ..Self::brush(a, b, 0.0, t0, t1) }
    }
    pub fn via(mut self, c: [f32; 2]) -> Self {
        self.c = Some(c);
        self
    }
    pub fn widths(mut self, w0: f32, w1: f32) -> Self {
        self.w = [w0, w1];
        self
    }
    pub fn ease(mut self, e: Ease) -> Self {
        self.ease = e;
        self
    }
    pub fn rag(mut self, r: f32) -> Self {
        self.rag = r;
        self
    }
    pub fn dry(mut self, d: f32) -> Self {
        self.dry = d;
        self
    }
    pub fn fill(mut self, f: Fill) -> Self {
        self.fill = f;
        self
    }
    pub fn paint(self, k: u8) -> Self {
        self.fill(Fill::Paint(k))
    }
    pub fn seed(mut self, s: f32) -> Self {
        self.seed = s;
        self
    }
    pub fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }
    pub fn rim(mut self, r: f32) -> Self {
        self.rim = r;
        self
    }
    pub fn fade_in(mut self, op0: f32, until: f32) -> Self {
        self.op = [op0, until];
        self
    }
    pub fn soft(mut self, s: f32) -> Self {
        self.soft = s;
        self
    }
    pub fn mottle(mut self, m: f32) -> Self {
        self.mottle = m;
        self
    }
    pub fn jag(mut self, j: f32) -> Self {
        self.jag = j;
        self
    }
    /// Blot stretch (x, y).
    pub fn stretch(mut self, sx: f32, sy: f32) -> Self {
        self.c = Some([sx, sy]);
        self
    }
    /// The tail follows the head over `span` progress after `t1`.
    pub fn streak(mut self, span: f32) -> Self {
        self.kind = Kind::Streak;
        self.tail = span;
        self
    }

    /// Head timing through `(p1, h1)` and `(p2, h2)` (head fraction at those progress values).
    pub fn knots(mut self, p1: f32, h1: f32, p2: f32, h2: f32) -> Self {
        self.knots = [p1, h1, p2, h2];
        self
    }

    /// Dry-brush gaps close by progress `p`.
    pub fn dry_until(mut self, p: f32) -> Self {
        self.dry_close = p;
        self
    }

    /// The width scales by `factor` between progress `p0` and `p1`.
    pub fn grow(mut self, p0: f32, p1: f32, factor: f32) -> Self {
        self.grow = [p0, p1, factor];
        self
    }

    /// Cover the other side (outside / ahead).
    pub fn invert(mut self) -> Self {
        self.invert = true;
        self
    }

    /// Fade out over progress `p0..p1`.
    pub fn fade_out(mut self, p0: f32, p1: f32) -> Self {
        self.fade_out = [p0, p1];
        self
    }

    /// Move the path to `a`, `c`, `b` over progress `p0..p1`.
    pub fn morph(mut self, a: [f32; 2], c: [f32; 2], b: [f32; 2], p0: f32, p1: f32) -> Self {
        self.morph = Some((a, b, c));
        self.morph_t = [p0, p1];
        self
    }

    fn record(&self) -> [[f32; 4]; 12] {
        let c = self.c.unwrap_or([(self.a[0] + self.b[0]) * 0.5, (self.a[1] + self.b[1]) * 0.5]);
        let (ma, mb, mc) = self.morph.unwrap_or((self.a, self.b, c));
        let fill = match self.fill {
            Fill::B => 0.0,
            Fill::A => 1.0,
            Fill::Paint(k) => 2.0 + k as f32,
        };
        [
            [self.a[0], self.a[1], c[0], c[1]],
            [self.b[0], self.b[1], self.w[0], self.w[1]],
            [self.t[0], self.t[1], self.kind as u8 as f32, self.ease as u8 as f32],
            [self.rag, self.dry, fill, self.seed],
            [self.style as u8 as f32, self.rim, self.op[0], self.op[1]],
            [self.soft, self.mottle, self.jag, self.tail],
            self.knots,
            [self.dry_close, self.grow[0], self.grow[1], self.grow[2]],
            [if self.invert { 1.0 } else { 0.0 }, self.fade_out[0], self.fade_out[1], 0.0],
            [ma[0], ma[1], mb[0], mb[1]],
            [mc[0], mc[1], self.morph_t[0], self.morph_t[1]],
            [0.0; 4],
        ]
    }
}

/// A paint (display-space colours).
#[derive(Clone, Copy, Debug)]
pub struct Paint {
    pub base: [f32; 3],
    pub light: [f32; 3],
    /// Contrast of the bristle streaks.
    pub streak: f32,
    pub sheen: f32,
    /// Wet-edge darkening.
    pub wet: f32,
    pub grain: f32,
    /// Darkening across the stroke (rolls, folds).
    pub shade: f32,
    pub alpha: f32,
}

impl Paint {
    pub const fn flat(c: [f32; 3]) -> Self {
        Self { base: c, light: c, streak: 0.0, sheen: 0.0, wet: 0.0, grain: 0.0, shade: 0.0, alpha: 1.0 }
    }
    /// Brushed gold leaf.
    pub const GOLD: Paint = Paint {
        base: [0.70, 0.48, 0.15],
        light: [1.0, 0.82, 0.42],
        streak: 0.9,
        sheen: 0.35,
        wet: 0.15,
        grain: 0.04,
        shade: 0.0,
        alpha: 1.0,
    };
    /// Off-white paper fibres.
    pub const PAPER: Paint = Paint {
        base: [0.80, 0.80, 0.78],
        light: [0.98, 0.98, 0.96],
        streak: 0.3,
        sheen: 0.0,
        wet: 0.0,
        grain: 0.05,
        shade: 0.0,
        alpha: 1.0,
    };

    fn record(&self) -> [[f32; 4]; 3] {
        [
            [self.base[0], self.base[1], self.base[2], self.streak],
            [self.light[0], self.light[1], self.light[2], self.sheen],
            [self.wet, self.grain, self.shade, self.alpha],
        ]
    }
}

/// One item: strokes in drawing order, paints, and the progress from which
/// the incoming clip is shown whole.
pub struct ArtItem {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub strokes: Vec<Stroke>,
    pub paints: Vec<Paint>,
    pub done: f32,
}

fn vec4s(rows: impl Iterator<Item = [f32; 4]>) -> (usize, String) {
    let v: Vec<String> = rows.map(|r| format!("vec4f({:?}, {:?}, {:?}, {:?})", r[0], r[1], r[2], r[3])).collect();
    (v.len(), v.join(",\n    "))
}

/// The item's WGSL (kit + tables + entry point).
pub fn wgsl(it: &ArtItem) -> String {
    let (ns, s) = vec4s(it.strokes.iter().flat_map(|s| s.record()));
    let mut paints = it.paints.clone();
    if paints.is_empty() {
        paints.push(Paint::flat([0.0; 3]));
    }
    let (np, p) = vec4s(paints.iter().flat_map(|p| p.record()));
    format!(
        "var<private> ART_S: array<vec4f, {ns}> = array<vec4f, {ns}>(\n    {s});\n\
         var<private> ART_P: array<vec4f, {np}> = array<vec4f, {np}>(\n    {p});\n\
         const ART_N: i32 = {n};\nconst ART_DONE: f32 = {done:?};\n{kit}\n\
         @fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return art_fx(in.uv); }}\n",
        n = it.strokes.len(),
        done = it.done,
        kit = include_str!("../shaders/art2/paint.wgsl"),
    )
}

pub struct ArtFx {
    info: FxInfo,
    shader: ShaderSrc,
}

impl Effect for ArtFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let u = Uniforms { progress: io.progress, time: io.time, ..Default::default() };
        cx.run(&self.shader, io.a, io.b, io.out, u);
    }
}

pub fn register_items(r: &mut Registry, items: Vec<ArtItem>) {
    for it in items {
        let shader = ShaderSrc::new(it.id, wgsl(&it));
        r.register(ArtFx { info: info(it.id, it.name, it.category, FxKind::Transition, it.description, &[]), shader });
    }
}

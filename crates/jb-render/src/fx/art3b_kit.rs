//! Shared machinery of the art3b motion-graphics transitions: each item is a
//! function from progress to a [`Scene`] (the two clips through cameras plus
//! an ordered list of vector primitives), rendered on the GPU by
//! `shaders/art3b/prims.wgsl`. The art is our own: shapes are authored in
//! code (rectangles, ellipses, stars, comic bursts, noisy blobs, capsules,
//! hearts, arcs and perspective quads) and keyframed with the helpers below.
//!
//! Coordinates are centred and aspect-correct: y spans −0.5..0.5 (down),
//! x spans −asp/2..asp/2 where `asp` is the frame aspect (16/9 for 1080p).
//! Colours are display-referred sRGB in 0..1.

use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};
use std::sync::OnceLock;

/// Camera of one clip in the background: zoom about the centre, shift
/// (centred units) and rotation (radians, clockwise on screen).
#[derive(Debug, Clone, Copy)]
pub struct Cam {
    pub zoom: f32,
    pub dx: f32,
    pub dy: f32,
    pub rot: f32,
}

impl Cam {
    pub const ID: Cam = Cam { zoom: 1.0, dx: 0.0, dy: 0.0, rot: 0.0 };
    pub fn zoom(z: f32) -> Cam {
        Cam { zoom: z, ..Cam::ID }
    }
    pub fn shift(dx: f32, dy: f32) -> Cam {
        Cam { dx, dy, ..Cam::ID }
    }
    pub fn with_shift(mut self, dx: f32, dy: f32) -> Cam {
        self.dx = dx;
        self.dy = dy;
        self
    }
    pub fn with_rot(mut self, r: f32) -> Cam {
        self.rot = r;
        self
    }
    fn v(&self) -> [f32; 4] {
        [self.zoom, self.dx, self.dy, self.rot]
    }
}

/// One frame of an item: background plus primitives drawn in order.
#[derive(Debug, Clone)]
pub struct Scene {
    pub a: Cam,
    pub b: Cam,
    /// Display-space weight of B in the background.
    pub mix: f32,
    /// Brightness gains of A and B in the background.
    pub gain_a: f32,
    pub gain_b: f32,
    /// Background dipped towards `dip.rgb` by `dip[3]`.
    pub dip: [f32; 4],
    /// Outside of a zoomed-out camera: black instead of the clamped edge.
    pub black_outside: bool,
    /// Gaussian blur of the clips (fraction of the frame height).
    pub blur_a: f32,
    pub blur_b: f32,
    pub prims: Vec<Prim>,
}

impl Scene {
    /// A at full, B hidden.
    pub fn new() -> Self {
        Scene {
            a: Cam::ID,
            b: Cam::ID,
            mix: 0.0,
            gain_a: 1.0,
            gain_b: 1.0,
            dip: [0.0; 4],
            black_outside: false,
            blur_a: 0.0,
            blur_b: 0.0,
            prims: Vec::new(),
        }
    }
    /// Background is A before `cut`, B from it on.
    pub fn cut(t: f32, cut: f32) -> Self {
        let mut s = Self::new();
        s.mix = if t >= cut { 1.0 } else { 0.0 };
        s
    }
    /// Background cross-fades from A to B between `t0` and `t1`.
    pub fn fade(t: f32, t0: f32, t1: f32) -> Self {
        let mut s = Self::new();
        s.mix = seg(t, t0, t1);
        s
    }
    pub fn dip(mut self, rgb: [f32; 3], amount: f32) -> Self {
        self.dip = [rgb[0], rgb[1], rgb[2], amount.clamp(0.0, 1.0)];
        self
    }
    pub fn push(&mut self, p: Prim) {
        self.prims.push(p);
    }
    pub fn extend(&mut self, ps: impl IntoIterator<Item = Prim>) {
        self.prims.extend(ps);
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// A vector primitive (see the layout in `prims.wgsl`).
#[derive(Debug, Clone, Copy)]
pub struct Prim {
    pub v: [[f32; 4]; 5],
}

const K_RECT: f32 = 1.0;
const K_ELLIPSE: f32 = 2.0;
const K_STAR: f32 = 3.0;
const K_BURST: f32 = 4.0;
const K_BLOB: f32 = 6.0;
const K_CAPSULE: f32 = 7.0;
const K_HALF: f32 = 8.0;
const K_QUAD: f32 = 9.0;
const K_HEART: f32 = 10.0;
const K_ARC: f32 = 11.0;
const K_CHEVRON: f32 = 12.0;
const K_RHOMBUS: f32 = 13.0;

impl Prim {
    fn shape(kind: f32, a: f32, b: f32, c: f32, cx: f32, cy: f32) -> Prim {
        Prim { v: [[kind, a, b, c], [cx, cy, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0], [0.0; 4], [0.0; 4]] }
    }
    /// Rectangle of half size (hw, hh) centred at (cx, cy).
    pub fn rect(cx: f32, cy: f32, hw: f32, hh: f32) -> Prim {
        Self::shape(K_RECT, hw, hh, 0.0, cx, cy)
    }
    /// Rectangle spanning x0..x1, y0..y1.
    pub fn rect_xy(x0: f32, y0: f32, x1: f32, y1: f32) -> Prim {
        Self::rect(0.5 * (x0 + x1), 0.5 * (y0 + y1), 0.5 * (x1 - x0).abs(), 0.5 * (y1 - y0).abs())
    }
    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Prim {
        Self::shape(K_ELLIPSE, rx, ry, 0.0, cx, cy)
    }
    pub fn circle(cx: f32, cy: f32, r: f32) -> Prim {
        Self::ellipse(cx, cy, r, r)
    }
    /// Star with `n` points; `inner` is the inner radius as a fraction of `r`.
    pub fn star(cx: f32, cy: f32, r: f32, n: f32, inner: f32) -> Prim {
        Self::shape(K_STAR, r, n, inner, cx, cy)
    }
    /// Regular polygon with `n` sides (a vertex on +x before rotation).
    pub fn poly(cx: f32, cy: f32, r: f32, n: f32) -> Prim {
        Self::star(cx, cy, r, n, (std::f32::consts::PI / n).cos())
    }
    /// Comic burst: `n` spikes whose lengths vary by up to `jitter` × r.
    pub fn burst(cx: f32, cy: f32, r: f32, n: f32, inner: f32, jitter: f32, seed: f32) -> Prim {
        let mut p = Self::shape(K_BURST, r, n, inner, cx, cy);
        p.v[4][1] = seed;
        p.v[4][2] = jitter;
        p
    }
    /// Circle with a noisy outline: amplitude `amp` (fraction of r), `freq` bumps.
    pub fn blob(cx: f32, cy: f32, r: f32, amp: f32, freq: f32, seed: f32) -> Prim {
        let mut p = Self::shape(K_BLOB, r, amp, freq, cx, cy);
        p.v[4][1] = seed;
        p
    }
    /// Capsule along x: half length `hl`, radius `r` (tapering to `r * taper`).
    pub fn capsule(cx: f32, cy: f32, hl: f32, r: f32, taper: f32) -> Prim {
        Self::shape(K_CAPSULE, hl, r, taper, cx, cy)
    }
    /// Capsule from (x0, y0) to (x1, y1).
    pub fn seg(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Prim {
        let (dx, dy) = (x1 - x0, y1 - y0);
        Self::capsule(0.5 * (x0 + x1), 0.5 * (y0 + y1), 0.5 * (dx * dx + dy * dy).sqrt(), r, 1.0)
            .rot(dy.atan2(dx))
    }
    /// Half plane: the side of the line through (cx, cy) opposite to the
    /// direction `angle` (radians) is inside.
    pub fn half(cx: f32, cy: f32, angle: f32) -> Prim {
        Self::shape(K_HALF, 0.0, 0.0, 0.0, cx, cy).rot(angle)
    }
    /// Heart of size `s` (about half its height).
    pub fn heart(cx: f32, cy: f32, s: f32) -> Prim {
        Self::shape(K_HEART, s, 0.0, 0.0, cx, cy)
    }
    /// Heart of size `s` stretched horizontally by `w`. Its tip is at
    /// cy + 0.55 s, its top at cy − 1.23 s, its widest span 1.95 s w at
    /// cy − 0.66 s.
    pub fn heart_wide(cx: f32, cy: f32, s: f32, w: f32) -> Prim {
        Self::shape(K_HEART, s, w, 0.0, cx, cy)
    }
    /// Arc of radius `r`, thickness `w`, spanning ±`half` radians about the rotation.
    pub fn arc(cx: f32, cy: f32, r: f32, w: f32, half: f32) -> Prim {
        Self::shape(K_ARC, r, w, half, cx, cy)
    }
    /// Chevron pointing along +x (rotate for other directions): tip at
    /// (cx, cy), horizontal thickness `th`, slope `k` (the arms fall back by
    /// `k` per unit of |y|), clipped to |y| < `hh`. `k = 0` is a vertical bar.
    pub fn chevron(cx: f32, cy: f32, th: f32, k: f32, hh: f32) -> Prim {
        Self::shape(K_CHEVRON, th, k, hh, cx, cy)
    }
    /// Rhombus with half diagonals (hx, hy) centred at (cx, cy).
    pub fn rhombus(cx: f32, cy: f32, hx: f32, hy: f32) -> Prim {
        Self::shape(K_RHOMBUS, hx, hy, 0.0, cx, cy)
    }
    /// Perspective quad through corners p0 (maps to texture top-left), p1
    /// (top-right), p2 (bottom-right), p3 (bottom-left); filled with the
    /// whole texture by default.
    pub fn quad(c: [[f32; 2]; 4]) -> Prim {
        let h = square_to_quad(c);
        let mut m = invert3(h).unwrap_or([[0.0; 3]; 3]);
        // Keep w positive inside the quad (at its centroid).
        let (gx, gy) = (0.25 * c.iter().map(|p| p[0]).sum::<f32>(), 0.25 * c.iter().map(|p| p[1]).sum::<f32>());
        if m[2][0] * gx + m[2][1] * gy + m[2][2] < 0.0 {
            m = m.map(|r| r.map(|v| -v));
        }
        Prim {
            v: [
                [K_QUAD, m[0][0], m[0][1], m[0][2]],
                [m[1][0], m[1][1], m[1][2], 0.0],
                [1.0, 1.0, 1.0, 1.0],
                [0.0, m[2][0], m[2][1], m[2][2]],
                [1.0, 0.0, 0.0, 1.0],
            ],
        }
    }
    /// Axis-aligned frame-shaped quad: the clip scaled by `s` about (cx, cy).
    pub fn card(cx: f32, cy: f32, hw: f32, hh: f32) -> Prim {
        Self::quad([[cx - hw, cy - hh], [cx + hw, cy - hh], [cx + hw, cy + hh], [cx - hw, cy + hh]])
    }
    /// Texture sub-rectangle of a quad (uv origin and size).
    pub fn sub(mut self, u0: f32, v0: f32, du: f32, dv: f32) -> Prim {
        self.v[4] = [du, u0, v0, dv];
        self
    }

    /// Scales a (non-quad) shape by `z` about the screen point (ox, oy):
    /// position, size, outline, softness and fill geometry.
    pub fn zoom(mut self, z: f32, ox: f32, oy: f32) -> Prim {
        let kind = self.v[0][0];
        if kind == K_QUAD {
            return self;
        }
        self.v[1][0] = ox + (self.v[1][0] - ox) * z;
        self.v[1][1] = oy + (self.v[1][1] - oy) * z;
        let k = kind as i32;
        match k {
            1 => {
                self.v[0][1] *= z;
                self.v[0][2] *= z;
                self.v[0][3] *= z;
            }
            2 | 7 | 11 | 13 => {
                self.v[0][1] *= z;
                self.v[0][2] *= z;
            }
            12 => {
                self.v[0][1] *= z;
                self.v[0][3] *= z;
            }
            3 | 4 | 6 | 10 => self.v[0][1] *= z,
            _ => {}
        }
        self.v[1][3] *= z;
        self.v[4][0] *= z;
        match (self.v[3][0] as i32) % 10 {
            1 | 2 => self.v[3][1] *= z,
            3 | 8 => {
                self.v[4][2] *= z;
                self.v[4][3] *= z;
            }
            4 => self.v[4][2] *= z,
            5 => self.v[4][2] /= z.max(1e-4),
            _ => {}
        }
        self
    }

    pub fn rot(mut self, a: f32) -> Prim {
        if self.v[0][0] != K_QUAD {
            self.v[1][2] = a;
        }
        self
    }
    pub fn round(mut self, r: f32) -> Prim {
        self.v[0][3] = r;
        self
    }
    pub fn soft(mut self, s: f32) -> Prim {
        if self.v[0][0] != K_QUAD {
            self.v[1][3] = s;
        }
        self
    }
    pub fn stroke(mut self, w: f32) -> Prim {
        if self.v[0][0] != K_QUAD {
            self.v[4][0] = w;
        }
        self
    }
    pub fn rgb(mut self, c: [f32; 3]) -> Prim {
        self.v[2][0] = c[0];
        self.v[2][1] = c[1];
        self.v[2][2] = c[2];
        self
    }
    pub fn col(self, hex: u32) -> Prim {
        self.rgb(hexc(hex))
    }
    pub fn alpha(mut self, a: f32) -> Prim {
        self.v[2][3] = a.clamp(0.0, 1.0);
        self
    }
    fn set_fill(&mut self, mode: f32) {
        let add = if self.v[3][0] >= 10.0 { 10.0 } else { 0.0 };
        self.v[3][0] = mode + add;
    }
    /// A window onto the outgoing clip as it is on screen.
    pub fn tex_a(mut self) -> Prim {
        self.set_fill(6.0);
        self
    }
    /// A window onto the incoming clip as it is on screen.
    pub fn tex_b(mut self) -> Prim {
        self.set_fill(7.0);
        self
    }
    /// A piece of the outgoing clip that moves with the shape: its content
    /// is the clip around (tcx, tcy) scaled by `tz`. Quads always carry
    /// their texture (see [`Prim::sub`]).
    pub fn piece_a(mut self, tcx: f32, tcy: f32, tz: f32) -> Prim {
        self.set_fill(1.0);
        self.v[3][1] = tz;
        self.v[3][2] = tcx;
        self.v[3][3] = tcy;
        self
    }
    /// A piece of the incoming clip (see [`Prim::piece_a`]).
    pub fn piece_b(mut self, tcx: f32, tcy: f32, tz: f32) -> Prim {
        self.set_fill(2.0);
        self.v[3][1] = tz;
        self.v[3][2] = tcx;
        self.v[3][3] = tcy;
        self
    }
    /// Quad textured with A (`b == false`) or B.
    pub fn quad_tex(mut self, b: bool) -> Prim {
        self.set_fill(if b { 2.0 } else { 1.0 });
        self
    }
    /// Linear gradient along the local y axis: own colour at `y0`, `c2` at `y1`.
    pub fn grad(mut self, c2: u32, y0: f32, y1: f32) -> Prim {
        if self.v[0][0] == K_QUAD {
            // Quads keep their homography where the second colour would go.
            return self;
        }
        self.set_fill(3.0);
        let c = hexc(c2);
        self.v[3][1] = c[0];
        self.v[3][2] = c[1];
        self.v[3][3] = c[2];
        self.v[4][2] = y0;
        self.v[4][3] = y1;
        self
    }
    /// Linear gradient along the local x axis: own colour at `x0`, `c2` at `x1`.
    pub fn gradx(self, c2: u32, x0: f32, x1: f32) -> Prim {
        if self.v[0][0] == K_QUAD {
            return self;
        }
        let mut p = self.grad(c2, x0, x1);
        p.set_fill(8.0);
        p
    }
    /// Radial gradient: `c2` at the centre to the own colour at radius `r`.
    pub fn radial(mut self, c2: u32, r: f32) -> Prim {
        if self.v[0][0] == K_QUAD {
            // Quads keep their homography where the second colour would go.
            return self;
        }
        self.set_fill(4.0);
        let c = hexc(c2);
        self.v[3][1] = c[0];
        self.v[3][2] = c[1];
        self.v[3][3] = c[2];
        self.v[4][2] = r;
        self
    }
    /// Noise texture between the own colour and `c2` at `freq` cells per unit.
    pub fn noise(mut self, c2: u32, freq: f32, drift: f32) -> Prim {
        if self.v[0][0] == K_QUAD {
            // Quads keep their homography where the second colour would go.
            return self;
        }
        self.set_fill(5.0);
        let c = hexc(c2);
        self.v[3][1] = c[0];
        self.v[3][2] = c[1];
        self.v[3][3] = c[2];
        self.v[4][2] = freq;
        self.v[4][3] = drift;
        self
    }
    /// Plain colour fill (drops gradients, noise and textures).
    pub fn solid(mut self) -> Prim {
        if self.v[0][0] != K_QUAD {
            self.set_fill(0.0);
        }
        self
    }
    /// Additive light instead of covering paint.
    pub fn add(mut self) -> Prim {
        if self.v[3][0] < 10.0 {
            self.v[3][0] += 10.0;
        }
        self
    }
    /// Seed of the noise of blobs, bursts and noise fills.
    pub fn seed(mut self, s: f32) -> Prim {
        self.v[4][1] = s;
        self
    }
}

/// `0xRRGGBB` to 0..1 RGB.
pub fn hexc(h: u32) -> [f32; 3] {
    [((h >> 16) & 255) as f32 / 255.0, ((h >> 8) & 255) as f32 / 255.0, (h & 255) as f32 / 255.0]
}

/// Homography taking the unit square (0,0),(1,0),(1,1),(0,1) to `c`.
fn square_to_quad(c: [[f32; 2]; 4]) -> [[f32; 3]; 3] {
    let [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = c.map(|p| p.map(|v| v as f64));
    let sx = x0 - x1 + x2 - x3;
    let sy = y0 - y1 + y2 - y3;
    let (dx1, dx2, dy1, dy2) = (x1 - x2, x3 - x2, y1 - y2, y3 - y2);
    let den = dx1 * dy2 - dx2 * dy1;
    let (g, h) = if den.abs() < 1e-12 { (0.0, 0.0) } else { ((sx * dy2 - dx2 * sy) / den, (dx1 * sy - sx * dy1) / den) };
    let m = [
        [x1 - x0 + g * x1, x3 - x0 + h * x3, x0],
        [y1 - y0 + g * y1, y3 - y0 + h * y3, y0],
        [g, h, 1.0],
    ];
    m.map(|r| r.map(|v| v as f32))
}

fn invert3(m: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let m = m.map(|r| r.map(|v| v as f64));
    let c = |r0: usize, c0: usize, r1: usize, c1: usize| m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0];
    let det = m[0][0] * c(1, 1, 2, 2) - m[0][1] * c(1, 0, 2, 2) + m[0][2] * c(1, 0, 2, 1);
    if det.abs() < 1e-14 {
        return None;
    }
    let inv = [
        [c(1, 1, 2, 2), -c(0, 1, 2, 2), c(0, 1, 1, 2)],
        [-c(1, 0, 2, 2), c(0, 0, 2, 2), -c(0, 0, 1, 2)],
        [c(1, 0, 2, 1), -c(0, 0, 2, 1), c(0, 0, 1, 1)],
    ];
    Some(inv.map(|r| r.map(|v| (v / det) as f32)))
}

// ---------------------------------------------------------------------------
// Animation helpers.

/// 0 before `a`, 1 after `b`, linear between.
pub fn seg(t: f32, a: f32, b: f32) -> f32 {
    ((t - a) / (b - a).max(1e-6)).clamp(0.0, 1.0)
}
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
pub fn smooth(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
pub fn ease_out(t: f32) -> f32 {
    let x = 1.0 - t.clamp(0.0, 1.0);
    1.0 - x * x * x
}
pub fn ease_in(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * x
}
pub fn ease_out2(t: f32) -> f32 {
    let x = 1.0 - t.clamp(0.0, 1.0);
    1.0 - x * x
}
pub fn ease_in2(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x
}
/// Back-out ease (overshoots, then settles).
pub fn back_out(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0) - 1.0;
    let k = 1.70158;
    1.0 + x * x * ((k + 1.0) * x + k)
}
/// Triangle envelope: 0 at `a` and `c`, 1 at `b`.
pub fn tri(t: f32, a: f32, b: f32, c: f32) -> f32 {
    if t < b {
        seg(t, a, b)
    } else {
        1.0 - seg(t, b, c)
    }
}
/// Piecewise-linear keyframes `(t, value)`, t ascending.
pub fn kf(t: f32, keys: &[(f32, f32)]) -> f32 {
    if keys.is_empty() {
        return 0.0;
    }
    if t <= keys[0].0 {
        return keys[0].1;
    }
    for w in keys.windows(2) {
        if t < w[1].0 {
            return lerp(w[0].1, w[1].1, seg(t, w[0].0, w[1].0));
        }
    }
    keys[keys.len() - 1].1
}
/// Smoothstep-interpolated keyframes.
pub fn kfs(t: f32, keys: &[(f32, f32)]) -> f32 {
    if keys.is_empty() {
        return 0.0;
    }
    if t <= keys[0].0 {
        return keys[0].1;
    }
    for w in keys.windows(2) {
        if t < w[1].0 {
            return lerp(w[0].1, w[1].1, smooth(seg(t, w[0].0, w[1].0)));
        }
    }
    keys[keys.len() - 1].1
}
/// Deterministic pseudo-random number in 0..1.
pub fn rnd(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B) ^ 0x27D4_EB2F;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}
/// Mix of two hex colours.
pub fn mixc(a: u32, b: u32, t: f32) -> [f32; 3] {
    let (a, b) = (hexc(a), hexc(b));
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = std::f32::consts::TAU;

// ---------------------------------------------------------------------------
// Effect.

/// Scene function: (progress, aspect) -> scene.
pub type SceneFn = fn(f32, f32) -> Scene;

/// One art3b item.
pub struct ArtItem {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub scene: SceneFn,
}

struct ArtFx {
    info: FxInfo,
    scene: SceneFn,
}

fn shader() -> &'static ShaderSrc {
    static S: OnceLock<ShaderSrc> = OnceLock::new();
    S.get_or_init(|| ShaderSrc::new("jb.art3b.prims", include_str!("../shaders/art3b/prims.wgsl")))
}

/// Number of primitives per draw (uniform space: 32 vec4, 5 per primitive).
const PER_DRAW: usize = 6;

impl Effect for ArtFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let out = io.out;
        let asp = out.width as f32 / out.height.max(1) as f32;
        let sc = (self.scene)(io.progress.clamp(0.0, 1.0), asp);
        let b = io.b.unwrap_or(io.a);
        let a_t = if sc.blur_a > 0.0 { cx.blur(io.a, sc.blur_a * io.a.height as f32, sc.blur_a * io.a.height as f32) } else { io.a.clone() };
        let b_t = if sc.blur_b > 0.0 { cx.blur(b, sc.blur_b * b.height as f32, sc.blur_b * b.height as f32) } else { b.clone() };
        let sh = shader();
        let mut u = Uniforms::default();
        u.progress = io.progress;
        u.time = io.time;
        u.e[0] = sc.a.v();
        u.e[1] = sc.b.v();
        u.e[2] = [sc.mix, sc.gain_a, sc.gain_b, if sc.black_outside { 1.0 } else { 0.0 }];
        u.e[3] = sc.dip;
        let mut p = Pass::full(sh, out, u);
        p.fs = "fs_base";
        p.a = Bind::Tex(&a_t);
        p.b = Bind::Tex(&b_t);
        cx.draw(p);
        for chunk in sc.prims.chunks(PER_DRAW) {
            let mut u = Uniforms::default();
            u.progress = io.progress;
            for (i, pr) in chunk.iter().enumerate() {
                for k in 0..5 {
                    u.p[i * 5 + k] = pr.v[k];
                }
            }
            let mut p = Pass::full(sh, out, u);
            p.vs = "vs_prim";
            p.fs = "fs_prim";
            p.a = Bind::Tex(&a_t);
            p.b = Bind::Tex(&b_t);
            p.blend = crate::fx::Blend::Over;
            p.load = true;
            p.vertices = 6 * chunk.len() as u32;
            cx.draw(p);
        }
    }
}

/// Registers every item as a transition.
pub fn register_items(r: &mut Registry, items: &[ArtItem]) {
    for it in items {
        r.register(ArtFx {
            info: info(it.id, it.name, it.category, FxKind::Transition, it.description, &[]),
            scene: it.scene,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homography_maps_corners() {
        let c = [[-0.5, -0.3], [0.6, -0.25], [0.4, 0.35], [-0.45, 0.3]];
        let h = square_to_quad(c);
        for (s, q) in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]].iter().zip(c) {
            let w = h[2][0] * s[0] + h[2][1] * s[1] + h[2][2];
            let x = (h[0][0] * s[0] + h[0][1] * s[1] + h[0][2]) / w;
            let y = (h[1][0] * s[0] + h[1][1] * s[1] + h[1][2]) / w;
            assert!((x - q[0]).abs() < 1e-5 && (y - q[1]).abs() < 1e-5, "{s:?} -> {x},{y} != {q:?}");
        }
        assert!(invert3(h).is_some());
    }

    #[test]
    fn keyframes() {
        assert_eq!(kf(0.5, &[(0.0, 0.0), (1.0, 2.0)]), 1.0);
        assert_eq!(kf(2.0, &[(0.0, 0.0), (1.0, 2.0)]), 2.0);
        assert!(rnd(3, 1) >= 0.0 && rnd(3, 1) < 1.0);
    }
}

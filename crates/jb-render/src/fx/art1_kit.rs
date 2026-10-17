//! Shared machinery of the "art1" items: light leaks, lens flares, film burns,
//! bokeh, sparkles, particles, glows, fire and energy overlays drawn with our
//! own procedural artwork (docs/fidelity-art1.md).
//!
//! Every item is one WGSL file under `shaders/art1/` that defines
//! `fn fx(uv: vec2f) -> vec4f` (and, for two-pass items, `fn pre(uv)`), on top
//! of the shared library `shaders/art1/kit.wgsl` (noise, glows, particles,
//! flare elements, paths, sparkles, simple drawn objects). Its keyframes live
//! in the same file as comment lines, so an item is data plus a short shader:
//!
//! ```text
//! //@time prog            key time: transition progress 0..1 (default), or
//! //@time sec 2.002       clip seconds, repeating with that period (effects)
//! //@p3                   following rows fill uniform slot u.p[3]
//! //@ 0.00 0.1 0.2 0.3 0.4    key time, then up to four values (linear
//! //@ 0.50 1.0 0.2 0.3 0.4    interpolation; a repeated time is a jump)
//! ```
//!
//! `//@base ID` renders the registered effect ID first (for example the
//! fitted colour-field stand-in of the same Filmora item, which carries its
//! measured timing and broad colour layout) and binds it as the aux texture
//! (`a1_base`), so the artwork can be composed over it.
//!
//! Two-pass items (`//@pre SIGMA`) first render `pre(uv)` at half size, blur
//! it with a gaussian of SIGMA (fraction of the frame height) and bind the
//! result as the aux texture (`sx`) of the main pass: bloom and edge glows.
//!
//! Timing, motion paths, coverage and palettes were measured on Filmora's
//! exports of the matching items; the artwork itself (shapes, particles,
//! objects) is drawn by these shaders and is our own. No Filmora asset and no
//! pixels of the exports are used.
//!
//! For development, `JB_ART1_WGSL_DIR=<dir>` loads the item files and the kit
//! from that directory at start-up instead of the copies built into the binary.

use crate::fx::{info, Bind, FxCx, Pass, ShaderSrc, Uniforms};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, Registry};
use std::borrow::Cow;
use std::sync::Arc;

/// The shared WGSL library.
pub const KIT: &str = include_str!("../shaders/art1/kit.wgsl");

/// One item: registry metadata plus its WGSL file (stem and built-in text).
pub struct ArtSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub kind: FxKind,
    pub description: &'static str,
    pub file: &'static str,
    pub wgsl: &'static str,
}

/// Keyframe table bound to one uniform slot: rows `[t, v0, v1, v2, v3]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Keys {
    pub slot: usize,
    pub rows: Vec<[f32; 5]>,
}

impl Keys {
    /// Value at key time `t` (clamped at the ends, linear in between).
    pub fn eval(&self, t: f32) -> [f32; 4] {
        let r = &self.rows;
        let pick = |row: &[f32; 5]| [row[1], row[2], row[3], row[4]];
        if r.is_empty() {
            return [0.0; 4];
        }
        if t <= r[0][0] {
            return pick(&r[0]);
        }
        for w in r.windows(2) {
            if t < w[1][0] {
                let k = (t - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
                let mut o = [0.0; 4];
                for (i, v) in o.iter_mut().enumerate() {
                    *v = w[0][i + 1] + (w[1][i + 1] - w[0][i + 1]) * k;
                }
                return o;
            }
        }
        pick(&r[r.len() - 1])
    }
}

/// Key time of an item: transition progress, or clip seconds with a period.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyTime {
    Progress,
    Seconds { period: f32 },
}

/// Parsed `//@` directives of an item file.
#[derive(Debug, Clone, PartialEq)]
pub struct Directives {
    pub time: KeyTime,
    pub keys: Vec<Keys>,
    /// Blur of the pre pass (fraction of the frame height), for two-pass items.
    pub pre_sigma: Option<f32>,
    /// Registered effect rendered first and bound as the aux texture (`//@base ID`).
    pub base: Option<String>,
}

/// Parses the `//@` lines of an item file (see the module docs).
pub fn parse_directives(src: &str) -> Result<Directives, String> {
    let mut d = Directives { time: KeyTime::Progress, keys: Vec::new(), pre_sigma: None, base: None };
    for (n, line) in src.lines().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("//@") else { continue };
        let err = |m: &str| format!("line {}: {m}: {line}", n + 1);
        let mut it = rest.split_whitespace();
        let head = it.next().unwrap_or("");
        if head == "time" {
            d.time = match it.next() {
                Some("prog") => KeyTime::Progress,
                Some("sec") => {
                    let period = it.next().and_then(|v| v.parse().ok()).ok_or_else(|| err("period"))?;
                    KeyTime::Seconds { period }
                }
                _ => return Err(err("time prog|sec PERIOD")),
            };
        } else if head == "base" {
            d.base = Some(it.next().ok_or_else(|| err("base id"))?.to_string());
        } else if head == "pre" {
            d.pre_sigma = Some(it.next().and_then(|v| v.parse().ok()).ok_or_else(|| err("sigma"))?);
        } else if let Some(slot) = head.strip_prefix('p') {
            let slot: usize = slot.parse().map_err(|_| err("slot"))?;
            if slot >= 32 {
                return Err(err("slot out of range"));
            }
            d.keys.push(Keys { slot, rows: Vec::new() });
        } else {
            let vals: Vec<f32> =
                rest.split_whitespace().map(|v| v.parse::<f32>()).collect::<Result<_, _>>().map_err(|_| err("number"))?;
            if vals.is_empty() {
                continue;
            }
            let keys = d.keys.last_mut().ok_or_else(|| err("row before a //@pN header"))?;
            let mut row = [0.0; 5];
            for (o, v) in row.iter_mut().zip(&vals) {
                *o = *v;
            }
            if vals.len() > 5 {
                return Err(err("more than four values"));
            }
            if keys.rows.last().is_some_and(|r| r[0] > row[0]) {
                return Err(err("key times must ascend"));
            }
            keys.rows.push(row);
        }
    }
    Ok(d)
}

/// A registered art item.
pub struct ArtFx {
    pub info: FxInfo,
    pub dir: Directives,
    pub main: ShaderSrc,
    /// The `//@base` effect, resolved at registration.
    pub base: Option<Arc<dyn Effect>>,
}

fn dev_file(name: &str) -> Option<String> {
    let dir = std::env::var_os("JB_ART1_WGSL_DIR")?;
    std::fs::read_to_string(std::path::Path::new(&dir).join(name)).ok()
}

impl ArtFx {
    pub fn new(spec: &ArtSpec) -> Self {
        let item: Cow<'static, str> =
            dev_file(&format!("{}.wgsl", spec.file)).map_or(Cow::Borrowed(spec.wgsl), Cow::Owned);
        let kit: Cow<'static, str> = dev_file("kit.wgsl").map_or(Cow::Borrowed(KIT), Cow::Owned);
        let dir = parse_directives(&item).unwrap_or_else(|e| panic!("{}: {e}", spec.file));
        let mut body = format!("{kit}\n{item}\n@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return fx(in.uv); }}\n");
        if dir.pre_sigma.is_some() {
            body.push_str("@fragment fn fs_pre(in: VsOut) -> @location(0) vec4f { return pre(in.uv); }\n");
        }
        let info = info(spec.id, spec.name, spec.category, spec.kind, spec.description, &[]);
        Self { main: ShaderSrc::new(spec.id, body), info, dir, base: None }
    }

    /// Key time for an invocation.
    pub fn key_time(&self, progress: f32, time: f32) -> f32 {
        match self.dir.time {
            KeyTime::Progress => progress,
            KeyTime::Seconds { period } if period > 0.0 => time.rem_euclid(period),
            KeyTime::Seconds { .. } => time,
        }
    }

    /// Uniforms of one invocation: keyed slots, key time in `e0.x`.
    pub fn uniforms(&self, progress: f32, time: f32) -> Uniforms {
        let mut u = Uniforms { progress, time, ..Default::default() };
        let t = self.key_time(progress, time);
        for k in &self.dir.keys {
            u.p[k.slot] = k.eval(t);
        }
        u.e[0] = [t, progress, time, 0.0];
        u
    }
}

impl Effect for ArtFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let u = self.uniforms(io.progress, io.time);
        let base = self.base.as_ref().map(|fx| {
            let t = cx.temp_like(io.out);
            fx.encode(cx, &FxIo { a: io.a, b: io.b, out: &t, progress: io.progress, time: io.time, params: io.params });
            t
        });
        let blurred = self.dir.pre_sigma.map(|sigma| {
            let (w, h) = ((io.out.width / 2).max(1), (io.out.height / 2).max(1));
            let pre = cx.temp(w, h);
            let mut p = Pass::full(&self.main, &pre, u);
            p.fs = "fs_pre";
            p.a = Bind::Tex(io.a);
            p.b = io.b.map_or(Bind::None, Bind::Tex);
            cx.draw(p);
            let s = sigma * h as f32;
            cx.blur(&pre, s, s)
        });
        let mut p = Pass::full(&self.main, io.out, u);
        p.a = Bind::Tex(io.a);
        p.b = io.b.map_or(Bind::None, Bind::Tex);
        if let Some(t) = blurred.as_ref().or(base.as_ref()) {
            p.aux = Bind::Tex(t);
        }
        cx.draw(p);
    }
}

/// Registers every spec.
pub fn register_all(r: &mut Registry, specs: &[ArtSpec]) {
    for s in specs {
        let mut fx = ArtFx::new(s);
        if let Some(id) = &fx.dir.base {
            let base = r.get(id).cloned().unwrap_or_else(|| panic!("{}: base {id} is not registered", s.id));
            // The base effect gets this item's parameters, so it must not take any.
            assert!(base.info().params.is_empty(), "{}: base {id} has parameters", s.id);
            fx.base = Some(base);
        }
        r.register(fx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives_parse_and_interpolate() {
        let src = "//@time sec 2.0\n//@pre 0.02\n//@p2\n//@ 0 0 1\n//@ 1 1 3\n//@ 1 5 5\nfn fx() {}\n";
        let d = parse_directives(src).unwrap();
        assert_eq!(d.time, KeyTime::Seconds { period: 2.0 });
        assert_eq!(d.pre_sigma, Some(0.02));
        let k = &d.keys[0];
        assert_eq!(k.slot, 2);
        assert_eq!(k.eval(0.5), [0.5, 2.0, 0.0, 0.0]);
        assert_eq!(k.eval(1.0), [5.0, 5.0, 0.0, 0.0]);
        assert_eq!(k.eval(-1.0), [0.0, 1.0, 0.0, 0.0]);
        assert!(parse_directives("//@ 0 1\n").is_err());
        assert!(parse_directives("//@p0\n//@ 1 0\n//@ 0 0\n").is_err());
    }
}

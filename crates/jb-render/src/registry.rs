//! Effect and transition registry.
//!
//! Every effect, filter, transition and animation is an [`Effect`] keyed by a
//! stable `kind` string (`jb.dissolve`, `jb.gaussian_blur`, ...). Imported
//! projects keep their original identifiers (`filmora:<id>`); the alias table
//! maps those onto our implementations. Kinds that resolve to nothing render
//! as a pass-through and are reported in [`crate::RenderReport`].

use crate::fx::FxCx;
use jb_core::param::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Which library tab / evaluation path an item belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxKind {
    /// Blends the outgoing clip (A) into the incoming one (B) in canvas space.
    Transition,
    /// Pixel effect applied to a clip in its own (layer) space.
    Effect,
    /// Color look (Filmora "Filters").
    Filter,
    /// Entry/exit animation: modifies the clip transform and opacity.
    Animation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamType {
    Float,
    /// Integer-valued slider (still stored as a float).
    Int,
    Bool,
    /// Linear RGBA color.
    Color,
    Vec2,
    /// Free text (curve points, LUT path, ...).
    Text,
}

/// Schema of one effect parameter, exposed to the UI.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    pub name: &'static str,
    pub label: &'static str,
    pub ty: ParamType,
    pub min: f32,
    pub max: f32,
    /// Default value; floats use `default[0]`, vec2 `[0..2]`, colors all 4.
    pub default: [f32; 4],
    pub text_default: &'static str,
    pub unit: &'static str,
}

impl ParamSpec {
    pub const fn float(name: &'static str, label: &'static str, min: f32, max: f32, default: f32) -> Self {
        Self {
            name,
            label,
            ty: ParamType::Float,
            min,
            max,
            default: [default, 0.0, 0.0, 0.0],
            text_default: "",
            unit: "",
        }
    }
    pub const fn int(name: &'static str, label: &'static str, min: f32, max: f32, default: f32) -> Self {
        Self { ty: ParamType::Int, ..Self::float(name, label, min, max, default) }
    }
    pub const fn boolean(name: &'static str, label: &'static str, default: bool) -> Self {
        Self { ty: ParamType::Bool, ..Self::float(name, label, 0.0, 1.0, if default { 1.0 } else { 0.0 }) }
    }
    pub const fn color(name: &'static str, label: &'static str, default: [f32; 4]) -> Self {
        Self { name, label, ty: ParamType::Color, min: 0.0, max: 1.0, default, text_default: "", unit: "" }
    }
    pub const fn vec2(name: &'static str, label: &'static str, min: f32, max: f32, default: [f32; 2]) -> Self {
        Self {
            name,
            label,
            ty: ParamType::Vec2,
            min,
            max,
            default: [default[0], default[1], 0.0, 0.0],
            text_default: "",
            unit: "",
        }
    }
    pub const fn text(name: &'static str, label: &'static str, default: &'static str) -> Self {
        Self {
            name,
            label,
            ty: ParamType::Text,
            min: 0.0,
            max: 0.0,
            default: [0.0; 4],
            text_default: default,
            unit: "",
        }
    }
    pub const fn unit(mut self, unit: &'static str) -> Self {
        self.unit = unit;
        self
    }
}

/// Static description of an effect.
#[derive(Debug, Clone)]
pub struct FxInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub kind: FxKind,
    pub description: &'static str,
    pub params: Vec<ParamSpec>,
}

/// Parameter values of one effect instance, with schema defaults filled in.
#[derive(Debug, Clone, Default)]
pub struct ParamValues {
    /// One vec4 slot per schema parameter, in schema order (shader layout).
    pub slots: Vec<[f32; 4]>,
    pub texts: BTreeMap<&'static str, String>,
    names: Vec<&'static str>,
}

impl ParamValues {
    pub fn resolve(info: &FxInfo, given: &BTreeMap<String, Value>) -> Self {
        let mut out = ParamValues::default();
        for spec in &info.params {
            let mut slot = spec.default;
            match given.get(spec.name) {
                Some(Value::Float(v)) => slot[0] = *v as f32,
                Some(Value::Bool(b)) => slot[0] = if *b { 1.0 } else { 0.0 },
                Some(Value::Vec2(v)) => {
                    slot[0] = v[0] as f32;
                    slot[1] = v[1] as f32;
                }
                Some(Value::Color(c)) => slot = c.map(|x| x as f32),
                Some(Value::Text(t)) => {
                    if spec.ty == ParamType::Text {
                        out.texts.insert(spec.name, t.clone());
                    } else if let Ok(v) = t.trim().parse::<f32>() {
                        slot[0] = v;
                    }
                }
                None => {
                    if spec.ty == ParamType::Text {
                        out.texts.insert(spec.name, spec.text_default.to_string());
                    }
                }
            }
            if matches!(spec.ty, ParamType::Float | ParamType::Int) && spec.max > spec.min {
                slot[0] = slot[0].clamp(spec.min, spec.max);
            }
            out.slots.push(slot);
            out.names.push(spec.name);
        }
        out
    }

    pub fn defaults(info: &FxInfo) -> Self {
        Self::resolve(info, &BTreeMap::new())
    }

    pub fn get(&self, name: &str) -> f32 {
        self.slot(name).map_or(0.0, |s| s[0])
    }

    pub fn slot(&self, name: &str) -> Option<[f32; 4]> {
        self.names.iter().position(|n| *n == name).map(|i| self.slots[i])
    }

    pub fn text(&self, name: &str) -> &str {
        self.texts.get(name).map_or("", |s| s.as_str())
    }
}

/// Transform/opacity state modified by animation effects. Units follow
/// [`jb_core::eval::ResolvedTransform`].
#[derive(Debug, Clone, PartialEq)]
pub struct AnimState {
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation: f64,
    pub opacity: f64,
    /// Extra gaussian blur in layer pixels (0 = none).
    pub blur: f64,
}

/// Inputs and output of one effect invocation.
pub struct FxIo<'a> {
    /// Layer image (or outgoing clip A for transitions), premultiplied, working space.
    pub a: &'a crate::pool::Tex,
    /// Incoming clip B for transitions.
    pub b: Option<&'a crate::pool::Tex>,
    /// Destination (same size as `a`).
    pub out: &'a crate::pool::Tex,
    /// 0..1 progress through the transition / effect range.
    pub progress: f32,
    /// Clip-local time in seconds (animated effects).
    pub time: f32,
    /// Resolved parameters. Sizes are expressed as fractions of the image
    /// height (a "1080p pixel" is 1/1080), so results do not depend on the
    /// render resolution.
    pub params: &'a ParamValues,
}

/// An effect implementation. Most are a single WGSL pass (see
/// [`crate::fx::ShaderFx`]); multi-pass ones implement `encode` themselves.
pub trait Effect: Send + Sync {
    fn info(&self) -> &FxInfo;

    /// Records the GPU passes that produce `io.out`. Animations keep the
    /// default (a copy).
    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        cx.copy(io.a, io.out);
    }

    /// Animation hook: modifies the layer transform/opacity.
    fn animate(&self, _state: &mut AnimState, _progress: f32, _params: &ParamValues) {}

    /// Whether `encode` has to run (false for pure animations).
    fn has_pixels(&self) -> bool {
        self.info().kind != FxKind::Animation
    }
}

#[derive(Default, Clone)]
pub struct Registry {
    items: HashMap<&'static str, Arc<dyn Effect>>,
    order: Vec<&'static str>,
    aliases: HashMap<String, &'static str>,
}

impl Registry {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Registry with every built-in effect, transition, filter and animation.
    pub fn builtin() -> Self {
        let mut r = Self::empty();
        crate::fx::register_builtin(&mut r);
        r
    }

    pub fn register(&mut self, fx: impl Effect + 'static) {
        self.register_arc(Arc::new(fx));
    }

    pub fn register_arc(&mut self, fx: Arc<dyn Effect>) {
        let id = fx.info().id;
        if self.items.insert(id, fx).is_some() {
            panic!("duplicate effect id {id:?}");
        }
        self.order.push(id);
    }

    /// Maps an external identifier (e.g. `filmora:12345`) onto a registered id.
    pub fn alias(&mut self, from: impl Into<String>, to: &'static str) {
        self.aliases.insert(from.into(), to);
    }

    /// Resolves a kind (or alias) to its implementation.
    pub fn get(&self, kind: &str) -> Option<&Arc<dyn Effect>> {
        self.items.get(kind).or_else(|| self.aliases.get(kind).and_then(|id| self.items.get(id)))
    }

    /// Canonical id for a kind, following aliases.
    pub fn canonical(&self, kind: &str) -> Option<&'static str> {
        self.get(kind).map(|e| e.info().id)
    }

    /// All registered items in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &FxInfo> {
        self.order.iter().map(|id| self.items[id].info())
    }

    pub fn of_kind(&self, kind: FxKind) -> impl Iterator<Item = &FxInfo> {
        self.iter().filter(move |i| i.kind == kind)
    }

    pub fn aliases(&self) -> impl Iterator<Item = (&str, &'static str)> {
        self.aliases.iter().map(|(k, v)| (k.as_str(), *v))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

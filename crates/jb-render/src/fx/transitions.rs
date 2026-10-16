//! Basic transitions, dissolves and light transitions.

use super::{info, shader};
use crate::registry::{FxKind::Transition, Registry};

pub fn register(r: &mut Registry) {
    shader(
        r,
        info("jb.dissolve", "Dissolve", "Basic", Transition, "Classic cross-fade.", &[]),
        "fn fx(uv: vec2f) -> vec4f { return mix(sa(uv), sb(uv), prog()); }",
    );
}

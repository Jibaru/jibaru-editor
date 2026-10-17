//! GPU compositor and effect/transition registry built on wgpu.
//!
//! ```no_run
//! # fn demo(project: &jb_core::Project, source: &dyn jb_core::FrameSource) -> anyhow::Result<()> {
//! let mut r = jb_render::Renderer::new(1920, 1080)?;
//! let plan = jb_core::eval::evaluate(project, project.main(), 0);
//! let frame = r.render(project, &plan, source); // RGBA8, sRGB, straight alpha
//! assert!(r.last_report().unknown_kinds.is_empty());
//! # let _ = frame; Ok(()) }
//! ```
//!
//! See [`registry`] for the effect catalog and [`Renderer`] for the pipeline.

pub mod aliases;
pub mod aliases_art3a;
pub mod aliases_art2;
pub mod aliases_lane1;
pub mod aliases_lane2;
pub mod aliases_lane3;
pub mod aliases_lane4;
pub mod aliases_art3b;
pub mod aliases_art1;
pub mod fx;
mod gpu;
pub mod library;
pub mod lut;
pub mod pool;
pub mod preview;
pub mod registry;
mod render;
pub mod testing;
pub mod text;

pub use gpu::{Gpu, GpuError};
pub use registry::{AnimState, Effect, FxInfo, FxIo, FxKind, ParamSpec, ParamType, ParamValues, Registry};
pub use render::{RenderOptions, RenderReport, Renderer};

//! The compositor: turns a [`FramePlan`] into pixels.
//!
//! Per layer: the source (decoded frame, nested sequence, title or solid
//! color) is resampled with crop and fit into a *layer texture*; the clip's
//! pixel effects run there (layer space), animations adjust its transform,
//! and the result is drawn as a transformed quad over the accumulator with
//! the clip's opacity and blend mode. Transitions render both sides onto
//! canvas-sized textures and blend those. Adjustment layers run their effects
//! over a copy of everything below them.

use crate::fx::{
    AuxImage, Bind, Blend, Dummies, FxCx, Pass, Pipelines, ShaderSrc, Uniforms, PRELUDE,
};
use crate::pool::{Tex, TexturePool, UniformArena};
use crate::registry::{AnimState, FxIo, FxKind, ParamValues, Registry};
use crate::text::TextEngine;
use crate::{Gpu, GpuError};
use jb_core::eval::{FramePlan, Layer, LayerContent, ResolvedEffect};
use jb_core::frame::{ColorSpace, FrameSource, PixelFormat, VideoFrame};
use jb_core::{BlendMode, FitMode, Project};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Rendering options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderOptions {
    /// Composite, blend and blur in linear light instead of display-referred
    /// (gamma-encoded) values. Off by default because Filmora, like most
    /// consumer editors, mixes in display space; color effects behave the same
    /// either way because they convert to display space internally.
    pub linear_light: bool,
    /// Hint passed to the frame source so decoders can scale down (previews).
    pub source_max_size: Option<(u32, u32)>,
}

/// Diagnostics of the last rendered frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderReport {
    /// Effect/transition kinds that have no implementation (rendered as pass-through).
    pub unknown_kinds: BTreeSet<String>,
    /// Non-fatal problems (missing media, bad frames, ...).
    pub errors: Vec<String>,
    pub layers: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SrcFormat {
    Rgba8 = 0,
    Nv12 = 1,
    P010 = 2,
    Working = 3,
    Solid = 4,
}

struct UploadSlot {
    format: PixelFormat,
    width: u32,
    height: u32,
    planes: Vec<(wgpu::Texture, wgpu::TextureView, [u32; 2])>,
    /// (media id, pts, data pointer) currently stored.
    content: Option<(String, i64, usize)>,
    used_frame: u64,
}

struct CoreShaders {
    comp: ShaderSrc,
    title: ShaderSrc,
}

fn core_shaders() -> &'static CoreShaders {
    static S: std::sync::OnceLock<CoreShaders> = std::sync::OnceLock::new();
    S.get_or_init(|| CoreShaders {
        comp: ShaderSrc::new("jb.internal.compositor", include_str!("shaders/compositor.wgsl")),
        title: ShaderSrc::new("jb.internal.title", include_str!("shaders/title.wgsl")),
    })
}

/// GPU compositor for one canvas size.
pub struct Renderer {
    gpu: Gpu,
    width: u32,
    height: u32,
    registry: Arc<Registry>,
    pub options: RenderOptions,
    pipelines: Pipelines,
    pool: TexturePool,
    uniforms: UniformArena,
    aux: crate::fx::AuxCache,
    dummies: Dummies,
    uploads: Vec<UploadSlot>,
    text: Option<TextEngine>,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
    readback: Option<wgpu::Buffer>,
    report: RenderReport,
    frame: u64,
}

impl Renderer {
    /// Creates a headless renderer on the best available GPU.
    pub fn new(width: u32, height: u32) -> Result<Self, GpuError> {
        Ok(Self::with_gpu(Gpu::headless()?, width, height))
    }

    /// Creates a renderer on an existing device (e.g. the editor's), so
    /// [`Renderer::render_to_texture`] results can be drawn without copies.
    pub fn with_gpu(gpu: Gpu, width: u32, height: u32) -> Self {
        let pipelines = Pipelines::new(&gpu.device);
        let dummies = Dummies::new(&gpu.device, &gpu.queue);
        let (output, output_view) = make_output(&gpu.device, width, height);
        Self {
            width,
            height,
            registry: Arc::new(Registry::builtin()),
            options: RenderOptions::default(),
            pipelines,
            pool: TexturePool::new(),
            uniforms: UniformArena::new(),
            aux: Default::default(),
            dummies,
            uploads: Vec::new(),
            text: None,
            output,
            output_view,
            readback: None,
            report: RenderReport::default(),
            frame: 0,
            gpu,
        }
    }

    pub fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Changes the output size (e.g. preview resolution).
    pub fn resize(&mut self, width: u32, height: u32) {
        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;
            let (t, v) = make_output(&self.gpu.device, width, height);
            self.output = t;
            self.output_view = v;
            self.readback = None;
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Replaces the effect registry (e.g. to add plugin effects or aliases).
    pub fn set_registry(&mut self, registry: Arc<Registry>) {
        self.registry = registry;
    }

    /// Diagnostics of the last frame.
    pub fn last_report(&self) -> &RenderReport {
        &self.report
    }

    /// Renders a frame and reads it back as straight-alpha sRGB RGBA8.
    pub fn render(&mut self, project: &Project, plan: &FramePlan, source: &dyn FrameSource) -> VideoFrame {
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        self.encode_frame(&mut encoder, project, plan, source);
        let (w, h) = (self.width, self.height);
        let bpr = (w * 4).div_ceil(256) * 256;
        let size = bpr as u64 * h as u64;
        if self.readback.as_ref().is_none_or(|b| b.size() != size) {
            self.readback = Some(self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("jb-readback"),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let buf = self.readback.clone().unwrap();
        encoder.copy_texture_to_buffer(
            self.output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.submit(encoder);
        buf.map_async(wgpu::MapMode::Read, .., |r| {
            if let Err(e) = r {
                panic!("readback map failed: {e}");
            }
        });
        self.gpu.device.poll(wgpu::PollType::wait_indefinitely()).expect("GPU device lost");
        let mut data = vec![0u8; (w * h * 4) as usize];
        {
            let view = buf.get_mapped_range(..).expect("mapped range");
            let row = (w * 4) as usize;
            for y in 0..h as usize {
                data[y * row..(y + 1) * row].copy_from_slice(&view[y * bpr as usize..y * bpr as usize + row]);
            }
        }
        buf.unmap();
        VideoFrame {
            width: w,
            height: h,
            format: PixelFormat::Rgba8,
            color_space: ColorSpace::Bt709,
            full_range: true,
            stride: (w * 4) as usize,
            data: Arc::new(data),
            pts: plan.time,
        }
    }

    /// Renders a frame into the renderer-owned output texture (RGBA8,
    /// sRGB-encoded values, straight alpha) without reading it back. The
    /// texture can be sampled through an `Rgba8UnormSrgb` view for automatic
    /// decoding. Valid until the next render call.
    pub fn render_to_texture(&mut self, project: &Project, plan: &FramePlan, source: &dyn FrameSource) -> &wgpu::Texture {
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        self.encode_frame(&mut encoder, project, plan, source);
        self.submit(encoder);
        &self.output
    }

    /// Blocks until all submitted GPU work is done (timing, tests).
    pub fn wait_idle(&self) {
        let _ = self.gpu.device.poll(wgpu::PollType::wait_indefinitely());
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.output
    }

    fn submit(&mut self, encoder: wgpu::CommandEncoder) {
        self.uniforms.flush(&self.gpu.queue);
        self.gpu.queue.submit([encoder.finish()]);
        self.pool.end_frame();
        self.aux.end_frame();
        let f = self.frame;
        self.uploads.retain(|s| f - s.used_frame < 120);
        self.frame += 1;
    }

    fn encode_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        project: &Project,
        plan: &FramePlan,
        source: &dyn FrameSource,
    ) {
        self.report = RenderReport::default();
        let flags = if self.options.linear_light { 1 } else { 0 };
        let canvas = [self.width, self.height];
        let mut f = Frame {
            cx: FxCx {
                device: &self.gpu.device,
                queue: &self.gpu.queue,
                encoder,
                pipelines: &mut self.pipelines,
                pool: &mut self.pool,
                uniforms: &mut self.uniforms,
                aux: &mut self.aux,
                dummies: &self.dummies,
                report: &mut self.report,
                flags,
            },
            registry: &self.registry,
            text: &mut self.text,
            uploads: &mut self.uploads,
            source,
            project,
            canvas,
            frame: self.frame,
            options: &self.options,
        };
        let bg = project.settings.background;
        let accum = f.render_plan(plan, Some(bg), 0);
        let sh = core_shaders();
        let mut p = Pass::full(&sh.comp, &accum, Uniforms::default());
        p.fs = "fs_encode";
        p.a = Bind::Tex(&accum);
        p.target = &self.output_view;
        p.format = wgpu::TextureFormat::Rgba8Unorm;
        p.target_size = canvas;
        f.cx.draw(p);
    }
}

fn make_output(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("jb-output"),
        size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
    });
    let v = t.create_view(&Default::default());
    (t, v)
}

/// A layer resampled into its own texture, ready to be composited.
struct Prepared {
    tex: Tex,
    /// Size of the fitted layer on the canvas, in canvas pixels (before scale).
    fitted: [f32; 2],
    state: AnimState,
}

struct Frame<'a> {
    cx: FxCx<'a>,
    registry: &'a Registry,
    text: &'a mut Option<TextEngine>,
    uploads: &'a mut Vec<UploadSlot>,
    source: &'a dyn FrameSource,
    #[allow(dead_code)]
    project: &'a Project,
    canvas: [u32; 2],
    frame: u64,
    options: &'a RenderOptions,
}

fn blend_code(m: BlendMode) -> u32 {
    match m {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay => 3,
        BlendMode::Darken => 4,
        BlendMode::Lighten => 5,
        BlendMode::ColorDodge => 6,
        BlendMode::ColorBurn => 7,
        BlendMode::HardLight => 8,
        BlendMode::SoftLight => 9,
        BlendMode::Difference => 10,
        BlendMode::Exclusion => 11,
        BlendMode::Add => 12,
    }
}

/// Linear RGBA -> working-space premultiplied clear color.
fn working_color(c: [f64; 4], linear: bool) -> [f64; 4] {
    let enc = |x: f64| {
        let x = x.max(0.0);
        if linear {
            x
        } else if x <= 0.0031308 {
            x * 12.92
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        }
    };
    [enc(c[0]) * c[3], enc(c[1]) * c[3], enc(c[2]) * c[3], c[3]]
}

impl<'a> Frame<'a> {
    fn canvas_f(&self) -> [f32; 2] {
        [self.canvas[0] as f32, self.canvas[1] as f32]
    }

    fn linear(&self) -> bool {
        self.cx.flags & 1 != 0
    }

    /// Renders `plan` into a new canvas-sized texture.
    fn render_plan(&mut self, plan: &FramePlan, background: Option<[f64; 4]>, depth: usize) -> Tex {
        let [w, h] = self.canvas;
        let accum = self.cx.temp(w, h);
        let clear = background.map_or([0.0; 4], |c| working_color(c, self.linear()));
        self.clear(&accum, clear);
        if depth > 16 {
            return accum;
        }
        for layer in &plan.layers {
            self.cx.report.layers += 1;
            self.draw_layer(&accum, layer, depth);
        }
        accum
    }

    fn clear(&mut self, t: &Tex, color: [f64; 4]) {
        let _ = self.cx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("jb-clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &t.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: color[0], g: color[1], b: color[2], a: color[3] }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    fn draw_layer(&mut self, accum: &Tex, layer: &Layer, depth: usize) {
        if let Some(tr) = &layer.transition {
            let [w, h] = self.canvas;
            let a = self.layer_on_canvas(layer, depth);
            let b = self.layer_on_canvas(&tr.incoming, depth);
            let out = self.cx.temp(w, h);
            match self.registry.get(&tr.kind).filter(|fx| fx.info().kind == FxKind::Transition).cloned() {
                Some(fx) => {
                    let params = ParamValues::resolve(fx.info(), &tr.params);
                    let io = FxIo {
                        a: &a,
                        b: Some(&b),
                        out: &out,
                        progress: tr.progress as f32,
                        time: jb_core::time::ticks_to_seconds(layer.local_time) as f32,
                        params: &params,
                    };
                    fx.encode(&mut self.cx, &io);
                }
                None => {
                    self.cx.report.unknown_kinds.insert(tr.kind.clone());
                    // Pass-through: a plain cut at the middle of the transition.
                    let src = if tr.progress < 0.5 { &a } else { &b };
                    self.cx.copy(src, &out);
                }
            }
            let c = self.canvas_f();
            self.composite(accum, &out, [c[0] * 0.5, c[1] * 0.5], [c[0] * 0.5, c[1] * 0.5], 0.0, 1.0, layer.blend_mode);
            return;
        }
        if let LayerContent::Adjustment = layer.content {
            let [w, h] = self.canvas;
            let input = self.cx.temp(w, h);
            self.cx.copy(accum, &input);
            let mut state = self.initial_state(layer);
            let out = self.apply_effects(input, &layer.effects, &mut state, layer.local_time);
            let c = self.canvas_f();
            let opacity = state.opacity.clamp(0.0, 1.0) as f32;
            self.composite(accum, &out, [c[0] * 0.5, c[1] * 0.5], [c[0] * 0.5, c[1] * 0.5], 0.0, opacity, layer.blend_mode);
            return;
        }
        if let Some(p) = self.prepare(layer, depth) {
            self.composite_prepared(accum, &p, layer.blend_mode);
        }
    }

    /// Renders one side of a transition onto a transparent canvas.
    fn layer_on_canvas(&mut self, layer: &Layer, depth: usize) -> Tex {
        let [w, h] = self.canvas;
        let t = self.cx.temp(w, h);
        self.clear(&t, [0.0; 4]);
        if let Some(p) = self.prepare(layer, depth) {
            self.composite_prepared(&t, &p, BlendMode::Normal);
        }
        t
    }

    fn composite_prepared(&mut self, accum: &Tex, p: &Prepared, mode: BlendMode) {
        let c = self.canvas_f();
        let s = &p.state;
        let center = [c[0] * (0.5 + s.position[0] as f32), c[1] * (0.5 + s.position[1] as f32)];
        let half = [p.fitted[0] * 0.5 * s.scale[0].abs() as f32, p.fitted[1] * 0.5 * s.scale[1].abs() as f32];
        let opacity = s.opacity.clamp(0.0, 1.0) as f32;
        if opacity <= 0.0 || half[0] < 0.01 || half[1] < 0.01 {
            return;
        }
        self.composite(accum, &p.tex, center, half, (s.rotation as f32).to_radians(), opacity, mode);
    }

    #[allow(clippy::too_many_arguments)]
    fn composite(&mut self, accum: &Tex, layer: &Tex, center: [f32; 2], half: [f32; 2], rot: f32, opacity: f32, mode: BlendMode) {
        let code = blend_code(mode);
        let backdrop = if code != 0 {
            let b = self.cx.temp_like(accum);
            self.cx.copy(accum, &b);
            Some(b)
        } else {
            None
        };
        let mut u = Uniforms::default();
        u.e[0] = [center[0], center[1], half[0], half[1]];
        u.e[1] = [rot.cos(), rot.sin(), opacity, code as f32];
        u.e[2] = [1.0, 0.0, 0.0, 0.0];
        let sh = core_shaders();
        let mut p = Pass::full(&sh.comp, accum, u);
        p.vs = "vs_quad";
        p.fs = "fs_composite";
        p.vertices = 4;
        p.load = true;
        p.a = Bind::Tex(layer);
        p.b = backdrop.as_ref().map_or(Bind::None, Bind::Tex);
        p.blend = if code == 0 { Blend::Over } else { Blend::Replace };
        self.cx.draw(p);
    }

    fn initial_state(&self, layer: &Layer) -> AnimState {
        let t = &layer.transform;
        AnimState { position: t.position, scale: t.scale, rotation: t.rotation, opacity: layer.opacity, blur: 0.0 }
    }

    /// Source -> layer texture -> effects.
    fn prepare(&mut self, layer: &Layer, depth: usize) -> Option<Prepared> {
        let canvas = self.canvas_f();
        let t = &layer.transform;
        let crop = t.crop.map(|v| v.clamp(0.0, 1.0) as f32);
        let crop_rect = [crop[0], crop[1], (1.0 - crop[2]).max(crop[0] + 1e-4), (1.0 - crop[3]).max(crop[1] + 1e-4)];
        // Source binding and size.
        let mut u = Uniforms::default();
        u.e[3] = [t.flip_h as u32 as f32, t.flip_v as u32 as f32, 0.0, 0.0];
        let nested_tex;
        let upload_idx;
        let src_size: [u32; 2];
        let format: SrcFormat;
        match &layer.content {
            LayerContent::Media { media_id, source_time } => {
                let frame = match self.source.frame(media_id, *source_time, self.options.source_max_size) {
                    Ok(Some(f)) => f,
                    Ok(None) => return None,
                    Err(e) => {
                        self.cx.report.errors.push(format!("media {media_id}: {e}"));
                        return None;
                    }
                };
                let idx = match self.upload(media_id, &frame) {
                    Ok(i) => i,
                    Err(e) => {
                        self.cx.report.errors.push(format!("media {media_id}: {e}"));
                        return None;
                    }
                };
                upload_idx = Some(idx);
                src_size = [frame.width, frame.height];
                format = match frame.format {
                    PixelFormat::Rgba8 => SrcFormat::Rgba8,
                    PixelFormat::Nv12 => SrcFormat::Nv12,
                    PixelFormat::P010 => SrcFormat::P010,
                };
                let (matrix, transfer) = match frame.color_space {
                    ColorSpace::Bt709 => (0.0, 0.0),
                    ColorSpace::Bt601 => (1.0, 0.0),
                    ColorSpace::Bt2020Pq => (2.0, 1.0),
                    ColorSpace::Bt2020Hlg => (2.0, 2.0),
                };
                u.e[1] = [format as u32 as f32, matrix, frame.full_range as u32 as f32, transfer];
                nested_tex = None;
            }
            LayerContent::Nested(plan) => {
                let tex = self.render_plan(plan, None, depth + 1);
                src_size = self.canvas;
                format = SrcFormat::Working;
                u.e[1] = [3.0, 0.0, 0.0, 0.0];
                nested_tex = Some(tex);
                upload_idx = None;
            }
            LayerContent::Title(td) => {
                let tex = self.title(td);
                src_size = self.canvas;
                format = SrcFormat::Working;
                u.e[1] = [3.0, 0.0, 0.0, 0.0];
                nested_tex = Some(tex);
                upload_idx = None;
            }
            LayerContent::Color(c) => {
                src_size = self.canvas;
                format = SrcFormat::Solid;
                u.e[1] = [4.0, 0.0, 0.0, 0.0];
                u.e[2] = c.map(|x| x as f32);
                nested_tex = None;
                upload_idx = None;
            }
            LayerContent::Adjustment => return None,
        }
        // Geometry: cropped source size, fitted size on the canvas, texture resolution.
        let cw = src_size[0] as f32 * (crop_rect[2] - crop_rect[0]);
        let ch = src_size[1] as f32 * (crop_rect[3] - crop_rect[1]);
        let fitted = match t.fit {
            FitMode::Fit => {
                let s = (canvas[0] / cw).min(canvas[1] / ch);
                [cw * s, ch * s]
            }
            FitMode::Fill => {
                let s = (canvas[0] / cw).max(canvas[1] / ch);
                [cw * s, ch * s]
            }
            FitMode::Stretch => canvas,
            FitMode::None => [cw, ch],
        };
        // Texture resolution follows the on-screen size, quantized to powers of
        // two so animated scales reuse the same pooled textures: shrunk layers
        // (picture-in-picture) are processed at reduced resolution, zoomed-in
        // ones up to the source resolution.
        let zoom = t.scale[0].abs().max(t.scale[1].abs()) as f32;
        let k = if zoom < 1.0 {
            2f32.powf(zoom.max(1.0 / 8.0).log2().ceil())
        } else {
            zoom.min((cw / fitted[0]).max(ch / fitted[1]).max(1.0))
        };
        let res = [((fitted[0] * k).round() as u32).clamp(1, 8192), ((fitted[1] * k).round() as u32).clamp(1, 8192)];
        u.e[0] = crop_rect;
        let tex = self.cx.temp(res[0], res[1]);
        {
            let sh = core_shaders();
            let mut p = Pass::full(&sh.comp, &tex, u);
            p.fs = "fs_prep";
            match (&nested_tex, upload_idx) {
                (Some(nt), _) => p.a = Bind::Tex(nt),
                (None, Some(i)) => {
                    let planes = &self.uploads[i].planes;
                    p.a = Bind::View(&planes[0].1, planes[0].2);
                    if planes.len() > 1 {
                        p.b = Bind::View(&planes[1].1, planes[1].2);
                    }
                }
                _ => {}
            }
            self.cx.draw(p);
        }
        let _ = format;
        let mut state = self.initial_state(layer);
        let tex = self.apply_effects(tex, &layer.effects, &mut state, layer.local_time);
        Some(Prepared { tex, fitted, state })
    }

    /// Runs `effects` over `input`, updating `state` for animations.
    fn apply_effects(&mut self, input: Tex, effects: &[ResolvedEffect], state: &mut AnimState, local_time: i64) -> Tex {
        let mut cur = input;
        let time = jb_core::time::ticks_to_seconds(local_time) as f32;
        for e in effects {
            let Some(fx) = self.registry.get(&e.kind).cloned() else {
                self.cx.report.unknown_kinds.insert(e.kind.clone());
                continue;
            };
            let info = fx.info();
            if info.kind == FxKind::Transition {
                self.cx.report.errors.push(format!("transition {} used as an effect", e.kind));
                continue;
            }
            let params = ParamValues::resolve(info, &e.params);
            fx.animate(state, e.progress as f32, &params);
            if fx.has_pixels() {
                let out = self.cx.temp_like(&cur);
                let io = FxIo { a: &cur, b: None, out: &out, progress: e.progress as f32, time, params: &params };
                fx.encode(&mut self.cx, &io);
                cur = out;
            }
        }
        if state.blur > 0.0 {
            let s = state.blur as f32 * cur.height as f32;
            cur = self.cx.blur(&cur, s, s);
        }
        cur
    }

    /// Canvas-sized title layer.
    fn title(&mut self, td: &jb_core::TitleData) -> Tex {
        let canvas = self.canvas;
        let engine = self.text.get_or_insert_with(TextEngine::new);
        let r = engine.raster(td, canvas);
        let key = format!("title:{:p}", Arc::as_ptr(&r));
        let view = self.cx.aux_texture(&key, || AuxImage {
            width: r.width,
            height: r.height,
            depth: 1,
            format: wgpu::TextureFormat::R16Float,
            data: bytemuck::cast_slice(&r.sdf).to_vec(),
        });
        let out = self.cx.temp(canvas[0], canvas[1]);
        let mut u = Uniforms::default();
        let ch = canvas[1] as f32;
        u.e[0] = [r.origin[0], r.origin[1], 0.0, 0.0];
        let stroke = td.stroke.as_ref();
        let shadow = td.shadow.as_ref();
        u.e[1] = [
            stroke.map_or(0.0, |s| s.width as f32 * ch),
            shadow.map_or(0.0, |s| s.offset[0] as f32 * ch),
            shadow.map_or(0.0, |s| s.offset[1] as f32 * ch),
            shadow.map_or(0.0, |s| s.blur as f32 * ch),
        ];
        u.e[2] = r.text_box;
        u.e[3] = [
            stroke.is_some() as u32 as f32,
            shadow.is_some() as u32 as f32,
            td.background.is_some() as u32 as f32,
            (td.font_size as f32 * ch) * 0.3,
        ];
        u.p[0] = td.color.map(|x| x as f32);
        if let Some(s) = stroke {
            u.p[1] = s.color.map(|x| x as f32);
        }
        if let Some(s) = shadow {
            u.p[2] = s.color.map(|x| x as f32);
        }
        if let Some(b) = td.background {
            u.p[3] = b.map(|x| x as f32);
        }
        let sh = core_shaders();
        let mut p = Pass::full(&sh.title, &out, u);
        p.fs = "fs_title";
        p.a = Bind::View(&view, [r.width, r.height]);
        self.cx.draw(p);
        out
    }

    /// Uploads a decoded frame, reusing a slot that already holds it.
    fn upload(&mut self, media_id: &str, f: &VideoFrame) -> anyhow::Result<usize> {
        let ptr = Arc::as_ptr(&f.data) as usize;
        let frame = self.frame;
        if let Some(i) = self.uploads.iter().position(|s| {
            s.content.as_ref().is_some_and(|(m, pts, p)| m == media_id && *pts == f.pts && *p == ptr)
                && s.format == f.format
                && s.width == f.width
                && s.height == f.height
        }) {
            self.uploads[i].used_frame = frame;
            return Ok(i);
        }
        let (w, h) = (f.width, f.height);
        let (cw, chh) = (w.div_ceil(2), h.div_ceil(2));
        let plane_specs: Vec<(wgpu::TextureFormat, [u32; 2], usize, usize)> = match f.format {
            // (format, size, byte offset, bytes per row)
            PixelFormat::Rgba8 => vec![(wgpu::TextureFormat::Rgba8Unorm, [w, h], 0, f.stride)],
            PixelFormat::Nv12 => vec![
                (wgpu::TextureFormat::R8Unorm, [w, h], 0, f.stride),
                (wgpu::TextureFormat::Rg8Unorm, [cw, chh], f.stride * h as usize, f.stride),
            ],
            PixelFormat::P010 => vec![
                (wgpu::TextureFormat::Rg8Unorm, [w, h], 0, f.stride),
                (wgpu::TextureFormat::Rgba8Unorm, [cw, chh], f.stride * h as usize, f.stride),
            ],
        };
        for (fmt, size, off, bpr) in &plane_specs {
            let bpp = fmt.block_copy_size(None).unwrap() as usize;
            let need = off + bpr * (size[1] as usize - 1) + size[0] as usize * bpp;
            if f.data.len() < need || *bpr < size[0] as usize * bpp {
                anyhow::bail!("frame buffer too small ({} bytes, need {need})", f.data.len());
            }
        }
        // Prefer the slot this media used before, else any slot free this frame.
        let fits = |s: &UploadSlot| s.used_frame != frame && s.format == f.format && s.width == w && s.height == h;
        let idx = self
            .uploads
            .iter()
            .position(|s| fits(s) && s.content.as_ref().is_some_and(|c| c.0 == media_id))
            .or_else(|| self.uploads.iter().position(fits));
        let idx = match idx {
            Some(i) => i,
            None => {
                let planes = plane_specs
                    .iter()
                    .map(|(fmt, size, _, _)| {
                        let t = self.cx.device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("jb-upload"),
                            size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: *fmt,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });
                        let v = t.create_view(&Default::default());
                        (t, v, *size)
                    })
                    .collect();
                self.uploads.push(UploadSlot { format: f.format, width: w, height: h, planes, content: None, used_frame: frame });
                self.uploads.len() - 1
            }
        };
        let slot = &mut self.uploads[idx];
        for ((tex, _, size), (_, _, off, bpr)) in slot.planes.iter().zip(&plane_specs) {
            self.cx.queue.write_texture(
                tex.as_image_copy(),
                &f.data[*off..],
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(*bpr as u32), rows_per_image: Some(size[1]) },
                wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
            );
        }
        slot.content = Some((media_id.to_string(), f.pts, ptr));
        slot.used_frame = frame;
        Ok(idx)
    }
}

#[allow(dead_code)]
fn prelude() -> &'static str {
    PRELUDE
}

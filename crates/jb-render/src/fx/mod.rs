//! Pass recording helpers shared by the compositor and every effect, plus the
//! built-in effect catalog.

use crate::pool::{Tex, TexturePool, UniformArena, WORK_FORMAT};
use crate::registry::{Effect, FxInfo, FxIo, FxKind, ParamSpec, ParamType, ParamValues, Registry};
use crate::RenderReport;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

pub mod animations;
pub mod color;
pub mod creative;
pub mod distort;
pub mod filters;
pub mod motion;
pub mod retro;
pub mod transitions;
pub mod wipes;

pub const PRELUDE: &str = include_str!("../shaders/prelude.wgsl");

/// Blend state of a pass's color target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blend {
    /// Overwrite the target.
    Replace,
    /// Premultiplied "over".
    Over,
}

/// Uniform block shared by every pass (layout of `U` in prelude.wgsl).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub size: [f32; 2],
    pub inv_size: [f32; 2],
    pub a_size: [f32; 2],
    pub b_size: [f32; 2],
    pub progress: f32,
    pub time: f32,
    pub seed: f32,
    pub flags: u32,
    pub e: [[f32; 4]; 4],
    pub p: [[f32; 4]; 32],
}

impl Default for Uniforms {
    fn default() -> Self {
        bytemuck::Zeroable::zeroed()
    }
}

impl Uniforms {
    pub fn with_params(params: &ParamValues) -> Self {
        let mut u = Self::default();
        for (i, s) in params.slots.iter().take(32).enumerate() {
            u.p[i] = *s;
        }
        u
    }
}

/// Source of a shader module: a cache key and a generator for its WGSL.
#[derive(Clone)]
pub struct ShaderSrc {
    pub name: Cow<'static, str>,
    pub build: fn(&str) -> String,
    /// Body handed to `build` (usually WGSL without the prelude).
    pub body: Cow<'static, str>,
}

impl ShaderSrc {
    /// A module made of the prelude followed by `body`.
    pub fn new(name: impl Into<Cow<'static, str>>, body: impl Into<Cow<'static, str>>) -> Self {
        fn build(body: &str) -> String {
            format!("{PRELUDE}\n{body}")
        }
        Self { name: name.into(), build, body: body.into() }
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct PipeKey {
    module: Cow<'static, str>,
    vs: &'static str,
    fs: &'static str,
    format: wgpu::TextureFormat,
    blend: Blend,
}

/// Lazily created shader modules and pipelines.
pub struct Pipelines {
    layout: wgpu::PipelineLayout,
    pub bind_layout: wgpu::BindGroupLayout,
    modules: HashMap<Cow<'static, str>, wgpu::ShaderModule>,
    pipes: HashMap<PipeKey, wgpu::RenderPipeline>,
    pub sampler: wgpu::Sampler,
    pub sampler_repeat: wgpu::Sampler,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        let tex = |binding, dim| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: dim,
                multisampled: false,
            },
            count: None,
        };
        let samp = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("jb-fx"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                    },
                    count: None,
                },
                samp(1),
                tex(2, wgpu::TextureViewDimension::D2),
                tex(3, wgpu::TextureViewDimension::D2),
                tex(4, wgpu::TextureViewDimension::D2),
                tex(5, wgpu::TextureViewDimension::D3),
                samp(6),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("jb-fx"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let mk_sampler = |mode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("jb-sampler"),
                address_mode_u: mode,
                address_mode_v: mode,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };
        Self {
            layout,
            bind_layout,
            modules: HashMap::new(),
            pipes: HashMap::new(),
            sampler: mk_sampler(wgpu::AddressMode::ClampToEdge),
            sampler_repeat: mk_sampler(wgpu::AddressMode::Repeat),
        }
    }

    fn pipeline(
        &mut self,
        device: &wgpu::Device,
        src: &ShaderSrc,
        vs: &'static str,
        fs: &'static str,
        format: wgpu::TextureFormat,
        blend: Blend,
    ) -> &wgpu::RenderPipeline {
        let key = PipeKey { module: src.name.clone(), vs, fs, format, blend };
        if !self.pipes.contains_key(&key) {
            let module = self.modules.entry(src.name.clone()).or_insert_with(|| {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&src.name),
                    source: wgpu::ShaderSource::Wgsl((src.build)(&src.body).into()),
                })
            });
            let blend_state = match blend {
                Blend::Replace => None,
                Blend::Over => Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            };
            let topology = if vs == "vs_quad" {
                wgpu::PrimitiveTopology::TriangleStrip
            } else {
                wgpu::PrimitiveTopology::TriangleList
            };
            let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&src.name),
                layout: Some(&self.layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState { topology, ..Default::default() },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: blend_state,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            self.pipes.insert(key.clone(), pipe);
        }
        &self.pipes[&key]
    }

    pub fn pipeline_count(&self) -> usize {
        self.pipes.len()
    }
}

/// Placeholder textures bound to unused slots.
pub struct Dummies {
    pub d2: wgpu::TextureView,
    pub d3: wgpu::TextureView,
}

impl Dummies {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mk = |dim, depth| {
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("jb-dummy"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: depth },
                mip_level_count: 1,
                sample_count: 1,
                dimension: dim,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                t.as_image_copy(),
                &[0, 0, 0, 0],
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
                wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            );
            t.create_view(&Default::default())
        };
        Self { d2: mk(wgpu::TextureDimension::D2, 1), d3: mk(wgpu::TextureDimension::D3, 1) }
    }
}

/// Textures built on the CPU once and kept across frames (curves, LUTs, masks).
#[derive(Default)]
pub struct AuxCache {
    items: HashMap<String, (wgpu::Texture, wgpu::TextureView, u64)>,
    frame: u64,
}

/// CPU image for [`FxCx::aux_texture`]; `depth > 1` makes a 3D texture.
pub struct AuxImage {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub format: wgpu::TextureFormat,
    pub data: Vec<u8>,
}

impl AuxCache {
    pub fn end_frame(&mut self) {
        self.frame += 1;
        let f = self.frame;
        self.items.retain(|_, v| f - v.2 < 600);
    }
}

/// A texture binding for a pass.
#[derive(Clone, Copy)]
pub enum Bind<'a> {
    None,
    Tex(&'a Tex),
    View(&'a wgpu::TextureView, [u32; 2]),
}

impl<'a> Bind<'a> {
    fn view(&self) -> Option<&'a wgpu::TextureView> {
        match *self {
            Bind::None => None,
            Bind::Tex(t) => Some(&t.view),
            Bind::View(v, _) => Some(v),
        }
    }
    fn size(&self) -> [u32; 2] {
        match *self {
            Bind::None => [1, 1],
            Bind::Tex(t) => t.size(),
            Bind::View(_, s) => s,
        }
    }
}

/// Description of one draw.
pub struct Pass<'a> {
    pub shader: &'a ShaderSrc,
    pub vs: &'static str,
    pub fs: &'static str,
    pub a: Bind<'a>,
    pub b: Bind<'a>,
    pub aux: Bind<'a>,
    pub lut: Option<&'a wgpu::TextureView>,
    pub target: &'a wgpu::TextureView,
    pub target_size: [u32; 2],
    pub format: wgpu::TextureFormat,
    pub blend: Blend,
    /// Keep the target's contents instead of clearing it.
    pub load: bool,
    pub clear: [f64; 4],
    pub uniforms: Uniforms,
    pub vertices: u32,
}

impl<'a> Pass<'a> {
    /// A full-screen pass of `shader` writing `out`.
    pub fn full(shader: &'a ShaderSrc, out: &'a Tex, uniforms: Uniforms) -> Self {
        Self {
            shader,
            vs: "vs_full",
            fs: "fs_main",
            a: Bind::None,
            b: Bind::None,
            aux: Bind::None,
            lut: None,
            target: &out.view,
            target_size: out.size(),
            format: out.format,
            blend: Blend::Replace,
            load: false,
            clear: [0.0; 4],
            uniforms,
            vertices: 3,
        }
    }
}

/// Everything an effect needs to record GPU work.
pub struct FxCx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub pipelines: &'a mut Pipelines,
    pub pool: &'a mut TexturePool,
    pub uniforms: &'a mut UniformArena,
    pub aux: &'a mut AuxCache,
    pub dummies: &'a Dummies,
    pub report: &'a mut RenderReport,
    /// Bit 0: working space is linear light.
    pub flags: u32,
}

impl<'a> FxCx<'a> {
    /// A pooled working-format texture.
    pub fn temp(&mut self, width: u32, height: u32) -> Tex {
        self.pool.get(self.device, width, height, WORK_FORMAT)
    }

    pub fn temp_like(&mut self, t: &Tex) -> Tex {
        self.temp(t.width, t.height)
    }

    /// Records one draw.
    pub fn draw(&mut self, pass: Pass<'_>) {
        let mut u = pass.uniforms;
        let [w, h] = pass.target_size;
        u.size = [w as f32, h as f32];
        u.inv_size = [1.0 / w as f32, 1.0 / h as f32];
        let (asz, bsz) = (pass.a.size(), pass.b.size());
        u.a_size = [asz[0] as f32, asz[1] as f32];
        u.b_size = [bsz[0] as f32, bsz[1] as f32];
        u.flags |= self.flags;
        let (chunk, offset) = self.uniforms.push(self.device, bytemuck::bytes_of(&u));
        let d2 = &self.dummies.d2;
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipelines.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: self.uniforms.buffer(chunk),
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                    }),
                },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.pipelines.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(pass.a.view().unwrap_or(d2)) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(pass.b.view().unwrap_or(d2)) },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(pass.aux.view().unwrap_or(d2)),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(pass.lut.unwrap_or(&self.dummies.d3)),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.pipelines.sampler_repeat),
                },
            ],
        });
        let pipe = self.pipelines.pipeline(self.device, pass.shader, pass.vs, pass.fs, pass.format, pass.blend);
        let c = pass.clear;
        let mut rp = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: pass.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if pass.load {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(wgpu::Color { r: c[0], g: c[1], b: c[2], a: c[3] })
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rp.set_pipeline(pipe);
        rp.set_bind_group(0, &bind, &[offset]);
        rp.draw(0..pass.vertices, 0..1);
    }

    /// Full-screen pass of `shader` from `a` (and `b`) into `out`.
    pub fn run(&mut self, shader: &ShaderSrc, a: &Tex, b: Option<&Tex>, out: &Tex, uniforms: Uniforms) {
        let mut p = Pass::full(shader, out, uniforms);
        p.a = Bind::Tex(a);
        p.b = b.map_or(Bind::None, Bind::Tex);
        self.draw(p);
    }

    /// Copies `src` into `dst` (resampling when sizes differ).
    pub fn copy(&mut self, src: &Tex, dst: &Tex) {
        if src.size() == dst.size() && src.format == dst.format {
            self.encoder.copy_texture_to_texture(
                src.texture.as_image_copy(),
                dst.texture.as_image_copy(),
                wgpu::Extent3d { width: src.width, height: src.height, depth_or_array_layers: 1 },
            );
        } else {
            self.run(&shaders().blit, src, None, dst, Uniforms::default());
        }
    }

    /// Gaussian blur of `src` with standard deviations in pixels of `src`.
    /// Large radii are computed on a downsampled copy.
    pub fn blur(&mut self, src: &Tex, sigma_x: f32, sigma_y: f32) -> Tex {
        let sh = shaders();
        if sigma_x < 0.25 && sigma_y < 0.25 {
            return src.clone();
        }
        let mut cur = src.clone();
        let (mut sx, mut sy) = (sigma_x, sigma_y);
        // Halve resolution while the kernel is wide; each halving halves sigma.
        while sx.max(sy) > 3.0 && cur.width.min(cur.height) > 16 {
            let half = self.temp((cur.width / 2).max(1), (cur.height / 2).max(1));
            // The 2x2 box of the downsample adds ~0.5px of blur at the new scale.
            self.run(&sh.blit, &cur, None, &half, Uniforms::default());
            let kx = half.width as f32 / cur.width as f32;
            let ky = half.height as f32 / cur.height as f32;
            sx = ((sx * kx).powi(2) - 0.25).max(0.0).sqrt();
            sy = ((sy * ky).powi(2) - 0.25).max(0.0).sqrt();
            cur = half;
        }
        for (dir, s) in [([1.0f32, 0.0f32], sx), ([0.0, 1.0], sy)] {
            if s < 0.2 {
                continue;
            }
            let out = self.temp_like(&cur);
            let mut u = Uniforms::default();
            u.e[0] = [dir[0] / cur.width as f32, dir[1] / cur.height as f32, s, (s * 3.0).ceil().min(14.0)];
            self.run(&sh.gauss, &cur, None, &out, u);
            cur = out;
        }
        if cur.size() != src.size() {
            let out = self.temp_like(src);
            self.run(&sh.upsample, &cur, None, &out, Uniforms::default());
            cur = out;
        }
        cur
    }

    /// Texture built once from CPU data and cached under `key`.
    pub fn aux_texture(&mut self, key: &str, make: impl FnOnce() -> AuxImage) -> wgpu::TextureView {
        let frame = self.aux.frame;
        if let Some(v) = self.aux.items.get_mut(key) {
            v.2 = frame;
            return v.1.clone();
        }
        let img = make();
        let dim = if img.depth > 1 { wgpu::TextureDimension::D3 } else { wgpu::TextureDimension::D2 };
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("jb-aux"),
            size: wgpu::Extent3d { width: img.width, height: img.height, depth_or_array_layers: img.depth },
            mip_level_count: 1,
            sample_count: 1,
            dimension: dim,
            format: img.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bpp = img.format.block_copy_size(None).unwrap_or(4);
        self.queue.write_texture(
            tex.as_image_copy(),
            &img.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(img.width * bpp),
                rows_per_image: Some(img.height),
            },
            wgpu::Extent3d { width: img.width, height: img.height, depth_or_array_layers: img.depth },
        );
        let view = tex.create_view(&Default::default());
        self.aux.items.insert(key.to_string(), (tex, view.clone(), frame));
        view
    }
}

/// Internal shaders used by helpers.
pub struct Shaders {
    pub blit: ShaderSrc,
    pub gauss: ShaderSrc,
    pub upsample: ShaderSrc,
}

pub fn shaders() -> &'static Shaders {
    static S: std::sync::OnceLock<Shaders> = std::sync::OnceLock::new();
    S.get_or_init(|| Shaders {
        blit: ShaderSrc::new(
            "jb.internal.blit",
            r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    // Box-filtered resample: four bilinear taps cover 4x4 source texels when
    // downscaling by 2, and degrade to plain bilinear when upscaling.
    let k = max(u.a_size * u.inv_size, vec2f(1.0));
    let o = 0.25 * k / u.a_size;
    if (k.x <= 1.01 && k.y <= 1.01) { return sa(in.uv); }
    return 0.25 * (sa(in.uv + vec2f(-o.x, -o.y)) + sa(in.uv + vec2f(o.x, -o.y)) + sa(in.uv + vec2f(-o.x, o.y)) + sa(in.uv + vec2f(o.x, o.y)));
}
"#,
        ),
        gauss: ShaderSrc::new(
            "jb.internal.gauss",
            r#"
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let step = u.e0.xy;
    let sigma = u.e0.z;
    let r = i32(u.e0.w);
    var acc = vec4f(0.0);
    var wsum = 0.0;
    for (var i = -r; i <= r; i = i + 1) {
        let x = f32(i);
        let w = exp(-0.5 * x * x / (sigma * sigma));
        acc = acc + w * sa(in.uv + step * x);
        wsum = wsum + w;
    }
    return acc / wsum;
}
"#,
        ),
        upsample: ShaderSrc::new(
            "jb.internal.upsample",
            r#"
// Bicubic B-spline upsampling with 4 bilinear taps (smooth, no blocky steps).
@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {
    let tex = u.a_size;
    let p = in.uv * tex - 0.5;
    let i = floor(p);
    let f = p - i;
    let f2 = f * f;
    let f3 = f2 * f;
    let w0 = (1.0 / 6.0) * (-f3 + 3.0 * f2 - 3.0 * f + 1.0);
    let w1 = (1.0 / 6.0) * (3.0 * f3 - 6.0 * f2 + 4.0);
    let w2 = (1.0 / 6.0) * (-3.0 * f3 + 3.0 * f2 + 3.0 * f + 1.0);
    let w3 = (1.0 / 6.0) * f3;
    let g0 = w0 + w1;
    let g1 = w2 + w3;
    let h0 = (i - 0.5 + w1 / g0) / tex;
    let h1 = (i + 1.5 + w3 / g1) / tex;
    let a = sa(vec2f(h0.x, h0.y));
    let b = sa(vec2f(h1.x, h0.y));
    let c = sa(vec2f(h0.x, h1.y));
    let d = sa(vec2f(h1.x, h1.y));
    return g0.y * (g0.x * a + g1.x * b) + g1.y * (g0.x * c + g1.x * d);
}
"#,
        ),
    })
}

/// Generates WGSL accessors `p_<name>()` for an effect's parameters.
pub fn param_accessors(params: &[ParamSpec]) -> String {
    let mut s = String::new();
    for (i, p) in params.iter().enumerate().take(32) {
        let (ty, sw) = match p.ty {
            ParamType::Float | ParamType::Int | ParamType::Bool => ("f32", ".x"),
            ParamType::Vec2 => ("vec2f", ".xy"),
            ParamType::Color => ("vec4f", ""),
            ParamType::Text => continue,
        };
        s.push_str(&format!("fn p_{}() -> {ty} {{ return u.p[{i}]{sw}; }}\n", p.name));
    }
    s
}

/// The most common effect: one full-screen WGSL pass. The body defines
/// `fn fx(uv: vec2f) -> vec4f`; inputs are `sa` (A) and `sb` (B), progress is
/// `prog()`, time `u.time`, and parameters `p_<name>()`.
pub struct ShaderFx {
    pub info: FxInfo,
    pub shader: ShaderSrc,
}

impl ShaderFx {
    pub fn new(info: FxInfo, body: impl Into<Cow<'static, str>>) -> Self {
        let body = format!(
            "{}\n{}\n@fragment fn fs_main(in: VsOut) -> @location(0) vec4f {{ return fx(in.uv); }}\n",
            param_accessors(&info.params),
            body.into()
        );
        let shader = ShaderSrc::new(info.id, body);
        Self { info, shader }
    }
}

impl Effect for ShaderFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        let mut u = Uniforms::with_params(io.params);
        u.progress = io.progress;
        u.time = io.time;
        cx.run(&self.shader, io.a, io.b, io.out, u);
    }
}

/// An effect whose passes are recorded by a function (multi-pass effects).
pub struct CustomFx {
    pub info: FxInfo,
    pub encode: fn(&CustomFx, &mut FxCx<'_>, &FxIo<'_>),
    /// Shaders used by `encode`, in an order the function knows.
    pub shaders: Vec<ShaderSrc>,
}

impl CustomFx {
    /// Builds a custom effect whose passes use `bodies` (each a WGSL body with
    /// the effect's parameter accessors and `fs_main`).
    pub fn new(info: FxInfo, bodies: &[&str], encode: fn(&CustomFx, &mut FxCx<'_>, &FxIo<'_>)) -> Self {
        let acc = param_accessors(&info.params);
        let shaders = bodies
            .iter()
            .enumerate()
            .map(|(i, b)| ShaderSrc::new(format!("{}#{i}", info.id), format!("{acc}\n{b}")))
            .collect();
        Self { info, encode, shaders }
    }

    /// Uniforms with this invocation's progress, time and parameters.
    pub fn uniforms(io: &FxIo<'_>) -> Uniforms {
        let mut u = Uniforms::with_params(io.params);
        u.progress = io.progress;
        u.time = io.time;
        u
    }
}

impl Effect for CustomFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn encode(&self, cx: &mut FxCx<'_>, io: &FxIo<'_>) {
        (self.encode)(self, cx, io)
    }
}

/// Transform-only animation.
pub struct AnimFx {
    pub info: FxInfo,
    pub apply: fn(&mut crate::registry::AnimState, f32, &ParamValues),
}

impl Effect for AnimFx {
    fn info(&self) -> &FxInfo {
        &self.info
    }

    fn animate(&self, state: &mut crate::registry::AnimState, progress: f32, params: &ParamValues) {
        (self.apply)(state, progress, params)
    }
}

/// Shorthand to build an [`FxInfo`].
pub fn info(
    id: &'static str,
    name: &'static str,
    category: &'static str,
    kind: FxKind,
    description: &'static str,
    params: &[ParamSpec],
) -> FxInfo {
    FxInfo { id, name, category, kind, description, params: params.to_vec() }
}

/// Registers every built-in item and alias.
pub fn register_builtin(r: &mut Registry) {
    transitions::register(r);
    wipes::register(r);
    motion::register(r);
    creative::register(r);
    distort::register(r);
    retro::register(r);
    filters::register(r);
    color::register(r);
    animations::register(r);
    crate::aliases::register(r);
}

/// Common helper: registers a single-pass shader effect.
pub fn shader(r: &mut Registry, info: FxInfo, body: impl Into<Cow<'static, str>>) {
    r.register(ShaderFx::new(info, body));
}

pub type ArcFx = Arc<dyn Effect>;

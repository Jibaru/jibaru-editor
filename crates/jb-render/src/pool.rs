//! Reusable GPU textures and uniform storage, so steady-state rendering does
//! not allocate GPU memory per frame.
//!
//! A texture handed out by the pool is free again as soon as its last
//! [`Tex`] handle is dropped. Reusing it later in the same frame is safe:
//! commands execute in recording order on one queue.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub const WORK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct TexInner {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    last_used: AtomicU64,
}


/// Shared handle to a pooled texture.
pub type Tex = Arc<TexInner>;

impl TexInner {
    pub fn size(&self) -> [u32; 2] {
        [self.width, self.height]
    }
}

pub struct TexturePool {
    free_after: u64,
    frame: u64,
    items: Vec<Tex>,
}

impl TexturePool {
    pub fn new() -> Self {
        Self { free_after: 60, frame: 0, items: Vec::new() }
    }

    /// A render target / sampled texture of the given size and format.
    pub fn get(&mut self, device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Tex {
        let width = width.max(1);
        let height = height.max(1);
        if let Some(t) = self
            .items
            .iter()
            .find(|t| Arc::strong_count(t) == 1 && t.width == width && t.height == height && t.format == format)
        {
            t.last_used.store(self.frame, Ordering::Relaxed);
            return t.clone();
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("jb-pool"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let t = Arc::new(TexInner { texture, view, width, height, format, last_used: AtomicU64::new(self.frame) });
        self.items.push(t.clone());
        t
    }

    /// Ends a frame: textures that stayed unused for a while are released.
    pub fn end_frame(&mut self) {
        self.frame += 1;
        let (frame, keep) = (self.frame, self.free_after);
        self.items.retain(|t| Arc::strong_count(t) > 1 || frame - t.last_used.load(Ordering::Relaxed) < keep);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn bytes(&self) -> u64 {
        self.items
            .iter()
            .map(|t| t.width as u64 * t.height as u64 * t.format.block_copy_size(None).unwrap_or(4) as u64)
            .sum()
    }
}

impl Default for TexturePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Size of one uniform slot (globals + 32 parameter vec4s), rounded to the
/// 256-byte dynamic offset alignment.
pub const SLOT_SIZE: u64 = 768;
const CHUNK_SLOTS: u64 = 1024;

/// Per-frame uniform storage: a list of large buffers addressed by dynamic
/// offsets. Contents are staged on the CPU and uploaded once per frame.
pub struct UniformArena {
    chunks: Vec<(wgpu::Buffer, Vec<u8>)>,
    used: u64,
}

impl UniformArena {
    pub fn new() -> Self {
        Self { chunks: Vec::new(), used: 0 }
    }

    /// Stores `data` (at most SLOT_SIZE bytes), returning (chunk, offset).
    pub fn push(&mut self, device: &wgpu::Device, data: &[u8]) -> (usize, u32) {
        assert!(data.len() as u64 <= SLOT_SIZE);
        let chunk = (self.used / CHUNK_SLOTS) as usize;
        let slot = self.used % CHUNK_SLOTS;
        self.used += 1;
        if chunk == self.chunks.len() {
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("jb-uniforms"),
                size: SLOT_SIZE * CHUNK_SLOTS,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.chunks.push((buf, vec![0; (SLOT_SIZE * CHUNK_SLOTS) as usize]));
        }
        let off = (slot * SLOT_SIZE) as usize;
        let staging = &mut self.chunks[chunk].1;
        staging[off..off + data.len()].copy_from_slice(data);
        (chunk, off as u32)
    }

    pub fn buffer(&self, chunk: usize) -> &wgpu::Buffer {
        &self.chunks[chunk].0
    }

    /// Uploads everything pushed since the last flush.
    pub fn flush(&mut self, queue: &wgpu::Queue) {
        let mut remaining = self.used;
        for (buf, staging) in &self.chunks {
            if remaining == 0 {
                break;
            }
            let n = remaining.min(CHUNK_SLOTS);
            queue.write_buffer(buf, 0, &staging[..(n * SLOT_SIZE) as usize]);
            remaining -= n;
        }
        self.used = 0;
    }
}

impl Default for UniformArena {
    fn default() -> Self {
        Self::new()
    }
}

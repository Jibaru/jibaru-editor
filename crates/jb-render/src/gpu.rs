//! Device acquisition. The renderer is headless: it never needs a surface,
//! and an application can hand it an existing device to share textures with
//! its own swapchain.

use std::sync::Arc;

/// Errors raised while creating the GPU context.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("no compatible GPU adapter found")]
    NoAdapter,
    #[error("failed to create GPU device: {0}")]
    Device(String),
}

/// A device/queue pair plus a description of the adapter behind it.
#[derive(Clone)]
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: Arc<wgpu::AdapterInfo>,
}

impl Gpu {
    /// Creates a headless device on the best available backend
    /// (Metal on macOS, D3D12/Vulkan on Windows, Vulkan elsewhere). Falls back
    /// to a software adapter (WARP, llvmpipe) when no hardware one exists.
    pub fn headless() -> Result<Self, GpuError> {
        pollster::block_on(Self::headless_async())
    }

    pub async fn headless_async() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let mut adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .ok();
        if adapter.is_none() {
            adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::None,
                    force_fallback_adapter: true,
                    compatible_surface: None,
                    ..Default::default()
                })
                .await
                .ok();
        }
        let adapter = adapter.ok_or(GpuError::NoAdapter)?;
        let info = adapter.get_info();
        let mut limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        limits.max_texture_dimension_2d = adapter.limits().max_texture_dimension_2d.min(16384);
        limits.max_texture_dimension_3d = adapter.limits().max_texture_dimension_3d.min(2048);
        limits.max_uniform_buffer_binding_size = 65536.min(adapter.limits().max_uniform_buffer_binding_size);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("jb-render"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Device(e.to_string()))?;
        Ok(Self { device, queue, adapter_info: Arc::new(info) })
    }

    /// Wraps a device created by the host application (e.g. the editor UI).
    pub fn from_device(device: wgpu::Device, queue: wgpu::Queue, adapter_info: wgpu::AdapterInfo) -> Self {
        Self { device, queue, adapter_info: Arc::new(adapter_info) }
    }

    /// True when running on a CPU/software implementation.
    pub fn is_software(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::Cpu
    }
}

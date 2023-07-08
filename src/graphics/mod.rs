use anyhow::Context;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureUsages,
};
use winit::window::Window;

use crate::{display::Display, utils::args::args};

use self::quad_renderer::QuadRenderer;

pub mod quad_renderer;

pub struct SurfaceContext {
    pub surface: Surface,
    pub config: SurfaceConfiguration,
}

impl SurfaceContext {
    pub fn new(
        window: &Window,
        instance: &Instance,
        adapter: &Adapter,
        device: &Device,
    ) -> anyhow::Result<Self> {
        let surface = unsafe { instance.create_surface(&window) }
            .context("Unable to recreate window surface")?;
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format =
            if let Some(format) = surface_caps.formats.iter().find(|f| f.is_srgb()).copied() {
                format
            } else if let Some(format) = surface_caps.formats.get(0).copied() {
                format
            } else {
                return Err(anyhow::anyhow!("No surface format found"));
            };
        let display_size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: display_size.width,
            height: display_size.height,
            alpha_mode: surface_caps
                .alpha_modes
                .first()
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No alpha modes found"))?,
            present_mode: PresentMode::AutoVsync,
            view_formats: vec![],
        };
        surface.configure(device, &config);
        Ok(Self { surface, config })
    }

    pub fn configure(&self, device: &Device) {
        self.surface.configure(device, &self.config);
    }
}

pub async fn init_wgpu(
    display: &Display,
) -> anyhow::Result<(Instance, Option<SurfaceContext>, Adapter, Device, Queue)> {
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });

    let surface = args()
        .headless
        .then(|| unsafe { instance.create_surface(display.get_winit_window()) })
        .transpose()
        .context("Unable to create window surface")?;
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            compatible_surface: surface.as_ref(),
            force_fallback_adapter: false,
        })
        .await
        .context("Unable to find suitable adapter (GPU)")?;
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                features: Features::PUSH_CONSTANTS,
                limits: Limits {
                    max_push_constant_size: u32::try_from(QuadRenderer::MAX_PUSH_CONSTANT_SIZE)
                        .expect("max push constant size too large to fit on an u32"),
                    ..Limits::downlevel_defaults()
                },
            },
            None,
        )
        .await
        .context("Unable to request wgpu device")?;
    Ok((instance, None, adapter, device, queue))
}

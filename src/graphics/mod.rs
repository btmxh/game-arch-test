use std::num::NonZeroU32;

use anyhow::Context;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureUsages,
};
use winit::dpi::PhysicalSize;

use crate::display::Display;

use self::quad_renderer::QuadRenderer;

pub mod quad_renderer;

pub async fn init_wgpu(
    display: &Display,
    display_size: PhysicalSize<NonZeroU32>,
) -> anyhow::Result<(
    Instance,
    Surface,
    Adapter,
    Device,
    Queue,
    SurfaceConfiguration,
)> {
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });

    let surface = unsafe { instance.create_surface(display.get_winit_window()) }
        .context("Unable to create window surface")?;
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            compatible_surface: Some(&surface),
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
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format =
        if let Some(format) = surface_caps.formats.iter().find(|f| f.is_srgb()).copied() {
            format
        } else if let Some(format) = surface_caps.formats.get(0).copied() {
            format
        } else {
            return Err(anyhow::anyhow!("No surface format found"));
        };
    let surface_configuration = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: display_size.width.get(),
        height: display_size.height.get(),
        alpha_mode: surface_caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No alpha modes found"))?,
        present_mode: PresentMode::AutoVsync,
        view_formats: vec![],
    };
    surface.configure(&device, &surface_configuration);
    Ok((
        instance,
        surface,
        adapter,
        device,
        queue,
        surface_configuration,
    ))
}

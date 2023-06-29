use crate::{
    events::GameUserEvent,
    exec::server::{
        draw::{RecvMsg, SendMsg, ServerChannel},
        BaseGameServer,
    },
    scene::{
        main::{core::clear::Clear, RootScene},
        Scene,
    },
    ui::utils::geom::UISize,
    utils::args::args,
};
use std::{borrow::Cow, collections::HashMap, num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::Context;
use wgpu::{
    Adapter, Backends, CommandEncoder, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, Operations, PowerPreference, PresentMode, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
    SurfaceConfiguration, SurfaceTexture, TextureUsages, TextureView,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::display::SendRawHandle;

use super::transform_stack::TransformStack;

pub struct DrawContext {
    pub test_logs: HashMap<Cow<'static, str>, String>,
    pub transform_stack: TransformStack,
    pub instance: Instance,
    pub adapter: Adapter,
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub surface_configuration: SurfaceConfiguration,
    pub ui_size: UISize,
    pub display_handles: SendRawHandle,
    pub base: BaseGameServer<SendMsg, RecvMsg>,
}

pub struct DrawingContext {
    pub surface_texture: SurfaceTexture,
    pub surface_texture_view: TextureView,
}
impl DrawingContext {
    pub fn begin_direct_render_pass<'a>(
        &'a self,
        encoder: &'a mut CommandEncoder,
        label: Option<&str>,
    ) -> RenderPass {
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.surface_texture_view,
                resolve_target: None,
                ops: Operations {
                    ..Default::default()
                },
            })],
            depth_stencil_attachment: None,
            label,
        })
    }
}

impl DrawContext {
    pub async fn new(
        proxy: EventLoopProxy<GameUserEvent>,
        display: &crate::display::Display,
    ) -> anyhow::Result<(Self, ServerChannel)> {
        let (base, sender, receiver) = BaseGameServer::new(proxy);
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
                    features: Features::empty(),
                    limits: Limits::downlevel_defaults(),
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
        let display_size = {
            let size = display.get_size();
            PhysicalSize {
                width: NonZeroU32::new(size.width).expect("display width is 0"),
                height: NonZeroU32::new(size.height).expect("display height is 0"),
            }
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
        let ui_size = display
            .get_size()
            .to_logical(display.get_scale_factor())
            .into();
        Ok((
            Self {
                base,
                display_handles: display.get_raw_handles(),
                ui_size,
                instance,
                surface,
                adapter,
                device,
                queue,
                surface_configuration,
                test_logs: HashMap::new(),
                transform_stack: TransformStack::default(),
            },
            ServerChannel { sender, receiver },
        ))
    }

    pub fn get_test_log(&mut self, name: &str) -> &mut String {
        if !self.test_logs.contains_key(name) {
            self.test_logs
                .insert(Cow::Owned(name.to_owned()), String::new());
        }

        self.test_logs.get_mut(name).unwrap()
    }

    pub fn pop_test_log(&mut self, name: &str) -> String {
        self.test_logs.remove(name).unwrap_or_default()
    }

    fn reconfigure(&mut self) {
        self.surface
            .configure(&self.device, &self.surface_configuration);
    }

    pub fn set_swap_interval(&mut self, swap_interval: PresentMode) -> anyhow::Result<()> {
        self.surface_configuration.present_mode = swap_interval;
        self.reconfigure();
        Ok(())
    }

    fn process_messages(
        &mut self,
        block: bool,
        root_scene: &mut Option<RootScene>,
    ) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .try_iter(block.then_some(Duration::from_millis(300)))
            .context("thread runner channel was unexpectedly closed")?
            .collect::<Vec<_>>();
        for message in messages {
            match message {
                RecvMsg::SetFrequencyProfiling(fp) => self.base.frequency_profiling = fp,
                RecvMsg::Execute(callback) => callback(self, root_scene),
            }
        }

        Ok(())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<NonZeroU32>, ui_size: UISize) {
        self.ui_size = ui_size;
        self.surface_configuration.width = new_size.width.get();
        self.surface_configuration.height = new_size.height.get();
        self.reconfigure();
    }

    pub fn draw(
        &mut self,
        root_scene: &mut Option<RootScene>,
        single: bool,
        runner_frequency: f64,
    ) -> anyhow::Result<()> {
        let headless = args().headless;
        self.base.run("Draw", runner_frequency);
        self.process_messages(single && headless, root_scene)?;
        if !headless {
            let output = self
                .surface
                .get_current_texture()
                .context("Unable to retrieve surface texture")?;
            let view = output.texture.create_view(&Default::default());
            let drawing_context = DrawingContext {
                surface_texture: output,
                surface_texture_view: view,
            };

            if let Some(root_scene) = root_scene {
                root_scene.draw(self, &drawing_context);
            } else {
                Arc::new(Clear).draw(self, &drawing_context);
            }
            drawing_context.surface_texture.present();
        }
        Ok(())
    }
}

use crate::{
    display::{Display, EventSender},
    exec::{
        dispatch::Dispatch,
        server::{draw::Message, BaseGameServer},
    },
    graphics::transform_stack::TransformStack,
    scene::main::RootScene,
    utils::{args::args, mpsc::Receiver},
};
use std::{borrow::Cow, collections::HashMap, num::NonZeroU32, time::Duration};

use anyhow::Context;
use trait_set::trait_set;
use wgpu::{
    Adapter, Backends, CommandEncoder, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PowerPreference, PresentMode, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
    SurfaceConfiguration, SurfaceTexture, TextureUsages, TextureView,
};
use winit::dpi::PhysicalSize;

use crate::display::SendRawHandle;

pub struct GraphicsContext {
    pub base: BaseGameServer<Message>,
    pub test_logs: HashMap<Cow<'static, str>, String>,
    pub transform_stack: TransformStack,
    pub instance: Instance,
    pub adapter: Adapter,
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub surface_configuration: SurfaceConfiguration,
    pub display_handles: SendRawHandle,
}

pub struct FrameContext {
    pub surface_texture: SurfaceTexture,
    pub surface_texture_view: TextureView,
}

impl FrameContext {
    pub fn begin_direct_render_pass<'a>(
        &'a self,
        encoder: &'a mut CommandEncoder,
        label: Option<&str>,
    ) -> RenderPass {
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.surface_texture_view,
                ops: Default::default(),
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            label,
        })
    }
}

impl GraphicsContext {
    pub async fn new(
        event_sender: EventSender,
        display: &Display,
        receiver: Receiver<Message>,
    ) -> anyhow::Result<Self> {
        let base = BaseGameServer::new(event_sender, receiver);
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
        Ok(Self {
            base,
            display_handles: display.get_raw_handles(),
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_configuration,
            test_logs: HashMap::new(),
            transform_stack: TransformStack::default(),
        })
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

    fn process_messages(&mut self, block: bool, root_scene: &RootScene) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .try_iter(block.then_some(Duration::from_millis(300)))
            .context("thread runner channel was unexpectedly closed")?
            .collect::<Vec<_>>();
        for message in messages {
            match message {
                Message::SetFrequencyProfiling(fp) => self.base.frequency_profiling = fp,
                Message::Execute(callback) => callback(DrawDispatchContext::new(self, root_scene)),
            }
        }

        Ok(())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<NonZeroU32>) {
        self.surface_configuration.width = new_size.width.get();
        self.surface_configuration.height = new_size.height.get();
        self.reconfigure();
    }

    pub fn draw(
        &mut self,
        root_scene: &RootScene,
        single: bool,
        runner_frequency: f64,
    ) -> anyhow::Result<()> {
        let headless = args().headless;
        self.base.run("Draw", runner_frequency);
        self.process_messages(single && headless, root_scene)?;
        if !headless {
            let mut frame = self
                .get_frame_context()
                .context("Unable to retrieve frame context to render")?;
            {
                let mut draw_context = DrawContext::new(self, &mut frame);
                root_scene.draw(&mut draw_context);
            }
            frame.surface_texture.present();
        }
        Ok(())
    }

    fn get_frame_context(&self) -> anyhow::Result<FrameContext> {
        let surface_texture = self
            .surface
            .get_current_texture()
            .context("Unable to retrieve current surface texture")?;
        Ok(FrameContext {
            surface_texture_view: surface_texture.texture.create_view(&Default::default()),
            surface_texture,
        })
    }
}

pub struct DrawContext<'a> {
    pub graphics: &'a mut GraphicsContext,
    pub frame: &'a mut FrameContext,
}

impl<'a> DrawContext<'a> {
    pub fn new(graphics: &'a mut GraphicsContext, frame: &'a mut FrameContext) -> Self {
        Self { graphics, frame }
    }
}

pub struct DrawDispatchContext<'a> {
    pub graphics: &'a mut GraphicsContext,
    pub root_scene: &'a RootScene,
}

impl<'a> DrawDispatchContext<'a> {
    pub fn new(graphics: &'a mut GraphicsContext, root_scene: &'a RootScene) -> Self {
        Self {
            graphics,
            root_scene,
        }
    }
}

trait_set! {
    pub trait DrawDispatch = for <'a> Dispatch<DrawDispatchContext<'a>>;
}

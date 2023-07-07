use crate::{display::Display, exec::dispatch::Dispatch, graphics, scene::main::RootScene};
use std::num::NonZeroU32;

use anyhow::Context;
use trait_set::trait_set;
use wgpu::{
    Adapter, CommandEncoder, Device, Instance, PresentMode, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDescriptor, Surface, SurfaceConfiguration, SurfaceTexture,
    TextureView,
};
use winit::dpi::PhysicalSize;

use super::common::SharedCommonContext;

pub struct GraphicsContext {
    pub common: SharedCommonContext,
    pub instance: Instance,
    pub adapter: Adapter,
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub surface_configuration: SurfaceConfiguration,
}

impl GraphicsContext {
    pub async fn new(common: SharedCommonContext, display: &Display) -> anyhow::Result<Self> {
        let display_size = {
            let size = display.get_size();
            PhysicalSize {
                width: NonZeroU32::new(size.width).expect("display width is 0"),
                height: NonZeroU32::new(size.height).expect("display height is 0"),
            }
        };
        let (instance, surface, adapter, device, queue, surface_configuration) =
            graphics::init_wgpu(display, display_size)
                .await
                .context("Unable to initialize wgpu objects")?;
        Ok(Self {
            common,
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_configuration,
        })
    }

    fn reconfigure<F>(&mut self, func: F)
    where
        F: FnOnce(&mut SurfaceConfiguration),
    {
        func(&mut self.surface_configuration);
        self.surface
            .configure(&self.device, &self.surface_configuration);
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

    pub fn set_swap_interval(&mut self, swap_interval: PresentMode) {
        self.reconfigure(|config| config.present_mode = swap_interval);
    }

    pub fn run_callback<F>(&mut self, callback: F, root_scene: &RootScene)
    where
        F: DrawDispatch,
    {
        callback(DrawDispatchContext::new(self, root_scene));
    }

    pub fn resize(&mut self, new_size: PhysicalSize<NonZeroU32>) {
        self.reconfigure(|config| {
            config.width = new_size.width.get();
            config.height = new_size.height.get();
        });
    }

    pub fn draw(&mut self, root_scene: &RootScene) -> anyhow::Result<()> {
        let mut frame = self
            .get_frame_context()
            .context("Unable to retrieve frame context to render")?;
        {
            let mut draw_context = DrawContext::new(self, &mut frame);
            root_scene.draw(&mut draw_context);
        }
        frame.surface_texture.present();
        Ok(())
    }
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

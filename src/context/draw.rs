use crate::{
    exec::dispatch::Dispatch,
    graphics::{self, SurfaceContext},
    scene::main::RootScene,
    utils::args::args,
};
use std::num::NonZeroU32;

use anyhow::Context;
use trait_set::trait_set;
use wgpu::{
    Adapter, CommandEncoder, Device, Instance, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, SurfaceTexture, TextureView,
};
use winit::dpi::PhysicalSize;

use super::common::SharedCommonContext;

pub struct GraphicsContext {
    pub common: SharedCommonContext,
    pub instance: Instance,
    pub adapter: Adapter,
    pub surface_context: Option<SurfaceContext>,
    pub device: Device,
    pub queue: Queue,
}

impl GraphicsContext {
    pub async fn new(common: SharedCommonContext) -> anyhow::Result<Self> {
        let (instance, adapter, device, queue) = graphics::init_wgpu()
            .await
            .context("Unable to initialize wgpu objects")?;
        Ok(Self {
            common,
            instance,
            adapter,
            surface_context: None,
            device,
            queue,
        })
    }

    fn get_frame_context(&self) -> anyhow::Result<Option<FrameContext>> {
        self.surface_context
            .as_ref()
            .map(|SurfaceContext { surface, .. }| {
                let surface_texture = surface
                    .get_current_texture()
                    .context("Unable to retrieve current surface texture")?;
                Ok(FrameContext {
                    surface_texture_view: surface_texture.texture.create_view(&Default::default()),
                    surface_texture,
                })
            })
            .transpose()
    }

    pub fn run_callback<F>(&mut self, callback: F, root_scene: &RootScene)
    where
        F: DrawDispatch,
    {
        callback(DrawDispatchContext::new(self, root_scene));
    }

    pub fn resize(&mut self, new_size: PhysicalSize<NonZeroU32>) {
        if let Some(context) = self.surface_context.as_mut() {
            context.config.width = new_size.width.get();
            context.config.height = new_size.height.get();
            context.configure(&self.device);
        } else if !args().headless {
            tracing::warn!("Attempting to resize surface while surface is not present");
        }
    }

    pub fn draw(&mut self, root_scene: &RootScene) -> anyhow::Result<()> {
        if let Some(mut frame) = self
            .get_frame_context()
            .context("Unable to retrieve frame context to render")?
        {
            {
                let mut draw_context: DrawContext<'_> = DrawContext::new(self, &mut frame);
                root_scene.draw(&mut draw_context);
            }
            frame.surface_texture.present();
        }

        Ok(())
    }

    pub fn create_surface(&mut self) -> anyhow::Result<()> {
        self.surface_context = Some(
            SurfaceContext::new(
                self.common.display.get_winit_window(),
                &self.instance,
                &self.adapter,
                &self.device,
            )
            .context("Unable to create surface")?,
        );
        Ok(())
    }

    pub fn destroy_surface(&mut self) {
        self.surface_context.take();
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

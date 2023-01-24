use crate::{
    events::GameUserEvent,
    exec::server::{
        draw::{RecvMsg, SendMsg, ServerChannel},
        BaseGameServer,
    },
    graphics::{debug_callback::enable_gl_debug_callback, HandleContainer, SendHandleContainer},
    scene::draw::DrawRoot,
};
use std::{ffi::CString, num::NonZeroU32};

use anyhow::Context;
use glutin::{
    config::Config,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::display::SendRawHandle;

pub struct DrawContext {
    pub handles: HandleContainer,
    pub swap_interval: SwapInterval,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub gl_display: Display,
    pub gl_config: Config,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub display_handles: SendRawHandle,
    pub base: BaseGameServer<SendMsg, RecvMsg>,
}

pub struct SendDrawContext {
    pub handles: SendHandleContainer,
    pub swap_interval: SwapInterval,
    pub gl_context: NotCurrentContext,
    pub gl_display: Display,
    pub gl_config: Config,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub display_handles: SendRawHandle,
    pub base: BaseGameServer<SendMsg, RecvMsg>,
}

impl SendDrawContext {
    pub fn new(
        proxy: EventLoopProxy<GameUserEvent>,
        gl_config: Config,
        display: &crate::display::Display,
    ) -> anyhow::Result<(Self, ServerChannel)> {
        let (base, sender, receiver) = BaseGameServer::new(proxy);
        let gl_display = gl_config.display();
        let context_attribs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .with_debug(cfg!(debug_assertions))
            .build(Some(display.get_raw_window_handle()));
        let gl_context = unsafe { gl_display.create_context(&gl_config, &context_attribs) }
            .context("unable to create OpenGL context")?;
        let display_size = display.get_size();
        let gl_surface = unsafe {
            gl_display
                .create_window_surface(
                    &gl_config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        display.get_raw_window_handle(),
                        NonZeroU32::new(display_size.width).unwrap(),
                        NonZeroU32::new(display_size.height).unwrap(),
                    ),
                )
                .context("unable to create window surface for OpenGL rendering")?
        };
        let current_gl_context = gl_context
            .make_current(&gl_surface)
            .context("unable to make OpenGL context current")?;
        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        });
        enable_gl_debug_callback();
        let gl_context = current_gl_context
            .make_not_current()
            .context("unable to make GL context not current")?;
        let display_size = {
            let size = display.get_size();
            PhysicalSize {
                width: NonZeroU32::new(size.width).expect("display width is 0"),
                height: NonZeroU32::new(size.height).expect("display height is 0"),
            }
        };
        Ok((
            Self {
                base,
                display_handles: display.get_raw_handles(),
                display_size,
                gl_display,
                gl_context,
                gl_config,
                swap_interval: SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
                handles: SendHandleContainer::new(),
            },
            ServerChannel {
                sender,
                receiver,
                current_id: 0,
            },
        ))
    }
}

impl DrawContext {
    pub fn set_swap_interval(&mut self, swap_interval: SwapInterval) -> anyhow::Result<()> {
        self.gl_surface
            .set_swap_interval(&self.gl_context, swap_interval)?;
        self.swap_interval = swap_interval;
        Ok(())
    }

    fn process_messages(&mut self, root_scene: &mut DrawRoot) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .try_iter(None)
            .context("thread runner channel was unexpectedly closed")?
            .collect::<Vec<_>>();
        for message in messages {
            match message {
                RecvMsg::SetFrequencyProfiling(fp) => self.base.frequency_profiling = fp,
                RecvMsg::ExecuteSync(callback) => {
                    let result = callback(self, root_scene);
                    self.base.send(SendMsg::ExecuteSyncReturn(result)).context(
                        "unable to send ExecuteSyncReturn message for Sync return mechanism",
                    )?;
                }
                RecvMsg::ExecuteEvent(callback) => {
                    callback(self, root_scene)
                        .into_iter()
                        .try_for_each(|evt| self.base.proxy.send_event(evt))
                        .map_err(|e| anyhow::format_err!("{}", e))
                        .context("unable to send event to event loop")?;
                }
            }
        }

        Ok(())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<NonZeroU32>) {
        self.gl_surface
            .resize(&self.gl_context, new_size.width, new_size.height);
        unsafe {
            gl::Viewport(
                0,
                0,
                new_size.width.get().try_into().unwrap(),
                new_size.height.get().try_into().unwrap(),
            );
        }
        self.display_size = new_size;
    }

    pub fn to_send(self) -> anyhow::Result<SendDrawContext> {
        let gl_context = self
            .gl_context
            .make_not_current()
            .context("unable to make OpenGL context not current")?;
        Ok(SendDrawContext {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            display_handles: self.display_handles,
            display_size: self.display_size,
            swap_interval: self.swap_interval,
            handles: self.handles.to_send(),
        })
    }

    pub fn draw(&mut self, root_scene: &mut DrawRoot, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Draw", runner_frequency);
        self.process_messages(root_scene)?;
        root_scene.draw(self)?;
        self.gl_surface.swap_buffers(&self.gl_context)?;
        Ok(())
    }
}

impl SendDrawContext {
    pub fn to_nonsend(self) -> anyhow::Result<DrawContext> {
        let gl_surface = unsafe {
            self.gl_display
                .create_window_surface(
                    &self.gl_config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        self.display_handles.0,
                        self.display_size.width,
                        self.display_size.height,
                    ),
                )
                .context("unable to create window surface for OpenGL rendering")?
        };
        let gl_context = self
            .gl_context
            .make_current(&gl_surface)
            .context("unable to make OpenGL context current")?;
        gl_surface.set_swap_interval(&gl_context, self.swap_interval)?;
        Ok(DrawContext {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            gl_surface,
            display_handles: self.display_handles,
            display_size: self.display_size,
            swap_interval: self.swap_interval,
            handles: self.handles.to_nonsend(),
        })
    }
}

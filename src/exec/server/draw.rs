use std::{ffi::CString, num::NonZeroU32};

use anyhow::Context;
use glutin::{
    config::Config,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};
use winit::dpi::PhysicalSize;

use super::{BaseGameServer, GameServer, SendGameServer, ServerChannel};
use crate::display::SendRawHandle;

pub enum SendMsg {}
pub enum RecvMsg {}
pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub raw_handles: SendRawHandle,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
}

pub struct SendServer {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub raw_handles: SendRawHandle,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_context: NotCurrentContext,
}

impl SendServer {
    pub fn new(
        gl_config: Config,
        display: &crate::display::Display,
    ) -> anyhow::Result<(Self, ServerChannel<SendMsg, RecvMsg>)> {
        let (base, channels) = BaseGameServer::new();
        let gl_display = gl_config.display();
        let context_attribs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .with_debug(cfg!(debug_assertions))
            .build(Some(display.get_raw_window_handle()));
        let gl_context = unsafe { gl_display.create_context(&gl_config, &context_attribs) }
            .context("unable to create OpenGL context")?;
        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        });
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
                raw_handles: display.get_raw_handles(),
                display_size,
                gl_display,
                gl_context,
                gl_config,
            },
            channels,
        ))
    }
}

impl GameServer for Server {
    fn to_send(self) -> anyhow::Result<Box<dyn SendGameServer>> {
        let gl_context = self
            .gl_context
            .make_not_current()
            .context("unable to make OpenGL context not current")?;
        Ok(Box::new(SendServer {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            raw_handles: self.raw_handles,
            display_size: self.display_size,
        }))
    }

    fn run(&mut self) -> anyhow::Result<()> {
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.2, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.gl_surface.swap_buffers(&self.gl_context)?;
        Ok(())
    }
}

impl SendGameServer for SendServer {
    fn server_kind(&self) -> super::ServerKind {
        super::ServerKind::Draw
    }

    fn downcast_draw(self: Box<Self>) -> anyhow::Result<Server> {
        let gl_surface = unsafe {
            self.gl_display
                .create_window_surface(
                    &self.gl_config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        self.raw_handles.0,
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
        Ok(Server {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            gl_surface,
            raw_handles: self.raw_handles,
            display_size: self.display_size,
        })
    }
}

#[test]
fn test_send_sync() {
    fn test_send<T: Send>() {}
    test_send::<SendServer>();
}

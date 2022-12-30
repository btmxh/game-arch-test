use std::{ffi::CString, num::NonZeroU32, time::SystemTime};

use anyhow::{bail, Context};
use async_trait::async_trait;
use glutin::{
    config::Config,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use winit::dpi::PhysicalSize;

use super::{BaseGameServer, GameServer, SendGameServer, ServerChannel};
use crate::{display::SendRawHandle, utils::mpsc::UnboundedReceiverExt};

pub enum SendMsg {
    Dummy,
    VSyncSet(Option<SwapInterval>),
}
pub enum RecvMsg {
    Dummy,
    SetVSync(SwapInterval),
    Resize(PhysicalSize<NonZeroU32>),
}
pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub raw_handles: SendRawHandle,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub swap_interval: SwapInterval,
}

pub struct SendServer {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub raw_handles: SendRawHandle,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_context: NotCurrentContext,
    pub swap_interval: SwapInterval,
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
                swap_interval: SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            },
            channels,
        ))
    }
}

impl Server {
    fn set_swap_interval(&mut self, swap_interval: SwapInterval) -> anyhow::Result<()> {
        self.gl_surface
            .set_swap_interval(&self.gl_context, swap_interval)?;
        self.swap_interval = swap_interval;
        Ok(())
    }

    fn process_messages(&mut self) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .receive_all_pending(false)
            .ok_or_else(|| anyhow::format_err!("thread runner channel was unexpectedly closed"))?;
        let mut vsync = None;
        let mut resize = None;
        for message in messages {
            match message {
                RecvMsg::SetVSync(swap_interval) => vsync = Some(swap_interval),
                RecvMsg::Resize(new_size) => resize = Some(new_size),
                _ => {}
            }
        }

        if let Some(swap_interval) = vsync {
            println!("{:?}", swap_interval);
            let result = self
                .set_swap_interval(swap_interval)
                .ok()
                .map(|_| swap_interval);
            self.base.send(SendMsg::VSyncSet(result))?;
        }

        if let Some(new_size) = resize {
            self.gl_surface
                .resize(&self.gl_context, new_size.width, new_size.height);
        }

        Ok(())
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
            swap_interval: self.swap_interval,
        }))
    }

    fn run(&mut self) -> anyhow::Result<()> {
        self.process_messages()?;
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.2, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.gl_surface.swap_buffers(&self.gl_context)?;
        println!(
            "{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs_f64()
        );
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
        gl_surface.set_swap_interval(&gl_context, self.swap_interval)?;
        Ok(Server {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            gl_surface,
            raw_handles: self.raw_handles,
            display_size: self.display_size,
            swap_interval: self.swap_interval,
        })
    }
}

#[async_trait]
pub trait DrawServerChannel {
    async fn set_vsync(&mut self, interval: SwapInterval) -> anyhow::Result<SwapInterval>;
    fn resize(&mut self, size: PhysicalSize<NonZeroU32>) -> anyhow::Result<()>;
}

#[async_trait]
impl DrawServerChannel for ServerChannel<SendMsg, RecvMsg> {
    async fn set_vsync(&mut self, interval: SwapInterval) -> anyhow::Result<SwapInterval> {
        self.send(RecvMsg::SetVSync(interval))?;
        if let SendMsg::VSyncSet(result) = self.recv()? {
            match result {
                Some(other_interval) if other_interval != interval => {
                    log::warn!("VSync swap interval '{:?}' not supported, falling back to swap interval '{:?}'", interval, result)
                }
                None => log::error!("Toggling VSync failed"),
                _ => {}
            }
            result.ok_or_else(|| anyhow::format_err!("failed to set swap interval"))
        } else {
            bail!("unexpected response message from thread")
        }
    }

    fn resize(&mut self, size: PhysicalSize<NonZeroU32>) -> anyhow::Result<()> {
        self.send(RecvMsg::Resize(size))
    }
}

#[test]
fn test_send_sync() {
    fn test_send<T: Send>() {}
    test_send::<SendServer>();
}

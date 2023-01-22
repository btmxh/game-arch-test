use crate::{
    events::GameUserEvent,
    graphics::{debug_callback::enable_gl_debug_callback, HandleContainer, SendHandleContainer},
    utils::{
        error::ResultExt,
        mpsc::{Receiver, Sender},
    },
};
use std::{any::Any, ffi::CString, num::NonZeroU32};

use anyhow::Context;
use glutin::{
    config::Config,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use super::{BaseGameServer, GameServer, GameServerChannel, GameServerSendChannel, SendGameServer};
use crate::display::SendRawHandle;

pub type DrawCallback = dyn FnMut(&Server) -> anyhow::Result<()> + Send;

pub enum SendMsg {
    ExecuteSyncReturn(Box<dyn Any + Send>),
}

type ExecuteSyncReturnType = Box<dyn Any + Send + 'static>;
type ExecuteEventReturnType = Box<dyn Iterator<Item = GameUserEvent>>;
type ExecuteCallback<R> = dyn FnOnce(&mut Server) -> R + Send;

pub enum RecvMsg {
    SetFrequencyProfiling(bool),
    ExecuteSync(Box<ExecuteCallback<ExecuteSyncReturnType>>),
    ExecuteEvent(Box<ExecuteCallback<ExecuteEventReturnType>>),
}
pub struct Server {
    pub draw_callback: Option<Box<DrawCallback>>,
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

pub struct SendServer {
    pub draw_callback: Option<Box<DrawCallback>>,
    pub handles: SendHandleContainer,
    pub swap_interval: SwapInterval,
    pub gl_context: NotCurrentContext,
    pub gl_display: Display,
    pub gl_config: Config,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub display_handles: SendRawHandle,
    pub base: BaseGameServer<SendMsg, RecvMsg>,
}

impl SendServer {
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
                draw_callback: None,
            },
            ServerChannel {
                sender,
                receiver,
                current_id: 0,
            },
        ))
    }
}

impl Server {
    pub fn set_swap_interval(&mut self, swap_interval: SwapInterval) -> anyhow::Result<()> {
        self.gl_surface
            .set_swap_interval(&self.gl_context, swap_interval)?;
        self.swap_interval = swap_interval;
        Ok(())
    }

    pub fn set_draw_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&Server) -> anyhow::Result<()> + Send + 'static,
    {
        self.draw_callback = Some(Box::new(callback));
    }

    #[allow(clippy::redundant_closure_call)]
    fn process_messages(&mut self) -> anyhow::Result<()> {
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
                    let result = callback(self);
                    if let Some(a7) = result.downcast_ref::<anyhow::Result<u32>>() {
                        tracing::info!("{:?}", a7);
                    }
                    self.base.send(SendMsg::ExecuteSyncReturn(result)).context(
                        "unable to send ExecuteSyncReturn message for Sync return mechanism",
                    )?;
                }
                RecvMsg::ExecuteEvent(callback) => {
                    callback(self)
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
}

impl GameServer for Server {
    fn to_send(self) -> anyhow::Result<SendGameServer> {
        let gl_context = self
            .gl_context
            .make_not_current()
            .context("unable to make OpenGL context not current")?;
        Ok(SendGameServer::Draw(SendServer {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            display_handles: self.display_handles,
            display_size: self.display_size,
            swap_interval: self.swap_interval,
            handles: self.handles.to_send(),
            draw_callback: self.draw_callback,
        }))
    }

    fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Draw", runner_frequency);
        self.process_messages()?;
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.2, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        if let Some(mut callback) = self.draw_callback.take() {
            callback(self)
                .context("error execute draw callback")
                .log_error();
            // borrow checker thing
            self.draw_callback = Some(callback);
        }
        self.gl_surface.swap_buffers(&self.gl_context)?;
        Ok(())
    }
}

impl SendServer {
    pub fn make_current(self) -> anyhow::Result<Server> {
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
        Ok(Server {
            base: self.base,
            gl_config: self.gl_config,
            gl_context,
            gl_display: self.gl_display,
            gl_surface,
            display_handles: self.display_handles,
            display_size: self.display_size,
            swap_interval: self.swap_interval,
            handles: self.handles.to_unsend(),
            draw_callback: self.draw_callback,
        })
    }
}

pub struct ServerChannel {
    sender: Sender<RecvMsg>,
    receiver: Receiver<SendMsg>,
    current_id: u64,
}

impl GameServerChannel<SendMsg, RecvMsg> for ServerChannel {
    fn receiver(&mut self) -> &mut Receiver<SendMsg> {
        &mut self.receiver
    }
}

impl GameServerSendChannel<RecvMsg> for ServerChannel {
    fn sender(&self) -> &Sender<RecvMsg> {
        &self.sender
    }
}

impl ServerChannel {
    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(RecvMsg::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }

    pub fn generate_id(&mut self) -> u64 {
        let id = self.current_id;
        self.current_id += 1;
        id
    }
}

#[test]
fn test_send_sync() {
    fn test_send<T: Send>() {}
    test_send::<SendServer>();
}

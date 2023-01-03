use crate::{
    events::GameUserEvent,
    exec::dispatch::ReturnMechanism,
    graphics::{debug_callback::enable_gl_debug_callback, tree::DrawTree, HandleContainer},
    handle_msg,
    utils::{
        error::ResultExt,
        mpsc::{UnboundedReceiver, UnboundedSender},
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

use super::{BaseGameServer, GameServer, GameServerChannel, SendGameServer};
use crate::{display::SendRawHandle, utils::mpsc::UnboundedReceiverExt};

pub type ExecuteCallbackReturnType = anyhow::Result<Box<dyn Any + Send + Sync>>;
pub type ExecuteCallback = dyn FnOnce(&mut Server) -> ExecuteCallbackReturnType + Send;

pub type DrawCallback = dyn Fn(&Server) -> anyhow::Result<()>;

pub enum SendMsg {
    ExecuteReturn(ExecuteCallbackReturnType),
}
pub enum RecvMsg {
    SetFrequencyProfiling(bool),
    Resize(PhysicalSize<NonZeroU32>),
    Execute(Box<ExecuteCallback>, Option<ReturnMechanism>),
}
pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub display_handles: SendRawHandle,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub swap_interval: SwapInterval,
    pub handles: HandleContainer,
    pub draw_tree: DrawTree,
}

pub struct SendServer {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub display_handles: SendRawHandle,
    pub display_size: PhysicalSize<NonZeroU32>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_context: NotCurrentContext,
    pub swap_interval: SwapInterval,
    pub handles: HandleContainer,
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
        gl::load_with(|symbol| {
            let symbol = CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        });
        enable_gl_debug_callback();
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
                handles: HandleContainer::new(),
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

    #[allow(clippy::redundant_closure_call)]
    fn process_messages(&mut self) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .receive_all_pending(false)
            .ok_or_else(|| anyhow::format_err!("thread runner channel was unexpectedly closed"))?;
        let mut resize = None;
        for message in messages {
            match message {
                RecvMsg::Resize(new_size) => resize = Some(new_size),
                RecvMsg::SetFrequencyProfiling(fp) => self.base.frequency_profiling = fp,
                RecvMsg::Execute(callback, ret) => {
                    let result = callback(self);
                    handle_msg!(
                        self,
                        ret,
                        "Execute",
                        || SendMsg::ExecuteReturn(result),
                        |id| { GameUserEvent::ExecuteReturn(result, id) }
                    )?;
                }
            }
        }

        if let Some(new_size) = resize {
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
            display_handles: self.display_handles,
            display_size: self.display_size,
            swap_interval: self.swap_interval,
            handles: self.handles,
        }))
    }

    fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Draw", runner_frequency);
        self.process_messages()?;
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.2, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.draw_tree
            .render(self)
            .context("error drawing draw tree")
            .log_error();
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
            handles: self.handles,
            draw_tree: DrawTree::new(),
        })
    }
}
pub struct ServerChannel {
    sender: UnboundedSender<RecvMsg>,
    receiver: UnboundedReceiver<SendMsg>,
    current_id: u64,
}

impl GameServerChannel<SendMsg, RecvMsg> for ServerChannel {
    fn sender(&self) -> &UnboundedSender<RecvMsg> {
        &self.sender
    }
    fn receiver(&mut self) -> &mut UnboundedReceiver<SendMsg> {
        &mut self.receiver
    }
}

impl ServerChannel {
    pub fn resize(&mut self, size: PhysicalSize<NonZeroU32>) -> anyhow::Result<()> {
        self.send(RecvMsg::Resize(size))
    }

    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(RecvMsg::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }

    pub fn generate_multi_ids(&mut self, num_ids: usize) -> u64 {
        let id = self.current_id;
        self.current_id += num_ids as u64;
        id
    }

    pub fn generate_id(&mut self) -> u64 {
        self.generate_multi_ids(1)
    }
}

#[test]
fn test_send_sync() {
    fn test_send<T: Send>() {}
    test_send::<SendServer>();
}

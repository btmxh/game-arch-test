use glutin::{
    config::Config,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::GlDisplay,
    surface::{Surface, WindowSurface, SurfaceAttributes},
};
use raw_window_handle::RawWindowHandle;

use super::{BaseGameServer, GameServer, SendGameServer, ServerChannel};
use crate::utils::mpsc;

pub enum SendMsg {}
pub enum RecvMsg {}
pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
}

pub struct SendServer {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub gl_config: Config,
    pub gl_display: Display,
    pub gl_context: NotCurrentContext,
}

impl SendServer {
    pub fn new(
        gl_config: Config,
        display: &crate::display::Display,
    ) -> anyhow::Result<(Self, ServerChannel<SendMsg, RecvMsg>)> {
        let (send_msg_sender, send_msg_receiver) = mpsc::unbounded_channel();
        let (recv_msg_sender, recv_msg_receiver) = mpsc::unbounded_channel();
        let gl_display = gl_config.display();
        let context_attribs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .with_debug(cfg!(debug_assertions))
            .build(Some(display.get_raw_window_handle()));
        let gl_context = unsafe { gl_display.create_context(&gl_config, &context_attribs) }?;
        Ok((
            Self {
                base: BaseGameServer {
                    sender: send_msg_sender,
                    receiver: recv_msg_receiver,
                },
                gl_display,
                gl_context,
                gl_config,
            },
            ServerChannel {
                sender: recv_msg_sender,
                receiver: send_msg_receiver,
            },
        ))
    }
}

impl GameServer<SendMsg, RecvMsg> for Server {
    fn to_send(self: Box<Self>) -> Box<dyn super::SendGameServer<SendMsg, RecvMsg>> {
        Box::new(SendServer {
            base: self.base,
            gl_config: self.gl_config,
            gl_context: self.gl_context.,
            gl_display: self.gl_display,
        })
    }
}

impl SendGameServer<SendMsg, RecvMsg> for SendServer {
    fn to_nonsend(self: Box<Self>) -> Box<dyn GameServer<SendMsg, RecvMsg>> {
        let gl_surface = self.gl_display.create_window_surface(&self.gl_config, &self.surface_attrs);
        Box::new(Server {
            base: self.base,
            gl_config: self.gl_config,
            gl_context: self.gl_context,
            gl_display: self.gl_display,
        })
    }
}

#[test]
fn test_send_sync() {
    fn test_send<T: Send>() {}
    test_send::<SendServer>();
}

use crate::{
    events::GameUserEvent,
    graphics::context::{DrawContext, SendDrawContext},
    scene::draw::DrawRoot,
    utils::mpsc::{Receiver, Sender},
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
type ExecuteCallback<R> = dyn FnOnce(&mut DrawContext, &mut DrawRoot) -> R + Send;

pub enum RecvMsg {
    SetFrequencyProfiling(bool),
    ExecuteSync(Box<ExecuteCallback<ExecuteSyncReturnType>>),
    ExecuteEvent(Box<ExecuteCallback<ExecuteEventReturnType>>),
}
pub struct Server {
    pub context: DrawContext,
    pub root_scene: DrawRoot,
}

pub struct SendServer {
    pub context: SendDrawContext,
    pub root_scene: DrawRoot,
}

impl GameServer for Server {
    fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.context.draw(&mut self.root_scene, runner_frequency)
    }

    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Draw(SendServer {
            context: self.context.to_send()?,
            root_scene: self.root_scene,
        }))
    }
}

impl SendServer {
    pub fn new(
        proxy: EventLoopProxy<GameUserEvent>,
        gl_config: Config,
        display: &crate::display::Display,
    ) -> anyhow::Result<(Self, ServerChannel)> {
        let (context, channel) = SendDrawContext::new(proxy, gl_config, display)?;
        Ok((
            Self {
                context,
                root_scene: DrawRoot::new()?,
            },
            channel,
        ))
    }

    pub fn to_nonsend(self) -> anyhow::Result<Server> {
        Ok(Server {
            context: self.context.to_nonsend()?,
            root_scene: self.root_scene,
        })
    }
}

pub struct ServerChannel {
    pub sender: Sender<RecvMsg>,
    pub receiver: Receiver<SendMsg>,
    pub current_id: u64,
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

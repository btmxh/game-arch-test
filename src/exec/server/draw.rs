use crate::{
    events::GameUserEvent,
    graphics::context::{DrawContext, SendDrawContext},
    scene::main::RootScene,
    utils::{
        args::args,
        mpsc::{Receiver, Sender},
    },
};
use std::any::Any;

use anyhow::Context;
use glutin::config::Config;
use winit::event_loop::EventLoopProxy;

use super::{
    GameServer, GameServerChannel, GameServerSendChannel, SendGameServer, ServerSendChannel,
};

pub type DrawCallback = dyn FnMut(&Server) -> anyhow::Result<()> + Send;

pub enum SendMsg {
    ExecuteSyncReturn(Box<dyn Any + Send>),
}

type ExecuteSyncReturnType = Box<dyn Any + Send + 'static>;
type ExecuteEventReturnType = Box<dyn Iterator<Item = GameUserEvent>>;
type ExecuteCallback<R> = dyn FnOnce(&mut DrawContext, &mut Option<RootScene>) -> R + Send;

pub enum RecvMsg {
    SetFrequencyProfiling(bool),
    ExecuteSync(Box<ExecuteCallback<ExecuteSyncReturnType>>),
    ExecuteEvent(Box<ExecuteCallback<ExecuteEventReturnType>>),
}
pub struct Server {
    pub context: DrawContext,
    pub root_scene: Option<RootScene>,
}

pub struct SendServer {
    pub context: SendDrawContext,
    pub root_scene: Option<RootScene>,
}

impl GameServer for Server {
    fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.context.draw(&mut self.root_scene, runner_frequency)
    }

    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Draw(Box::new(SendServer {
            context: self.context.to_send()?,
            root_scene: self.root_scene,
        })))
    }

    fn does_run(&self) -> bool {
        !args().headless
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
                root_scene: None,
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

fn execute_draw_event<F, R>(
    channel: &impl GameServerSendChannel<RecvMsg>,
    callback: F,
) -> anyhow::Result<()>
where
    R: IntoIterator<Item = GameUserEvent> + Send + 'static,
    F: FnOnce(&mut DrawContext, &mut Option<RootScene>) -> R + Send + 'static,
{
    channel.send(RecvMsg::ExecuteEvent(Box::new(
        move |context, root_scene| Box::new(callback(context, root_scene).into_iter()),
    )))
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

    pub fn execute_draw_event<F, R>(&self, callback: F) -> anyhow::Result<()>
    where
        R: IntoIterator<Item = GameUserEvent> + Send + 'static,
        F: FnOnce(&mut DrawContext, &mut Option<RootScene>) -> R + Send + 'static,
    {
        self::execute_draw_event(self, callback)
    }
}

impl ServerSendChannel<RecvMsg> {
    pub fn execute_draw_event<F, R>(&self, callback: F) -> anyhow::Result<()>
    where
        R: IntoIterator<Item = GameUserEvent> + Send + 'static,
        F: FnOnce(&mut DrawContext, &mut Option<RootScene>) -> R + Send + 'static,
    {
        self::execute_draw_event(self, callback)
    }
}

#[test]
fn test_send_sync() {
    fn test_send<T: Send>() {}
    test_send::<SendServer>();
}

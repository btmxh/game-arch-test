use crate::{
    events::GameUserEvent,
    graphics::context::{DrawContext, SendDrawContext},
    scene::main::RootScene,
    utils::{
        error::ResultExt,
        mpsc::{Receiver, Sender},
    },
};
use anyhow::{anyhow, Context};
use glutin::config::Config;
use trait_set::trait_set;
use winit::event_loop::EventLoopProxy;

use super::{GameServer, GameServerChannel, GameServerSendChannel, SendGameServer};

pub type SendMsg = ();

trait_set! {
    pub trait DrawDispatch = FnOnce(&mut DrawContext, &mut Option<RootScene>) + Send;
}

pub enum RecvMsg {
    SetFrequencyProfiling(bool),
    Execute(Box<dyn DrawDispatch>),
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
    fn run(&mut self, single: bool, runner_frequency: f64) -> anyhow::Result<()> {
        self.context
            .draw(&mut self.root_scene, single, runner_frequency)
    }

    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Draw(Box::new(SendServer {
            context: self.context.to_send()?,
            root_scene: self.root_scene,
        })))
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

pub trait ServerSendChannelExt: GameServerSendChannel<RecvMsg> {
    fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(RecvMsg::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }

    fn execute<F>(&self, callback: F) -> anyhow::Result<()>
    where
        F: DrawDispatch + 'static,
    {
        self.send(RecvMsg::Execute(Box::new(callback)))
            .context("unable to send execute message to draw server")
    }

    fn execute_draw_event<F, R>(&self, callback: F) -> anyhow::Result<()>
    where
        R: IntoIterator<Item = GameUserEvent> + Send + 'static,
        F: FnOnce(&mut DrawContext, &mut Option<RootScene>) -> R + Send + 'static,
    {
        self.execute(move |context, root_scene| {
            for event in callback(context, root_scene) {
                context
                    .base
                    .proxy
                    .send_event(event)
                    .map_err(|e| anyhow!("{e}"))
                    .context("unable to send events to main thread")
                    .log_warn();
            }
        })
    }
}

impl<T> ServerSendChannelExt for T where T: GameServerSendChannel<RecvMsg> {}

#[test]
fn test_send_sync() {
    use crate::assert_send;
    assert_send!(SendServer);
}

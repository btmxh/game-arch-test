use anyhow::Context;
use async_trait::async_trait;
use winit::event_loop::EventLoopProxy;

use crate::{
    events::GameUserEvent,
    exec::dispatch::DispatchMsg,
    utils::mpsc::{UnboundedReceiver, UnboundedReceiverExt, UnboundedSender},
};

use super::{BaseGameServer, GameServer, GameServerChannel, SendGameServer, GameServerSendChannel};

pub enum SendMsg {
    Dispatch(DispatchMsg),
}
pub enum RecvMsg {
    SetFrequencyProfiling(bool),
}

pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
}

pub struct ServerChannel {
    sender: UnboundedSender<RecvMsg>,
    receiver: UnboundedReceiver<SendMsg>,
}

impl GameServerChannel<SendMsg, RecvMsg> for ServerChannel {
    fn receiver(&mut self) -> &mut UnboundedReceiver<SendMsg> {
        &mut self.receiver
    }
}

impl GameServerSendChannel<RecvMsg> for ServerChannel {
    fn sender(&self) -> &UnboundedSender<RecvMsg> {
        &self.sender
    }
}

#[async_trait(?Send)]
impl GameServer for Server {
    async fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Audio", runner_frequency);
        let messages = self
            .base
            .receiver
            .receive_all_pending(false)
            .await
            .context("thread runner channel was unexpectedly closed")?;
        for message in messages {
            match message {
                RecvMsg::SetFrequencyProfiling(fp) => {
                    self.base.frequency_profiling = fp;
                }
            }
        }
        Ok(())
    }
    fn to_send(self) -> anyhow::Result<Box<dyn SendGameServer>> {
        Ok(Box::new(self))
    }
}

impl SendGameServer for Server {
    fn server_kind(&self) -> super::ServerKind {
        super::ServerKind::Audio
    }

    fn downcast_audio(self: Box<Self>) -> anyhow::Result<self::Server> {
        Ok(*self)
    }
}

impl Server {
    pub fn new(proxy: EventLoopProxy<GameUserEvent>) -> (Self, ServerChannel) {
        let (base, sender, receiver) = BaseGameServer::new(proxy);
        (Self { base }, ServerChannel { receiver, sender })
    }
}

impl ServerChannel {
    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(RecvMsg::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }
}

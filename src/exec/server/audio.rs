use anyhow::Context;
use winit::event_loop::EventLoopProxy;

use crate::{
    events::GameUserEvent,
    exec::dispatch::DispatchMsg,
    utils::mpsc::{Receiver, Sender},
};

use super::{BaseGameServer, GameServer, GameServerChannel, GameServerSendChannel, SendGameServer};

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
    sender: Sender<RecvMsg>,
    receiver: Receiver<SendMsg>,
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

impl GameServer for Server {
    fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Audio", runner_frequency);
        let messages = self
            .base
            .receiver
            .try_iter(None)
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
    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Audio(Box::new(self)))
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

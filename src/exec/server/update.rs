use std::time::Instant;

use anyhow::Context;

use super::{BaseGameServer, GameServer, SendGameServer};
use crate::{
    context::update::{TimeoutDispatchHandle, UpdateContext},
    display::EventSender,
    utils::{mpsc::Receiver, uid::Uid},
};

pub enum Message {
    SetFrequencyProfiling(bool),
    SetTimeout(Instant, Uid),
    CancelTimeout(TimeoutDispatchHandle),
}

pub struct Server {
    base: BaseGameServer<Message>,
    context: UpdateContext,
}

impl GameServer for Server {
    fn run(&mut self, _: bool, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Update", runner_frequency);
        let messages = self
            .base
            .receiver
            .try_iter(None)
            .context("thread runner channel was unexpectedly closed")?;
        for message in messages {
            match message {
                Message::SetTimeout(timeout_instant, id) => {
                    self.context.set_timeout(timeout_instant, id);
                }
                Message::CancelTimeout(handle) => {
                    self.context.cancel_timeout(handle);
                }
                Message::SetFrequencyProfiling(fp) => {
                    self.base.frequency_profiling = fp;
                }
            };
        }

        self.context
            .update(&self.base)
            .context("Error while updating update server")?;
        Ok(())
    }
    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Update(Box::new(self)))
    }
}

impl Server {
    pub fn new(event_sender: EventSender, receiver: Receiver<Message>) -> Self {
        Self {
            base: BaseGameServer::new(event_sender, receiver),
            context: UpdateContext::new(),
        }
    }
}

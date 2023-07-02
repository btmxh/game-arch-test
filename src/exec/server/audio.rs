use anyhow::Context;

use crate::{
    display::EventSender,
    utils::mpsc::{self, Sender},
};

use super::{BaseGameServer, GameServer, SendGameServer};

pub enum Message {
    SetFrequencyProfiling(bool),
}

pub struct Server {
    pub base: BaseGameServer<Message>,
}

impl GameServer for Server {
    fn run(&mut self, _: bool, runner_frequency: f64) -> anyhow::Result<()> {
        for _ in 0..self.base.run("Audio", runner_frequency) {
            let messages = self
                .base
                .receiver
                .try_iter(None)
                .context("thread runner channel was unexpectedly closed")?;
            for message in messages {
                match message {
                    Message::SetFrequencyProfiling(fp) => {
                        self.base.frequency_profiling = fp;
                    }
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
    pub fn new(event_sender: EventSender, receiver: mpsc::Receiver<Message>) -> Self {
        Self {
            base: BaseGameServer::new(event_sender, receiver),
        }
    }
}

impl Sender<Message> {
    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(Message::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling message")
    }
}

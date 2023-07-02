use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Context;

use super::{BaseGameServer, GameServer, SendGameServer};
use crate::{
    display::EventSender,
    events::GameUserEvent,
    utils::{
        mpsc::{Receiver, Sender},
        uid::Uid,
    },
};

pub enum Message {
    SetFrequencyProfiling(bool),
    SetTimeout(Instant, Uid),
    CancelTimeout(Uid),
}

pub struct Server {
    pub base: BaseGameServer<Message>,
    pub timeouts: HashMap<Uid, Instant>,
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
                Message::SetTimeout(inst, id) => {
                    self.timeouts.insert(id, inst);
                }
                Message::CancelTimeout(id) => {
                    self.timeouts.remove(&id);
                }
                Message::SetFrequencyProfiling(fp) => {
                    self.base.frequency_profiling = fp;
                }
            };
        }
        let mut done_timeouts = Vec::new();
        self.timeouts.retain(|&id, &mut end| {
            if Instant::now() >= end {
                done_timeouts.push(id);
                false
            } else {
                true
            }
        });
        if !done_timeouts.is_empty() {
            self.base
                .event_sender
                .send_event(GameUserEvent::UpdateDispatch(done_timeouts))
                .map_err(|e| anyhow::format_err!("{}", e))
                .context("unable to send event to event loop")?;
        }
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
            timeouts: HashMap::new(),
        }
    }
}

impl Sender<Message> {
    pub fn set_timeout(&self, duration: Duration, id: Uid) -> anyhow::Result<()> {
        self.send(Message::SetTimeout(Instant::now() + duration, id))
            .context("unable to send timeout request")
    }

    pub fn cancel_timeout(&self, id: Uid) -> anyhow::Result<()> {
        self.send(Message::CancelTimeout(id))
            .context("unable to send cancel timeout request")
    }

    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(Message::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }
}

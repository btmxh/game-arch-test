use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Context;
use async_trait::async_trait;
use winit::event_loop::EventLoopProxy;

use super::{BaseGameServer, GameServer, GameServerChannel, GameServerSendChannel, SendGameServer};
use crate::{
    events::GameUserEvent,
    exec::dispatch::{DispatchId, DispatchMsg},
    utils::mpsc::{UnboundedReceiver, UnboundedReceiverExt, UnboundedSender},
};

pub enum SendMsg {}
pub enum RecvMsg {
    SetFrequencyProfiling(bool),
    SetTimeout(Instant, DispatchId),
    CancelTimeout(DispatchId),
}

pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub timeouts: HashMap<DispatchId, Instant>,
}

#[async_trait(?Send)]
impl GameServer for Server {
    async fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        self.base.run("Update", runner_frequency);
        let messages = self
            .base
            .receiver
            .receive_all_pending(false)
            .await
            .context("thread runner channel was unexpectedly closed")?;
        for message in messages {
            match message {
                RecvMsg::SetTimeout(inst, id) => {
                    self.timeouts.insert(id, inst);
                }
                RecvMsg::CancelTimeout(id) => {
                    self.timeouts.remove(&id);
                }
                RecvMsg::SetFrequencyProfiling(fp) => {
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
                .proxy
                .send_event(GameUserEvent::Dispatch(DispatchMsg::ExecuteDispatch(
                    done_timeouts,
                )))
                .map_err(|e| anyhow::format_err!("{}", e))
                .context("unable to send event to event loop")?;
        }
        Ok(())
    }
    fn to_send(self) -> anyhow::Result<Box<dyn SendGameServer>> {
        Ok(Box::new(self))
    }
}

impl SendGameServer for Server {
    fn server_kind(&self) -> super::ServerKind {
        super::ServerKind::Update
    }

    fn downcast_update(self: Box<Self>) -> anyhow::Result<self::Server> {
        Ok(*self)
    }
}

impl Server {
    pub fn new(proxy: EventLoopProxy<GameUserEvent>) -> (Self, ServerChannel) {
        let (base, sender, receiver) = BaseGameServer::new(proxy);
        (
            Self {
                base,
                timeouts: HashMap::new(),
            },
            ServerChannel { sender, receiver },
        )
    }
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

impl ServerChannel {
    pub fn set_timeout(&self, duration: Duration, id: DispatchId) -> anyhow::Result<()> {
        self.send(RecvMsg::SetTimeout(Instant::now() + duration, id))
            .context("unable to send timeout request")
    }

    pub fn cancel_timeout(&self, id: DispatchId) -> anyhow::Result<()> {
        self.send(RecvMsg::CancelTimeout(id))
            .context("unable to send cancel timeout request")
    }

    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(RecvMsg::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }
}

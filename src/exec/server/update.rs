use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Context;
use winit::event_loop::EventLoopProxy;

use super::{BaseGameServer, GameServer, GameServerChannel, SendGameServer};
use crate::{
    events::GameUserEvent,
    utils::{
        error::ResultExt,
        mpsc::{UnboundedReceiver, UnboundedReceiverExt, UnboundedSender},
    },
};

pub enum SendMsg {
    Dispatch(Vec<u64>),
}
pub enum RecvMsg {
    SetTimeout(Timeout),
}

pub struct Timeout(Instant, u64);

pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
    pub proxy: EventLoopProxy<GameUserEvent>,
    pub timeouts: Vec<Timeout>,
}

impl GameServer for Server {
    fn run(&mut self) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .receive_all_pending(false)
            .context("thread runner channel was unexpectedly closed")?;
        for message in messages {
            match message {
                RecvMsg::SetTimeout(timeout) => self.timeouts.push(timeout),
            }
        }
        let mut done_timeouts = Vec::new();
        self.timeouts.retain(|Timeout(end, id)| {
            if Instant::now() >= *end {
                done_timeouts.push(*id);
                false
            } else {
                true
            }
        });
        if !done_timeouts.is_empty() {
            self.proxy
                .send_event(GameUserEvent::SetTimeoutDispatch(done_timeouts))?;
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
        let (base, sender, receiver) = BaseGameServer::new();
        (
            Self {
                base,
                proxy,
                timeouts: Vec::new(),
            },
            ServerChannel {
                sender,
                receiver,
                dispatches: HashMap::new(),
                dispatch_counter: 0,
            },
        )
    }
}

pub struct ServerChannel {
    sender: UnboundedSender<RecvMsg>,
    receiver: UnboundedReceiver<SendMsg>,
    dispatches: HashMap<u64, Box<dyn FnOnce()>>,
    dispatch_counter: u64,
}

impl GameServerChannel<SendMsg, RecvMsg> for ServerChannel {
    fn sender(&self) -> &UnboundedSender<RecvMsg> {
        &self.sender
    }
    fn receiver(&mut self) -> &mut UnboundedReceiver<SendMsg> {
        &mut self.receiver
    }
}

impl ServerChannel {
    pub fn dispatch(&mut self, id: u64) {
        let dispatch = self
            .dispatches
            .remove(&id)
            .ok_or_else(|| anyhow::format_err!("dispatch with ID {} not found", id))
            .log_warn();
        if let Some(dispatch) = dispatch {
            dispatch();
        }
    }

    pub fn set_timeout<F>(&mut self, duration: Duration, callback: F) -> anyhow::Result<()>
    where
        F: FnOnce() + 'static,
    {
        let id = self.dispatch_counter;
        self.dispatch_counter += 1;
        self.dispatches.insert(id, Box::new(callback));
        self.send(RecvMsg::SetTimeout(Timeout(Instant::now() + duration, id)))
            .context("unable to send timeout request")
    }
}

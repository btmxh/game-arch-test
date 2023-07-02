use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

use anyhow::Context;

use crate::{
    exec::{
        dispatch::EventDispatch,
        server::{
            update::{self, Message},
            BaseGameServer,
        },
    },
    utils::{
        mpsc::{self, Receiver, Sender},
        uid::Uid,
    },
};

pub enum DispatchHandle {
    Single(Uid),
    Multiple(Vec<Uid>),
}

pub struct UpdateContext {
    base: BaseGameServer<update::Message>,
    dispatch_handles: BTreeMap<Instant, DispatchHandle>,
}

pub struct UpdateSender {
    dispatches: HashMap<Uid, Box<dyn EventDispatch>>,
    sender: Sender<Message>,
}

pub struct TimeoutCallbackHandle(Uid);

impl UpdateSender {
    pub fn new() -> (Self, Receiver<Message>) {
        let (sender, receiver) = mpsc::channels();
        (
            Self {
                dispatches: HashMap::new(),
                sender,
            },
            receiver,
        )
    }

    pub fn set_timeout<F>(
        &mut self,
        timeout: Duration,
        callback: F,
    ) -> anyhow::Result<TimeoutCallbackHandle>
    where
        F: EventDispatch + 'static,
    {
        let id = Uid::new();
        let old_value = self.dispatches.insert(id, Box::new(callback));
        debug_assert!(old_value.is_none());
        self.sender.set_timeout(timeout, id)?;
        Ok(TimeoutCallbackHandle(id))
    }

    pub fn cancel(&self, handle: TimeoutCallbackHandle) -> anyhow::Result<()> {
        self.sender.cancel_timeout(handle.0)
    }

    pub fn pop(&mut self, id: Uid) -> Option<Box<dyn EventDispatch>> {
        self.dispatches.remove(&id)
    }

    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.sender
            .send(Message::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }
}

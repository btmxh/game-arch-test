use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

use anyhow::Context;
use smallvec::{smallvec, SmallVec};

use crate::{
    events::GameUserEvent,
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

pub struct TimeoutDispatchHandle {
    pub id: Uid,
    timeout_instant: Instant,
}

pub type TimeoutDispatchHandleSet = SmallVec<[Uid; 2]>;

pub struct UpdateContext {
    dispatch_handles: BTreeMap<Instant, TimeoutDispatchHandleSet>,
}

struct ReverseInstant(Instant);

impl PartialEq for ReverseInstant {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for ReverseInstant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0).reverse())
    }
}

impl Eq for ReverseInstant {}

impl Ord for ReverseInstant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0).reverse()
    }
}

impl UpdateContext {
    pub fn new() -> Self {
        Self {
            dispatch_handles: BTreeMap::new(),
        }
    }

    pub fn set_timeout(&mut self, timeout_instant: Instant, id: Uid) {
        self.dispatch_handles
            .entry(timeout_instant)
            .and_modify(|handles| handles.push(id))
            .or_insert_with(|| smallvec![id]);
    }

    pub fn cancel_timeout(&mut self, handle: TimeoutDispatchHandle) {
        {
            if let Some(id_set) = self.dispatch_handles.get_mut(&handle.timeout_instant) {
                if id_set.len() >= 2 {
                    let maybe_index = id_set.iter().position(|x| *x == handle.id);
                    // ordering may be important here
                    // so for the moment don't use swap_remove
                    if let Some(index) = maybe_index {
                        id_set.remove(index);
                    }

                    return;
                }
            } else {
                return;
            }
        }

        // most probable case
        self.dispatch_handles.remove(&handle.timeout_instant);
    }

    pub fn update(&mut self, base: &BaseGameServer<update::Message>) -> anyhow::Result<()> {
        let handles = self
            .dispatch_handles
            .split_off(&Instant::now())
            .into_values()
            .flatten()
            .collect::<TimeoutDispatchHandleSet>();
        if !handles.is_empty() {
            base.event_sender
                .send_event(GameUserEvent::UpdateDispatch(handles))?;
        }

        Ok(())
    }
}

pub struct UpdateSender {
    dispatches: HashMap<Uid, Box<dyn EventDispatch>>,
    sender: Sender<Message>,
}

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
    ) -> anyhow::Result<TimeoutDispatchHandle>
    where
        F: EventDispatch + 'static,
    {
        let id = Uid::new();
        let old_value = self.dispatches.insert(id, Box::new(callback));
        debug_assert!(old_value.is_none());
        let timeout_instant = Instant::now() + timeout;
        self.sender
            .send(Message::SetTimeout(timeout_instant, id))
            .context("Unable to send SetTimeout message to update server")?;
        Ok(TimeoutDispatchHandle {
            id,
            timeout_instant,
        })
    }

    pub fn cancel(&self, handle: TimeoutDispatchHandle) -> anyhow::Result<()> {
        self.sender
            .send(Message::CancelTimeout(handle))
            .context("Unable to send CancelTimeout message to update server")
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

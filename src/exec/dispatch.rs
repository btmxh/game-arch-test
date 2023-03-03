use std::collections::HashMap;

use trait_set::trait_set;

use crate::{scene::main::RootScene, utils::uid::Uid};

use super::main_ctx::MainContext;

trait_set! {
    pub trait EventDispatch = FnOnce(&mut MainContext, &mut RootScene) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct DispatchList {
    dispatches: HashMap<Uid, Box<dyn EventDispatch>>,
}

impl DispatchList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<F>(&mut self, callback: F) -> Uid
    where
        F: EventDispatch + 'static,
    {
        self.push_boxed(Box::new(callback))
    }

    pub fn push_boxed(&mut self, callback: Box<dyn EventDispatch>) -> Uid {
        let id = Uid::new();
        debug_assert!(!self.dispatches.contains_key(&id));
        self.dispatches.insert(id, callback);
        id
    }

    pub fn pop(&mut self, id: Uid) -> Option<Box<dyn EventDispatch>> {
        self.dispatches.remove(&id)
    }
}

#[derive(Debug)]
pub enum DispatchMsg {
    ExecuteDispatch(Vec<Uid>),
}

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum ReturnMechanism {
//     Sync,
//     Event(Option<DispatchId>),
// }

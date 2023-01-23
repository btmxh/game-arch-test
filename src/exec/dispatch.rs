use std::collections::HashMap;

use crate::scene::main::EventRoot;

use super::{
    executor::GameServerExecutor,
    main_ctx::MainContext,
    task::{Cancellable, CancellationToken},
};

pub type DispatchId = u64;
pub type DispatchFnType = dyn FnOnce(
    &mut MainContext,
    &mut GameServerExecutor,
    &mut EventRoot,
    DispatchId,
) -> anyhow::Result<()>;

#[derive(Default)]
pub struct DispatchList {
    dispatches: HashMap<DispatchId, (Box<DispatchFnType>, CancellationToken)>,
    count: DispatchId,
}

impl DispatchList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<F>(&mut self, callback: F, cancel_token: CancellationToken) -> DispatchId
    where
        F: FnOnce(
                &mut MainContext,
                &mut GameServerExecutor,
                &mut EventRoot,
                DispatchId,
            ) -> anyhow::Result<()>
            + 'static,
    {
        self.push_boxed(Box::new(callback), cancel_token)
    }

    pub fn push_boxed(
        &mut self,
        callback: Box<DispatchFnType>,
        cancel_token: CancellationToken,
    ) -> DispatchId {
        let id = self.count;
        self.count += 1;
        debug_assert!(!self.dispatches.contains_key(&id));
        self.dispatches.insert(id, (callback, cancel_token));
        id
    }

    pub fn pop(&mut self, id: DispatchId) -> Option<(Box<DispatchFnType>, CancellationToken)> {
        self.dispatches.remove(&id)
    }

    pub fn handle_dispatch_msg(
        &mut self,
        msg: DispatchMsg,
    ) -> HashMap<DispatchId, Box<DispatchFnType>> {
        let mut dispatches = HashMap::new();
        match msg {
            DispatchMsg::CancelDispatch(ids) => ids.iter().for_each(|&id| {
                self.pop(id);
            }),
            DispatchMsg::ExecuteDispatch(ids) => {
                ids.iter()
                    .filter_map(|&id| self.pop(id).map(|d| (id, d)))
                    .for_each(|(id, (callback, cancel_token))| {
                        if !cancel_token.is_cancelled() {
                            dispatches.insert(id, callback);
                        }
                    });
            }
        };
        dispatches
    }
}

#[derive(Debug)]
pub enum DispatchMsg {
    CancelDispatch(Vec<DispatchId>),
    ExecuteDispatch(Vec<DispatchId>),
}

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum ReturnMechanism {
//     Sync,
//     Event(Option<DispatchId>),
// }

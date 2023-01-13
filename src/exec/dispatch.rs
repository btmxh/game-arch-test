use std::collections::HashMap;

use super::main_ctx::MainContext;

pub type DispatchId = u64;
pub type DispatchFnType = dyn FnOnce(&mut MainContext, DispatchId);

#[derive(Default)]
pub struct DispatchList {
    dispatches: HashMap<DispatchId, Box<DispatchFnType>>,
    count: DispatchId,
}

impl DispatchList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<F>(&mut self, callback: F) -> DispatchId
    where
        F: FnOnce(&mut MainContext, DispatchId) + 'static,
    {
        self.push_boxed(Box::new(callback))
    }

    pub fn push_boxed(&mut self, callback: Box<DispatchFnType>) -> DispatchId {
        let id = self.count;
        self.count += 1;
        debug_assert!(!self.dispatches.contains_key(&id));
        self.dispatches.insert(id, callback);
        id
    }

    pub fn pop(&mut self, id: DispatchId) -> Option<Box<DispatchFnType>> {
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
                    .for_each(|(id, callback)| {
                        dispatches.insert(id, callback);
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

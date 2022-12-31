use std::collections::HashMap;

use super::main_ctx::MainContext;

pub type DispatchFnType = dyn FnOnce(&mut MainContext);
pub type DispatchId = u64;

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
        F: FnOnce(&mut MainContext) + 'static,
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

    pub fn handle_dispatch_msg(&mut self, msg: DispatchMsg) -> Vec<Box<DispatchFnType>> {
        let mut dispatches = Vec::new();
        match msg {
            DispatchMsg::CancelDispatch(ids) => ids.iter().for_each(|&id| {
                self.pop(id);
            }),
            DispatchMsg::ExecuteDispatch(ids) => ids
                .iter()
                .filter_map(|&id| self.pop(id))
                .for_each(|d| dispatches.push(d)),
        };
        dispatches
    }
}

#[derive(Debug)]
pub enum DispatchMsg {
    CancelDispatch(Vec<DispatchId>),
    ExecuteDispatch(Vec<DispatchId>),
}

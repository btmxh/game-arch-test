use std::fmt::Debug;

use rand::{thread_rng, RngCore};

#[derive(Debug)]
#[allow(dead_code)]
pub struct DebugHandle {
    id: u32,
    drop_check: bool,
}

impl DebugHandle {
    pub fn new(drop_check: bool) -> Self {
        Self {
            id: thread_rng().next_u32(),
            drop_check,
        }
    }
}

impl Drop for DebugHandle {
    fn drop(&mut self) {
        if self.drop_check {
            tracing::debug!("{:?} was dropped", self);
        }
    }
}

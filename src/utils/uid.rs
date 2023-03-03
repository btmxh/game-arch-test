use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uid(u64);

static UID_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Uid {
    pub fn new() -> Self {
        Self(UID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn get(&self) -> u64 {
        self.0
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }
}

impl Default for Uid {
    fn default() -> Self {
        Self::new()
    }
}

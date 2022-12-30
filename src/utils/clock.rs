use std::time::{Instant, SystemTime};

pub trait Clock {
    fn now(&self) -> f64;

    fn ellapsed(&self, since: f64) -> f64 {
        self.now() - since
    }
}

pub struct SteadyClock {
    start: Instant,
}

impl SteadyClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SteadyClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SteadyClock {
    fn now(&self) -> f64 {
        Instant::now()
            .saturating_duration_since(self.start)
            .as_secs_f64()
    }
}

pub fn debug_get_time() -> f64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

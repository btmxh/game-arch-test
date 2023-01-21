use std::time::Duration;

pub mod dispatch;
pub mod executor;
pub mod main_ctx;
pub mod runner;
pub mod server;
pub mod task;

const NUM_GAME_LOOPS: usize = 3;

#[cfg(debug_assertions)]
// one year, basically Duration::MAX without the overflowing
const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_secs(31556926);

#[cfg(not(debug_assertions))]
const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_millis(300);

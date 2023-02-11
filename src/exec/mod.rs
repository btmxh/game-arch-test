use std::time::Duration;

pub mod dispatch;
pub mod executor;
pub mod main_ctx;
pub mod runner;
pub mod server;
pub mod task;

const NUM_GAME_LOOPS: usize = 3;

#[cfg(debug_assertions)]
pub const DEFAULT_RECV_TIMEOUT: Duration = crate::utils::ONE_YEAR;

#[cfg(not(debug_assertions))]
pub const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_millis(300);

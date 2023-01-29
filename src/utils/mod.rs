use std::time::Duration;

pub mod args;
pub mod clock;
pub mod debug_handle;
pub mod enclose;
pub mod error;
pub mod frequency_runner;
pub mod log;
pub mod mpsc;
pub mod mutex;
pub mod send_sync;
pub mod sync;

// one year, basically Duration::MAX without the overflowing
pub const ONE_YEAR: Duration = Duration::from_secs(31556926);

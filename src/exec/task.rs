use std::{
    mem::ManuallyDrop,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    enclose,
    utils::{error::ResultExt, mpsc},
};
use delegate::delegate;
use executors::{
    crossbeam_workstealing_pool::{small_pool, ThreadPool},
    parker::{SmallThreadData, StaticParker},
    Executor,
};

pub struct TaskExecutor(ManuallyDrop<ThreadPool<StaticParker<SmallThreadData>>>);

#[derive(Clone)]
pub struct CancellationToken(Arc<AtomicBool>);
pub struct JoinToken(mpsc::Receiver<()>);
pub struct TaskHandle {
    pub cancel: CancellationToken,
    pub join: JoinToken,
}
pub struct DropTaskHandle(pub TaskHandle);

impl Drop for DropTaskHandle {
    fn drop(&mut self) {
        self.cancel();
        self.join();
    }
}

impl Drop for TaskExecutor {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.0) }
            .shutdown()
            .map_err(|e| anyhow::format_err!("error shutdown TaskExecutor: {e}"))
            .log_error();
    }
}

impl TaskHandle {
    pub fn as_drop_handle(self) -> DropTaskHandle {
        DropTaskHandle(self)
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl JoinToken {
    pub fn new() -> (mpsc::Sender<()>, Self) {
        let (sender, receiver) = mpsc::channels();
        (sender, Self(receiver))
    }
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self(ManuallyDrop::new(small_pool(4)))
    }

    #[allow(unused_mut)]
    pub fn execute<F>(&self, callback: F) -> TaskHandle
    where
        F: FnOnce(CancellationToken) + Send + 'static,
    {
        let cancel = CancellationToken::new();
        let (sender, join) = JoinToken::new();
        self.0.execute(enclose!((cancel) move || {
            callback(cancel);
            if let Err(err) = sender.send(()) {
                tracing::trace!("join message not sent: {}", err);
            }
        }));
        TaskHandle { cancel, join }
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Cancellable {
    fn cancel(&self);
    fn is_cancelled(&self) -> bool;
}

pub trait Joinable {
    fn join_timeout(&self, timeout: Duration) -> bool;
    fn has_joined(&self) -> bool;

    fn join(&self) {
        let result = self.join_timeout(crate::utils::ONE_YEAR);
        debug_assert!(result);
    }
}

impl Cancellable for CancellationToken {
    fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed)
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Joinable for JoinToken {
    fn join_timeout(&self, timeout: Duration) -> bool {
        self.0.recv_timeout(timeout).is_ok()
    }

    fn has_joined(&self) -> bool {
        self.0.is_disconnected()
    }
}

impl Joinable for TaskHandle {
    delegate! {
        to self.join {
            fn join_timeout(&self, timeout: Duration) -> bool;
            fn has_joined(&self) -> bool;
            fn join(&self);
        }
    }
}

impl Cancellable for TaskHandle {
    delegate! {
        to self.cancel {
            fn cancel(&self);
            fn is_cancelled(&self) -> bool;
        }
    }
}
impl Joinable for DropTaskHandle {
    delegate! {
        to self.0 {
            fn join_timeout(&self, timeout: Duration) -> bool;
            fn has_joined(&self) -> bool;
            fn join(&self);
        }
    }
}

impl Cancellable for DropTaskHandle {
    delegate! {
        to self.0 {
            fn cancel(&self);
            fn is_cancelled(&self) -> bool;
        }
    }
}

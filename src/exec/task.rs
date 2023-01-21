use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{
    enclose,
    utils::{error::ResultExt, mpsc},
};
use anyhow::Context;
use executors::{
    crossbeam_workstealing_pool::{small_pool, ThreadPool},
    parker::{SmallThreadData, StaticParker},
    Executor,
};

pub struct TaskExecutor(ThreadPool<StaticParker<SmallThreadData>>);

#[derive(Clone)]
pub struct CancellationToken(Arc<AtomicBool>);
pub struct FinishToken(mpsc::Receiver<()>);
pub struct TaskHandle(CancellationToken, FinishToken);
pub struct DropTaskHandle(TaskHandle);

impl Drop for DropTaskHandle {
    fn drop(&mut self) {
        self.0 .0.cancel();
        self.0 .1.join().log_error();
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

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed)
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl FinishToken {
    pub fn new() -> (mpsc::Sender<()>, Self) {
        let (sender, receiver) = mpsc::channels();
        (sender, Self(receiver))
    }

    pub fn join(&self) -> anyhow::Result<()> {
        self.0.recv().context("unable to receive join message")
    }
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self(small_pool(4))
    }

    #[allow(unused_mut)]
    pub fn execute<F>(&self, callback: F) -> TaskHandle
    where
        F: FnOnce(CancellationToken) + Send + 'static,
    {
        let cancel_token = CancellationToken::new();
        let (sender, finish_token) = FinishToken::new();
        self.0.execute(enclose!((cancel_token) move || {
            callback(cancel_token);
            if let Err(err) = sender.send(()) {
                tracing::trace!("join message not sent: {}", err);
            }
        }));
        TaskHandle(cancel_token, finish_token)
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

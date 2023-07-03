use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    num::NonZeroUsize,
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

use anyhow::Context;
use executors::{
    crossbeam_workstealing_pool::{large_pool, ThreadPool},
    parker::{LargeThreadData, StaticParker},
    Executor,
};

pub struct TaskExecutor(ManuallyDrop<ThreadPool<StaticParker<LargeThreadData>>>);

#[derive(Clone)]
pub struct CancellationToken(Arc<AtomicBool>);
pub struct JoinToken<R>(mpsc::Receiver<R>);
pub struct TaskHandle<R> {
    pub cancel: CancellationToken,
    pub join: JoinToken<R>,
}
pub struct DropTaskHandle<R>(pub TaskHandle<R>);
pub struct DropCancelJoin<C: Cancellable, J: Joinable<R>, R>(pub C, pub J, PhantomData<fn() -> R>);

impl<C, J, R> Drop for DropCancelJoin<C, J, R>
where
    C: Cancellable,
    J: Joinable<R>,
{
    fn drop(&mut self) {
        self.0.cancel();
        self.1.join();
    }
}

impl<C, J, R> DropCancelJoin<C, J, R>
where
    C: Cancellable,
    J: Joinable<R>,
{
    pub fn new(cancel: C, join: J) -> Self {
        Self(cancel, join, PhantomData)
    }
}

impl Drop for TaskExecutor {
    fn drop(&mut self) {
        let executor = unsafe { ManuallyDrop::take(&mut self.0) };
        std::thread::spawn(enclose!((executor) move || {
            executor.shutdown()
            .map_err(|e| anyhow::format_err!("error shutdown TaskExecutor: {e}"))
            .log_error();
        }));
    }
}

impl<R> TaskHandle<R> {
    pub fn into_drop_handle(self) -> DropTaskHandle<R> {
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

impl<R> JoinToken<R> {
    pub fn new() -> (mpsc::Sender<R>, Self) {
        let (sender, receiver) = mpsc::channels();
        (sender, Self(receiver))
    }
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self(ManuallyDrop::new(large_pool(
            std::thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4)
                .min(LargeThreadData::MAX_THREADS),
        )))
    }

    pub fn execute<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.0.execute(callback)
    }

    pub fn execute_return<F, R>(&self, callback: F) -> JoinToken<R>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        let (sender, token) = JoinToken::new();
        self.execute(move || {
            sender
                .send(callback())
                .context("Unable to execute task")
                .log_error();
        });
        token
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

pub enum TryJoinTaskResult<R> {
    JoinedResultTaken,
    NotJoined,
    Joined(R),
}

pub enum JoinTaskResult<R> {
    Done(R),
    ResultTaken,
}

pub trait Joinable<R> {
    // None => joined, result already taken
    // Some(None) => not joined
    // Some(Some(...)) => result
    fn join_timeout(&self, timeout: Duration) -> TryJoinTaskResult<R>;
    fn has_joined(&self) -> bool;

    // None => joined, result already taken
    // Some(Some(...)) => result
    // panic => not joined after a year
    fn join(&self) -> JoinTaskResult<R> {
        match self.join_timeout(crate::utils::ONE_YEAR) {
            TryJoinTaskResult::Joined(result) => JoinTaskResult::Done(result),
            TryJoinTaskResult::JoinedResultTaken => JoinTaskResult::ResultTaken,
            _ => panic!("one year has passed"),
        }
    }

    fn try_join(&self) -> TryJoinTaskResult<R> {
        self.join_timeout(Duration::ZERO)
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

impl<R> Joinable<R> for JoinToken<R> {
    fn join_timeout(&self, timeout: Duration) -> TryJoinTaskResult<R> {
        match self.0.recv_timeout(timeout) {
            Ok(Some(result)) => TryJoinTaskResult::Joined(result),
            Err(_) => TryJoinTaskResult::JoinedResultTaken,
            Ok(None) => TryJoinTaskResult::NotJoined,
        }
    }

    fn try_join(&self) -> TryJoinTaskResult<R> {
        match self.0.try_recv() {
            Ok(Some(result)) => TryJoinTaskResult::Joined(result),
            Err(_) => TryJoinTaskResult::JoinedResultTaken,
            Ok(None) => TryJoinTaskResult::NotJoined,
        }
    }

    fn join(&self) -> JoinTaskResult<R> {
        match self.0.recv() {
            Ok(result) => JoinTaskResult::Done(result),
            Err(_) => JoinTaskResult::ResultTaken,
        }
    }

    fn has_joined(&self) -> bool {
        self.0.is_disconnected()
    }
}

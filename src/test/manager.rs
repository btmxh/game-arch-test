use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;

use crate::{
    context::update::UpdateSender,
    display::EventSender,
    events::GameUserEvent,
    utils::{args::args, error::ResultExt, mutex::Mutex},
};

use super::tree::ParentTestNode;

struct TestManagerInner {
    pub root: Arc<ParentTestNode>,
    event_sender: Mutex<EventSender>,
    done_init: AtomicBool,
}

pub struct TestManager {
    inner: Arc<TestManagerInner>,
    // logs: HashMap<Cow<'static, str>, String>,
}

pub type OptionTestManager = Option<TestManager>;

enum TestExitCode {
    Complete = 0,
    Failed = 1,
    Timeout = 2,
}

impl TestManager {
    const TEST_TIMEOUT: Duration = Duration::new(30, 0);
    pub fn new_if_enabled(
        update_sender: &mut UpdateSender,
        event_sender: &EventSender,
    ) -> anyhow::Result<OptionTestManager> {
        args()
            .test
            .then(|| TestManager::new(update_sender, event_sender.clone()))
            .transpose()
    }

    pub fn new(
        update_sender: &mut UpdateSender,
        event_sender: EventSender,
    ) -> anyhow::Result<Self> {
        let slf = Self {
            inner: Arc::<TestManagerInner>::new_cyclic(|weak| {
                let weak = weak.clone();
                TestManagerInner {
                    event_sender: Mutex::new(event_sender),
                    root: ParentTestNode::new_root("root", move |_, result| {
                        if let Some(slf) = weak.upgrade() {
                            if !slf.done_init.load(Ordering::Relaxed) {
                                return;
                            }

                            let exit_code = if result.is_ok() {
                                TestExitCode::Complete
                            } else {
                                TestExitCode::Failed
                            };
                            tracing::info!(
                                "all test finished, result of root test is {:?}",
                                result
                            );
                            slf.event_sender
                                .lock()
                                .send_event(GameUserEvent::Exit(exit_code as _))
                                .log_warn();
                        }
                    }),
                    done_init: AtomicBool::new(false),
                }
            }),
        };

        let inner = slf.inner.clone();
        update_sender
            .set_timeout(Self::TEST_TIMEOUT, move |_| inner.set_timeout_func())
            .context("unable to set test timeout")?;

        Ok(slf)
    }

    pub fn finish_init(&self) {
        self.inner.done_init.store(true, Ordering::Relaxed);
        let result = self.inner.root.result.lock();
        let exit_code = match *result {
            Some(Ok(_)) => TestExitCode::Complete,
            Some(Err(_)) => TestExitCode::Failed,
            None => return,
        };
        self.inner
            .event_sender
            .lock()
            .send_event(GameUserEvent::Exit(exit_code as _))
            .log_warn();
    }

    pub fn root(&self) -> &Arc<ParentTestNode> {
        &self.inner.root
    }
}

impl TestManagerInner {
    pub fn set_timeout_func(&self) {
        let result = self.root.result.lock();
        let exit_code = match *result {
            Some(Ok(_)) => TestExitCode::Complete,
            Some(Err(_)) => TestExitCode::Failed,
            None => TestExitCode::Timeout,
        };
        self.event_sender
            .lock()
            .send_event(GameUserEvent::Exit(exit_code as _))
            .log_warn();
    }
}

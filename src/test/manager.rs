use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;

use crate::{
    display::EventSender,
    events::GameUserEvent,
    exec::{dispatch::DispatchList, server::update},
    utils::{args::args, error::ResultExt, mpsc::Sender, mutex::Mutex},
};

use super::tree::ParentTestNode;

pub struct TestManager {
    pub root: Arc<ParentTestNode>,
    event_sender: Mutex<EventSender>,
    done_init: AtomicBool,
}

pub struct ArcTestManager {
    inner: Arc<TestManager>,
    // logs: HashMap<Cow<'static, str>, String>,
}

pub type RealArcTestManager = Option<ArcTestManager>;

enum TestExitCode {
    Complete = 0,
    Failed = 1,
    Timeout = 2,
}

pub fn new_test_manager(
    update_sender: &Sender<update::Message>,
    dispatch_list: &mut DispatchList,
    event_sender: &EventSender,
) -> anyhow::Result<RealArcTestManager> {
    args()
        .test
        .then(|| ArcTestManager::new(update_sender, dispatch_list, event_sender.clone()))
        .transpose()
}

impl ArcTestManager {
    const TEST_TIMEOUT: Duration = Duration::new(30, 0);
    pub fn new(
        update_sender: &Sender<update::Message>,
        dispatch_list: &mut DispatchList,
        event_sender: EventSender,
    ) -> anyhow::Result<Self> {
        let slf = Self {
            inner: Arc::<TestManager>::new_cyclic(|weak| {
                let weak = weak.clone();
                TestManager {
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
        let uid = dispatch_list.push(move |_| inner.set_timeout_func());
        update_sender
            .set_timeout(Self::TEST_TIMEOUT, uid)
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

impl TestManager {
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

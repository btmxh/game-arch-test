use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use winit::event_loop::EventLoopProxy;

use crate::{
    events::GameUserEvent,
    utils::{error::ResultExt, mutex::Mutex},
};

use self::tree::ParentTestNode;

pub mod assert;
pub mod result;
pub mod tree;

pub struct TestManager {
    pub root: Arc<ParentTestNode>,
    proxy: Mutex<EventLoopProxy<GameUserEvent>>,
    done_init: AtomicBool,
}

enum TestExitCode {
    Complete = 0,
    Failed = 1,
    Timeout = 2,
}

impl TestManager {
    pub fn new(proxy: EventLoopProxy<GameUserEvent>) -> Arc<Self> {
        Arc::<Self>::new_cyclic(|weak| {
            let weak = weak.clone();
            Self {
                proxy: Mutex::new(proxy),
                root: ParentTestNode::new_root("root", move |_, result| {
                    if let Some(slf) = weak.upgrade() {
                        if slf.done_init.load(Ordering::Relaxed) {
                            return;
                        }

                        let exit_code = if result.is_ok() {
                            TestExitCode::Complete
                        } else {
                            TestExitCode::Failed
                        };
                        tracing::info!("all test finished, result of root test is {:?}", result);
                        slf.proxy
                            .lock()
                            .send_event(GameUserEvent::Exit(exit_code as _))
                            .log_warn();
                    }
                }),
                done_init: AtomicBool::new(false),
            }
        })
    }

    pub fn set_timeout_func(&self) {
        let result = self.root.result.lock();
        let exit_code = match *result {
            Some(Ok(_)) => TestExitCode::Complete,
            Some(Err(_)) => TestExitCode::Failed,
            None => TestExitCode::Timeout,
        };
        self.proxy
            .lock()
            .send_event(GameUserEvent::Exit(exit_code as _))
            .log_warn();
    }

    pub fn finish_init(&self) {
        self.done_init.store(true, Ordering::Relaxed);
        let result = self.root.result.lock();
        let exit_code = match *result {
            Some(Ok(_)) => TestExitCode::Complete,
            Some(Err(_)) => TestExitCode::Failed,
            None => return,
        };
        self.proxy
            .lock()
            .send_event(GameUserEvent::Exit(exit_code as _))
            .log_warn();
    }
}

use std::sync::Arc;

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
                    let exit_code = if result.is_ok() {
                        TestExitCode::Complete
                    } else {
                        TestExitCode::Failed
                    };
                    tracing::info!("all test finished, result of root test is {:?}", result);
                    if let Some(slf) = weak.upgrade() {
                        slf.proxy
                            .lock()
                            .send_event(GameUserEvent::Exit(exit_code as _))
                            .log_warn();
                    }
                }),
            }
        })
    }

    pub fn set_timeout_func(self: Arc<Self>) {
        self.proxy
            .lock()
            .send_event(GameUserEvent::Exit(TestExitCode::Timeout as _))
            .log_warn();
    }
}

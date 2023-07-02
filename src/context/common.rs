use std::sync::Arc;

use crate::exec::task::TaskExecutor;

pub struct CommonContext {
    pub task_executor: TaskExecutor,
}

pub type SharedCommonContext = Arc<CommonContext>;

impl CommonContext {
    pub fn new() -> SharedCommonContext {
        Arc::new(Self {
            task_executor: TaskExecutor::new(),
        })
    }
}

#[test]
fn test_send_sync() {
    use crate::assert_send;
    use crate::assert_sync;
    assert_sync!(CommonContext);
    assert_send!(SharedCommonContext);
    assert_sync!(SharedCommonContext);
}

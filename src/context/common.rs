use std::{num::NonZeroU32, sync::Arc};

use anyhow::Context;
use winit::{dpi::PhysicalSize, event_loop::EventLoopWindowTarget, window::WindowId};

use crate::{display::Display, exec::task::TaskExecutor};

use super::event::{EventContext, EventDispatchContext};

pub struct CommonContext {
    pub task_executor: TaskExecutor,
    pub display: Display,
}

pub type SharedCommonContext = Arc<CommonContext>;

impl CommonContext {
    pub fn new<T>(event_loop: &EventLoopWindowTarget<T>) -> anyhow::Result<SharedCommonContext> {
        let display = Display::new(event_loop, PhysicalSize::new(1280, 720), "hello")
            .context("Unable to create display")?;
        Ok(Arc::new(Self {
            task_executor: TaskExecutor::new(),
            display,
        }))
    }
}

pub trait HasCommonContext {
    fn common(&self) -> &SharedCommonContext;

    fn check_window_id(&self, id: &WindowId) -> bool {
        return *id == self.common().display.get_window_id();
    }

    fn non_zero_size(&self) -> anyhow::Result<PhysicalSize<NonZeroU32>> {
        let size = self.common().display.get_size();
        Ok(PhysicalSize {
            width: NonZeroU32::new(size.width).context("display width is 0")?,
            height: NonZeroU32::new(size.height).context("display height is 0")?,
        })
    }
}

impl HasCommonContext for SharedCommonContext {
    fn common(&self) -> &SharedCommonContext {
        self
    }
}

impl HasCommonContext for EventContext {
    fn common(&self) -> &SharedCommonContext {
        &self.common
    }
}

impl<'a> HasCommonContext for EventDispatchContext<'a> {
    fn common(&self) -> &SharedCommonContext {
        self.event.common()
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

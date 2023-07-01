use super::{draw::GraphicsContext, event::EventContext};

pub struct ExecutorInitArgs;
pub struct InitContext<'a> {
    pub executor_args: ExecutorInitArgs,
    pub event: &'a mut EventContext,
    pub graphics: &'a mut GraphicsContext,
}

impl<'a> InitContext<'a> {
    pub fn new(event: &'a mut EventContext, graphics: &'a mut GraphicsContext) -> Self {
        Self {
            event,
            graphics,
            executor_args: ExecutorInitArgs,
        }
    }
}

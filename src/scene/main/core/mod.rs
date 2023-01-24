use anyhow::Context;

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

use self::redraw::Redraw;

pub mod redraw;

pub struct Core {
    redraw: Redraw,
}

impl Core {
    pub fn new(
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            redraw: Redraw::new(executor, main_ctx).context("unable to initialize redraw scene")?,
        })
    }
    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        self.redraw.handle_event(executor, main_ctx, event)
    }
}

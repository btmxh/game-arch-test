use anyhow::Context;

use crate::{events::GameEvent, exec::main_ctx::MainContext};

use self::redraw::Redraw;

pub mod redraw;

pub struct Core {
    redraw: Redraw,
}

impl Core {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self {
            redraw: Redraw::new(main_ctx).context("unable to initialize redraw scene")?,
        })
    }
    pub fn handle_event(
        &mut self,

        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        self.redraw.handle_event(main_ctx, event)
    }
}

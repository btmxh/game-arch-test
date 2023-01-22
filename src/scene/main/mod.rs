use anyhow::Context;

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

use self::bg::Background;

pub mod bg;

pub struct EventRoot {
    background: Background,
}

impl EventRoot {
    pub fn new(
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            background: Background::new(executor, main_ctx)
                .context("unable to initialize background scene")?,
        })
    }

    #[allow(clippy::nonminimal_bool)]
    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: GameEvent,
    ) -> anyhow::Result<()> {
        let _ = self.background.handle_event(executor, main_ctx, &event)?
            // add other scenes here
            || false;
        Ok(())
    }
}

use anyhow::Context;

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

use self::{bg::Background, core::Core, handle_resize::HandleResize, utility::Utility};

pub mod bg;
pub mod core;
pub mod handle_resize;
pub mod utility;

pub struct EventRoot {
    handle_resize: Option<HandleResize>,
    core: Core,
    background: Background,
    utility: Utility,
}

impl EventRoot {
    pub fn new(
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            handle_resize: Some(
                HandleResize::new(executor, main_ctx)
                    .context("unable to initialize handle resize scene")?,
            ),
            core: Core::new(executor, main_ctx)
                .context("unable to initialize handle core scene")?,
            background: Background::new(executor, main_ctx)
                .context("unable to initialize background scene")?,
            utility: Utility::new(executor, main_ctx)
                .context("unable to initialize utility scene")?,
        })
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: GameEvent,
    ) -> anyhow::Result<()> {
        let _ = {
            if let Some(mut handle_resize) = self.handle_resize.take() {
                let result = handle_resize.handle_event(executor, main_ctx, self, &event)?;
                self.handle_resize = Some(handle_resize);
                result
            } else {
                false
            }
        } || self.core.handle_event(executor, main_ctx, &event)?
            || self.background.handle_event(executor, main_ctx, &event)?
            || self.utility.handle_event(executor, main_ctx, &event)?;
        Ok(())
    }
}

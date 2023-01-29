use anyhow::Context;

use crate::{
    events::GameEvent,
    exec::main_ctx::MainContext,
    ui::{self, scenes::UIEventScene},
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
    ui: ui::scenes::UIEventScene,
}

impl EventRoot {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self {
            handle_resize: Some(
                HandleResize::new(main_ctx).context("unable to initialize handle resize scene")?,
            ),
            core: Core::new(main_ctx).context("unable to initialize handle core scene")?,
            background: Background::new(main_ctx)
                .context("unable to initialize background scene")?,
            utility: Utility::new(main_ctx).context("unable to initialize utility scene")?,
            ui: UIEventScene::new(main_ctx),
        })
    }

    pub fn handle_event(
        &mut self,

        main_ctx: &mut MainContext,
        event: GameEvent,
    ) -> anyhow::Result<()> {
        let _ = {
            if let Some(mut handle_resize) = self.handle_resize.take() {
                let result = handle_resize.handle_event(main_ctx, self, &event)?;
                self.handle_resize = Some(handle_resize);
                result
            } else {
                false
            }
        } || self.core.handle_event(main_ctx, &event)?
            || self.background.handle_event(main_ctx, &event)?
            || self.utility.handle_event(main_ctx, &event)?
            || self.ui.handle_event(main_ctx, &event)?;
        Ok(())
    }
}

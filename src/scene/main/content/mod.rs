use anyhow::Context;

use crate::{events::GameEvent, exec::main_ctx::MainContext, utils::error::ResultExt};

use self::bg::Background;

pub mod bg;

pub struct Content {
    background: Background,
}

impl Content {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self {
            background: Background::new(main_ctx)
                .context("unable to initialize background scene")?,
        })
    }

    pub fn handle_event(&mut self, main_ctx: &mut MainContext, event: &GameEvent) -> bool {
        self.background
            .handle_event(main_ctx, event)
            .log_error()
            .unwrap_or_default()
    }
}

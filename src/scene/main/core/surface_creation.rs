use std::ops::Not;

use anyhow::Context;

use crate::{
    context::event::{EventDispatchContext, Executable},
    events::GameEvent,
    utils::{args::args, error::ResultExt},
};

pub struct Scene;
impl Scene {
    pub fn handle_event<'a>(
        &self,
        context: &mut EventDispatchContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        if !args().headless {
            return Some(event);
        }

        match &event {
            GameEvent::Resumed => {
                let execution_result = context
                    .execute_draw_sync(|context| {
                        context
                            .graphics
                            .create_surface()
                            .context("Unable to create render surface")
                    })
                    .context("Unable to execute render surface recreation")
                    .log_error()?;
                if let Err(err) = execution_result {
                    context.event.event_sender.exit_with_error(err);
                }
            }

            GameEvent::Suspended => {
                let execution_result = context
                    .execute_draw_sync(|context| context.graphics.destroy_surface())
                    .context("Unable to execute render surface recreation");
                if let Err(err) = execution_result {
                    context.event.event_sender.exit_with_error(err);
                }
            }

            _ => {}
        };

        Some(event)
    }

    pub fn new() -> Option<Self> {
        args().headless.not().then_some(Self)
    }
}

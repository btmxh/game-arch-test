use anyhow::Context;
use winit::event::Event;

use crate::{
    context::{
        common::HasCommonContext,
        event::{EventDispatchContext, Executable},
    },
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
        match event {
            Event::RedrawRequested(window_id) if context.check_window_id(&window_id) => {
                if args().block_event_loop {
                    // somewhat hacky way of waiting a buffer swap
                    if context.executor.main_runner.base.container.draw.is_some() {
                        context
                            .executor
                            .main_runner
                            .base
                            .run_single(true)
                            .context("error executing main runner while redrawing")
                            .log_warn();
                    } else {
                        context
                            .execute_draw_sync(|context| context.graphics.draw(context.root_scene))
                            .and_then(|x| x) // flatten()
                            .context("error triggering redraw in draw server")
                            .log_warn();
                    }
                }
                None
            }

            event => Some(event),
        }
    }
}

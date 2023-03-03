use anyhow::Context;
use winit::event::Event;

use crate::{
    events::GameEvent,
    exec::main_ctx::MainContext,
    scene::main::RootScene,
    utils::{args::args, error::ResultExt},
};

pub fn handle_event<'a>(
    ctx: &mut MainContext,
    _: &RootScene,
    event: GameEvent<'a>,
) -> Option<GameEvent<'a>> {
    match event {
        Event::RedrawRequested(window_id) if ctx.display.get_window_id() == window_id => {
            if args().block_event_loop {
                // somewhat hacky way of waiting a buffer swap
                if ctx.executor.main_runner.base.container.draw.is_some() {
                    ctx.executor
                        .main_runner
                        .base
                        .run_single(true)
                        .context("error executing main runner while redrawing")
                        .log_warn();
                } else {
                    ctx.execute_draw_sync(|context, root_scene| {
                        context.draw(root_scene, false, 0.0)
                    })
                    .context("error triggering redraw in draw server")
                    .log_warn();
                }
            }
            None
        }

        event => Some(event),
    }
}

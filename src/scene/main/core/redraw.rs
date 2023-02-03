use std::sync::Arc;

use anyhow::Context;
use winit::event::Event;

use crate::{
    events::GameEvent,
    exec::main_ctx::MainContext,
    scene::{main::EventRoot, Scene},
    utils::{args::args, error::ResultExt},
};

pub struct Redraw;

impl Scene for Redraw {
    fn handle_event<'a>(
        self: Arc<Self>,
        ctx: &mut MainContext,
        _: &EventRoot,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match event {
            Event::RedrawRequested(window_id) if ctx.display.get_window_id() == window_id => {
                Self::redraw(ctx)
                    .context("unable to send redraw request")
                    .log_warn();
                None
            }

            event => Some(event),
        }
    }
}

impl Redraw {
    fn redraw(main_ctx: &mut MainContext) -> anyhow::Result<()> {
        if args().block_event_loop {
            // somewhat hacky way of waiting a buffer swap
            if main_ctx.executor.main_runner.base.container.draw.is_some() {
                main_ctx
                    .executor
                    .main_runner
                    .base
                    .run_single()
                    .expect("error running main runner");
            } else {
                main_ctx.execute_draw_sync(|_, _| Ok(()))?;
                main_ctx.execute_draw_sync(|_, _| Ok(()))?;
            }
        }

        Ok(())
    }
}

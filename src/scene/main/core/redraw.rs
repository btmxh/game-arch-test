use winit::event::Event;

use crate::{events::GameEvent, exec::main_ctx::MainContext, utils::args::args};

pub struct Redraw;

impl Redraw {
    pub fn new(_: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

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

    pub fn handle_event(
        &mut self,

        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        match event {
            Event::RedrawRequested(window_id) if main_ctx.display.get_window_id() == *window_id => {
                Self::redraw(main_ctx)?;
            }

            _ => {}
        }

        Ok(false)
    }
}

use winit::event::Event;

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
    utils::args::args,
};

pub struct Redraw;

impl Redraw {
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn redraw(executor: &mut GameServerExecutor, main_ctx: &mut MainContext) -> anyhow::Result<()> {
        if args().block_event_loop {
            // somewhat hacky way of waiting a buffer swap
            if executor.main_runner.base.container.draw.is_some() {
                executor
                    .main_runner
                    .base
                    .run_single()
                    .expect("error running main runner");
            } else {
                executor.execute_draw_sync(&mut main_ctx.channels.draw, |_, _| Ok(()))?;
                executor.execute_draw_sync(&mut main_ctx.channels.draw, |_, _| Ok(()))?;
            }
        }

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        match event {
            Event::RedrawRequested(window_id) if main_ctx.display.get_window_id() == *window_id => {
                Self::redraw(executor, main_ctx)?;
            }

            _ => {}
        }

        Ok(false)
    }
}

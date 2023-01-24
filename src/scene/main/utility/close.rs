use anyhow::Context;
use winit::event::{Event, WindowEvent};

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

pub struct Close;

impl Close {
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn handle_event(
        &mut self,
        _executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if main_ctx.display.get_window_id() == *window_id => {
                main_ctx
                    .event_loop_proxy
                    .send_event(GameUserEvent::Exit)
                    .map_err(|e| anyhow::format_err!("{}", e))
                    .context("unable to send event to event loop")?;
            }

            _ => {}
        }

        Ok(false)
    }
}

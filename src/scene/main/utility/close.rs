use anyhow::Context;
use winit::event::{Event, WindowEvent};

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::main_ctx::MainContext,
    scene::{main::EventRoot, Scene},
    utils::error::ResultExt,
};

pub struct Close;

impl Scene for Close {
    fn handle_event<'a>(
        self: std::sync::Arc<Self>,
        ctx: &mut MainContext,
        _: &EventRoot,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match &event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if ctx.display.get_window_id() == *window_id => {
                ctx.event_loop_proxy
                    .send_event(GameUserEvent::Exit)
                    .map_err(|e| anyhow::format_err!("{}", e))
                    .context("unable to send event to event loop")
                    .log_warn();
            }

            _ => {}
        }

        Some(event)
    }
}

use anyhow::Context;
use winit::event::{Event, WindowEvent};

use crate::{
    context::event::EventDispatchContext,
    events::{GameEvent, GameUserEvent},
    utils::error::ResultExt,
};

pub struct Scene;
impl Scene {
    pub fn handle_event<'a>(
        &self,
        context: &mut EventDispatchContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match &event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if context.event.display.get_window_id() == *window_id => {
                context
                    .event
                    .event_sender
                    .send_event(GameUserEvent::Exit(0))
                    .map_err(|e| anyhow::format_err!("{}", e))
                    .context("unable to send event to event loop")
                    .log_warn();
            }

            _ => {}
        }

        Some(event)
    }
}

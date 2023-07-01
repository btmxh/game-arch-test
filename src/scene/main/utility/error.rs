use winit::event::Event;

use crate::{
    context::event::EventHandleContext,
    events::{GameEvent, GameUserEvent},
};

pub struct Scene;
impl Scene {
    pub fn handle_event<'a>(
        &self,
        _: &mut EventHandleContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match event {
            Event::UserEvent(GameUserEvent::Error(error)) => {
                tracing::error!("GameUserEvent::Error caught: {}", error);
                None
            }

            event => Some(event),
        }
    }
}

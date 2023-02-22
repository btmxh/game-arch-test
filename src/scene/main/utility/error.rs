use winit::event::Event;

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::main_ctx::MainContext,
    scene::main::RootScene,
};

pub fn handle_event<'a>(
    _: &mut MainContext,
    _: &RootScene,
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

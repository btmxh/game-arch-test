use winit::event::Event;

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::main_ctx::MainContext,
    scene::{main::RootScene, Scene},
};

pub struct Error;

impl Scene for Error {
    fn handle_event<'a>(
        self: std::sync::Arc<Self>,
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
}

impl Error {
    pub fn new(_: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn handle_event(&mut self, _: &mut MainContext, event: &GameEvent) -> anyhow::Result<bool> {
        Ok(if let Event::UserEvent(GameUserEvent::Error(e)) = event {
            tracing::error!("GameUserEvent::Error caught: {}", e);
            true
        } else {
            false
        })
    }
}

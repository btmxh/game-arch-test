use winit::event::Event;

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

pub struct Error;

impl Error {
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn handle_event(
        &mut self,
        _: &mut GameServerExecutor,
        _: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        Ok(if let Event::UserEvent(GameUserEvent::Error(e)) = event {
            tracing::error!("GameUserEvent::Error caught: {}", e);
            true
        } else {
            false
        })
    }
}

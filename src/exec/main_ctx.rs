use std::time::Duration;

use winit::{event::Event, event_loop::EventLoopProxy};

use crate::{
    display::Display,
    events::{GameEvent, GameUserEvent},
    graphics::wrappers::vertex_array::VertexArrayHandle,
    scene::main::EventRoot,
    utils::error::ResultExt,
};

use super::{
    dispatch::{DispatchId, DispatchList},
    executor::GameServerExecutor,
    server::ServerChannels,
    task::CancellationToken,
};

pub struct MainContext {
    pub dummy_vao: VertexArrayHandle,
    pub channels: ServerChannels,
    pub dispatch_list: DispatchList,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub display: Display,
}

impl MainContext {
    pub fn new(
        _executor: &mut GameServerExecutor,
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        mut channels: ServerChannels,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            dummy_vao: VertexArrayHandle::new(&mut channels.draw, "dummy vertex array")?,
            display,
            event_loop_proxy,
            dispatch_list: DispatchList::new(),
            channels,
        })
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        root_scene: &mut EventRoot,
        event: GameEvent<'_>,
    ) -> anyhow::Result<()> {
        match event {
            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                for (id, d) in self.dispatch_list.handle_dispatch_msg(msg).into_iter() {
                    d(self, executor, root_scene, id)?;
                }
            }

            Event::UserEvent(GameUserEvent::Execute(callback)) => {
                callback(self, executor, root_scene).log_error();
            }

            Event::UserEvent(GameUserEvent::Error(e)) => {
                tracing::error!("GameUserEvent::Error caught: {}", e);
            }

            event => {
                root_scene.handle_event(executor, self, event)?;
            }
        };
        Ok(())
    }

    pub fn set_timeout<F>(
        &mut self,
        timeout: Duration,
        callback: F,
    ) -> anyhow::Result<(DispatchId, CancellationToken)>
    where
        F: FnOnce(
                &mut MainContext,
                &mut GameServerExecutor,
                &mut EventRoot,
                DispatchId,
            ) -> anyhow::Result<()>
            + 'static,
    {
        let cancel_token = CancellationToken::new();
        let id = self.dispatch_list.push(callback, cancel_token.clone());
        self.channels.update.set_timeout(timeout, id)?;
        Ok((id, cancel_token))
    }
}

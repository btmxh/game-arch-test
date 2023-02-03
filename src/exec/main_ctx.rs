use std::time::{Duration, Instant};

use anyhow::bail;
use tracing_appender::non_blocking::WorkerGuard;
use winit::{
    event::Event,
    event_loop::{EventLoop, EventLoopProxy},
};

use crate::{
    display::Display,
    events::{GameEvent, GameUserEvent},
    graphics::{context::DrawContext, wrappers::vertex_array::VertexArrayHandle},
    scene::main::EventRoot,
    utils::error::ResultExt,
};

use super::{
    dispatch::{DispatchId, DispatchList},
    executor::GameServerExecutor,
    server::{draw, GameServerChannel, GameServerSendChannel, ServerChannels},
    task::{CancellationToken, TaskExecutor},
};

pub struct MainContext {
    pub executor: GameServerExecutor,
    pub dummy_vao: VertexArrayHandle,
    pub task_executor: TaskExecutor,
    pub channels: ServerChannels,
    pub dispatch_list: DispatchList,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub display: Display,
}

impl MainContext {
    pub fn new(
        executor: GameServerExecutor,
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        mut channels: ServerChannels,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            executor,
            dummy_vao: VertexArrayHandle::new(&mut channels.draw, "dummy vertex array")?,
            task_executor: TaskExecutor::new(),
            display,
            event_loop_proxy,
            dispatch_list: DispatchList::new(),
            channels,
        })
    }

    pub fn handle_event(
        &mut self,

        root_scene: &mut EventRoot,
        event: GameEvent<'_>,
    ) -> anyhow::Result<()> {
        match event {
            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                for (id, d) in self.dispatch_list.handle_dispatch_msg(msg).into_iter() {
                    d(self, root_scene, id)?;
                }
            }

            Event::UserEvent(GameUserEvent::Execute(callback)) => {
                callback(self, root_scene).log_error();
            }

            Event::UserEvent(GameUserEvent::Error(e)) => {
                tracing::error!("GameUserEvent::Error caught: {}", e);
            }

            event => {
                root_scene.handle_event(self, event);
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
        F: FnOnce(&mut MainContext, &mut EventRoot, DispatchId) -> anyhow::Result<()> + 'static,
    {
        let cancel_token = CancellationToken::new();
        let id = self.dispatch_list.push(callback, cancel_token.clone());
        self.channels.update.set_timeout(timeout, id)?;
        Ok((id, cancel_token))
    }

    pub fn execute_blocking_task<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.task_executor.execute(f)
    }

    #[allow(irrefutable_let_patterns)]
    pub fn execute_draw_sync<F, R>(&mut self, callback: F) -> anyhow::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut DrawContext, &mut Option<EventRoot>) -> anyhow::Result<R> + Send + 'static,
    {
        if let Some(server) = self.executor.main_runner.base.container.draw.as_mut() {
            callback(&mut server.context, &mut server.root_scene)
        } else {
            self.channels
                .draw
                .send(draw::RecvMsg::ExecuteSync(Box::new(
                    move |context, root_scene| Box::new(callback(context, root_scene)),
                )))?;
            if let draw::SendMsg::ExecuteSyncReturn(result) = self.channels.draw.recv()? {
                Ok(result
                    .downcast::<anyhow::Result<R>>()
                    .map(|bx| *bx)
                    .map_err(|_| {
                        anyhow::format_err!("unable to downcast callback return value")
                    })??)
            } else {
                bail!("unexpected response message from thread");
            }
        }
    }

    pub fn run(
        mut self,
        event_loop: EventLoop<GameUserEvent>,
        mut root_scene: EventRoot,
        guard: Option<WorkerGuard>,
    ) -> ! {
        use winit::event_loop::ControlFlow;
        event_loop.run(move |event, _target, control_flow| {
            // guarantee drop order
            fn unused<T>(_: &T) {}
            unused(&root_scene);
            unused(&self);
            unused(&guard);
            match event {
                Event::MainEventsCleared => {
                    self.executor
                        .main_runner
                        .base
                        .run_single()
                        .expect("error running main runner");
                }

                Event::UserEvent(GameUserEvent::Exit) => control_flow.set_exit(),

                event => self
                    .handle_event(&mut root_scene, event)
                    .expect("error handling events"),
            }

            match *control_flow {
                ControlFlow::ExitWithCode(_) => {
                    self.executor.stop();
                }

                _ => {
                    *control_flow = if self.executor.main_runner.base.container.is_empty() {
                        ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100))
                    } else {
                        ControlFlow::Poll
                    }
                }
            };
        })
    }
}

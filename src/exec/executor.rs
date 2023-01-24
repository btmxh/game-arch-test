use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use tracing_appender::non_blocking::WorkerGuard;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    events::GameUserEvent,
    graphics::context::DrawContext,
    scene::{draw::DrawRoot, main::EventRoot},
    utils::error::ResultExt,
};

use super::{
    main_ctx::MainContext,
    runner::{
        container::ServerContainer, MainRunner, Runner, RunnerId, ServerMover, ThreadRunnerHandle,
        MAIN_RUNNER_ID,
    },
    server::{
        audio, draw, update, GameServerChannel, GameServerSendChannel, SendGameServer, ServerKind,
    },
    task::TaskExecutor,
    NUM_GAME_LOOPS,
};

pub struct GameServerExecutor {
    pub main_runner: MainRunner,
    thread_runners: [Option<ThreadRunnerHandle>; NUM_GAME_LOOPS],
    task_executor: TaskExecutor,
}

impl GameServerExecutor {
    fn move_server_from(
        &mut self,
        from: RunnerId,
        kind: ServerKind,
    ) -> anyhow::Result<SendGameServer> {
        match from {
            MAIN_RUNNER_ID => self.main_runner.take_server_check(kind),
            _ => self.thread_runners[usize::from(from)]
                .as_mut()
                .ok_or_else(|| anyhow::format_err!("runner {} hasn't been constructed", from))?
                .take_server_check(kind),
        }
    }

    fn move_server_to(&mut self, to: RunnerId, server: SendGameServer) -> anyhow::Result<()> {
        match to {
            MAIN_RUNNER_ID => self.main_runner.emplace_server_check(server),
            _ => self.thread_runners[usize::from(to)]
                .get_or_insert_with(|| ThreadRunnerHandle::new(to))
                .emplace_server_check(server),
        }
    }

    pub fn move_server(
        &mut self,
        from: RunnerId,
        to: RunnerId,
        kind: ServerKind,
    ) -> anyhow::Result<()> {
        let server = self
            .move_server_from(from, kind)
            .with_context(|| format!("unable to move {:?} server from runner id {}", kind, from))?;
        self.move_server_to(to, server)
            .with_context(|| format!("unable to move {:?} server to runner id {}", kind, to))
    }

    pub fn set_frequency(&mut self, id: RunnerId, frequency: f64) -> anyhow::Result<()> {
        match id {
            MAIN_RUNNER_ID => self.main_runner.base.frequency = frequency,
            _ => self.thread_runners[usize::from(id)]
                .as_mut()
                .ok_or_else(|| anyhow::format_err!("runner {} hasn't been constructed", id))?
                .set_frequency(frequency)?,
        }
        Ok(())
    }

    pub fn new(
        audio: audio::Server,
        draw: draw::SendServer,
        update: update::Server,
    ) -> anyhow::Result<Self> {
        let mut container = ServerContainer {
            audio: Some(audio),
            draw: None,
            update: Some(update),
        };
        container.emplace_server_check(SendGameServer::Draw(Box::new(draw)))?;
        Ok(Self {
            thread_runners: Default::default(),
            main_runner: MainRunner {
                base: Runner {
                    container,
                    ..Default::default()
                },
            },
            task_executor: TaskExecutor::new(),
        })
    }

    pub fn stop(&mut self) {
        for runner in self.thread_runners.iter_mut() {
            if let Some(runner) = runner.take() {
                runner
                    .stop()
                    .context("error stopping runner thread")
                    .log_error();
                if runner.join() {
                    tracing::error!("runner thread panicked");
                }
            }
        }
    }

    pub fn run(
        mut self,
        event_loop: EventLoop<GameUserEvent>,
        mut main_ctx: MainContext,
        mut root_scene: EventRoot,
        guard: Option<WorkerGuard>,
    ) -> ! {
        event_loop.run(move |event, _target, control_flow| {
            // guarantee drop order
            fn unused<T>(_: &T) {}
            unused(&guard);
            unused(&root_scene);
            unused(&main_ctx);
            unused(&self);
            match event {
                Event::MainEventsCleared => {
                    self.main_runner
                        .base
                        .run_single()
                        .expect("error running main runner");
                }

                Event::UserEvent(GameUserEvent::Exit) => control_flow.set_exit(),

                event => main_ctx
                    .handle_event(&mut self, &mut root_scene, event)
                    .expect("error handling events"),
            }

            match *control_flow {
                ControlFlow::ExitWithCode(_) => {
                    self.stop();
                }

                _ => {
                    *control_flow = if self.main_runner.base.container.is_empty() {
                        ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100))
                    } else {
                        ControlFlow::Poll
                    }
                }
            };
        })
    }

    #[allow(irrefutable_let_patterns)]
    pub fn execute_draw_sync<F, R>(
        &mut self,
        channel: &mut draw::ServerChannel,
        callback: F,
    ) -> anyhow::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut DrawContext, &mut DrawRoot) -> anyhow::Result<R> + Send + 'static,
    {
        if let Some(server) = self.main_runner.base.container.draw.as_mut() {
            callback(&mut server.context, &mut server.root_scene)
        } else {
            channel.send(draw::RecvMsg::ExecuteSync(Box::new(
                move |context, root_scene| Box::new(callback(context, root_scene)),
            )))?;
            if let draw::SendMsg::ExecuteSyncReturn(result) = channel.recv()? {
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

    pub fn execute_draw_event<F, R>(
        channel: &impl GameServerSendChannel<draw::RecvMsg>,
        callback: F,
    ) -> anyhow::Result<()>
    where
        R: IntoIterator<Item = GameUserEvent> + Send + 'static,
        F: FnOnce(&mut DrawContext, &mut DrawRoot) -> R + Send + 'static,
    {
        channel.send(draw::RecvMsg::ExecuteEvent(Box::new(
            move |context, root_scene| Box::new(callback(context, root_scene).into_iter()),
        )))
    }

    pub fn execute_blocking_task<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.task_executor.execute(f)
    }
}

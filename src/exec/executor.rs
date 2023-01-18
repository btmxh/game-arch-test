use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use futures::{executor::block_on, Future};
use tracing_appender::non_blocking::WorkerGuard;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

use crate::{events::GameUserEvent, utils::error::ResultExt};

use super::{
    main_ctx::MainContext,
    runner::{
        container::ServerContainer, MainRunner, Runner, RunnerId, ServerMover, ThreadRunnerHandle,
        MAIN_RUNNER_ID,
    },
    server::{
        audio, draw, update, GameServerChannel, GameServerSendChannel, SendGameServer, ServerKind,
    },
    task::{TaskExecutor, TaskHandle},
    NUM_GAME_LOOPS,
};

pub struct GameServerExecutor {
    main_runner: MainRunner,
    thread_runners: [Option<ThreadRunnerHandle>; NUM_GAME_LOOPS],
    task_executor: TaskExecutor,
}

impl GameServerExecutor {
    async fn move_server_from(
        &mut self,
        from: RunnerId,
        kind: ServerKind,
    ) -> anyhow::Result<Box<dyn SendGameServer>> {
        match from {
            MAIN_RUNNER_ID => self.main_runner.take_server_check(kind).await,
            _ => {
                self.thread_runners[usize::from(from)]
                    .as_mut()
                    .ok_or_else(|| anyhow::format_err!("runner {} hasn't been constructed", from))?
                    .take_server_check(kind)
                    .await
            }
        }
    }

    async fn move_server_to(
        &mut self,
        to: RunnerId,
        server: Box<dyn SendGameServer>,
    ) -> anyhow::Result<()> {
        match to {
            MAIN_RUNNER_ID => self.main_runner.emplace_server_check(server).await,
            _ => {
                self.thread_runners[usize::from(to)]
                    .get_or_insert_with(|| ThreadRunnerHandle::new(to))
                    .emplace_server_check(server)
                    .await
            }
        }
    }

    pub async fn move_server(
        &mut self,
        from: RunnerId,
        to: RunnerId,
        kind: ServerKind,
    ) -> anyhow::Result<()> {
        let server = self
            .move_server_from(from, kind)
            .await
            .with_context(|| format!("unable to move {:?} server from runner id {}", kind, from))?;
        self.move_server_to(to, server)
            .await
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

    pub async fn new(
        audio: audio::Server,
        draw: draw::SendServer,
        update: update::Server,
    ) -> anyhow::Result<Self> {
        let mut container = ServerContainer {
            audio: Some(audio),
            draw: None,
            update: Some(update),
        };
        container.emplace_server_check(Box::new(draw)).await?;
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

    pub async fn stop(&mut self) {
        for runner in self.thread_runners.iter_mut() {
            if let Some(runner) = runner.take() {
                runner
                    .stop()
                    .context("error stopping runner thread")
                    .log_error();
                if runner.join().await {
                    tracing::error!("runner thread panicked");
                }
            }
        }
    }

    pub fn run(
        mut self,
        event_loop: EventLoop<GameUserEvent>,
        mut main_ctx: MainContext,
        guard: Option<WorkerGuard>,
    ) -> ! {
        event_loop.run(move |event, _target, control_flow| {
            // guarantee drop order
            fn unused<T>(_: &T) {}
            unused(&guard);
            unused(&main_ctx);
            unused(&self);
            block_on(async {
                match event {
                    Event::MainEventsCleared => {
                        self.main_runner
                            .base
                            .run_single()
                            .await
                            .expect("error running main runner");
                    }

                    Event::UserEvent(GameUserEvent::Exit) => control_flow.set_exit(),

                    event => main_ctx
                        .handle_event(&mut self, event)
                        .await
                        .expect("error handling events"),
                }

                match *control_flow {
                    ControlFlow::ExitWithCode(_) => {
                        self.stop().await;
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
        })
    }

    #[allow(irrefutable_let_patterns)]
    pub async fn execute_draw_sync<F, R>(
        &mut self,
        channel: &mut draw::ServerChannel,
        callback: F,
    ) -> anyhow::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut draw::Server) -> anyhow::Result<R> + Send + 'static,
    {
        tracing::info!("a7");
        if let Some(server) = self.main_runner.base.container.draw.as_mut() {
            callback(server)
        } else {
            channel.send(draw::RecvMsg::ExecuteSync(Box::new(move |server| {
                Box::new(callback(server))
            })))?;
            let msg = channel.receiver().recv().await.unwrap();
            // if let draw::SendMsg::ExecuteSyncReturn(result) = channel.recv().await? {
            if let draw::SendMsg::ExecuteSyncReturn(result) = msg {
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
        F: FnOnce(&mut draw::Server) -> R + Send + 'static,
    {
        channel.send(draw::RecvMsg::ExecuteEvent(Box::new(move |server| {
            Box::new(callback(server).into_iter())
        })))
    }

    pub fn execute_blocking_task<F>(&mut self, f: F) -> TaskHandle
    where
        F: FnOnce() -> anyhow::Result<()> + Send + 'static,
    {
        self.task_executor.execute(move || {
            if let Err(e) = f() {
                tracing::error!("error while running blocking task: {}", e);
            }
        })
    }
}

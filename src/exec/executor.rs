use std::{
    any::Any,
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};

use crate::{
    events::{GameEvent, GameUserEvent},
    utils::error::ResultExt,
};

use super::{
    dispatch::ReturnMechanism,
    runner::{
        container::ServerContainer, MainRunner, Runner, RunnerId, ServerMover, ThreadRunnerHandle,
        MAIN_RUNNER_ID,
    },
    server::{
        audio,
        draw::{self, ExecuteCallbackReturnType},
        update, GameServerChannel, SendGameServer, ServerKind,
    },
    NUM_GAME_LOOPS,
};

pub struct GameServerExecutor {
    main_runner: MainRunner,
    thread_runners: [Option<ThreadRunnerHandle>; NUM_GAME_LOOPS],
    proxy: EventLoopProxy<GameUserEvent>,
}

struct ExecutorAndHandler<F> {
    // ensure drop order
    handler: F,
    executor: GameServerExecutor,
}

impl GameServerExecutor {
    fn move_server_from(
        &mut self,
        from: RunnerId,
        kind: ServerKind,
    ) -> anyhow::Result<Box<dyn SendGameServer>> {
        match from {
            MAIN_RUNNER_ID => self.main_runner.take_server_check(kind),
            _ => self.thread_runners[usize::from(from)]
                .as_mut()
                .ok_or_else(|| anyhow::format_err!("runner {} hasn't been constructed", from))?
                .take_server_check(kind),
        }
    }

    fn move_server_to(
        &mut self,
        to: RunnerId,
        server: Box<dyn SendGameServer>,
    ) -> anyhow::Result<()> {
        match to {
            MAIN_RUNNER_ID => self.main_runner.emplace_server_check(server),
            _ => self.thread_runners[usize::from(to)]
                .get_or_insert_with(Default::default)
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
        proxy: EventLoopProxy<GameUserEvent>,
        audio: audio::Server,
        draw: draw::SendServer,
        update: update::Server,
    ) -> anyhow::Result<Self> {
        let mut container = ServerContainer {
            audio: Some(audio),
            draw: None,
            update: Some(update),
        };
        container.emplace_server_check(Box::new(draw))?;
        Ok(Self {
            thread_runners: Default::default(),
            main_runner: MainRunner {
                base: Runner {
                    container,
                    ..Default::default()
                },
            },
            proxy,
        })
    }

    pub fn stop(&mut self) {
        self.thread_runners
            .iter_mut()
            .filter_map(|r| r.as_mut())
            .for_each(|r| {
                r.stop().context("error stopping runner thread").log_error();
            });
        self.thread_runners
            .iter_mut()
            .filter_map(|r| r.take())
            .for_each(|r| {
                if r.join() {
                    tracing::error!("runner thread panicked");
                }
            })
    }

    pub fn run<F>(self, event_loop: EventLoop<GameUserEvent>, event_handler: F) -> !
    where
        F: FnMut(&mut Self, GameEvent) -> anyhow::Result<()> + 'static,
    {
        let mut enh = ExecutorAndHandler {
            executor: self,
            handler: event_handler,
        };
        event_loop.run(move |event, _target, control_flow| {
            match event {
                Event::MainEventsCleared => {
                    enh.executor
                        .main_runner
                        .base
                        .run_single()
                        .expect("error running main runner");
                }

                Event::UserEvent(GameUserEvent::Exit) => control_flow.set_exit(),

                event => (enh.handler)(&mut enh.executor, event).expect("error handling events"),
            }

            match *control_flow {
                ControlFlow::ExitWithCode(_) => {
                    enh.executor.stop();
                }

                _ => {
                    *control_flow = if enh.executor.main_runner.base.container.is_empty() {
                        ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100))
                    } else {
                        ControlFlow::Poll
                    }
                }
            };
        })
    }

    #[allow(irrefutable_let_patterns)]
    pub async fn execute_draw<F>(
        &mut self,
        channel: &mut draw::ServerChannel,
        ret: Option<ReturnMechanism>,
        callback: F,
    ) -> anyhow::Result<Option<Box<dyn Any + Send + Sync>>>
    where
        F: FnOnce(&mut draw::Server) -> ExecuteCallbackReturnType + Send + 'static,
    {
        if let Some(server) = self.main_runner.base.container.draw.as_mut() {
            let result = callback(server);
            match ret {
                Some(ReturnMechanism::Event(id)) => {
                    self.proxy
                        .send_event(GameUserEvent::ExecuteReturn(result, id))
                        .context("unable to send ExecuteReturn event for Event return mechanism")?;
                }
                Some(ReturnMechanism::Sync) => return result.map(Some),
                _ => {}
            }
        } else {
            channel.send(draw::RecvMsg::Execute(Box::new(callback), ret))?;
            if let Some(ReturnMechanism::Sync) = ret {
                if let draw::SendMsg::ExecuteReturn(result) = channel.recv().await? {
                    return result.map(Some);
                } else {
                    bail!("unexpected response message from thread");
                }
            }
        }

        Ok(None)
    }
}

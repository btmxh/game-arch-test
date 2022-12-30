use std::time::{Duration, Instant};

use anyhow::Context;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

use crate::utils::error::ResultExt;

use super::{
    runner::{
        container::ServerContainer, MainRunner, Runner, RunnerId, ServerMover, ThreadRunnerHandle,
        MAIN_RUNNER_ID,
    },
    server::{audio, draw, update, SendGameServer, ServerKind},
    NUM_GAME_LOOPS,
};

pub struct GameServerExecutor {
    main_runner: MainRunner,
    thread_runners: [Option<ThreadRunnerHandle>; NUM_GAME_LOOPS],
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
                    log::error!("runner thread panicked");
                }
            })
    }

    pub fn run<F>(mut self, event_loop: EventLoop<()>, mut event_handler: F) -> !
    where
        F: FnMut(Event<()>) -> anyhow::Result<()> + 'static,
    {
        event_loop.run(move |event, _target, control_flow| {
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

            match event {
                Event::MainEventsCleared => {
                    self.main_runner
                        .base
                        .run_single()
                        .expect("error running main runner");
                }

                Event::UserEvent(()) => control_flow.set_exit(),

                event => event_handler(event).expect("error handling events"),
            }
        })
    }
}

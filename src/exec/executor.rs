use anyhow::Context;

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
    pub main_runner: MainRunner,
    thread_runners: [Option<ThreadRunnerHandle>; NUM_GAME_LOOPS],
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
            .with_context(|| format!("unable to move {kind:?} server from runner id {from}"))?;
        self.move_server_to(to, server)
            .with_context(|| format!("unable to move {kind:?} server to runner id {to}"))
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
        draw: draw::Server,
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
}

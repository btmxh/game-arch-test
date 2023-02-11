use crate::exec::server::{audio, draw, update, GameServer, SendGameServer, ServerKind};

use super::ServerMover;

#[derive(Default)]
pub struct ServerContainer {
    pub audio: Option<audio::Server>,
    pub draw: Option<draw::Server>,
    pub update: Option<update::Server>,
}

impl ServerMover for ServerContainer {
    fn take_server(&mut self, kind: ServerKind) -> anyhow::Result<Option<SendGameServer>> {
        match kind {
            ServerKind::Audio => self.audio.take().map(|s| s.to_send()).transpose(),
            ServerKind::Draw => self.draw.take().map(|s| s.to_send()).transpose(),
            ServerKind::Update => self.update.take().map(|s| s.to_send()).transpose(),
        }
    }

    fn emplace_server(&mut self, server: SendGameServer) -> anyhow::Result<()> {
        match server {
            SendGameServer::Audio(server) => self.audio = Some(*server),
            SendGameServer::Draw(server) => self.draw = Some(server.to_nonsend()?),
            SendGameServer::Update(server) => self.update = Some(*server),
        }
        Ok(())
    }
}

impl ServerContainer {
    pub fn run_single(
        &mut self,
        is_main_runner: bool,
        runner_frequency: f64,
    ) -> anyhow::Result<()> {
        fn run<S: GameServer>(
            server: &mut Option<S>,
            single: bool,
            runner_frequency: f64,
        ) -> anyhow::Result<()> {
            if let Some(server) = server {
                server.run(single, runner_frequency)?;
            }
            Ok(())
        }

        let single = !is_main_runner
            && [
                self.audio.is_some(),
                self.draw.is_some(),
                self.update.is_some(),
            ]
            .into_iter()
            .filter(|b| *b)
            .count()
                <= 1;
        run(&mut self.audio, single, runner_frequency)?;
        run(&mut self.draw, single, runner_frequency)?;
        run(&mut self.update, single, runner_frequency)?;
        Ok(())
    }

    pub fn does_run(&self) -> bool {
        self.audio.is_some() || self.update.is_some() || self.draw.is_some()
    }
}

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
    pub fn run_single(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        fn run<S: GameServer>(server: &mut Option<S>, runner_frequency: f64) -> anyhow::Result<()> {
            if let Some(server) = server {
                if server.does_run() {
                    server.run(runner_frequency)?;
                }
            }
            Ok(())
        }

        run(&mut self.audio, runner_frequency)?;
        run(&mut self.draw, runner_frequency)?;
        run(&mut self.update, runner_frequency)?;
        Ok(())
    }

    pub fn does_run(&self) -> bool {
        self.audio
            .as_ref()
            .map(|s| s.does_run())
            .unwrap_or_default()
            || self.draw.as_ref().map(|s| s.does_run()).unwrap_or_default()
            || self
                .update
                .as_ref()
                .map(|s| s.does_run())
                .unwrap_or_default()
    }
}

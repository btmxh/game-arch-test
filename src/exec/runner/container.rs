use async_trait::async_trait;

use crate::exec::server::{audio, draw, update, GameServer, SendGameServer, ServerKind};

use super::ServerMover;

#[derive(Default)]
pub struct ServerContainer {
    pub audio: Option<audio::Server>,
    pub draw: Option<draw::Server>,
    pub update: Option<update::Server>,
}

#[async_trait(?Send)]
impl ServerMover for ServerContainer {
    async fn take_server(&mut self, kind: ServerKind) -> anyhow::Result<Option<Box<dyn SendGameServer>>> {
        match kind {
            ServerKind::Audio => self.audio.take().map(|s| s.to_send()).transpose(),
            ServerKind::Draw => self.draw.take().map(|s| s.to_send()).transpose(),
            ServerKind::Update => self.update.take().map(|s| s.to_send()).transpose(),
        }
    }

    fn emplace_server(&mut self, server: Box<dyn SendGameServer>) -> anyhow::Result<()> {
        match server.server_kind() {
            ServerKind::Audio => self.audio = Some(server.downcast_audio()?),
            ServerKind::Draw => self.draw = Some(server.downcast_draw()?),
            ServerKind::Update => self.update = Some(server.downcast_update()?),
        }
        Ok(())
    }
}

impl ServerContainer {
    pub async fn run_single(&mut self, runner_frequency: f64) -> anyhow::Result<()> {
        if let Some(server) = self.audio.as_mut() {
            server.run(runner_frequency).await?;
        }
        if let Some(server) = self.draw.as_mut() {
            server.run(runner_frequency).await?;
        }
        if let Some(server) = self.update.as_mut() {
            server.run(runner_frequency).await?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.audio.is_none() && self.draw.is_none() && self.update.is_none()
    }
}

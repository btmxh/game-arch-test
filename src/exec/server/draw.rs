use std::sync::Arc;

use crate::{
    context::draw::{DrawDispatch, GraphicsContext},
    scene::main::RootScene,
    utils::mpsc::Sender,
};
use anyhow::Context;

use super::{GameServer, SendGameServer};

pub enum Message {
    SetFrequencyProfiling(bool),
    Execute(Box<dyn DrawDispatch>),
}
pub struct Server {
    pub context: GraphicsContext,
    pub root_scene: Arc<RootScene>,
}

impl GameServer for Server {
    fn run(&mut self, single: bool, runner_frequency: f64) -> anyhow::Result<()> {
        self.context
            .draw(&self.root_scene, single, runner_frequency)
    }

    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Draw(Box::new(self)))
    }
}

impl Server {
    pub fn new(context: GraphicsContext, root_scene: Arc<RootScene>) -> anyhow::Result<Self> {
        Ok(Self {
            context,
            root_scene,
        })
    }
}

impl Sender<Message> {
    pub fn set_frequency_profiling(&self, fp: bool) -> anyhow::Result<()> {
        self.send(Message::SetFrequencyProfiling(fp))
            .context("unable to send frequency profiling request")
    }

    pub fn execute<F>(&self, callback: F) -> anyhow::Result<()>
    where
        F: DrawDispatch + 'static,
    {
        self.send(Message::Execute(Box::new(callback)))
            .context("unable to send execute message to draw server")
    }
}

#[test]
fn test_send_sync() {
    use crate::assert_send;
    assert_send!(Server);
}

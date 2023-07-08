use std::{sync::Arc, time::Duration};

use crate::{
    context::draw::{DrawDispatch, GraphicsContext},
    display::EventSender,
    scene::main::RootScene,
    utils::{
        args::args,
        mpsc::{Receiver, Sender},
    },
};
use anyhow::Context;

use super::{BaseGameServer, GameServer, SendGameServer};

pub enum Message {
    SetFrequencyProfiling(bool),
    Execute(Box<dyn DrawDispatch>),
}
pub struct Server {
    pub base: BaseGameServer<Message>,
    pub context: GraphicsContext,
    pub root_scene: Arc<RootScene>,
}

impl GameServer for Server {
    fn run(&mut self, single: bool, runner_frequency: f64) -> anyhow::Result<()> {
        let headless = args().headless;
        for _ in 0..self.base.run("Draw", runner_frequency) {
            self.process_messages(single && headless)?;
            // no need for headless checks, since headless run will have no surface to render to
            self.context
                .draw(&self.root_scene)
                .context("error while drawing frame")?;
        }
        Ok(())
    }

    fn to_send(self) -> anyhow::Result<SendGameServer> {
        Ok(SendGameServer::Draw(Box::new(self)))
    }
}

impl Server {
    pub fn new(
        event_sender: EventSender,
        receiver: Receiver<Message>,
        context: GraphicsContext,
        root_scene: Arc<RootScene>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            base: BaseGameServer::new(event_sender, receiver),
            context,
            root_scene,
        })
    }

    fn process_messages(&mut self, block: bool) -> anyhow::Result<()> {
        let messages = self
            .base
            .receiver
            .try_iter(block.then_some(Duration::from_millis(300)))
            .context("thread runner channel was unexpectedly closed")?
            .collect::<Vec<_>>();
        for message in messages {
            match message {
                Message::SetFrequencyProfiling(fp) => self.base.frequency_profiling = fp,
                Message::Execute(callback) => self.context.run_callback(callback, &self.root_scene),
            }
        }

        Ok(())
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

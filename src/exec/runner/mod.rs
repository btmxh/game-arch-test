use std::thread::JoinHandle;

use anyhow::{bail, Context};

use crate::utils::{
    clock::SteadyClock,
    mpsc::{self, UnboundedReceiverExt},
    sync::{ClockSync, OFClockSync},
};

use self::container::ServerContainer;

use super::server::{SendGameServer, ServerKind};

pub mod container;

pub enum FromRunnerMsg {
    MoveServer(Option<Box<dyn SendGameServer>>),
}
pub enum ToRunnerMsg {
    RequestServer(ServerKind),
    MoveServer(Box<dyn SendGameServer>),
    SetFrequency(f64),
    Stop,
}

#[derive(Default)]
pub struct Runner {
    pub container: ServerContainer,
    pub sync: OFClockSync<SteadyClock>,
    pub frequency: f64,
}

impl Runner {
    pub fn run_single(&mut self) -> anyhow::Result<()> {
        self.container.run_single()?;
        self.sync.sync(self.frequency);
        Ok(())
    }
}

pub struct ThreadRunner {
    base: Runner,
    sender: mpsc::UnboundedSender<FromRunnerMsg>,
    receiver: mpsc::UnboundedReceiver<ToRunnerMsg>,
}

pub struct ThreadRunnerHandle {
    join_handle: JoinHandle<()>,
    sender: mpsc::UnboundedSender<ToRunnerMsg>,
    receiver: mpsc::UnboundedReceiver<FromRunnerMsg>,
}

impl ThreadRunner {
    fn send(&self, msg: FromRunnerMsg) -> anyhow::Result<()> {
        self.sender
            .send(msg)
            .map_err(|e| anyhow::format_err!("{}", e))
    }

    pub fn run(mut self) {
        loop {
            let pending_msgs = self
                .receiver
                .receive_all_pending(self.base.container.is_empty())
                .expect("thread runner channel was unexpectedly closed");
            for msg in pending_msgs {
                match msg {
                    ToRunnerMsg::Stop => return,
                    ToRunnerMsg::MoveServer(server) => self
                        .base
                        .container
                        .emplace_server_check(server)
                        .expect("error emplacing server"),
                    ToRunnerMsg::RequestServer(kind) => {
                        let server = self
                            .base
                            .container
                            .take_server(kind)
                            .expect("error taking server");
                        self.send(FromRunnerMsg::MoveServer(server))
                            .expect("thread runner channel was unexpectedly closed");
                    }
                    ToRunnerMsg::SetFrequency(frequency) => self.base.frequency = frequency,
                }
            }

            self.base.run_single().expect("error while running servers");
        }
    }
}

impl ThreadRunnerHandle {
    pub fn new() -> Self {
        let (to_send, to_recv) = mpsc::unbounded_channel();
        let (from_send, from_recv) = mpsc::unbounded_channel();
        Self {
            join_handle: std::thread::spawn(move || {
                ThreadRunner {
                    base: Runner::default(),
                    sender: from_send,
                    receiver: to_recv,
                }
                .run();
            }),
            sender: to_send,
            receiver: from_recv,
        }
    }

    fn send(&self, msg: ToRunnerMsg) -> anyhow::Result<()> {
        self.sender
            .send(msg)
            .map_err(|e| anyhow::format_err!("{}", e))
            .context("thread runner channel was unexpectedly closed")
    }

    fn recv(&mut self) -> anyhow::Result<FromRunnerMsg> {
        self.receiver
            .blocking_recv()
            .ok_or_else(|| anyhow::format_err!("thread runner channel was unexpectedly closed"))
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.send(ToRunnerMsg::Stop)
    }

    pub fn join(self) -> bool {
        self.join_handle.join().is_err()
    }

    pub fn set_frequency(&self, frequency: f64) -> anyhow::Result<()> {
        self.send(ToRunnerMsg::SetFrequency(frequency))
    }
}

impl Default for ThreadRunnerHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ServerMover {
    fn take_server(&mut self, kind: ServerKind) -> anyhow::Result<Option<Box<dyn SendGameServer>>>;
    fn emplace_server(&mut self, server: Box<dyn SendGameServer>) -> anyhow::Result<()>;

    fn take_server_check(&mut self, kind: ServerKind) -> anyhow::Result<Box<dyn SendGameServer>> {
        self.take_server(kind)?.ok_or_else(|| {
            anyhow::format_err!(
                "{} server not found in container",
                match kind {
                    ServerKind::Audio => "audio",
                    ServerKind::Draw => "draw",
                    ServerKind::Update => "update",
                }
            )
        })
    }

    fn emplace_server_check(&mut self, server: Box<dyn SendGameServer>) -> anyhow::Result<()> {
        debug_assert!(
            self.take_server(server.server_kind())
                .context("checking for existing server, expected None, but an error occurred")?
                .is_none(),
            "invalid state: server already existed before emplacement"
        );
        self.emplace_server(server)
    }
}

impl ServerMover for MainRunner {
    fn take_server(&mut self, kind: ServerKind) -> anyhow::Result<Option<Box<dyn SendGameServer>>> {
        self.base.container.take_server(kind)
    }

    fn emplace_server(&mut self, server: Box<dyn SendGameServer>) -> anyhow::Result<()> {
        self.base.container.emplace_server(server)
    }
}

impl ServerMover for ThreadRunnerHandle {
    #[allow(irrefutable_let_patterns)]
    fn take_server(&mut self, kind: ServerKind) -> anyhow::Result<Option<Box<dyn SendGameServer>>> {
        self.send(ToRunnerMsg::RequestServer(kind))
            .context("unable to request server from runner thread")?;
        let message = self
            .recv()
            .context("unable to receive server from runner thread")?;
        if let FromRunnerMsg::MoveServer(server) = message {
            Ok(server)
        } else {
            bail!("invalid thread runner response")
        }
    }

    fn emplace_server(&mut self, server: Box<dyn SendGameServer>) -> anyhow::Result<()> {
        self.send(ToRunnerMsg::MoveServer(server))
    }
}

pub struct MainRunner {
    pub base: Runner,
}

pub type RunnerId = u8;
pub const MAIN_RUNNER_ID: RunnerId = 3;

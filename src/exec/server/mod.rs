use crate::{
    events::GameUserEvent,
    utils::{
        frequency_runner::FrequencyProfiler,
        mpsc::{self, Receiver, Sender},
    },
};
use anyhow::Context;
use rand::{thread_rng, Rng};
use winit::event_loop::EventLoopProxy;

pub mod audio;
pub mod draw;
pub mod update;

pub enum BaseSendMsg {
    SetRelativeFrequency(f64),
}

pub struct BaseGameServer<SendMsg, RecvMsg> {
    pub sender: Sender<SendMsg>,
    pub proxy: EventLoopProxy<GameUserEvent>,
    pub receiver: Receiver<RecvMsg>,
    pub frequency_profiling: bool,
    pub frequency_profiler: FrequencyProfiler,
    pub relative_frequency: f64,
    pub timer: f64,
}

pub trait GameServerSendChannel<RecvMsg> {
    fn sender(&self) -> &Sender<RecvMsg>;
    fn send(&self, message: RecvMsg) -> anyhow::Result<()> {
        self.sender()
            .send(message)
            .map_err(|e| anyhow::format_err!("{}", e))
            .context(
                "unable to send message to (local) game server (the server was probably closed)",
            )
    }

    fn clone_sender(&self) -> ServerSendChannel<RecvMsg> {
        ServerSendChannel(self.sender().clone())
    }
}

pub trait GameServerChannel<SendMsg, RecvMsg>: GameServerSendChannel<RecvMsg> {
    fn receiver(&mut self) -> &mut Receiver<SendMsg>;

    fn recv(&mut self) -> anyhow::Result<SendMsg> {
        self.receiver().recv().context(
            "unable to receive message from (local) game server (the server was probably closed)",
        )
    }
}

pub struct ServerSendChannel<RecvMsg>(Sender<RecvMsg>);

impl<RecvMsg> GameServerSendChannel<RecvMsg> for ServerSendChannel<RecvMsg> {
    fn sender(&self) -> &Sender<RecvMsg> {
        &self.0
    }
}

pub struct ServerChannels {
    pub audio: audio::ServerChannel,
    pub draw: draw::ServerChannel,
    pub update: update::ServerChannel,
}

impl<SendMsg, RecvMsg> BaseGameServer<SendMsg, RecvMsg> {
    pub fn send(&self, message: SendMsg) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .map_err(|e| anyhow::format_err!("{}", e))
            .context("Unable to send message from (local) game server (the main event loop receiver was closed)")
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServerKind {
    Audio,
    Draw,
    Update,
}

pub trait GameServer {
    fn run(&mut self, runner_frequency: f64) -> anyhow::Result<()>;
    fn to_send(self) -> anyhow::Result<Box<dyn SendGameServer>>;
}
pub trait SendGameServer: Send {
    fn server_kind(&self) -> ServerKind;

    fn downcast_audio(self: Box<Self>) -> anyhow::Result<audio::Server> {
        panic!("invalid downcast")
    }

    fn downcast_draw(self: Box<Self>) -> anyhow::Result<draw::Server> {
        panic!("invalid downcast")
    }

    fn downcast_update(self: Box<Self>) -> anyhow::Result<update::Server> {
        panic!("invalid downcast")
    }
}

impl<SendMsg, RecvMsg> BaseGameServer<SendMsg, RecvMsg> {
    pub fn new(proxy: EventLoopProxy<GameUserEvent>) -> (Self, Sender<RecvMsg>, Receiver<SendMsg>) {
        let (send_sender, send_receiver) = mpsc::channels();
        let (recv_sender, recv_receiver) = mpsc::channels();
        (
            Self {
                receiver: recv_receiver,
                sender: send_sender,
                proxy,
                frequency_profiler: FrequencyProfiler::default(),
                frequency_profiling: false,
                relative_frequency: 1.0,
                timer: 0.0,
            },
            recv_sender,
            send_receiver,
        )
    }

    pub fn run(&mut self, server_name: &str, intended_frequency: f64) -> usize {
        if let Some(frequency) = self.frequency_profiler.update_and_get_frequency() {
            if self.frequency_profiling && thread_rng().gen::<f64>() * frequency < 1.0 {
                tracing::debug!(
                    "{} server running frequency: {} (delta time delay: {}ms)",
                    server_name,
                    frequency,
                    (1.0 / frequency - 1.0 / intended_frequency) * 1e3
                );
            }
        }

        self.timer += self.relative_frequency;
        let run_count = self.timer.floor();
        self.timer -= run_count;
        run_count as _
    }
}

#[macro_export]
macro_rules! handle_msg {
    ($self: expr, $ret: expr, $name: expr, $make_sync: expr, $make_event: expr) => {
        match $ret {
            Some(ReturnMechanism::Sync) => $self.base.send(($make_sync)()).with_context(|| {
                format!("unable to send {} message for Sync return mechanism", $name)
            }),
            Some(ReturnMechanism::Event(id)) => $self
                .base
                .proxy
                .send_event(($make_event)(id))
                .with_context(|| {
                    format!("unable to send {} event for Event return mechanism", $name)
                }),
            None => Ok(()),
        }
    };
}

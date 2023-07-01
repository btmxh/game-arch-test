use crate::{
    events::GameUserEvent,
    utils::{
        frequency_runner::FrequencyProfiler,
        mpsc::{Receiver, Sender},
    },
};
use rand::{thread_rng, Rng};
use winit::event_loop::EventLoopProxy;

pub mod audio;
pub mod draw;
pub mod update;

pub enum BaseSendMsg {
    SetRelativeFrequency(f64),
}

pub struct BaseGameServer<Message> {
    pub proxy: EventLoopProxy<GameUserEvent>,
    pub receiver: Receiver<Message>,
    pub frequency_profiling: bool,
    pub frequency_profiler: FrequencyProfiler,
    pub relative_frequency: f64,
    pub timer: f64,
}

pub struct ServerChannels {
    pub audio: Sender<audio::Message>,
    pub draw: Sender<draw::Message>,
    pub update: Sender<update::Message>,
}

impl<Message> BaseGameServer<Message> {
    // pub fn send(&self, _message: Message) -> anyhow::Result<()> {
    // todo!()
    // self.proxy.send_event(GameUserEvent::FromServerMessage(message))
    // .context("Unable to send message from (local) game server (the main event loop receiver was closed)")
    // }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServerKind {
    Audio,
    Draw,
    Update,
}

pub trait GameServer {
    fn run(&mut self, single: bool, runner_frequency: f64) -> anyhow::Result<()>;
    fn to_send(self) -> anyhow::Result<SendGameServer>;
}

pub enum SendGameServer {
    Audio(Box<audio::Server>),
    Update(Box<update::Server>),
    Draw(Box<draw::Server>),
}

impl SendGameServer {
    pub fn server_kind(&self) -> ServerKind {
        match self {
            Self::Audio(_) => ServerKind::Audio,
            Self::Draw(_) => ServerKind::Draw,
            Self::Update(_) => ServerKind::Update,
        }
    }
}

impl<Message> BaseGameServer<Message> {
    pub fn new(proxy: EventLoopProxy<GameUserEvent>, receiver: Receiver<Message>) -> Self {
        Self {
            receiver,
            proxy,
            frequency_profiler: FrequencyProfiler::default(),
            frequency_profiling: false,
            relative_frequency: 1.0,
            timer: 0.0,
        }
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

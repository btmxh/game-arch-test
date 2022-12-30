use crate::utils::mpsc::{self, UnboundedReceiver, UnboundedSender};
use anyhow::Context;

pub mod audio;
pub mod draw;
pub mod update;

pub enum BaseSendMsg {
    SetRelativeFrequency(f64),
}

pub struct BaseGameServer<SendMsg, RecvMsg> {
    pub sender: UnboundedSender<SendMsg>,
    pub receiver: UnboundedReceiver<RecvMsg>,
}

pub trait GameServerChannel<SendMsg, RecvMsg> {
    fn sender(&self) -> &UnboundedSender<RecvMsg>;
    fn receiver(&mut self) -> &mut UnboundedReceiver<SendMsg>;
    fn send(&self, message: RecvMsg) -> anyhow::Result<()> {
        self.sender()
            .send(message)
            .map_err(|e| anyhow::format_err!("{}", e))
            .context("unable to send message to (local) game server")
    }

    fn recv(&mut self) -> anyhow::Result<SendMsg> {
        self.receiver().blocking_recv().ok_or_else(|| {
            anyhow::format_err!("unable to receive message from (local) game server")
        })
    }
}

pub struct ServerChannels {
    pub audio: audio::ServerChannel,
    pub draw: draw::ServerChannel,
    pub update: update::ServerChannel,
}

impl<SendMsg, RecvMsg> BaseGameServer<SendMsg, RecvMsg> {
    pub fn send(&mut self, message: SendMsg) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .map_err(|e| anyhow::format_err!("{}", e))
            .context("Unable to send message from (local) game server")
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServerKind {
    Audio,
    Draw,
    Update,
}

pub trait GameServer {
    fn run(&mut self) -> anyhow::Result<()>;
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
    pub fn new() -> (Self, UnboundedSender<RecvMsg>, UnboundedReceiver<SendMsg>) {
        let (send_sender, send_receiver) = mpsc::unbounded_channel();
        let (recv_sender, recv_receiver) = mpsc::unbounded_channel();
        (
            Self {
                receiver: recv_receiver,
                sender: send_sender,
            },
            recv_sender,
            send_receiver,
        )
    }
}

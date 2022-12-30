use crate::utils::mpsc::{UnboundedReceiver, UnboundedSender};

use super::{BaseGameServer, GameServer, GameServerChannel, SendGameServer};

pub enum SendMsg {}
pub enum RecvMsg {}

pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
}

pub struct ServerChannel {
    sender: UnboundedSender<RecvMsg>,
    receiver: UnboundedReceiver<SendMsg>,
}

impl GameServerChannel<SendMsg, RecvMsg> for ServerChannel {
    fn sender(&self) -> &UnboundedSender<RecvMsg> {
        &self.sender
    }
    fn receiver(&mut self) -> &mut UnboundedReceiver<SendMsg> {
        &mut self.receiver
    }
}

impl GameServer for Server {
    fn run(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    fn to_send(self) -> anyhow::Result<Box<dyn SendGameServer>> {
        Ok(Box::new(self))
    }
}

impl SendGameServer for Server {
    fn server_kind(&self) -> super::ServerKind {
        super::ServerKind::Audio
    }

    fn downcast_audio(self: Box<Self>) -> anyhow::Result<self::Server> {
        Ok(*self)
    }
}

impl Server {
    pub fn new() -> (Self, ServerChannel) {
        let (base, sender, receiver) = BaseGameServer::new();
        (Self { base }, ServerChannel { receiver, sender })
    }
}

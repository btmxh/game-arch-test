use super::{BaseGameServer, GameServer, SendGameServer, ServerChannel};

pub enum SendMsg {}
pub enum RecvMsg {}

pub struct Server {
    pub base: BaseGameServer<SendMsg, RecvMsg>,
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
        super::ServerKind::Update
    }

    fn downcast_update(self: Box<Self>) -> anyhow::Result<self::Server> {
        Ok(*self)
    }
}

impl Server {
    pub fn new() -> (Self, ServerChannel<SendMsg, RecvMsg>) {
        let (base, channels) = BaseGameServer::new();
        (Self { base }, channels)
    }
}

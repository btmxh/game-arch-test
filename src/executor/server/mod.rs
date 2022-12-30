use crate::utils::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    IntoAnyhowError,
};
use anyhow::Context;

pub mod audio;
pub mod draw;
pub mod update;

pub struct BaseGameServer<SendMsg, RecvMsg> {
    pub sender: UnboundedSender<SendMsg>,
    pub receiver: UnboundedReceiver<RecvMsg>,
}

pub struct ServerChannel<SendMsg, RecvMsg> {
    pub sender: UnboundedSender<RecvMsg>,
    pub receiver: UnboundedReceiver<SendMsg>,
}

impl<SendMsg, RecvMsg> BaseGameServer<SendMsg, RecvMsg> {
    pub fn send(&mut self, message: SendMsg) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .map_err(|e| e.into_anyhow_error())
            .context("Unable to send message to (local) game server")?;
        Ok(())
    }
}

pub trait GameServer<SendMsg, RecvMsg> {
    fn to_send(self: Box<Self>) -> Box<dyn SendGameServer<SendMsg, RecvMsg>>;
}
pub trait SendGameServer<SendMsg, RecvMsg> {
    fn to_nonsend(self: Box<Self>) -> Box<dyn GameServer<SendMsg, RecvMsg>>;
}

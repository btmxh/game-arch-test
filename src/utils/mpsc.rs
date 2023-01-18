// use async_trait::async_trait;
// pub use tokio::sync::mpsc::{
//     error::{SendError, TryRecvError},
//     unbounded_channel, UnboundedReceiver, UnboundedSender,
// };

use anyhow::Context;
use failure::{format_err, Error};

pub struct Sender<T>(flume::Sender<T>);
pub struct Receiver<T>(flume::Receiver<T>);

pub fn channels<T>() -> (Sender<T>, Receiver<T>) {
    let (s, r) = flume::unbounded();
    (Sender(s), Receiver(r))
}

impl<T> Sender<T> {
    pub fn send(&self, msg: T) -> anyhow::Result<()> {
        self.0
            .send(msg)
            .map_err(|_| anyhow::format_err!("unable to send message"))
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> anyhow::Result<T> {
        self.0.recv().context("unable to receive message")
    }

    pub fn try_recv(&self) -> anyhow::Result<Option<T>> {
        match self.0.try_recv() {
            Err(flume::TryRecvError::Empty) => Ok(None),
            r => r.context("unable to receive message").map(Some),
        }
    }

    pub fn try_iter(&self) -> Result<impl Iterator<Item = T> + '_, Error> {
        if self.0.is_disconnected() {
            return Err(format_err!("channels were disconnected"));
        }

        Ok(self.0.try_iter())
    }

    pub fn try_recv_block(&self) -> Result<impl Iterator<Item = T> + '_, Error> {
        self.recv()
            .map_err(|_| format_err!("channels were disconnected"))
            .map(|msg| std::iter::once(msg).chain(self.0.try_iter()))
    }
}

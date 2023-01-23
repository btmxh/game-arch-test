use std::time::Duration;

use flume::TryRecvError;

pub struct Receiver<T>(flume::Receiver<T>);
pub struct Sender<T>(flume::Sender<T>);

impl<T> Receiver<T> {
    pub fn recv(&self) -> anyhow::Result<T> {
        Ok(self.0.recv()?)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> anyhow::Result<Option<T>> {
        match self.0.recv_timeout(timeout) {
            Err(flume::RecvTimeoutError::Timeout) => Ok(None),
            r => Ok(r.map(Some)?),
        }
    }

    pub fn try_recv(&self) -> anyhow::Result<Option<T>> {
        match self.0.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            r => Ok(r.map(Some)?),
        }
    }

    pub fn try_iter(
        &self,
        block_timeout: Option<Duration>,
    ) -> anyhow::Result<impl Iterator<Item = T> + '_> {
        let first = match block_timeout {
            Some(timeout) => self.recv_timeout(timeout)?,
            None => None,
        };
        Ok(first.into_iter().chain(self.0.try_iter()))
    }

    pub fn is_disconnected(&self) -> bool {
        self.0.is_disconnected()
    }
}

impl<T> Sender<T> {
    pub fn send(&self, msg: T) -> anyhow::Result<()> {
        self.0
            .send(msg)
            .map_err(|_| anyhow::Error::msg("mpsc::SendError(...)"))
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub fn channels<T>() -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = flume::unbounded();
    (Sender(sender), Receiver(receiver))
}

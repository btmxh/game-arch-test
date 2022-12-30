use mpsc::{error::{SendError, TryRecvError}, UnboundedReceiver, UnboundedSender};
pub use tokio::sync::mpsc;

pub trait UnboundedReceiverExt<T> {
    fn receiver_all_pending(&mut self) -> Option<Vec<T>>;
}

impl<T> UnboundedReceiverExt<T> for UnboundedReceiver<T> {
    fn receiver_all_pending(&mut self) -> Option<Vec<T>> {
        let mut pending_messages = Vec::new();
        loop {
            match self.try_recv() {
                Ok(msg) => pending_messages.push(msg),
                Err(TryRecvError::Disconnected) => return None,
                Err(TryRecvError::Empty) => return Some(pending_messages),
            }
        }
    }
}

pub trait IntoAnyhowError {
    fn into_anyhow_error(self) -> anyhow::Error;
}

impl<T> IntoAnyhowError for SendError<T> {
    fn into_anyhow_error(self) -> anyhow::Error {
        anyhow::Error::msg("mpsc::SendError(...)")
    }
}


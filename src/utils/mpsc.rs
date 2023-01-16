use async_trait::async_trait;
pub use tokio::sync::mpsc::{
    error::{SendError, TryRecvError},
    unbounded_channel, UnboundedReceiver, UnboundedSender,
};

#[async_trait(?Send)]
pub trait UnboundedReceiverExt<T> {
    async fn receive_all_pending(&mut self, block: bool) -> Option<Vec<T>>;
}

#[async_trait(?Send)]
impl<T> UnboundedReceiverExt<T> for UnboundedReceiver<T> {
    async fn receive_all_pending(&mut self, block: bool) -> Option<Vec<T>> {
        let mut pending_messages = Vec::new();
        if block {
            match self.recv().await {
                Some(msg) => pending_messages.push(msg),
                None => return None,
            }
        }
        loop {
            match self.try_recv() {
                Ok(msg) => pending_messages.push(msg),
                Err(TryRecvError::Disconnected) => return None,
                Err(TryRecvError::Empty) => return Some(pending_messages),
            }
        }
    }
}

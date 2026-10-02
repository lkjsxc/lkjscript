use super::Shared;
use std::sync::Arc;

/// Single receiver. Drop closes admission and disposes queued, already accepted
/// values. A dequeued value belongs to its consumer and must be joined separately.
pub(crate) struct Receiver<T> {
    pub(super) shared: Arc<Shared<T>>,
}
impl<T> Receiver<T> {
    pub(super) fn new(shared: Arc<Shared<T>>) -> Self {
        Self { shared }
    }
    pub(crate) async fn recv(&mut self) -> Option<T> {
        loop {
            let changed = self.shared.readable.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let value = {
                let mut state = self.shared.lock();
                if let Some(value) = state.queue.pop_front() {
                    Some(value)
                } else if state.closed || state.producers_done {
                    return None;
                } else {
                    None
                }
            };
            if let Some(value) = value {
                self.shared.writable.notify_waiters();
                return Some(value);
            }
            changed.await;
        }
    }
    pub(crate) fn close(&mut self) {
        let pending = {
            let mut state = self.shared.lock();
            state.closed = true;
            std::mem::take(&mut state.queue)
        };
        self.shared.writable.notify_waiters();
        self.shared.readable.notify_waiters();
        // Destructors may reenter other runtime boundaries. Never invoke them
        // while holding the custody lock or defer them to the last sender.
        drop(pending);
    }
}
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close();
    }
}

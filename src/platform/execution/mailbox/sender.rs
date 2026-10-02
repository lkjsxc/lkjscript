use super::{ReserveError, SendError, Shared};
use std::sync::Arc;

// One shared lifetime owner avoids a second manually maintained handle count.
// Every reservation keeps this owner alive until it commits or releases capacity.
struct Producers<T> {
    shared: Arc<Shared<T>>,
}
impl<T> Drop for Producers<T> {
    fn drop(&mut self) {
        self.shared.lock().producers_done = true;
        self.shared.readable.notify_waiters();
    }
}
pub(crate) struct Sender<T> {
    producers: Arc<Producers<T>>,
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            producers: Arc::clone(&self.producers),
        }
    }
}
impl<T> Sender<T> {
    pub(super) fn new(shared: Arc<Shared<T>>) -> Self {
        Self {
            producers: Arc::new(Producers { shared }),
        }
    }
    pub(crate) fn try_reserve(&self) -> Result<Permit<T>, ReserveError> {
        let mut state = self.producers.shared.lock();
        if state.closed {
            return Err(ReserveError::Closed);
        }
        if state.queue.len() + state.reserved == state.capacity {
            return Err(ReserveError::Full);
        }
        state.reserved += 1;
        Ok(Permit {
            sender: self.clone(),
            active: true,
        })
    }
    pub(crate) async fn reserve(&self) -> Result<Permit<T>, ReserveError> {
        loop {
            // Register before observing capacity. A close or capacity release
            // between the check and await must not strand this waiter.
            let changed = self.producers.shared.writable.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            match self.try_reserve() {
                Ok(permit) => return Ok(permit),
                Err(ReserveError::Closed) => return Err(ReserveError::Closed),
                Err(ReserveError::Full) => changed.await,
            }
        }
    }
    pub(crate) async fn send(&self, value: T) -> Result<(), SendError<T>> {
        match self.reserve().await {
            Ok(permit) => permit.send(value),
            Err(_) => Err(SendError(value)),
        }
    }
}

/// The slot is reserved; no payload has been accepted yet. Not Clone.
pub(crate) struct Permit<T> {
    sender: Sender<T>,
    active: bool,
}
impl<T> Permit<T> {
    pub(crate) fn send(mut self, value: T) -> Result<(), SendError<T>> {
        let shared = &self.sender.producers.shared;
        let result = {
            let mut state = shared.lock();
            state.reserved -= 1;
            self.active = false;
            if state.closed {
                Err(SendError(value))
            } else {
                // The queue was fully allocated before reservations existed.
                // This same lock serializes close and the custody commit.
                state.queue.push_back(value);
                Ok(())
            }
        };
        if result.is_ok() {
            shared.readable.notify_one();
        }
        // On refusal, close already wakes all capacity waiters. No payload drop
        // or user callback occurs under the custody lock.
        result
    }
}
impl<T> Drop for Permit<T> {
    fn drop(&mut self) {
        if self.active {
            self.sender.producers.shared.lock().reserved -= 1;
            self.sender.producers.shared.writable.notify_waiters();
        }
    }
}

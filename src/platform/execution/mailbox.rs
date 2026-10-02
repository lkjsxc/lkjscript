//! Bounded, single-receiver custody with a nonallocating acceptance point.
//! Reservation is not acceptance; receiver close refuses even an existing permit.
//! This carries runtime messages, not graph-level cross-invocation authority.
use super::ExecutionError;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::Notify;

mod receiver;
mod sender;
pub(crate) use receiver::Receiver;
pub(crate) use sender::{Permit, Sender};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReserveError {
    Full,
    Closed,
}

/// Only never-accepted values can occur here. Do not include payloads in diagnostics.
pub(crate) struct SendError<T>(pub(crate) T);
impl<T> std::fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SendError(<unaccepted message>)")
    }
}

struct State<T> {
    queue: VecDeque<T>,
    capacity: usize,
    reserved: usize,
    closed: bool,
    producers_done: bool,
}
struct Shared<T> {
    state: Mutex<State<T>>,
    readable: Notify,
    writable: Notify,
}
impl<T> Shared<T> {
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Preallocate all message slots before issuing any reservation. This bounds queue
/// entries plus reservations, not pending-sender payloads, active receivers or RSS.
/// Payload validation/accounting belongs to the caller and precedes Permit::send.
pub(crate) fn channel<T>(capacity: usize) -> Result<(Sender<T>, Receiver<T>), ExecutionError> {
    if capacity == 0 {
        return Err(ExecutionError::resource(
            "mailbox_capacity",
            "mailbox capacity must be positive",
        ));
    }
    let mut queue = VecDeque::new();
    queue.try_reserve_exact(capacity).map_err(|_| {
        ExecutionError::resource(
            "mailbox_storage",
            "cannot preallocate mailbox message slots",
        )
    })?;
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            queue,
            capacity,
            reserved: 0,
            closed: false,
            producers_done: false,
        }),
        readable: Notify::new(),
        writable: Notify::new(),
    });
    Ok((Sender::new(Arc::clone(&shared)), Receiver::new(shared)))
}

#[cfg(test)]
mod concurrency_tests;
#[cfg(test)]
mod tests;

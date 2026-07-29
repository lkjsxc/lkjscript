use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{HostError, HostResult};

/// The cancellation contract shared by independently composable capabilities.
pub trait Cancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;

    fn check(&self) -> HostResult<()> {
        if self.is_cancelled() {
            Err(HostError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Cloneable, thread-safe cancellation token.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl Cancellation for CancellationToken {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

//! Invocation-wide cumulative admission shared by structured children.
//! Local observations remain local; successful reservations are never refunded.
use super::vm::NormalizedRunPolicy;
use crate::platform::execution::{ExecutionError, cumulative_charge};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy)]
struct StorageUsage {
    bytes: u64,
    items: u64,
}

pub(super) struct SharedBudget {
    remaining_steps: Option<AtomicU64>,
    storage: Mutex<StorageUsage>,
    maximum_bytes: Option<u64>,
    maximum_items: Option<u64>,
}

impl SharedBudget {
    pub(super) fn new(
        policy: NormalizedRunPolicy,
        remaining_steps: Option<u64>,
        bytes: u64,
        items: u64,
    ) -> Self {
        Self {
            remaining_steps: remaining_steps.map(AtomicU64::new),
            storage: Mutex::new(StorageUsage { bytes, items }),
            maximum_bytes: policy.maximum_allocated_bytes,
            maximum_items: policy.maximum_collection_items,
        }
    }

    pub(super) fn step(&self, code: &'static str) -> Result<(), ExecutionError> {
        if let Some(remaining) = &self.remaining_steps {
            remaining
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_sub(1)
                })
                .map_err(|_| {
                    ExecutionError::resource(code, "structured invocation exhausted its work quota")
                })?;
        }
        Ok(())
    }

    /// Check both ledgers before changing either, including map-node reservations.
    pub(super) fn reserve(&self, bytes: u64, items: u64) -> Result<(), ExecutionError> {
        // Limits cannot change during an invocation. Observations are maintained
        // by each evaluator and joined separately; an unmetered ledger is unused.
        if self.maximum_bytes.is_none() && self.maximum_items.is_none() {
            return Ok(());
        }
        let mut usage = self
            .storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let next_items = cumulative_charge(
            usage.items,
            items,
            self.maximum_items,
            "normalized_collection_items",
            "structured invocation exhausted its aggregate collection quota",
        )?;
        let next_bytes = cumulative_charge(
            usage.bytes,
            bytes,
            self.maximum_bytes,
            "normalized_allocation",
            "structured invocation exhausted its aggregate allocation quota",
        )?;
        *usage = StorageUsage {
            bytes: next_bytes,
            items: next_items,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmetered_storage_does_not_wait_for_a_shared_ledger() {
        let budget = SharedBudget::new(NormalizedRunPolicy::foreground(), None, 0, 0);
        let held = budget.storage.lock().unwrap();
        std::thread::scope(|scope| {
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            let budget = &budget;
            scope.spawn(move || sender.send(budget.reserve(u64::MAX, u64::MAX)).unwrap());
            let result = receiver.recv_timeout(std::time::Duration::from_secs(5));
            // Release before asserting, so a regression cannot deadlock scope join.
            drop(held);
            assert!(result.unwrap().is_ok());
        });
    }

    #[test]
    fn shared_reservation_is_atomic_and_children_cannot_replenish_work() {
        let policy = NormalizedRunPolicy {
            maximum_allocated_bytes: Some(10),
            maximum_collection_items: Some(5),
            ..NormalizedRunPolicy::default()
        };
        let budget = SharedBudget::new(policy, Some(2), 4, 1);
        assert!(budget.reserve(7, 4).is_err());
        assert!(budget.reserve(6, 4).is_ok());
        assert!(budget.reserve(1, 0).is_err());
        assert!(budget.reserve(0, 1).is_err());
        assert!(budget.step("work").is_ok());
        assert!(budget.step("work").is_ok());
        assert!(budget.step("work").is_err());
    }
}

//! Invocation-wide cumulative admission shared by structured children.
//! Local observations remain local; successful reservations are never refunded.
use super::vm::NormalizedRunPolicy;
use crate::platform::execution::{ExecutionError, finite_charge};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
            storage: Mutex::new(StorageUsage {
                bytes: if policy.maximum_allocated_bytes.is_some() {
                    bytes
                } else {
                    0
                },
                items: if policy.maximum_collection_items.is_some() {
                    items
                } else {
                    0
                },
            }),
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
        // Only finite dimensions belong in this permission ledger. Unbounded
        // dimensions retain their independently flagged evaluator observations.
        let next_items = match self.maximum_items {
            Some(maximum) => finite_charge(
                usage.items,
                items,
                maximum,
                "normalized_collection_items",
                "structured invocation exhausted its aggregate collection quota",
            )?,
            None => 0,
        };
        let next_bytes = match self.maximum_bytes {
            Some(maximum) => finite_charge(
                usage.bytes,
                bytes,
                maximum,
                "normalized_allocation",
                "structured invocation exhausted its aggregate allocation quota",
            )?,
            None => 0,
        };
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

    #[test]
    fn shared_storage_preserves_zero_exact_maximum_and_mixed_unbounded_dimensions() {
        for bytes_dimension in [false, true] {
            for maximum in [None, Some(0), Some(4), Some(5), Some(u64::MAX)] {
                let mut policy = NormalizedRunPolicy::foreground();
                if bytes_dimension {
                    policy.maximum_allocated_bytes = maximum;
                } else {
                    policy.maximum_collection_items = maximum;
                }
                let budget = SharedBudget::new(policy, None, 0, 0);
                assert!(budget.reserve(0, 0).is_ok());
                let required = if maximum == Some(u64::MAX) {
                    u64::MAX
                } else {
                    5
                };
                let (bytes, items) = if bytes_dimension {
                    (required, 0)
                } else {
                    (0, required)
                };
                let before = *budget.storage.lock().unwrap();
                assert_eq!(
                    budget.reserve(bytes, items).is_ok(),
                    maximum.is_none_or(|n| n >= required)
                );
                if maximum.is_some_and(|n| n < required) {
                    assert_eq!(*budget.storage.lock().unwrap(), before);
                }
                if maximum == Some(required) {
                    let before = *budget.storage.lock().unwrap();
                    assert!(
                        budget
                            .reserve(u64::from(bytes_dimension), u64::from(!bytes_dimension))
                            .is_err()
                    );
                    assert_eq!(*budget.storage.lock().unwrap(), before);
                    assert!(budget.reserve(0, 0).is_ok());
                }
            }
        }
        let budget = SharedBudget::new(
            NormalizedRunPolicy {
                maximum_allocated_bytes: Some(0),
                ..NormalizedRunPolicy::foreground()
            },
            None,
            0,
            u64::MAX,
        );
        assert!(budget.reserve(0, u64::MAX).is_ok());
        assert!(budget.reserve(0, 1).is_ok());
        let before = *budget.storage.lock().unwrap();
        assert!(budget.reserve(1, u64::MAX).is_err());
        assert_eq!(*budget.storage.lock().unwrap(), before);
    }
}

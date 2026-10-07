//! Shared directory of worker custody, not a promise of idle availability.
//! Cross-owner lock order is directory, destination state, source state. Local
//! dispatch and receipt completion use only their owner state. No job or join
//! executes under those locks.
use super::*;
use std::sync::Weak;

pub(super) struct WorkerCustody {
    owner: Weak<Inner>,
    slot: usize,
}

impl WorkerCustody {
    pub(super) fn new(owner: &Arc<Inner>, slot: usize) -> Self {
        Self {
            owner: Arc::downgrade(owner),
            slot,
        }
    }

    pub(super) fn belongs_to(&self, owner: &Arc<Inner>) -> bool {
        self.owner.ptr_eq(&Arc::downgrade(owner))
    }
}

impl StructuredExecutorHandle {
    pub(super) fn claim_idle(
        &self,
        state: &mut State,
        custody: &mut Vec<WorkerCustody>,
    ) -> Option<usize> {
        // The caller dropped its state lock to acquire the directory. A receipt
        // may have returned meanwhile; local work still wins over a foreign owner.
        if let Some(slot) = state.claim_local() {
            return Some(slot);
        }
        let mut index = custody.len();
        while index != 0 {
            index -= 1;
            let entry = &custody[index];
            if entry.belongs_to(&self.inner) {
                continue;
            }
            let Some(owner) = entry.owner.upgrade() else {
                custody.swap_remove(index);
                continue;
            };
            let mut source = lock(&owner.state);
            if !source.observation.dispatch_open {
                custody.swap_remove(index);
                continue;
            }
            if !source.reusable(entry.slot) {
                // Active, unreceived and failed-result workers retain custody but
                // confer no right to run. Their later receipt needs no reindexing.
                continue;
            }
            let Some(mut worker) = source.workers[entry.slot].take() else {
                continue;
            };
            worker.available = false;
            source.observation.remaining_workers -= 1;
            source.observation.workers_handed_off =
                source.observation.workers_handed_off.saturating_add(1);
            state.observation.workers_received =
                state.observation.workers_received.saturating_add(1);
            let slot = state.retain_worker(worker);
            // Keep exactly one weak entry for this physical worker. Its mailbox,
            // join handle and unreleased physical reservation move as one value.
            custody[index] = WorkerCustody::new(&self.inner, slot);
            return Some(slot);
        }
        None
    }
}

impl State {
    fn reusable(&self, slot: usize) -> bool {
        self.workers
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|worker| worker.available && !worker.thread.is_finished())
    }

    pub(super) fn claim_local(&mut self) -> Option<usize> {
        let slot = (0..self.workers.len())
            .rev()
            .find(|&slot| self.reusable(slot))?;
        self.release_slot(slot, false);
        Some(slot)
    }

    pub(super) fn release_slot(&mut self, slot: usize, reusable: bool) {
        if let Some(worker) = self.workers.get_mut(slot).and_then(Option::as_mut) {
            worker.available = reusable && self.observation.dispatch_open;
        }
    }

    pub(super) fn retain_worker(&mut self, worker: Worker) -> usize {
        // Repeated handoffs cannot grow a historical slot inventory unboundedly.
        let slot = if let Some(slot) = self.workers.iter().position(Option::is_none) {
            self.workers[slot] = Some(worker);
            slot
        } else {
            let slot = self.workers.len();
            self.workers.push(Some(worker));
            slot
        };
        self.observation.remaining_workers += 1;
        slot
    }
}

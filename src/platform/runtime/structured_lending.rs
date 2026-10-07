//! Idle-worker custody transfer under one bounded process-capacity catalogue.
//! Lock order is catalogue, destination state, source state. No job or thread join
//! runs under those locks. Single-owner observers/waiters never acquire the catalogue.
use super::*;
use std::sync::Weak;

pub(super) struct IdleWorker {
    owner: Weak<Inner>,
    slot: usize,
}

impl IdleWorker {
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
        idle: &mut Vec<IdleWorker>,
    ) -> Option<usize> {
        // Prefer local reuse. Identity here is lifetime ownership, never meaning.
        while let Some(index) = idle.iter().rposition(|entry| entry.belongs_to(&self.inner)) {
            let entry = idle.swap_remove(index);
            if state.reusable(entry.slot) {
                return Some(entry.slot);
            }
        }
        while let Some(entry) = idle.pop() {
            let Some(owner) = entry.owner.upgrade() else {
                continue;
            };
            let mut source = lock(&owner.state);
            if !source.observation.dispatch_open || !source.reusable(entry.slot) {
                continue;
            }
            let Some(worker) = source.workers[entry.slot].take() else {
                continue;
            };
            // Transfer the mailbox, thread join handle and unreleased physical
            // reservation as one value. No task, program or grant follows it.
            source.observation.remaining_workers -= 1;
            source.observation.workers_handed_off =
                source.observation.workers_handed_off.saturating_add(1);
            state.observation.workers_received =
                state.observation.workers_received.saturating_add(1);
            return Some(state.retain_worker(worker));
        }
        None
    }
}

impl State {
    fn reusable(&self, slot: usize) -> bool {
        self.workers
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|worker| !worker.thread.is_finished())
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

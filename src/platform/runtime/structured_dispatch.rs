//! Owner-local dispatch; consult shared custody only after a local miss.
use super::*;

impl StructuredExecutorHandle {
    fn reserve_slot(&self) -> (MutexGuard<'_, State>, Option<usize>) {
        let mut state = lock(&self.inner.state);
        if !state.observation.dispatch_open {
            return (state, None);
        }
        if let Some(slot) = state.claim_local() {
            return (state, Some(slot));
        }
        // Never acquire shared custody while retaining an owner lock. Recheck
        // local availability and dispatch closure after changing lock domains.
        drop(state);
        let mut custody = lock(&self.inner.capacity.custody);
        let mut state = lock(&self.inner.state);
        let slot = if state.observation.dispatch_open {
            self.claim_idle(&mut state, &mut custody)
                .or_else(|| self.start_worker(&mut state, &mut custody))
        } else {
            None
        };
        (state, slot)
    }

    pub(super) fn dispatch<R: Send + 'static, F: FnOnce() -> R + Send + 'static>(
        &self,
        control: &ExecutionControl,
        right: F,
    ) -> Dispatch<R, F> {
        let (mut state, slot) = self.reserve_slot();
        let Some(slot) = slot else {
            state.observation.inline_fallbacks =
                state.observation.inline_fallbacks.saturating_add(1);
            return Dispatch::Inline(right);
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let job_control = control.clone();
        let job: Job = Box::new(move || {
            let mut guard = CancelOnUnwind::new(job_control.clone());
            let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(right)) {
                Ok(value) => Ok(value),
                Err(_) => {
                    job_control.cancel();
                    Err(worker_failure())
                }
            };
            let _ = sender.send(outcome);
            guard.armed = false;
        });
        let command = Command::Run(job);
        #[cfg(test)]
        let refuse = std::mem::take(&mut state.refuse_next_submission);
        #[cfg(not(test))]
        let refuse = false;
        let submitted = if refuse {
            Err(mpsc::TrySendError::Full(command))
        } else {
            match &state.workers[slot] {
                Some(worker) => worker.sender.try_send(command),
                None => Err(mpsc::TrySendError::Disconnected(command)),
            }
        };
        match submitted {
            Ok(()) => {
                let observation = &mut state.observation;
                observation.active_dispatches += 1;
                observation.maximum_active_dispatches = observation
                    .maximum_active_dispatches
                    .max(observation.active_dispatches);
                Dispatch::Accepted(JoinedChild {
                    inner: Arc::clone(&self.inner),
                    slot: Some(slot),
                    receiver,
                    control: control.clone(),
                })
            }
            Err(mpsc::TrySendError::Full(Command::Run(job))) => {
                state.release_slot(slot, true);
                state.observation.inline_fallbacks =
                    state.observation.inline_fallbacks.saturating_add(1);
                Dispatch::Refused(job, receiver)
            }
            Err(mpsc::TrySendError::Disconnected(Command::Run(job))) => {
                // A directory entry establishes custody, not availability. The
                // failed worker remains unavailable and owned until joined.
                state.observation.inline_fallbacks =
                    state.observation.inline_fallbacks.saturating_add(1);
                Dispatch::Refused(job, receiver)
            }
            Err(mpsc::TrySendError::Full(Command::Stop))
            | Err(mpsc::TrySendError::Disconnected(Command::Stop)) => {
                unreachable!("dispatch submits only child jobs")
            }
        }
    }

    fn start_worker(&self, state: &mut State, custody: &mut Vec<WorkerCustody>) -> Option<usize> {
        let reservation = self.inner.capacity.reserve()?;
        #[cfg(test)]
        if std::mem::take(&mut state.refuse_next_worker_start) {
            return None;
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("lkjscript-structured".to_owned())
            .spawn(move || {
                while let Ok(Command::Run(job)) = receiver.recv() {
                    // The typed wrapper reports child panics. This outer boundary
                    // also retains the worker if a panic payload's destructor fails.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                }
            })
            .ok()?;
        let slot = state.retain_worker(Worker {
            sender,
            thread,
            available: false,
            _reservation: reservation,
        });
        custody.push(WorkerCustody::new(&self.inner, slot));
        state.observation.workers_started = state.observation.workers_started.saturating_add(1);
        Some(slot)
    }
}

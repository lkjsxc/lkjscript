//! Lexically borrowed execution uses the same process capacity and joins before
//! returning. Region buffers never enter a reference-counted worker envelope.
use super::*;

struct ScopedDispatch<'a> {
    inner: &'a Inner,
    _reservation: ThreadReservation,
    started: bool,
}

impl Drop for ScopedDispatch<'_> {
    fn drop(&mut self) {
        let mut state = lock(&self.inner.state);
        let observation = &mut state.observation;
        observation.active_dispatches -= 1;
        if self.started {
            observation.completed_dispatches = observation.completed_dispatches.saturating_add(1);
            observation.joined_workers = observation.joined_workers.saturating_add(1);
            observation.remaining_workers -= 1;
        }
        drop(state);
        self.inner.joined.notify_all();
    }
}

impl StructuredExecutorHandle {
    pub(crate) fn run_scoped<L, R: Send>(
        &self,
        control: &ExecutionControl,
        left: impl FnOnce() -> L,
        right: impl FnOnce() -> R + Send,
    ) -> Result<StructuredPair<L, R>, ExecutionError> {
        control.check()?;
        let reservation = {
            let mut state = lock(&self.inner.state);
            let reservation = state
                .observation
                .dispatch_open
                .then(|| self.inner.capacity.reserve())
                .flatten();
            #[cfg(test)]
            let reservation = if std::mem::take(&mut state.refuse_next_worker_start) {
                None
            } else {
                reservation
            };
            if reservation.is_some() {
                state.observation.active_dispatches += 1;
                state.observation.maximum_active_dispatches = state
                    .observation
                    .maximum_active_dispatches
                    .max(state.observation.active_dispatches);
            } else {
                state.observation.inline_fallbacks =
                    state.observation.inline_fallbacks.saturating_add(1);
            }
            reservation
        };
        let Some(reservation) = reservation else {
            return Ok(StructuredPair {
                left: left(),
                right: right(),
                dispatched: false,
            });
        };
        let mut receipt = ScopedDispatch {
            inner: &self.inner,
            _reservation: reservation,
            started: false,
        };
        let work = Mutex::new(Some(right));
        let invoke = || {
            let work = lock(&work).take().ok_or_else(worker_failure)?;
            Ok(work())
        };
        thread::scope(|scope| {
            let child = thread::Builder::new()
                .name("lkjscript-region".to_owned())
                .spawn_scoped(scope, || {
                    let mut guard = CancelOnUnwind::new(control.clone());
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(invoke))
                        .map_err(|_| worker_failure())?;
                    guard.armed = false;
                    result
                });
            let Ok(child) = child else {
                let mut state = lock(&self.inner.state);
                state.observation.inline_fallbacks =
                    state.observation.inline_fallbacks.saturating_add(1);
                drop(state);
                return Ok(StructuredPair {
                    left: left(),
                    right: invoke()?,
                    dispatched: false,
                });
            };
            receipt.started = true;
            {
                let mut state = lock(&self.inner.state);
                state.observation.workers_started =
                    state.observation.workers_started.saturating_add(1);
                state.observation.remaining_workers += 1;
            }
            // If the caller unwinds, cancel before scope joins the child.
            let mut guard = CancelOnUnwind::new(control.clone());
            let left = left();
            let right = child.join().map_err(|_| worker_failure())??;
            guard.armed = false;
            Ok(StructuredPair {
                left,
                right,
                dispatched: true,
            })
        })
    }
}

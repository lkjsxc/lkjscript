//! Shared notify-before-observation ordering for broadcast idle transitions.
use tokio::sync::Notify;

pub(crate) async fn wait_until_idle(idle: &Notify, is_idle: impl Fn() -> bool) {
    loop {
        // notify_waiters retains wakeups for already-created Notified futures,
        // even before their first poll. Register before observing shared state.
        let notified = idle.notified();
        if is_idle() {
            return;
        }
        notified.await;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "bounded regression assertions")]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Waker};
    use std::time::Duration;

    fn pending(future: Pin<&mut impl Future<Output = ()>>) {
        assert!(
            future
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
    }

    async fn completes(future: impl Future<Output = ()>) {
        tokio::time::timeout(Duration::from_secs(1), future)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn already_idle_needs_no_notification() {
        completes(wait_until_idle(&Notify::new(), || true)).await;
    }

    #[tokio::test]
    async fn completion_between_count_read_and_wait_cannot_be_lost() {
        let idle = Notify::new();
        let active = AtomicBool::new(true);
        completes(wait_until_idle(&idle, || {
            // Capture the busy observation, then complete on behalf of the other
            // scope before returning it. No sleeps or scheduler lottery are needed.
            let was_idle = !active.swap(false, Ordering::AcqRel);
            idle.notify_waiters();
            was_idle
        }))
        .await;
    }

    #[tokio::test]
    async fn one_completion_wakes_every_registered_observer() {
        let idle = Notify::new();
        let active = AtomicBool::new(true);
        let first = wait_until_idle(&idle, || !active.load(Ordering::Acquire));
        let second = wait_until_idle(&idle, || !active.load(Ordering::Acquire));
        tokio::pin!(first, second);
        pending(first.as_mut());
        pending(second.as_mut());
        active.store(false, Ordering::Release);
        idle.notify_waiters();
        completes(async {
            tokio::join!(first, second);
        })
        .await;
    }

    #[tokio::test]
    async fn cancelled_observer_does_not_consume_completion() {
        let idle = Notify::new();
        let active = AtomicBool::new(true);
        {
            let cancelled = wait_until_idle(&idle, || !active.load(Ordering::Acquire));
            tokio::pin!(cancelled);
            pending(cancelled.as_mut());
        }
        active.store(false, Ordering::Release);
        idle.notify_waiters();
        completes(wait_until_idle(&idle, || !active.load(Ordering::Acquire))).await;
    }
}

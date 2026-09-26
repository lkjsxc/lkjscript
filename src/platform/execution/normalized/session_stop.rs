//! Monotonic session shutdown, retained across absent or cancelled subscribers.
use tokio::sync::watch;

#[derive(Clone)]
pub(super) struct SessionStop(watch::Sender<bool>);

impl SessionStop {
    pub(super) fn new() -> Self {
        let (sender, _) = watch::channel(false);
        Self(sender)
    }

    pub(super) fn request(&self) {
        // send() loses the value when no driver has subscribed yet.
        self.0.send_replace(true);
    }

    pub(super) async fn requested(&self) {
        let mut receiver = self.0.subscribe();
        // A new subscriber considers the current value seen. Awaiting changed()
        // alone would miss a stop already retained by another parent session.
        let _ = receiver.wait_for(|stopping| *stopping).await;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "bounded regression assertions")]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
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
    async fn stop_before_any_subscriber_is_retained() {
        let stop = SessionStop::new();
        stop.request();
        completes(stop.requested()).await;
    }

    #[tokio::test]
    async fn late_subscriber_observes_stop_already_seen_by_another_scope() {
        let stop = SessionStop::new();
        let existing = stop.requested();
        tokio::pin!(existing);
        pending(existing.as_mut());
        stop.request();
        completes(stop.requested()).await;
        completes(existing).await;
    }

    #[tokio::test]
    async fn clone_request_wakes_all_pending_scopes() {
        let stop = SessionStop::new();
        let first = stop.requested();
        let second = stop.requested();
        tokio::pin!(first, second);
        pending(first.as_mut());
        pending(second.as_mut());
        stop.clone().request();
        completes(async {
            tokio::join!(first, second);
        })
        .await;
    }

    #[tokio::test]
    async fn cancelled_waiter_does_not_consume_a_later_stop() {
        let stop = SessionStop::new();
        {
            let cancelled = stop.requested();
            tokio::pin!(cancelled);
            pending(cancelled.as_mut());
        }
        stop.request();
        completes(stop.requested()).await;
    }

    #[tokio::test]
    async fn repeated_requests_and_completed_waiters_never_clear_stop() {
        let stop = SessionStop::new();
        stop.request();
        completes(stop.requested()).await;
        stop.request();
        completes(stop.clone().requested()).await;
        completes(stop.requested()).await;
    }
}

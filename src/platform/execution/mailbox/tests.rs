use super::*;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Waker};

struct Payload {
    id: usize,
    bytes: Box<[u8]>,
    drops: Arc<AtomicUsize>,
}
impl Payload {
    fn new(id: usize, drops: &Arc<AtomicUsize>) -> Self {
        Self {
            id,
            bytes: vec![17, 29, 43].into_boxed_slice(),
            drops: Arc::clone(drops),
        }
    }
}
impl Drop for Payload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn mailbox_rejects_zero_and_overflowing_storage_before_endpoints_exist() {
    assert_eq!(channel::<u8>(0).err().unwrap().code, "mailbox_capacity");
    assert_eq!(
        channel::<[u8; 2]>(usize::MAX).err().unwrap().code,
        "mailbox_storage"
    );
}

#[tokio::test]
async fn mailbox_counts_reservations_and_commits_without_queue_growth_or_payload_copy() {
    for capacity in [1, 2, 7] {
        let (tx, mut rx) = channel::<Payload>(capacity).unwrap();
        let allocated = rx.shared.lock().queue.capacity();
        let drops = Arc::new(AtomicUsize::new(0));
        for cycle in 0..20 {
            let permits: Vec<_> = (0..capacity).map(|_| tx.try_reserve().unwrap()).collect();
            assert_eq!(tx.try_reserve().err(), Some(ReserveError::Full));
            let mut pointers = Vec::new();
            for (id, permit) in permits.into_iter().enumerate() {
                let value = Payload::new(id + cycle * capacity, &drops);
                pointers.push(value.bytes.as_ptr() as usize);
                permit.send(value).unwrap();
                assert_eq!(rx.shared.lock().queue.capacity(), allocated);
            }
            assert_eq!(tx.try_reserve().err(), Some(ReserveError::Full));
            for (id, pointer) in pointers.into_iter().enumerate() {
                let value = rx.recv().await.unwrap();
                assert_eq!(value.id, id + cycle * capacity);
                assert_eq!(value.bytes.as_ptr() as usize, pointer);
            }
        }
        assert_eq!(drops.load(Ordering::SeqCst), 20 * capacity);
        drop(tx);
        assert!(rx.recv().await.is_none());
    }
}

#[tokio::test]
async fn mailbox_close_before_commit_returns_the_original_never_accepted_owner() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = channel(1).unwrap();
    let permit = tx.try_reserve().unwrap();
    let value = Payload::new(1, &drops);
    let pointer = value.bytes.as_ptr();
    rx.close();
    let refused = permit.send(value).unwrap_err().0;
    assert_eq!(refused.bytes.as_ptr(), pointer);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(tx.try_reserve().err(), Some(ReserveError::Closed));
    assert!(rx.recv().await.is_none());
    // An actual returned owner, unlike an unobserved receipt, can be retried.
    let (next, mut receiver) = channel(1).unwrap();
    next.send(refused).await.unwrap();
    let value = receiver.recv().await.unwrap();
    assert_eq!(value.bytes.as_ptr(), pointer);
    drop(value);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mailbox_close_disposes_queued_but_not_active_or_unaccepted_values() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = channel(2).unwrap();
    tx.send(Payload::new(0, &drops)).await.unwrap();
    let active = rx.recv().await.unwrap();
    tx.send(Payload::new(1, &drops)).await.unwrap();
    let reserved = tx.try_reserve().unwrap();
    rx.close();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(active.id, 0);
    let refused = reserved.send(Payload::new(2, &drops)).unwrap_err();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(refused);
    drop(active);
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    // A remaining sender cannot retain accepted queue payloads after receiver exit.
    drop(rx);
    assert_eq!(tx.try_reserve().err(), Some(ReserveError::Closed));
}

#[tokio::test]
async fn mailbox_pending_send_cancellation_and_unused_permits_release_custody() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = channel(1).unwrap();
    let reserved = tx.try_reserve().unwrap();
    let mut pending = Box::pin(tx.send(Payload::new(0, &drops)));
    assert!(
        pending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(pending);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(tx.send(Payload::new(1, &drops))); // Unpolled future also owns its argument.
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    drop(reserved);
    tx.send(Payload::new(2, &drops)).await.unwrap();
    drop(rx.recv().await.unwrap());
    assert_eq!(drops.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn mailbox_last_permit_keeps_receiver_alive_until_commit_or_release() {
    for commit in [false, true] {
        let (tx, mut rx) = channel(1).unwrap();
        let permit = tx.try_reserve().unwrap();
        drop(tx);
        let mut waiting = Box::pin(rx.recv());
        assert!(
            waiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        drop(waiting);
        if commit {
            permit.send(41).unwrap();
            assert_eq!(rx.recv().await, Some(41));
        } else {
            drop(permit);
        }
        assert_eq!(rx.recv().await, None);
    }
}

#[test]
fn mailbox_close_drops_payloads_outside_the_custody_lock() {
    struct Reentrant {
        shared: std::sync::Weak<Shared<Reentrant>>,
        observed: Arc<AtomicUsize>,
    }
    impl Drop for Reentrant {
        fn drop(&mut self) {
            if let Some(shared) = self.shared.upgrade() {
                assert!(
                    shared.state.try_lock().is_ok(),
                    "payload drop held the queue lock"
                );
                self.observed.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
    let (tx, rx) = channel(1).unwrap();
    let observed = Arc::new(AtomicUsize::new(0));
    let value = Reentrant {
        shared: Arc::downgrade(&rx.shared),
        observed: Arc::clone(&observed),
    };
    tx.try_reserve().unwrap().send(value).unwrap();
    drop(rx);
    assert_eq!(observed.load(Ordering::SeqCst), 1);
    assert_eq!(tx.try_reserve().err(), Some(ReserveError::Closed));
}

#[tokio::test]
async fn mailbox_zero_sized_messages_still_obey_logical_capacity() {
    let (tx, mut rx) = channel(2).unwrap();
    let first = tx.try_reserve().unwrap();
    let second = tx.try_reserve().unwrap();
    assert_eq!(tx.try_reserve().err(), Some(ReserveError::Full));
    first.send(()).unwrap();
    second.send(()).unwrap();
    assert_eq!(tx.try_reserve().err(), Some(ReserveError::Full));
    assert_eq!(rx.recv().await, Some(()));
    drop(tx.try_reserve().unwrap());
    assert_eq!(rx.recv().await, Some(()));
}

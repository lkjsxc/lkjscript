use super::*;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Wake, Waker};
use std::time::Duration;

#[derive(Default)]
struct Wakes(AtomicUsize);
impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn mailbox_wakes_all_reserved_capacity_waiters_without_lost_cancellation_notifications() {
    let (tx, mut rx) = channel::<u8>(2).unwrap();
    let one = tx.try_reserve().unwrap();
    let two = tx.try_reserve().unwrap();
    let wakes = Arc::new(Wakes::default());
    let waker = Waker::from(Arc::clone(&wakes));
    let mut context = Context::from_waker(&waker);
    let mut first = Box::pin(tx.reserve());
    let mut cancelled = Box::pin(tx.reserve());
    let mut second = Box::pin(tx.reserve());
    assert!(first.as_mut().poll(&mut context).is_pending());
    assert!(cancelled.as_mut().poll(&mut context).is_pending());
    assert!(second.as_mut().poll(&mut context).is_pending());
    drop(cancelled);
    drop(one);
    drop(two);
    assert!(wakes.0.load(Ordering::SeqCst) >= 2);
    let first = match first.as_mut().poll(&mut context) {
        std::task::Poll::Ready(Ok(p)) => p,
        _ => panic!("released capacity must wake the first surviving waiter"),
    };
    let second = match second.as_mut().poll(&mut context) {
        std::task::Poll::Ready(Ok(p)) => p,
        _ => panic!("released capacity must wake the other surviving waiter"),
    };
    let mut closed = Box::pin(tx.reserve());
    assert!(closed.as_mut().poll(&mut context).is_pending());
    let before = wakes.0.load(Ordering::SeqCst);
    rx.close();
    assert!(wakes.0.load(Ordering::SeqCst) > before);
    assert!(matches!(
        closed.as_mut().poll(&mut context),
        std::task::Poll::Ready(Err(ReserveError::Closed))
    ));
    assert_eq!(first.send(7).unwrap_err().0, 7);
    assert_eq!(second.send(9).unwrap_err().0, 9);
}

#[test]
fn mailbox_cancelled_receiver_does_not_steal_wake_or_last_producer_termination() {
    let (tx, mut rx) = channel::<u8>(1).unwrap();
    let wakes = Arc::new(Wakes::default());
    let waker = Waker::from(Arc::clone(&wakes));
    let mut context = Context::from_waker(&waker);
    let mut cancelled = Box::pin(rx.recv());
    assert!(cancelled.as_mut().poll(&mut context).is_pending());
    drop(cancelled);
    tx.try_reserve().unwrap().send(11).unwrap();
    let mut receive = Box::pin(rx.recv());
    assert_eq!(
        receive.as_mut().poll(&mut context),
        std::task::Poll::Ready(Some(11))
    );
    drop(receive);
    let mut terminal = Box::pin(rx.recv());
    assert!(terminal.as_mut().poll(&mut context).is_pending());
    let before = wakes.0.load(Ordering::SeqCst);
    drop(tx);
    assert!(wakes.0.load(Ordering::SeqCst) > before);
    assert_eq!(
        terminal.as_mut().poll(&mut context),
        std::task::Poll::Ready(None)
    );
}

struct Counted {
    id: usize,
    drops: Arc<Vec<AtomicUsize>>,
}
impl Drop for Counted {
    fn drop(&mut self) {
        self.drops[self.id].fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mailbox_multithreaded_producers_preserve_unique_messages_and_joined_cleanup() {
    const PRODUCERS: usize = 4;
    const EACH: usize = 256;
    for capacity in [1, 2, 7] {
        let drops = Arc::new(
            (0..PRODUCERS * EACH)
                .map(|_| AtomicUsize::new(0))
                .collect::<Vec<_>>(),
        );
        let (tx, mut rx) = channel::<Counted>(capacity).unwrap();
        let mut producers = tokio::task::JoinSet::new();
        for producer in 0..PRODUCERS {
            let sender = tx.clone();
            let drops = Arc::clone(&drops);
            producers.spawn(async move {
                for index in 0..EACH {
                    sender
                        .send(Counted {
                            id: producer * EACH + index,
                            drops: Arc::clone(&drops),
                        })
                        .await
                        .unwrap();
                    if index % 7 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            });
        }
        drop(tx);
        let result = tokio::time::timeout(Duration::from_secs(15), async {
            let mut received = vec![0_usize; PRODUCERS * EACH];
            while let Some(value) = rx.recv().await {
                received[value.id] += 1;
                assert_eq!(received[value.id], 1, "duplicate delivery");
                tokio::task::yield_now().await;
            }
            received
        })
        .await;
        if result.is_err() {
            producers.abort_all();
            rx.close();
        }
        let mut failures = 0;
        while let Some(joined) = producers.join_next().await {
            failures += usize::from(joined.is_err());
        }
        assert_eq!(failures, 0, "all producer tasks must finish and be joined");
        assert_eq!(result.unwrap(), vec![1; PRODUCERS * EACH]);
        assert!(drops.iter().all(|count| count.load(Ordering::SeqCst) == 1));
    }
}

#[test]
fn mailbox_close_commit_races_never_duplicate_or_strand_payloads() {
    // Real threads exercise both sides of the same custody lock. This is bounded
    // stress, not exhaustive scheduler, fairness or memory-model verification.
    for iteration in 0..128 {
        let drops = Arc::new(vec![AtomicUsize::new(0)]);
        let (tx, rx) = channel::<Counted>(1).unwrap();
        let permit = tx.try_reserve().unwrap();
        let start = Arc::new(std::sync::Barrier::new(2));
        let sender_start = Arc::clone(&start);
        let counts = Arc::clone(&drops);
        let sender = std::thread::spawn(move || {
            sender_start.wait();
            if iteration % 2 == 0 {
                std::thread::yield_now();
            }
            permit.send(Counted {
                id: 0,
                drops: counts,
            })
        });
        start.wait();
        if iteration % 2 != 0 {
            std::thread::yield_now();
        }
        drop(rx);
        let result = sender.join().unwrap();
        match result {
            Ok(()) => assert_eq!(drops[0].load(Ordering::SeqCst), 1),
            Err(unaccepted) => {
                assert_eq!(drops[0].load(Ordering::SeqCst), 0);
                drop(unaccepted);
            }
        }
        assert_eq!(drops[0].load(Ordering::SeqCst), 1);
        assert_eq!(tx.try_reserve().err(), Some(ReserveError::Closed));
    }
}

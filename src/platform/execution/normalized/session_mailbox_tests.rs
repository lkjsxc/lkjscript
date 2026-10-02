//! Exercise the actual session batching path, not just the generic mailbox.
use super::*;
use std::task::{Context, Poll, Waker};

#[tokio::test]
async fn session_mailbox_closed_reserved_batch_refuses_without_waiting_for_a_completion_timeout() {
    let (tx, rx) = mailbox::channel(1).unwrap();
    let reserved = tx.reserve().await.unwrap();
    let text: Arc<str> = Arc::from("not accepted");
    let retained = Arc::downgrade(&text);
    drop(rx);
    let mut send = Box::pin(send_reserved(
        reserved,
        vec![SessionOutbound::Text(text)],
        None,
        Duration::from_secs(30),
    ));
    assert_eq!(
        send.as_mut().poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(()))
    );
    drop(send);
    assert!(retained.upgrade().is_none());
}

#[tokio::test]
async fn session_mailbox_lost_completion_does_not_revoke_or_repeat_an_accepted_batch() {
    let (tx, mut rx) = mailbox::channel(1).unwrap();
    let reserved = tx.reserve().await.unwrap();
    let text: Arc<str> = Arc::from("accepted once");
    let pointer = text.as_ptr() as usize;
    let retained = Arc::downgrade(&text);
    let mut send = Box::pin(send_reserved(
        reserved,
        vec![SessionOutbound::Text(text)],
        None,
        Duration::from_secs(30),
    ));
    assert!(
        send.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(send); // Acceptance committed; only the completion observer is cancelled.
    assert!(retained.upgrade().is_some());
    let command = rx.recv().await.unwrap();
    let SessionOutbound::Text(text) = &command.messages[0] else {
        panic!("text batch");
    };
    assert_eq!(text.as_ptr() as usize, pointer);
    assert!(command.finished.send(true).is_err());
    drop(command.messages);
    assert!(retained.upgrade().is_none());
    drop(tx);
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn session_mailbox_cancelled_capacity_wait_disposes_the_unaccepted_batch() {
    let (tx, mut rx) = mailbox::channel(1).unwrap();
    let reserved = tx.reserve().await.unwrap();
    let text: Arc<str> = Arc::from("capacity wait");
    let retained = Arc::downgrade(&text);
    let mut send = Box::pin(send_writer(
        &tx,
        vec![SessionOutbound::Text(text)],
        None,
        Duration::from_secs(30),
    ));
    assert!(
        send.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(send);
    assert!(retained.upgrade().is_none());
    drop(reserved);
    drop(tx);
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn session_mailbox_aborted_writer_is_joined_and_releases_its_dequeued_batch() {
    let (tx, mut rx) = mailbox::channel(1).unwrap();
    let reserved = tx.reserve().await.unwrap();
    let text: Arc<str> = Arc::from("active writer");
    let retained = Arc::downgrade(&text);
    let mut send = Box::pin(send_reserved(
        reserved,
        vec![SessionOutbound::Text(text)],
        None,
        Duration::from_secs(30),
    ));
    assert!(
        send.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let (dequeued, observed) = oneshot::channel();
    let child = tokio::spawn(async move {
        let command = rx.recv().await.unwrap();
        let _ = dequeued.send(());
        std::future::pending::<()>().await;
        drop(command);
    });
    let observation = tokio::time::timeout(Duration::from_secs(5), observed).await;
    let before_join = retained.upgrade().is_some();
    join_child(child, Duration::ZERO).await; // Existing lifecycle must abort AND await.
    assert!(observation.unwrap().is_ok());
    assert!(before_join);
    assert!(retained.upgrade().is_none());
    assert_eq!(send.await, Err(()));
    assert_eq!(tx.try_reserve().err(), Some(mailbox::ReserveError::Closed));
    let (unrelated, mut other) = mailbox::channel(1).unwrap();
    unrelated.send(83).await.unwrap();
    assert_eq!(other.recv().await, Some(83));
}

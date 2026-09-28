//! Pause the real admission prefix before queue accounting, not a timing-only stress test.
use super::*;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

pub(super) struct RegistrationPause {
    entered: SyncSender<()>,
    resume: Receiver<()>,
}

pub(super) fn pause_before_registration(inner: &ResidentKernelInner) {
    let pause = lock_unpoisoned(&inner.registration_pause).take();
    if let Some(pause) = pause {
        pause.entered.send(()).expect("registration entered");
        pause
            .resume
            .recv_timeout(Duration::from_secs(10))
            .expect("bounded registration resume");
    }
}

struct Capture(Arc<AtomicBool>);

impl Drop for Capture {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[test]
fn stop_cannot_overtake_accepted_capture_registration() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let kernel = ResidentKernel::new(ResidentLimits::default()).unwrap();
    let (entered, paused) = sync_channel(1);
    let (resume, proceed) = sync_channel(1);
    *lock_unpoisoned(&kernel.inner.registration_pause) = Some(RegistrationPause {
        entered,
        resume: proceed,
    });
    let dropped = Arc::new(AtomicBool::new(false));
    let capture = Capture(dropped.clone());
    let invoking = kernel.clone();
    let invocation = runtime.spawn(async move {
        invoking
            .invoke(move |_| {
                drop(capture);
                Ok(7)
            })
            .await
    });
    paused
        .recv_timeout(Duration::from_secs(10))
        .expect("paused after acceptance, before queue registration");
    assert!(!dropped.load(Ordering::Acquire));
    let cleaned_after_drop = Arc::new(AtomicBool::new(false));
    let cleanup_observation = cleaned_after_drop.clone();
    let cleanup_capture = dropped.clone();
    let stopping = kernel.clone();
    let (completed, completion) = sync_channel(1);
    let shutdown = runtime.spawn(async move {
        let receipt = stopping
            .shutdown(move || {
                cleanup_observation
                    .store(cleanup_capture.load(Ordering::Acquire), Ordering::Release);
                Vec::new()
            })
            .await;
        completed.send(()).expect("shutdown completed");
        receipt
    });
    let overtook_registration = completion.recv_timeout(Duration::from_millis(500)).is_ok();
    // Always release and join both owners before making the regression assertion.
    resume.send(()).expect("resume registration");
    let (result, receipt) = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(10), async {
            let result = invocation.await.unwrap();
            let receipt = shutdown.await.unwrap();
            (result, receipt)
        })
        .await
        .expect("joined invocation and shutdown")
    });
    // Admission can win or lose the subsequent worker race; either outcome must
    // release the capture before adapter cleanup, and may not disappear from drain.
    if let Err(error) = result {
        assert_eq!(error.code, "resident_shutting_down");
    }
    assert!(dropped.load(Ordering::Acquire));
    assert!(
        !overtook_registration,
        "shutdown reported idle while an accepted invocation still owned its capture"
    );
    assert!(
        cleaned_after_drop.load(Ordering::Acquire),
        "adapter cleanup ran before the accepted capture was released"
    );
    assert!(receipt.drained_before_cancellation);
    assert_eq!(receipt.remaining_tasks, 0);
    assert!(receipt.cleanup_failures.is_empty());
    assert!(!kernel.observe().accepting);
    assert_eq!(kernel.observe().queued, 0);
    assert_eq!(kernel.observe().active, 0);
    assert_eq!(kernel.observe_permits().admission_permits, 0);
    assert_eq!(kernel.observe_permits().worker_permits, 0);
}

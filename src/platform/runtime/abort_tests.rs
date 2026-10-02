use super::*;
use std::future::Future;
use std::task::{Context, Poll, Waker};

struct ShutdownResource(Arc<AtomicBool>);

impl Drop for ShutdownResource {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

struct ReleaseOnDrop(Option<std::sync::mpsc::Sender<()>>);

impl ReleaseOnDrop {
    fn release(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.release();
    }
}

fn observe_cleanup(
    resource_dropped: Arc<AtomicBool>,
    cleanups: Arc<AtomicUsize>,
    premature: Arc<AtomicBool>,
) -> impl FnOnce() -> Vec<ExecutionError> {
    move || {
        if !resource_dropped.load(Ordering::Acquire) {
            premature.store(true, Ordering::Release);
        }
        cleanups.fetch_add(1, Ordering::AcqRel);
        Vec::new()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_grace_expiry_joins_before_cleanup_even_after_waiter_cancellation() {
    for cancel_first_shutdown in [false, true] {
        let kernel = ResidentKernel::new(ResidentLimits {
            maximum_concurrent_tasks: 1,
            maximum_queued_tasks: 0,
            shutdown_grace_milliseconds: 1,
            cancellation_grace_milliseconds: 1,
            ..ResidentLimits::default()
        })
        .expect("kernel");
        let resource_dropped = Arc::new(AtomicBool::new(false));
        let resource = ShutdownResource(resource_dropped.clone());
        let cleanups = Arc::new(AtomicUsize::new(0));
        let premature = Arc::new(AtomicBool::new(false));
        let (started, running) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let mut release = ReleaseOnDrop(Some(release));
        let executing = kernel.clone();
        let invocation = tokio::spawn(async move {
            executing
                .invoke(move |control| {
                    let _resource = resource;
                    let _ = started.send(control);
                    held.recv_timeout(Duration::from_secs(5))
                        .expect("bounded callback release");
                    Ok(42)
                })
                .await
        });
        let startup = tokio::time::timeout(Duration::from_secs(1), running).await;
        let mut shutdown = Box::pin(kernel.shutdown(observe_cleanup(
            resource_dropped.clone(),
            cleanups.clone(),
            premature.clone(),
        )));
        let mut context = Context::from_waker(Waker::noop());
        let mut early = None;
        // Drive both finite grace phases explicitly. Each sleep completes after
        // the corresponding 1ms timer, without relying on worker scheduling.
        for _ in 0..3 {
            if early.is_none()
                && let Poll::Ready(receipt) = shutdown.as_mut().poll(&mut context)
            {
                early = Some(receipt);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let remained_pending = early.is_none();
        let cleaned_while_owned = cleanups.load(Ordering::Acquire);
        let active_while_owned = kernel.observe().active;
        let cancelled = startup
            .as_ref()
            .ok()
            .and_then(|result| result.as_ref().ok())
            .is_some_and(ExecutionControl::is_cancelled);
        let (receipt, repeated_pending, joined) = if cancel_first_shutdown {
            drop(shutdown);
            let mut repeated = Box::pin(kernel.shutdown(observe_cleanup(
                resource_dropped.clone(),
                cleanups.clone(),
                premature.clone(),
            )));
            let before_release = repeated.as_mut().poll(&mut context);
            let repeated_pending = before_release.is_pending();
            release.release();
            let joined = invocation.await;
            let receipt = match before_release {
                Poll::Ready(receipt) => receipt,
                Poll::Pending => repeated.await,
            };
            (receipt, repeated_pending, joined)
        } else {
            release.release();
            let joined = invocation.await;
            let receipt = match early {
                Some(receipt) => receipt,
                None => shutdown.await,
            };
            (receipt, true, joined)
        };
        let repeated = kernel.shutdown(Vec::new).await;
        // All assertions follow release and join, including the old-code failure.
        assert!(startup.is_ok());
        assert!(cancelled);
        assert!(remained_pending, "grace expiry must not abandon owned work");
        assert!(repeated_pending, "a replacement shutdown must still join");
        assert_eq!(cleaned_while_owned, 0);
        assert_eq!(active_while_owned, 1);
        assert_eq!(
            joined.expect("join invocation").expect("callback").value,
            42
        );
        assert!(resource_dropped.load(Ordering::Acquire));
        assert!(!premature.load(Ordering::Acquire));
        assert_eq!(cleanups.load(Ordering::Acquire), 1);
        assert_eq!(receipt.remaining_tasks, 0);
        assert_eq!(repeated.remaining_tasks, 0);
        assert!(repeated.cleanup_failures.is_empty());
        if !cancel_first_shutdown {
            assert_eq!(receipt.cleanup_failures.len(), 1);
            assert_eq!(
                receipt.cleanup_failures[0].code,
                "resident_cancellation_stalled"
            );
            assert_eq!(
                receipt.cleanup_failures[0].class,
                ExecutionFailureClass::Infrastructure
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborting_queued_task_does_not_disturb_the_running_worker() {
    let kernel = ResidentKernel::new(ResidentLimits {
        maximum_concurrent_tasks: 1,
        maximum_queued_tasks: 1,
        shutdown_grace_milliseconds: 100,
        cancellation_grace_milliseconds: 100,
        ..ResidentLimits::default()
    })
    .expect("kernel");
    let (started, running) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let mut active = Box::pin(kernel.invoke(move |_| {
        started.send(()).expect("announce running worker");
        held.recv_timeout(Duration::from_secs(5))
            .expect("release worker");
        Ok(42)
    }));
    let mut context = Context::from_waker(Waker::noop());
    assert!(active.as_mut().poll(&mut context).is_pending());
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("worker startup")
        .expect("started");
    let called = Arc::new(AtomicBool::new(false));
    let observed = called.clone();
    let clone = kernel.clone();
    let mut queued = Box::pin(async move {
        clone
            .invoke(move |_| {
                observed.store(true, Ordering::Release);
                Ok(())
            })
            .await
    });
    assert!(queued.as_mut().poll(&mut context).is_pending());
    assert_eq!((kernel.observe().queued, kernel.observe().active), (1, 1));
    let task = tokio::spawn(queued);
    task.abort();
    let cancelled = task.await.err().expect("aborted task");
    // Always release and join the real worker before checking cancellation state.
    release.send(()).expect("release active worker");
    let result = active.await.expect("active invocation remains valid");
    let observation = kernel.observe();
    let permits = kernel.observe_permits();
    let shutdown = kernel.shutdown(Vec::new).await;
    assert!(cancelled.is_cancelled());
    assert!(!called.load(Ordering::Acquire));
    assert_eq!(result.value, 42);
    assert_eq!((observation.queued, observation.active), (0, 0));
    assert_eq!(observation.completed, 1);
    assert_eq!((permits.admission_permits, permits.worker_permits), (0, 0));
    assert!(shutdown.drained_before_cancellation);
    assert_eq!(shutdown.remaining_tasks, 0);
}

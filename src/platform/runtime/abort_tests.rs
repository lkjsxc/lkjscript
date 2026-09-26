use super::*;
use std::future::Future;
use std::task::{Context, Waker};

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

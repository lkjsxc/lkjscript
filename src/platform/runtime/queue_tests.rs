use super::*;
use std::future::Future;
use std::task::{Context, Poll, Wake, Waker};

fn kernel() -> ResidentKernel {
    ResidentKernel::new(ResidentLimits {
        maximum_concurrent_tasks: 1,
        maximum_queued_tasks: 2,
        shutdown_grace_milliseconds: 100,
        cancellation_grace_milliseconds: 100,
        ..ResidentLimits::default()
    })
    .expect("kernel")
}

fn poll<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

async fn assert_idle(kernel: &ResidentKernel) {
    let observation = kernel.observe();
    assert_eq!((observation.queued, observation.active), (0, 0));
    let permits = kernel.observe_permits();
    assert_eq!((permits.admission_permits, permits.worker_permits), (0, 0));
    let shutdown = kernel.shutdown(Vec::new).await;
    assert!(shutdown.drained_before_cancellation);
    assert_eq!(shutdown.remaining_tasks, 0);
    assert!(shutdown.cleanup_failures.is_empty());
}

struct CapturedResource {
    inner: Arc<ResidentKernelInner>,
    queued_on_drop: Arc<AtomicUsize>,
}

impl Drop for CapturedResource {
    fn drop(&mut self) {
        self.queued_on_drop
            .store(self.inner.queued.load(Ordering::Acquire), Ordering::Release);
    }
}

#[tokio::test]
async fn cancelled_queue_releases_captures_before_reporting_idle() {
    let kernel = kernel();
    let occupied = kernel.inner.workers.clone().acquire_owned().await.unwrap();
    let observed = Arc::new(AtomicUsize::new(usize::MAX));
    let resource = CapturedResource {
        inner: kernel.inner.clone(),
        queued_on_drop: observed.clone(),
    };
    let mut queued = Box::pin(kernel.invoke(move |_| {
        let _resource = resource;
        Ok(())
    }));
    assert!(poll(queued.as_mut()).is_pending());
    drop(queued);
    assert_eq!(observed.load(Ordering::Acquire), 1);
    drop(occupied);
    assert_idle(&kernel).await;
}

#[tokio::test]
async fn repeated_queue_cancellation_restores_admission_capacity() {
    let kernel = kernel();
    let occupied = kernel.inner.workers.clone().acquire_owned().await.unwrap();
    for _ in 0..128 {
        let mut queued = Box::pin(kernel.invoke(|_| Ok(())));
        assert!(poll(queued.as_mut()).is_pending());
        assert_eq!(kernel.observe().queued, 1);
        drop(queued);
        assert_eq!(kernel.observe().queued, 0);
        assert_eq!(kernel.observe_permits().admission_permits, 0);
    }
    drop(occupied);
    let next = kernel.invoke(|_| Ok(42)).await.expect("next invocation");
    assert_eq!(next.value, 42);
    assert_eq!(next.task_id, 1, "cancelled waiters never became executions");
    assert_eq!(kernel.observe().admitted, 129);
    assert_eq!(kernel.observe().completed, 1);
    assert_idle(&kernel).await;
}

#[tokio::test]
async fn cancelling_a_waiter_wakes_an_already_waiting_shutdown() {
    let kernel = kernel();
    let occupied = kernel.inner.workers.clone().acquire_owned().await.unwrap();
    let mut queued = Box::pin(kernel.invoke(|_| Ok(())));
    assert!(poll(queued.as_mut()).is_pending());
    let cleanups = Arc::new(AtomicUsize::new(0));
    let calls = cleanups.clone();
    let mut shutdown = Box::pin(kernel.shutdown(move || {
        calls.fetch_add(1, Ordering::AcqRel);
        Vec::new()
    }));
    assert!(poll(shutdown.as_mut()).is_pending());
    assert_eq!(cleanups.load(Ordering::Acquire), 0);
    drop(queued);
    drop(occupied);
    let receipt = tokio::time::timeout(Duration::from_secs(1), shutdown)
        .await
        .expect("shutdown notification");
    assert!(receipt.drained_before_cancellation);
    assert_eq!(receipt.remaining_tasks, 0);
    assert_eq!(cleanups.load(Ordering::Acquire), 1);
    assert_idle(&kernel).await;
    assert_eq!(cleanups.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn identity_exhaustion_releases_pending_work_without_execution() {
    let kernel = kernel();
    kernel.inner.next_task.store(u64::MAX, Ordering::Release);
    let result = kernel
        .invoke::<(), _>(|_| panic!("exhausted invocation ran"))
        .await;
    assert_eq!(
        result.err().expect("exhausted identity").code,
        "resident_task_identity_exhausted"
    );
    assert_eq!(kernel.observe().completed, 0);
    assert_idle(&kernel).await;
}

#[tokio::test]
async fn closing_workers_rejects_and_releases_queued_work() {
    let kernel = kernel();
    let occupied = kernel.inner.workers.clone().acquire_owned().await.unwrap();
    let mut queued = Box::pin(kernel.invoke(|_| Ok(())));
    assert!(poll(queued.as_mut()).is_pending());
    kernel.inner.workers.close();
    let result = queued.await;
    assert_eq!(
        result.err().expect("closed workers").code,
        "resident_shutting_down"
    );
    drop(occupied);
    assert_eq!(kernel.observe().completed, 0);
    assert_idle(&kernel).await;
}

struct ActivationWake {
    inner: Arc<ResidentKernelInner>,
    active_on_first_wake: AtomicUsize,
}

impl Wake for ActivationWake {
    fn wake(self: Arc<Self>) {
        let _ = self.active_on_first_wake.compare_exchange(
            usize::MAX,
            self.inner.active.load(Ordering::Acquire),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_to_active_handoff_never_notifies_a_false_idle_gap() {
    let kernel = kernel();
    let occupied = kernel.inner.workers.clone().acquire_owned().await.unwrap();
    let (release, held) = std::sync::mpsc::channel();
    let mut invocation = Box::pin(kernel.invoke(move |_| {
        held.recv_timeout(Duration::from_secs(2))
            .expect("release worker");
        Ok(())
    }));
    assert!(poll(invocation.as_mut()).is_pending());
    let wake = Arc::new(ActivationWake {
        inner: kernel.inner.clone(),
        active_on_first_wake: AtomicUsize::new(usize::MAX),
    });
    let waker = Waker::from(wake.clone());
    let mut notification = Box::pin(kernel.inner.idle.notified());
    assert!(
        notification
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    drop(occupied);
    assert!(poll(invocation.as_mut()).is_pending());
    let active_on_wake = wake.active_on_first_wake.load(Ordering::Acquire);
    // Release and join before assertions, including an intentional mutation failure.
    release.send(()).expect("release");
    invocation.await.expect("invocation");
    assert_eq!(
        active_on_wake, 1,
        "queue removal must already have an active owner"
    );
    assert_idle(&kernel).await;
}

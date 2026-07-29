mod common;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Barrier, Mutex};
use std::time::Duration;

use common::*;
use lkjscript_resource::*;

struct CountingExecutor {
    counts: Vec<AtomicUsize>,
    failures: BTreeMap<TaskId, String>,
    delay: Duration,
    failure_barrier: Option<Barrier>,
}
impl CountingExecutor {
    fn new(tasks: usize) -> Self {
        Self {
            counts: (0..tasks).map(|_| AtomicUsize::new(0)).collect(),
            failures: BTreeMap::new(),
            delay: Duration::from_millis(0),
            failure_barrier: None,
        }
    }
}
impl TaskExecutor for CountingExecutor {
    type Output = Vec<u8>;
    type Error = String;

    fn execute(&self, task: TaskId, _worker: WorkerId) -> Result<Self::Output, String> {
        std::thread::sleep(self.delay);
        let Some(count) = self.counts.get(task.slot as usize) else {
            return Err("unknown-task".to_owned());
        };
        count.fetch_add(1, Ordering::SeqCst);
        if self.failures.contains_key(&task) {
            if let Some(barrier) = &self.failure_barrier {
                barrier.wait();
            }
        }
        self.failures
            .get(&task)
            .cloned()
            .map_or_else(|| Ok(vec![task.slot as u8]), Err)
    }
}

struct CountingBinder(Mutex<Vec<WorkerId>>);
impl WorkerBinder for CountingBinder {
    fn bind(&self, worker: WorkerId, _allowed: &CpuSet) -> ResourceResult<()> {
        self.0
            .lock()
            .map_err(|_| ResourceError::new("poison", "binder poisoned"))?
            .push(worker);
        Ok(())
    }
}

fn config(workers: usize, capacity: usize) -> ResourceResult<RuntimeConfig> {
    Ok(RuntimeConfig {
        workers: (0..workers)
            .map(|slot| {
                Ok(WorkerDescriptor {
                    id: WorkerId::new(slot as u32, 1),
                    allowed: cpus(&[slot as u32])?,
                    group: WorkerGroupId::new(0, 1),
                    numa_node: Some(0),
                })
            })
            .collect::<ResourceResult<Vec<_>>>()?,
        queue_capacity: capacity,
        spin_limit: 0,
        policy: SchedulePolicy::GlobalFifo,
    })
}

#[test]
fn scoped_runtime_executes_exactly_once_steals_parks_and_joins() -> ResourceResult<()> {
    let dag = graph(&[
        vec![],
        vec![],
        vec![],
        vec![],
        vec![id(0), id(1), id(2), id(3)],
    ])?;
    let mut executor = CountingExecutor::new(5);
    executor.delay = Duration::from_millis(3);
    let binder = CountingBinder(Mutex::new(Vec::new()));
    let report = ScopedRuntime::run(&dag, config(4, 8)?, &executor, &binder)?;
    assert_eq!(report.outputs.len(), 5);
    assert!(executor
        .counts
        .iter()
        .all(|count| count.load(Ordering::SeqCst) == 1));
    assert!(report.metrics.steals > 0);
    assert!(report.metrics.parks > 0);
    assert_eq!(report.metrics.active_workers, 0);
    assert_eq!(
        binder
            .0
            .lock()
            .map_err(|_| ResourceError::new("poison", "test binder"))?
            .len(),
        4
    );
    Ok(())
}

#[test]
fn runtime_reports_stable_failures_and_cancels_descendants() -> ResourceResult<()> {
    let dag = graph(&[vec![], vec![], vec![id(0)], vec![id(1)], vec![id(2), id(3)]])?;
    let mut executor = CountingExecutor::new(5);
    executor.failures = BTreeMap::from([(id(0), "zero".to_owned()), (id(1), "one".to_owned())]);
    executor.delay = Duration::from_millis(2);
    executor.failure_barrier = Some(Barrier::new(2));
    let report = ScopedRuntime::run(&dag, config(2, 8)?, &executor, &NoopWorkerBinder)?;
    assert_eq!(report.selected_failure, Some((id(0), "zero".to_owned())));
    assert_eq!(
        report.failures,
        vec![(id(0), "zero".to_owned()), (id(1), "one".to_owned())]
    );
    assert!(report.cancelled.contains(&id(4)));
    assert_eq!(report.metrics.active_workers, 0);
    Ok(())
}

#[test]
fn runtime_rejects_queue_full_and_stale_descriptors() -> ResourceResult<()> {
    let dag = graph(&[vec![], vec![], vec![]])?;
    let executor = CountingExecutor::new(3);
    assert_eq!(
        ScopedRuntime::run(&dag, config(2, 2)?, &executor, &NoopWorkerBinder)
            .map_err(|error| error.code),
        Err("queue-full")
    );
    let mut stale = config(1, 4)?;
    stale.workers[0].id.generation = 0;
    assert_eq!(
        ScopedRuntime::run(&dag, stale, &executor, &NoopWorkerBinder).map_err(|error| error.code),
        Err("worker-id")
    );
    Ok(())
}

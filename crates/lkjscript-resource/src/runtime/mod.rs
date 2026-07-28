use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crate::{
    CpuSet, ResourceError, ResourceResult, SchedulePolicy, TaskId, VerifiedTaskGraph,
    WorkerGroupId, WorkerId,
};

mod model;
mod worker;
pub use model::{RuntimeConfig, RuntimeMetrics, RuntimeReport, WorkerDescriptor};
use worker::{validate_config, worker_loop};

pub trait TaskExecutor: Sync {
    type Output: Send;
    type Error: Clone + Send;

    fn execute(&self, task: TaskId, worker: WorkerId) -> Result<Self::Output, Self::Error>;
}
pub trait WorkerBinder: Sync {
    fn bind(&self, worker: WorkerId, allowed: &CpuSet) -> ResourceResult<()>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopWorkerBinder;
impl WorkerBinder for NoopWorkerBinder {
    fn bind(&self, _worker: WorkerId, _allowed: &CpuSet) -> ResourceResult<()> {
        Ok(())
    }
}
pub(crate) struct Control<O, E> {
    pub(crate) dependencies: BTreeMap<TaskId, usize>,
    pub(crate) successors: BTreeMap<TaskId, Vec<TaskId>>,
    pub(crate) attempted: BTreeSet<TaskId>,
    pub(crate) completed: BTreeSet<TaskId>,
    pub(crate) outputs: BTreeMap<TaskId, O>,
    pub(crate) failures: BTreeMap<TaskId, E>,
    pub(crate) remaining: usize,
    pub(crate) shutdown: bool,
    pub(crate) wake_epoch: u64,
    pub(crate) metrics: RuntimeMetrics,
}
pub(crate) struct Shared<O, E> {
    pub(crate) queues: Vec<Mutex<VecDeque<TaskId>>>,
    pub(crate) control: Mutex<Control<O, E>>,
    pub(crate) wake: Condvar,
    pub(crate) queue_capacity: usize,
    pub(crate) policy: SchedulePolicy,
    pub(crate) worker_masks: Vec<CpuSet>,
    pub(crate) worker_groups: Vec<WorkerGroupId>,
    pub(crate) worker_numa: Vec<Option<u32>>,
    pub(crate) preferred: BTreeMap<TaskId, usize>,
}

pub struct ScopedRuntime;

impl ScopedRuntime {
    pub fn run<E: TaskExecutor, B: WorkerBinder>(
        graph: &VerifiedTaskGraph,
        config: RuntimeConfig,
        executor: &E,
        binder: &B,
    ) -> ResourceResult<RuntimeReport<E::Output, E::Error>> {
        validate_config(&config)?;
        let mut dependencies = BTreeMap::new();
        let mut successors: BTreeMap<TaskId, Vec<TaskId>> = BTreeMap::new();
        for task in graph.tasks() {
            dependencies.insert(task.id, task.dependencies.len());
            for dependency in &task.dependencies {
                successors.entry(*dependency).or_default().push(task.id);
            }
        }
        for list in successors.values_mut() {
            list.sort();
        }
        let shared = Arc::new(Shared {
            queues: (0..config.workers.len())
                .map(|_| Mutex::new(VecDeque::new()))
                .collect(),
            control: Mutex::new(Control {
                dependencies,
                successors,
                attempted: BTreeSet::new(),
                completed: BTreeSet::new(),
                outputs: BTreeMap::new(),
                failures: BTreeMap::new(),
                remaining: graph.tasks().len(),
                shutdown: false,
                wake_epoch: 1,
                metrics: RuntimeMetrics::default(),
            }),
            wake: Condvar::new(),
            queue_capacity: config.queue_capacity,
            policy: config.policy,
            worker_masks: config
                .workers
                .iter()
                .map(|worker| worker.allowed.clone())
                .collect(),
            worker_groups: config.workers.iter().map(|worker| worker.group).collect(),
            worker_numa: config
                .workers
                .iter()
                .map(|worker| worker.numa_node)
                .collect(),
            preferred: graph
                .tasks()
                .iter()
                .map(|task| {
                    let slot = match config.policy {
                        SchedulePolicy::OwnerCompute => task.result_owner.slot,
                        _ => task.id.slot,
                    };
                    (task.id, slot as usize % config.workers.len())
                })
                .collect(),
        });
        let roots: Vec<_> = graph
            .tasks()
            .iter()
            .filter(|task| task.dependencies.is_empty())
            .map(|task| task.id)
            .collect();
        crate::runtime_support::enqueue_ready(&shared, 0, &roots)?;
        thread::scope(|scope| -> ResourceResult<()> {
            let mut handles = Vec::new();
            for (index, descriptor) in config.workers.iter().enumerate() {
                let shared = Arc::clone(&shared);
                handles.push(scope.spawn(move || {
                    worker_loop(
                        index,
                        descriptor,
                        config.spin_limit,
                        &shared,
                        executor,
                        binder,
                    )
                }));
            }
            for handle in handles {
                handle
                    .join()
                    .map_err(|_| ResourceError::new("worker-panic", "scoped worker panicked"))??;
            }
            Ok(())
        })?;
        let shared = Arc::try_unwrap(shared).map_err(|_| {
            ResourceError::new("runtime-live-share", "worker retained runtime state")
        })?;
        let control = shared
            .control
            .into_inner()
            .map_err(|_| ResourceError::new("poison", "runtime control poisoned"))?;
        let cancelled = graph
            .tasks()
            .iter()
            .map(|task| task.id)
            .filter(|task| {
                !control.completed.contains(task) && !control.failures.contains_key(task)
            })
            .collect();
        if control.attempted.len() != control.metrics.executed as usize
            || control.metrics.active_workers != 0
        {
            return Err(ResourceError::new(
                "runtime-invariant",
                "exactly-once or live-worker invariant failed",
            ));
        }
        let failures: Vec<_> = control.failures.into_iter().collect();
        let selected_failure = failures.first().cloned();
        Ok(RuntimeReport {
            outputs: control.outputs.into_iter().collect(),
            failures,
            selected_failure,
            cancelled,
            metrics: control.metrics,
        })
    }
}

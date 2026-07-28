use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crate::runtime_support::enqueue_many;
use crate::{CpuSet, ResourceError, ResourceResult, TaskId, VerifiedTaskGraph, WorkerId};

mod worker;
use worker::{validate_config, worker_loop};

pub trait TaskExecutor: Sync {
    type Output: Clone + Send;
    type Error: Clone + Send;

    fn execute(&self, task: TaskId) -> Result<Self::Output, Self::Error>;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerDescriptor {
    pub id: WorkerId,
    pub allowed: CpuSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeConfig {
    pub workers: Vec<WorkerDescriptor>,
    pub queue_capacity: usize,
    pub spin_limit: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeMetrics {
    pub executed: u64,
    pub steals: u64,
    pub spins: u64,
    pub parks: u64,
    pub queue_high_water: usize,
    pub active_workers: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeReport<O, E> {
    pub outputs: Vec<(TaskId, O)>,
    pub failures: Vec<(TaskId, E)>,
    pub selected_failure: Option<(TaskId, E)>,
    pub cancelled: Vec<TaskId>,
    pub metrics: RuntimeMetrics,
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
        });
        let roots: Vec<_> = graph
            .tasks()
            .iter()
            .filter(|task| task.dependencies.is_empty())
            .map(|task| task.id)
            .collect();
        enqueue_many(&shared, 0, &roots)?;
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
        let control = shared
            .control
            .lock()
            .map_err(|_| ResourceError::new("poison", "runtime control poisoned"))?;
        let failures: Vec<_> = control
            .failures
            .iter()
            .map(|(task, detail)| (*task, detail.clone()))
            .collect();
        let selected_failure = failures.first().cloned();
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
        Ok(RuntimeReport {
            outputs: control
                .outputs
                .iter()
                .map(|(task, output)| (*task, output.clone()))
                .collect(),
            failures,
            selected_failure,
            cancelled,
            metrics: control.metrics,
        })
    }
}

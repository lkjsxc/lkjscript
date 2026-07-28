use std::collections::BTreeSet;
use std::thread;

use super::{RuntimeConfig, Shared, TaskExecutor, WorkerBinder, WorkerDescriptor};
use crate::runtime_support::{
    complete_task, enqueue_ready, lock_control, take_task, update_active,
};
use crate::{ResourceError, ResourceResult};

pub(super) fn validate_config(config: &RuntimeConfig) -> ResourceResult<()> {
    if config.workers.is_empty()
        || config.queue_capacity == 0
        || config.policy == crate::SchedulePolicy::Sequential && config.workers.len() != 1
    {
        return Err(ResourceError::new(
            "runtime-config",
            "workers and queue capacity required",
        ));
    }
    let mut slots = BTreeSet::new();
    if config
        .workers
        .iter()
        .any(|worker| worker.id.generation == 0 || !slots.insert(worker.id.slot))
    {
        return Err(ResourceError::new(
            "worker-id",
            "stale or duplicate worker descriptor",
        ));
    }
    Ok(())
}

pub(super) fn worker_loop<E: TaskExecutor, B: WorkerBinder>(
    index: usize,
    descriptor: &WorkerDescriptor,
    spin_limit: usize,
    shared: &Shared<E::Output, E::Error>,
    executor: &E,
    binder: &B,
) -> ResourceResult<()> {
    binder.bind(descriptor.id, &descriptor.allowed)?;
    update_active(shared, true)?;
    let result = worker_tasks(index, descriptor.id, spin_limit, shared, executor);
    update_active(shared, false)?;
    result
}

fn worker_tasks<E: TaskExecutor>(
    index: usize,
    worker: crate::WorkerId,
    spin_limit: usize,
    shared: &Shared<E::Output, E::Error>,
    executor: &E,
) -> ResourceResult<()> {
    let mut spins = 0;
    let mut seen_epoch = 0;
    loop {
        if let Some((task, victim)) = take_task(shared, index)? {
            spins = 0;
            {
                let mut control = lock_control(shared)?;
                if control.shutdown {
                    break;
                }
                if !control.attempted.insert(task) {
                    return Err(ResourceError::new("exactly-once", "task dequeued twice"));
                }
                control.metrics.executed = control.metrics.executed.saturating_add(1);
                if let Some(victim) = victim {
                    control.metrics.steals = control.metrics.steals.saturating_add(1);
                    if shared.worker_groups[index] == shared.worker_groups[victim] {
                        control.metrics.same_group_steals =
                            control.metrics.same_group_steals.saturating_add(1);
                    } else {
                        control.metrics.cross_group_steals =
                            control.metrics.cross_group_steals.saturating_add(1);
                    }
                    if matches!(
                        (shared.worker_numa[index], shared.worker_numa[victim]),
                        (Some(left), Some(right)) if left != right
                    ) {
                        control.metrics.cross_numa_steals =
                            control.metrics.cross_numa_steals.saturating_add(1);
                    }
                }
            }
            let ready = complete_task(shared, task, executor.execute(task, worker))?;
            if !ready.is_empty() {
                enqueue_ready(shared, index, &ready)?;
            }
            continue;
        }
        let mut control = lock_control(shared)?;
        if control.shutdown {
            break;
        }
        if control.wake_epoch != seen_epoch {
            seen_epoch = control.wake_epoch;
            continue;
        }
        if spins < spin_limit {
            spins += 1;
            control.metrics.spins = control.metrics.spins.saturating_add(1);
            drop(control);
            thread::yield_now();
            continue;
        }
        control.metrics.parks = control.metrics.parks.saturating_add(1);
        control = shared
            .wake
            .wait(control)
            .map_err(|_| ResourceError::new("poison", "runtime wait poisoned"))?;
        seen_epoch = control.wake_epoch;
        spins = 0;
    }
    Ok(())
}

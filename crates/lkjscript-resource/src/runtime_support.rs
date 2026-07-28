use std::sync::MutexGuard;
use std::time::Instant;

use crate::runtime::{Control, Shared};
use crate::{ResourceError, ResourceResult, TaskId};

pub(crate) fn lock_control<O, E>(
    shared: &Shared<O, E>,
) -> ResourceResult<MutexGuard<'_, Control<O, E>>> {
    shared
        .control
        .lock()
        .map_err(|_| ResourceError::new("poison", "runtime control poisoned"))
}

pub(crate) fn update_active<O, E>(shared: &Shared<O, E>, entering: bool) -> ResourceResult<()> {
    let mut control = lock_control(shared)?;
    if entering {
        control.metrics.active_workers = control.metrics.active_workers.saturating_add(1);
    } else {
        control.metrics.active_workers = control.metrics.active_workers.saturating_sub(1);
    }
    Ok(())
}

pub(crate) fn take_task<O, E>(
    shared: &Shared<O, E>,
    worker: usize,
) -> ResourceResult<Option<(TaskId, Option<usize>, u64)>> {
    let mut local = shared.queues[worker]
        .lock()
        .map_err(|_| ResourceError::new("poison", "worker queue poisoned"))?;
    if let Some(task) = local.pop_front() {
        return Ok(Some((task, None, queue_wait(shared, task)?)));
    }
    drop(local);
    if matches!(
        shared.policy,
        crate::SchedulePolicy::Sequential | crate::SchedulePolicy::StaticPartition
    ) {
        return Ok(None);
    }
    let mut victims: Vec<_> = (0..shared.queues.len())
        .filter(|victim| *victim != worker)
        .collect();
    if shared.policy == crate::SchedulePolicy::HierarchicalLocality {
        victims.sort_by_key(|victim| {
            let overlap = shared.worker_masks[worker]
                .as_slice()
                .iter()
                .any(|cpu| shared.worker_masks[*victim].contains(*cpu));
            (!overlap, *victim)
        });
    } else {
        victims.sort_by_key(|victim| (victim + shared.queues.len() - worker) % shared.queues.len());
    }
    for victim in victims {
        let mut queue = shared.queues[victim]
            .lock()
            .map_err(|_| ResourceError::new("poison", "victim queue poisoned"))?;
        if let Some(task) = queue.pop_back() {
            return Ok(Some((task, Some(victim), queue_wait(shared, task)?)));
        }
    }
    Ok(None)
}

pub(crate) fn enqueue_ready<O, E>(
    shared: &Shared<O, E>,
    current: usize,
    tasks: &[TaskId],
) -> ResourceResult<()> {
    for task in tasks {
        let worker = match shared.policy {
            crate::SchedulePolicy::Sequential | crate::SchedulePolicy::GlobalFifo => 0,
            crate::SchedulePolicy::StaticPartition | crate::SchedulePolicy::OwnerCompute => {
                shared.preferred.get(task).copied().unwrap_or(current)
            }
            crate::SchedulePolicy::LocalWorkStealing
            | crate::SchedulePolicy::HierarchicalLocality => {
                shared.preferred.get(task).copied().unwrap_or(current)
            }
        };
        enqueue_many(shared, worker, &[*task])?;
    }
    Ok(())
}

pub(crate) fn enqueue_many<O, E>(
    shared: &Shared<O, E>,
    worker: usize,
    tasks: &[TaskId],
) -> ResourceResult<()> {
    if tasks.is_empty() {
        return Ok(());
    }
    let mut queue = shared.queues[worker % shared.queues.len()]
        .lock()
        .map_err(|_| ResourceError::new("poison", "worker queue poisoned"))?;
    if queue.len().saturating_add(tasks.len()) > shared.queue_capacity {
        drop(queue);
        let mut control = lock_control(shared)?;
        control.shutdown = true;
        control.wake_epoch = control.wake_epoch.saturating_add(1);
        drop(control);
        shared.wake.notify_all();
        return Err(ResourceError::new(
            "queue-full",
            "worker queue capacity exceeded",
        ));
    }
    let mut enqueued = shared
        .enqueued
        .lock()
        .map_err(|_| ResourceError::new("poison", "enqueue times poisoned"))?;
    if tasks.iter().any(|task| enqueued.contains_key(task)) {
        return Err(ResourceError::new(
            "enqueue-duplicate",
            "task already queued",
        ));
    }
    let now = Instant::now();
    enqueued.extend(tasks.iter().map(|task| (*task, now)));
    queue.extend(tasks.iter().copied());
    let high_water = queue.len();
    drop(queue);
    let mut control = lock_control(shared)?;
    control.metrics.queue_high_water = control.metrics.queue_high_water.max(high_water);
    control.wake_epoch = control.wake_epoch.saturating_add(1);
    drop(control);
    shared.wake.notify_all();
    Ok(())
}

fn queue_wait<O, E>(shared: &Shared<O, E>, task: TaskId) -> ResourceResult<u64> {
    let queued = shared
        .enqueued
        .lock()
        .map_err(|_| ResourceError::new("poison", "enqueue times poisoned"))?
        .remove(&task)
        .ok_or_else(|| ResourceError::new("enqueue-missing", "task lacks queue timestamp"))?;
    Ok(u64::try_from(queued.elapsed().as_nanos()).unwrap_or(u64::MAX))
}

pub(crate) fn complete_task<O, E>(
    shared: &Shared<O, E>,
    task: TaskId,
    outcome: Result<O, E>,
) -> ResourceResult<Vec<TaskId>> {
    let mut control = lock_control(shared)?;
    match outcome {
        Ok(output) => {
            control.completed.insert(task);
            control.outputs.insert(task, output);
            control.remaining = control.remaining.saturating_sub(1);
        }
        Err(detail) => {
            control.failures.insert(task, detail);
            control.remaining = control.remaining.saturating_sub(1);
            control.shutdown = true;
        }
    }
    let mut ready = Vec::new();
    if !control.shutdown {
        let successors = control.successors.get(&task).cloned().unwrap_or_default();
        for successor in successors {
            let count = control
                .dependencies
                .get_mut(&successor)
                .ok_or_else(|| ResourceError::new("dependency-state", "missing successor state"))?;
            *count = count
                .checked_sub(1)
                .ok_or_else(|| ResourceError::new("dependency-state", "dependency underflow"))?;
            if *count == 0 {
                ready.push(successor);
            }
        }
        ready.sort();
        if control.remaining == 0 {
            control.shutdown = true;
        }
    }
    control.wake_epoch = control.wake_epoch.saturating_add(1);
    drop(control);
    shared.wake.notify_all();
    Ok(ready)
}

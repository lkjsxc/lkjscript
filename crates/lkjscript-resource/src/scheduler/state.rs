use std::collections::BTreeMap;

use super::{TaskState, TraceEvent};
use crate::{ResourceError, ResourceResult, TaskId, WorkerId};

pub(super) fn transition(
    states: &mut BTreeMap<TaskId, TaskState>,
    events: &mut Vec<TraceEvent>,
    count: &mut usize,
    limit: usize,
    task: TaskId,
    to: TaskState,
    worker: Option<WorkerId>,
) -> ResourceResult<()> {
    let from = states
        .get(&task)
        .copied()
        .ok_or_else(|| ResourceError::new("state-task", "unknown task"))?;
    if !legal(from, to) {
        return Err(ResourceError::new("state-transition", "illegal transition"));
    }
    states.insert(task, to);
    *count = count.saturating_add(1);
    if events.len() < limit {
        events.push(TraceEvent {
            task,
            from,
            to,
            worker,
        });
    }
    Ok(())
}

pub(super) fn legal(from: TaskState, to: TaskState) -> bool {
    matches!(
        (from, to),
        (TaskState::Created, TaskState::Waiting)
            | (TaskState::Waiting, TaskState::Ready)
            | (TaskState::Ready, TaskState::Running)
            | (TaskState::Running, TaskState::Completed)
            | (TaskState::Running, TaskState::Failed)
            | (TaskState::Waiting, TaskState::Cancelled)
            | (TaskState::Ready, TaskState::Cancelled)
            | (TaskState::Completed, TaskState::Retired)
            | (TaskState::Failed, TaskState::Retired)
            | (TaskState::Cancelled, TaskState::Retired)
    )
}

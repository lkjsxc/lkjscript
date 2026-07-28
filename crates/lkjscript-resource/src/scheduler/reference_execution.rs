use std::collections::{BTreeMap, BTreeSet};

use super::state::transition;
use super::{ReferenceReport, ReferenceScheduler, SchedulingPolicy, SchedulingTrace, TaskState};
use crate::{ResourceError, ResourceResult, TaskId, VerifiedTaskGraph, WorkerId};

impl ReferenceScheduler {
    pub fn run<P: SchedulingPolicy>(
        graph: &VerifiedTaskGraph,
        policy: &P,
        workers: usize,
        failures: &BTreeMap<TaskId, String>,
        trace_limit: usize,
    ) -> ResourceResult<ReferenceReport> {
        if workers == 0 {
            return Err(ResourceError::new("scheduler-workers", "zero workers"));
        }
        let mut states = BTreeMap::new();
        let mut events = Vec::new();
        let mut transitions = 0;
        for task in graph.tasks() {
            states.insert(task.id, TaskState::Created);
            transition(
                &mut states,
                &mut events,
                &mut transitions,
                trace_limit,
                task.id,
                TaskState::Waiting,
                None,
            )?;
        }
        let mut completed = BTreeSet::new();
        let mut selected_failure = None;
        loop {
            let mut ready: Vec<TaskId> = graph
                .tasks()
                .iter()
                .filter(|task| {
                    states.get(&task.id) == Some(&TaskState::Waiting)
                        && task.dependencies.iter().all(|dep| completed.contains(dep))
                })
                .map(|task| task.id)
                .collect();
            if ready.is_empty() {
                break;
            }
            ready.sort_by_key(|task| policy.rank(graph, *task, workers));
            for task in &ready {
                transition(
                    &mut states,
                    &mut events,
                    &mut transitions,
                    trace_limit,
                    *task,
                    TaskState::Ready,
                    None,
                )?;
            }
            if let Some(task) = ready
                .iter()
                .filter(|task| failures.contains_key(task))
                .min()
                .copied()
            {
                let worker = WorkerId::new(task.slot % workers as u32, 1);
                transition(
                    &mut states,
                    &mut events,
                    &mut transitions,
                    trace_limit,
                    task,
                    TaskState::Running,
                    Some(worker),
                )?;
                transition(
                    &mut states,
                    &mut events,
                    &mut transitions,
                    trace_limit,
                    task,
                    TaskState::Failed,
                    Some(worker),
                )?;
                selected_failure = Some((task, failures.get(&task).cloned().unwrap_or_default()));
                break;
            }
            for (index, task) in ready.into_iter().enumerate() {
                let worker = WorkerId::new((index % workers) as u32, 1);
                transition(
                    &mut states,
                    &mut events,
                    &mut transitions,
                    trace_limit,
                    task,
                    TaskState::Running,
                    Some(worker),
                )?;
                transition(
                    &mut states,
                    &mut events,
                    &mut transitions,
                    trace_limit,
                    task,
                    TaskState::Completed,
                    Some(worker),
                )?;
                completed.insert(task);
            }
        }
        if selected_failure.is_some() {
            let pending: Vec<TaskId> = states
                .iter()
                .filter_map(|(task, state)| {
                    matches!(state, TaskState::Waiting | TaskState::Ready).then_some(*task)
                })
                .collect();
            for task in pending {
                transition(
                    &mut states,
                    &mut events,
                    &mut transitions,
                    trace_limit,
                    task,
                    TaskState::Cancelled,
                    None,
                )?;
            }
        }
        let terminal: Vec<TaskId> = states
            .iter()
            .filter_map(|(task, state)| {
                matches!(
                    state,
                    TaskState::Completed | TaskState::Failed | TaskState::Cancelled
                )
                .then_some(*task)
            })
            .collect();
        for task in terminal {
            transition(
                &mut states,
                &mut events,
                &mut transitions,
                trace_limit,
                task,
                TaskState::Retired,
                None,
            )?;
        }
        if states.values().any(|state| *state != TaskState::Retired) {
            return Err(ResourceError::new(
                "scheduler-incomplete",
                "DAG did not complete",
            ));
        }
        Ok(ReferenceReport {
            states,
            failure: selected_failure,
            trace: SchedulingTrace {
                graph: graph.id(),
                truncated: transitions > events.len(),
                events,
            },
        })
    }
}

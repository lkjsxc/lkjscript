use std::collections::{BTreeMap, BTreeSet};

use crate::graph::task_map;
use crate::{
    AccessMode, AccessRecord, ResourceError, ResourceResult, TaskId, TaskNode, TaskScopeId,
    UnverifiedTaskGraph,
};

pub(super) fn verify_dependencies(
    graph: &UnverifiedTaskGraph,
) -> ResourceResult<BTreeSet<(TaskId, TaskId)>> {
    let map = task_map(&graph.tasks);
    let scopes: BTreeMap<TaskScopeId, Option<TaskScopeId>> = graph
        .scopes
        .iter()
        .map(|scope| (scope.id, scope.parent))
        .collect();
    let mut reach = BTreeSet::new();
    for task in &graph.tasks {
        let mut stack = task.dependencies.clone();
        let mut seen = BTreeSet::new();
        while let Some(dep) = stack.pop() {
            let index = map
                .get(&dep)
                .copied()
                .ok_or_else(|| ResourceError::new("dependency-id", "unknown dependency"))?;
            if dep == task.id {
                return Err(ResourceError::new(
                    "dependency-cycle",
                    "task dependency cycle",
                ));
            }
            if seen.insert(dep) {
                if !scope_contains(&scopes, graph.tasks[index].scope, task.scope) {
                    return Err(ResourceError::new(
                        "scope-containment",
                        "dependency escapes structured scope",
                    ));
                }
                reach.insert((dep, task.id));
                stack.extend(graph.tasks[index].dependencies.iter().copied());
            }
        }
    }
    Ok(reach)
}

fn scope_contains(
    scopes: &BTreeMap<TaskScopeId, Option<TaskScopeId>>,
    outer: TaskScopeId,
    inner: TaskScopeId,
) -> bool {
    let mut cursor = Some(inner);
    while let Some(id) = cursor {
        if id == outer {
            return true;
        }
        cursor = scopes.get(&id).copied().flatten();
    }
    false
}

pub(super) fn verify_accesses(
    tasks: &[TaskNode],
    reach: &BTreeSet<(TaskId, TaskId)>,
) -> ResourceResult<()> {
    let mut produced = BTreeSet::new();
    let mut consumed = BTreeSet::new();
    for task in tasks {
        let exact = task
            .accesses
            .iter()
            .filter(|access| {
                access.mode == AccessMode::Produce
                    && access.owner == task.result_owner
                    && access.range.is_none()
            })
            .count();
        if exact != 1 || !produced.insert(task.result_owner) {
            return Err(ResourceError::new(
                "result-owner",
                "result must have one exact produced owner",
            ));
        }
        for access in task
            .accesses
            .iter()
            .filter(|access| access.mode == AccessMode::Consume)
        {
            if !consumed.insert(access.owner) {
                return Err(ResourceError::new(
                    "owner-consume",
                    "owner consumed more than once",
                ));
            }
        }
    }
    for (left_index, left) in tasks.iter().enumerate() {
        for right in tasks.iter().skip(left_index + 1) {
            if !reach.contains(&(left.id, right.id))
                && !reach.contains(&(right.id, left.id))
                && left
                    .accesses
                    .iter()
                    .any(|a| right.accesses.iter().any(|b| conflict(a, b)))
            {
                return Err(ResourceError::new(
                    "access-conflict",
                    "unordered conflicting accesses",
                ));
            }
        }
    }
    Ok(())
}

fn conflict(left: &AccessRecord, right: &AccessRecord) -> bool {
    if left.owner != right.owner
        || left.mode == AccessMode::IdentityOnly
        || right.mode == AccessMode::IdentityOnly
    {
        return false;
    }
    let mutates = |mode| {
        matches!(
            mode,
            AccessMode::Write | AccessMode::Consume | AccessMode::Produce
        )
    };
    if !mutates(left.mode) && !mutates(right.mode) {
        return false;
    }
    match (left.range, right.range) {
        (Some(a), Some(b)) => a.overlaps(b),
        _ => true,
    }
}

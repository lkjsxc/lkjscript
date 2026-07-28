use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AccessMode, GraphLimits, ResourceError, ResourceResult, TaskScopeId, UnverifiedTaskGraph,
};

pub(super) fn verify_bounds(
    graph: &UnverifiedTaskGraph,
    limits: GraphLimits,
) -> ResourceResult<()> {
    let deps = graph
        .tasks
        .iter()
        .map(|task| task.dependencies.len())
        .sum::<usize>();
    let accesses = graph
        .tasks
        .iter()
        .map(|task| task.accesses.len())
        .sum::<usize>();
    let compute = graph
        .tasks
        .iter()
        .try_fold(0_u64, |sum, task| sum.checked_add(task.compute_units));
    let scratch = graph
        .tasks
        .iter()
        .try_fold(0_u64, |sum, task| sum.checked_add(task.scratch_bytes));
    if graph.tasks.is_empty()
        || graph.tasks.len() > limits.max_tasks
        || deps > limits.max_dependencies
        || accesses > limits.max_accesses
        || compute.is_none_or(|sum| sum > limits.max_compute_units)
        || scratch.is_none_or(|sum| sum > limits.max_scratch_bytes)
    {
        return Err(ResourceError::new(
            "graph-ceiling",
            "task graph exceeds a ceiling",
        ));
    }
    if graph
        .tasks
        .iter()
        .any(|task| task.blocking || !task.portable || !task.cleanup)
    {
        return Err(ResourceError::new(
            "task-contract",
            "tasks must be portable compute with cleanup",
        ));
    }
    Ok(())
}

pub(super) fn verify_ids(graph: &UnverifiedTaskGraph) -> ResourceResult<()> {
    let valid = |slot: u32, generation: u32| generation != 0 && slot != u32::MAX;
    let mut task_slots = BTreeSet::new();
    let mut results = BTreeSet::new();
    let mut access_slots = BTreeSet::new();
    for task in &graph.tasks {
        if !valid(task.id.slot, task.id.generation)
            || !task_slots.insert(task.id.slot)
            || !valid(task.result.slot, task.result.generation)
            || !results.insert(task.result)
            || !valid(task.result_owner.slot, task.result_owner.generation)
        {
            return Err(ResourceError::new(
                "graph-id",
                "invalid, stale, or reused task identifier",
            ));
        }
        for access in &task.accesses {
            if !valid(access.id.slot, access.id.generation)
                || !access_slots.insert(access.id.slot)
                || !valid(access.owner.slot, access.owner.generation)
            {
                return Err(ResourceError::new(
                    "access-id",
                    "invalid or reused access identifier",
                ));
            }
            if access.mode == AccessMode::IdentityOnly && access.range.is_some() {
                return Err(ResourceError::new(
                    "identity-range",
                    "identity-only access has a range",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn verify_scopes(graph: &UnverifiedTaskGraph) -> ResourceResult<()> {
    let scopes: BTreeMap<TaskScopeId, Option<TaskScopeId>> = graph
        .scopes
        .iter()
        .map(|scope| (scope.id, scope.parent))
        .collect();
    if scopes.len() != graph.scopes.len() || scopes.is_empty() {
        return Err(ResourceError::new("scope-id", "duplicate or empty scopes"));
    }
    for scope in &graph.scopes {
        if scope.id.generation == 0
            || scope
                .parent
                .is_some_and(|parent| !scopes.contains_key(&parent))
        {
            return Err(ResourceError::new("scope-parent", "invalid scope parent"));
        }
        let mut cursor = scope.parent;
        let mut seen = BTreeSet::new();
        while let Some(id) = cursor {
            if !seen.insert(id) {
                return Err(ResourceError::new("scope-cycle", "scope cycle"));
            }
            cursor = scopes.get(&id).copied().flatten();
        }
    }
    if graph
        .tasks
        .iter()
        .any(|task| !scopes.contains_key(&task.scope))
    {
        return Err(ResourceError::new("task-scope", "task uses unknown scope"));
    }
    Ok(())
}

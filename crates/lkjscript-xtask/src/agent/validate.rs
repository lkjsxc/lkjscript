use std::collections::BTreeSet;
use std::path::Path;

use super::model::{CheckpointRequest, WorkState};

pub fn checkpoint(
    root: &Path,
    request: &CheckpointRequest,
    current: Option<&WorkState>,
) -> Result<(), String> {
    if request.schema != "lkjscript.agent-work-state-checkpoint" || request.version != 1 {
        return Err("unsupported checkpoint schema or version".into());
    }
    let state = &request.state;
    validate(root, state, true)?;
    if request.expected_state_revision == 0 {
        if current.is_some() || state.state_revision != 1 {
            return Err(
                "revision-zero checkpoint requires absent state and next revision 1".into(),
            );
        }
    } else {
        let old = current.ok_or("expected existing state is absent")?;
        if old.state_revision != request.expected_state_revision {
            return Err(format!(
                "stale state revision: expected {}, current {}",
                request.expected_state_revision, old.state_revision
            ));
        }
        let next = old
            .state_revision
            .checked_add(1)
            .ok_or("state revision overflow")?;
        if state.state_revision != next {
            return Err(format!("next state revision must be {next}"));
        }
        immutable(old, state)?;
        super::history::append_only(old, state)?;
        super::git::require_ancestor(
            root,
            &old.current_repository_revision,
            &state.current_repository_revision,
        )?;
    }
    Ok(())
}

pub fn validate(root: &Path, state: &WorkState, require_head: bool) -> Result<(), String> {
    shape(state)?;
    super::git::require_ancestor(
        root,
        &state.base_repository_revision,
        &state.current_repository_revision,
    )?;
    if require_head {
        let head = super::git::head(root)?;
        if head != state.current_repository_revision {
            return Err(format!(
                "repository revision mismatch: state {}, HEAD {head}",
                state.current_repository_revision
            ));
        }
    }
    super::references::validate(root, state)?;
    for commit in &state.produced_commits {
        super::git::require_commit(root, commit)?;
        super::git::require_ancestor(root, &state.base_repository_revision, commit)?;
        super::git::require_ancestor(root, commit, &state.current_repository_revision)?;
    }
    Ok(())
}

pub fn shape(state: &WorkState) -> Result<(), String> {
    super::bounds::state(state)?;
    if state.schema != "lkjscript.agent-work-state" || state.version != 1 {
        return Err("unsupported work-state schema or version".into());
    }
    task_id(&state.task_id)?;
    if state.state_revision == 0 {
        return Err("state revision must be positive".into());
    }
    super::history::sequence(state)?;
    super::references::shape(state)?;
    unique("capsule scope", &state.selected_capsule_scope)?;
    unique("produced commits", &state.produced_commits)?;
    Ok(())
}

pub fn task_id(value: &str) -> Result<(), String> {
    let valid = (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err("task id must match ^[a-z][a-z0-9-]{0,63}$".into())
    }
}

fn unique(name: &str, values: &[String]) -> Result<(), String> {
    let unique: BTreeSet<_> = values.iter().collect();
    if unique.len() == values.len() {
        Ok(())
    } else {
        Err(format!("duplicate {name}"))
    }
}

fn immutable(old: &WorkState, new: &WorkState) -> Result<(), String> {
    if old.task_id != new.task_id
        || old.base_repository_revision != new.base_repository_revision
        || old.goal != new.goal
        || old.hard_constraints != new.hard_constraints
        || old.selected_capsule_scope != new.selected_capsule_scope
    {
        return Err(
            "task identity, base, goal, constraints, and capsule scope are immutable".into(),
        );
    }
    Ok(())
}

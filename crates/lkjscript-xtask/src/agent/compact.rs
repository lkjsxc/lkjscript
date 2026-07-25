use std::collections::BTreeSet;
use std::path::Path;

use super::model::{ActionOutcome, WorkState};

pub fn run(root: &Path, task_id: &str) -> Result<bool, String> {
    let _lock = super::checkpoint_lock::acquire(root, task_id)?;
    let state = super::storage::load(root, task_id)?.ok_or("task state does not exist")?;
    super::validate::validate(root, &state, true)?;
    let Some(compacted) = compact(&state)? else {
        return Ok(false);
    };
    let bytes = super::json::encode_state(&compacted)?;
    super::storage::write_state(root, task_id, &bytes)?;
    Ok(true)
}

pub fn compact(state: &WorkState) -> Result<Option<WorkState>, String> {
    let mut superseded_by_retained = BTreeSet::new();
    let mut retained = Vec::new();
    for action in state.completed_actions.iter().rev() {
        if action.outcome != ActionOutcome::Completed
            || !superseded_by_retained.contains(&action.sequence)
        {
            superseded_by_retained.extend(action.supersedes.iter().copied());
            retained.push(action.clone());
        }
    }
    retained.reverse();
    if retained.len() == state.completed_actions.len() {
        return Ok(None);
    }
    let mut result = state.clone();
    result.state_revision = result
        .state_revision
        .checked_add(1)
        .ok_or("state revision overflow")?;
    result.completed_actions = retained;
    Ok(Some(result))
}

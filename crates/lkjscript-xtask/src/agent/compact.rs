use std::collections::BTreeSet;
use std::path::Path;

use super::model::{ActionOutcome, WorkState};

pub fn run(root: &Path, task_id: &str) -> Result<bool, String> {
    let _lock = super::storage::lock(root, task_id)?;
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
    let superseded: BTreeSet<_> = state
        .completed_actions
        .iter()
        .flat_map(|action| action.supersedes.iter().copied())
        .collect();
    let retained: Vec<_> = state
        .completed_actions
        .iter()
        .filter(|action| {
            !superseded.contains(&action.sequence) || action.outcome != ActionOutcome::Completed
        })
        .cloned()
        .collect();
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

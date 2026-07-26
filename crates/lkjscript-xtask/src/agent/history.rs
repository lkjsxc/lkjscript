use std::collections::BTreeSet;

use super::model::WorkState;

pub fn sequence(state: &WorkState) -> Result<(), String> {
    increasing(
        "completed action",
        state.completed_actions.iter().map(|item| item.sequence),
    )?;
    increasing(
        "command result",
        state.command_results.iter().map(|item| item.sequence),
    )?;
    let actions: BTreeSet<_> = state
        .completed_actions
        .iter()
        .map(|action| action.sequence)
        .collect();
    for action in &state.completed_actions {
        let mut supersedes = BTreeSet::new();
        for superseded in &action.supersedes {
            if *superseded == 0
                || *superseded >= action.sequence
                || !actions.contains(superseded)
                || !supersedes.insert(*superseded)
            {
                return Err(format!(
                    "action {} has invalid superseded sequence {superseded}",
                    action.sequence
                ));
            }
        }
    }
    Ok(())
}

fn increasing(name: &str, values: impl Iterator<Item = u64>) -> Result<(), String> {
    let mut previous = 0;
    for value in values {
        if value == 0 || value <= previous {
            return Err(format!("{name} sequence must be positive and increasing"));
        }
        previous = value;
    }
    Ok(())
}

pub fn append_only(old: &WorkState, new: &WorkState) -> Result<(), String> {
    semantic_append_only(old, new)?;
    prefix(
        "accepted decisions",
        &old.accepted_decisions,
        &new.accepted_decisions,
    )?;
    prefix(
        "completed actions",
        &old.completed_actions,
        &new.completed_actions,
    )?;
    prefix(
        "command results",
        &old.command_results,
        &new.command_results,
    )?;
    prefix(
        "produced commits",
        &old.produced_commits,
        &new.produced_commits,
    )?;
    prefix(
        "invalidated assumptions",
        &old.invalidated_assumptions,
        &new.invalidated_assumptions,
    )?;
    prefix(
        "artifact references",
        &old.artifact_references,
        &new.artifact_references,
    )?;
    prefix(
        "evidence references",
        &old.evidence_references,
        &new.evidence_references,
    )
}

fn semantic_append_only(old: &WorkState, new: &WorkState) -> Result<(), String> {
    if let Some(old_session) = &old.semantic_context.session {
        let Some(new_session) = &new.semantic_context.session else {
            return Err("semantic session reference cannot be removed".into());
        };
        if old_session.schema != new_session.schema
            || old_session.contract != new_session.contract
            || old_session.identity != new_session.identity
        {
            return Err("semantic session identity is immutable".into());
        }
    }
    prefix(
        "semantic completed transactions",
        &old.semantic_context.completed_transactions,
        &new.semantic_context.completed_transactions,
    )
}

fn prefix<T: PartialEq>(name: &str, old: &[T], new: &[T]) -> Result<(), String> {
    if new.starts_with(old) {
        Ok(())
    } else {
        Err(format!("{name} must be append-only"))
    }
}

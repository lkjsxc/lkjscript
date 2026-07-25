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

fn prefix<T: PartialEq>(name: &str, old: &[T], new: &[T]) -> Result<(), String> {
    if new.starts_with(old) {
        Ok(())
    } else {
        Err(format!("{name} must be append-only"))
    }
}

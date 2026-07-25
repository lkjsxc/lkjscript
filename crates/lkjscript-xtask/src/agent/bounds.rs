use super::model::{CheckpointRequest, ContentReference, WorkState};

pub const REQUEST_BYTES: usize = 262_144;
pub const STATE_BYTES: usize = 262_144;
pub const QUARANTINE_BYTES: u64 = 1_048_576;
pub const CONTEXT_BYTES_WEAK: usize = 65_536;
pub const CONTEXT_BYTES_STRONG: usize = 262_144;
pub const OUTPUT_BYTES: usize = 524_288;
pub const STRING_BYTES: usize = 4_096;
pub const COLLECTION_ITEMS: usize = 128;
pub const HISTORY_ITEMS: usize = 256;
pub const REFERENCE_ITEMS: usize = 128;
pub const WORK_ITEMS: usize = 4_096;
pub const REFERENCE_BYTES: u64 = 1_048_576;
pub const CAPSULE_FILES: usize = 256;
pub const GIT_OUTPUT_BYTES: usize = 4_194_304;

pub fn output(bytes: usize) -> Result<(), String> {
    if bytes > STATE_BYTES {
        Err(limit("state output bytes", STATE_BYTES, bytes))
    } else {
        Ok(())
    }
}

pub fn checked_work(current: usize, charge: usize) -> Result<usize, String> {
    let attempted = current
        .checked_add(charge)
        .ok_or("aggregate work overflow")?;
    if attempted > WORK_ITEMS {
        Err(limit("aggregate work", WORK_ITEMS, attempted))
    } else {
        Ok(attempted)
    }
}

pub fn request(request: &CheckpointRequest, input_bytes: usize) -> Result<(), String> {
    if input_bytes > REQUEST_BYTES {
        return Err(limit("request bytes", REQUEST_BYTES, input_bytes));
    }
    state(&request.state)
}

pub fn state(value: &WorkState) -> Result<(), String> {
    let mut work = 0usize;
    text("task_id", &value.task_id, &mut work)?;
    text("base revision", &value.base_repository_revision, &mut work)?;
    text(
        "current revision",
        &value.current_repository_revision,
        &mut work,
    )?;
    text("goal", &value.goal, &mut work)?;
    strings("hard_constraints", &value.hard_constraints, &mut work)?;
    list(
        "selected_capsule_scope",
        value.selected_capsule_scope.len(),
        16,
        &mut work,
    )?;
    for capsule in &value.selected_capsule_scope {
        text("selected capsule", capsule, &mut work)?;
    }
    super::semantic_bounds::validate(&value.semantic_context, &mut work)?;
    refs("accepted_decisions", &value.accepted_decisions, &mut work)?;
    list(
        "completed_actions",
        value.completed_actions.len(),
        HISTORY_ITEMS,
        &mut work,
    )?;
    for action in &value.completed_actions {
        text("action", &action.action, &mut work)?;
        text("action summary", &action.summary, &mut work)?;
        numbers("supersedes", action.supersedes.len(), &mut work)?;
        strings("action evidence", &action.evidence, &mut work)?;
    }
    list(
        "command_results",
        value.command_results.len(),
        HISTORY_ITEMS,
        &mut work,
    )?;
    for command in &value.command_results {
        text("command", &command.command, &mut work)?;
        text("command summary", &command.summary, &mut work)?;
        strings("command evidence", &command.evidence, &mut work)?;
    }
    let history = value
        .completed_actions
        .len()
        .checked_add(value.command_results.len())
        .ok_or("history count overflow")?;
    if history > HISTORY_ITEMS {
        return Err(limit("combined history", HISTORY_ITEMS, history));
    }
    for (name, values) in fact_lists(value) {
        strings(name, values, &mut work)?;
    }
    refs("artifact_references", &value.artifact_references, &mut work)?;
    refs("evidence_references", &value.evidence_references, &mut work)?;
    let references = value
        .accepted_decisions
        .len()
        .checked_add(value.artifact_references.len())
        .and_then(|count| count.checked_add(value.evidence_references.len()))
        .ok_or("reference count overflow")?;
    if references > REFERENCE_ITEMS {
        return Err(limit("combined references", REFERENCE_ITEMS, references));
    }
    checked_work(work, 0).map(|_| ())
}

fn fact_lists(value: &WorkState) -> [(&'static str, &Vec<String>); 6] {
    [
        ("produced_commits", &value.produced_commits),
        ("open_defects", &value.open_defects),
        ("risks", &value.risks),
        ("external_blockers", &value.external_blockers),
        ("next_actions", &value.next_actions),
        ("invalidated_assumptions", &value.invalidated_assumptions),
    ]
}

fn refs(name: &str, values: &[ContentReference], work: &mut usize) -> Result<(), String> {
    list(name, values.len(), REFERENCE_ITEMS, work)?;
    for value in values {
        text("reference path", &value.path, work)?;
        text("reference sha256", &value.sha256, work)?;
    }
    Ok(())
}

fn strings(name: &str, values: &[String], work: &mut usize) -> Result<(), String> {
    list(name, values.len(), COLLECTION_ITEMS, work)?;
    for value in values {
        text(name, value, work)?;
    }
    Ok(())
}

fn numbers(name: &str, count: usize, work: &mut usize) -> Result<(), String> {
    list(name, count, COLLECTION_ITEMS, work)
}

pub(super) fn list(
    name: &str,
    count: usize,
    limit_value: usize,
    work: &mut usize,
) -> Result<(), String> {
    if count > limit_value {
        return Err(limit(name, limit_value, count));
    }
    *work = checked_work(*work, count)?;
    Ok(())
}

pub(super) fn text(name: &str, value: &str, work: &mut usize) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    if value.len() > STRING_BYTES {
        return Err(limit(name, STRING_BYTES, value.len()));
    }
    *work = checked_work(*work, 1)?;
    Ok(())
}

pub(super) fn limit(category: &str, limit_value: usize, actual: usize) -> String {
    format!("{category} exceeds limit {limit_value}: {actual}")
}

use std::fs;

use crate::agent::model::{
    ActionOutcome, CommandResult, ContentReference, SemanticReference, SemanticSessionReference,
};
use crate::agent::{bounds, json, validate};

use super::support;

#[test]
fn rejects_malformed_duplicate_unknown_and_trailing_json() {
    let repo = support::repository("parse-rejection");
    let state = support::state(&repo, "parse-task");
    let request = support::request(state, 0);
    let valid = serde_json::to_string(&request).unwrap();
    let cases = [
        "{".to_owned(),
        valid.replacen("\"schema\":", "\"schema\":\"duplicate\",\"schema\":", 1),
        valid.replacen("{", "{\"unknown\":true,", 1),
        format!("{valid} false"),
    ];
    for (index, text) in cases.iter().enumerate() {
        let path = repo.root.join(format!("request-{index}.json"));
        fs::write(&path, text).unwrap();
        assert!(json::read_request(&path).is_err(), "accepted case {index}");
    }
}

#[test]
fn enforces_exact_and_plus_one_string_and_collection_limits() {
    let repo = support::repository("bounds");
    let mut state = support::state(&repo, "bounds-task");
    state.goal = "x".repeat(bounds::STRING_BYTES);
    assert!(bounds::state(&state).is_ok());
    state.goal.push('x');
    assert!(bounds::state(&state)
        .unwrap_err()
        .contains("goal exceeds limit"));
    state.goal = "goal".into();
    state.risks = vec!["risk".into(); bounds::COLLECTION_ITEMS];
    assert!(bounds::state(&state).is_ok());
    state.risks.push("overflow".into());
    assert!(bounds::state(&state)
        .unwrap_err()
        .contains("risks exceeds limit"));
}

#[test]
fn enforces_history_reference_output_and_work_boundaries() {
    let repo = support::repository("aggregate-bounds");
    let mut state = support::state(&repo, "aggregate-task");
    state.completed_actions = (1..=128)
        .map(|sequence| support::action(sequence, ActionOutcome::Completed, Vec::new()))
        .collect();
    state.command_results = (1..=128)
        .map(|sequence| CommandResult {
            sequence,
            command: "command".into(),
            exit: 0,
            summary: "summary".into(),
            evidence: Vec::new(),
        })
        .collect();
    assert!(bounds::state(&state).is_ok());
    state.command_results.push(CommandResult {
        sequence: 129,
        command: "overflow".into(),
        exit: 1,
        summary: "overflow".into(),
        evidence: Vec::new(),
    });
    assert!(bounds::state(&state)
        .unwrap_err()
        .contains("combined history"));

    let mut references = support::state(&repo, "reference-bound-task");
    references.artifact_references = (0..127)
        .map(|index| ContentReference {
            path: format!("artifact-{index}"),
            sha256: "0".repeat(64),
        })
        .collect();
    assert!(bounds::state(&references).is_ok());
    references.artifact_references.push(ContentReference {
        path: "artifact-overflow".into(),
        sha256: "0".repeat(64),
    });
    assert!(bounds::state(&references)
        .unwrap_err()
        .contains("combined references"));

    assert!(bounds::output(bounds::STATE_BYTES).is_ok());
    assert!(bounds::output(bounds::STATE_BYTES + 1).is_err());
    assert_eq!(
        bounds::checked_work(0, bounds::WORK_ITEMS).unwrap(),
        bounds::WORK_ITEMS
    );
    assert!(bounds::checked_work(0, bounds::WORK_ITEMS + 1).is_err());
    assert!(bounds::checked_work(usize::MAX, 1)
        .unwrap_err()
        .contains("overflow"));
}

#[test]
fn semantic_context_is_closed_versioned_and_bounded() {
    let repo = support::repository("semantic-context");
    let mut state = support::state(&repo, "semantic-context-task");
    state.semantic_context.session = Some(SemanticSessionReference {
        schema: "lkjscript.semantic-session".into(),
        version: 1,
        identity: "1".repeat(64),
        source_revision: "2".repeat(64),
    });
    state
        .semantic_context
        .target_entities
        .push(SemanticReference {
            schema: "lkjscript.semantic-source".into(),
            version: 1,
            source_revision: "2".repeat(64),
            identity: "declaration:main".into(),
        });
    assert!(validate::shape(&state).is_ok());
    state.semantic_context.target_entities[0].schema = "unknown".into();
    assert!(validate::shape(&state).is_err());
    state.semantic_context.target_entities[0].schema = "lkjscript.semantic-source".into();
    state.semantic_context.target_entities[0].source_revision = "x".repeat(64);
    assert!(validate::shape(&state).is_err());
    state.semantic_context.target_entities = (0..=bounds::REFERENCE_ITEMS)
        .map(|index| SemanticReference {
            schema: "lkjscript.semantic-source".into(),
            version: 1,
            source_revision: "2".repeat(64),
            identity: format!("entity:{index}"),
        })
        .collect();
    assert!(bounds::state(&state)
        .unwrap_err()
        .contains("combined semantic references"));

    let mut old = support::request(support::state(&repo, "old-version-task"), 0);
    old.version = 1;
    assert!(validate::checkpoint(&repo.root, &old, None).is_err());
}

#[test]
fn supersession_requires_unique_existing_actions() {
    let repo = support::repository("supersession-shape");
    let mut state = support::state(&repo, "supersession-task");
    state.completed_actions = vec![
        support::action(1, ActionOutcome::Completed, Vec::new()),
        support::action(2, ActionOutcome::Completed, vec![1, 1]),
    ];
    assert!(validate::shape(&state).is_err());
    state.completed_actions[1].supersedes = vec![1];
    assert!(validate::shape(&state).is_ok());
    state.completed_actions[1].supersedes = vec![3];
    assert!(validate::shape(&state).is_err());
}

#[test]
fn rejects_request_byte_limit_plus_one_before_parsing() {
    let repo = support::repository("request-size");
    let exact = support::request(support::state(&repo, "request-limit-task"), 0);
    assert!(bounds::request(&exact, bounds::REQUEST_BYTES).is_ok());
    assert!(bounds::request(&exact, bounds::REQUEST_BYTES + 1).is_err());
    let path = repo.root.join("large-request.json");
    fs::write(&path, vec![b' '; bounds::REQUEST_BYTES + 1]).unwrap();
    assert!(json::read_request(&path)
        .unwrap_err()
        .contains("byte limit"));
}

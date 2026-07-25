use crate::agent::compact;
use crate::agent::context;
use crate::agent::model::{ActionOutcome, ResumeContext};
use crate::agent::validate;

use super::support;

#[test]
fn rejects_stale_and_non_monotonic_checkpoint_preconditions() {
    let repo = support::repository("stale");
    let old = support::state(&repo, "stale-task");
    let mut next = old.clone();
    next.state_revision = 2;
    next.completed_actions
        .push(support::action(2, ActionOutcome::Completed, Vec::new()));
    let stale = support::request(next.clone(), 0);
    assert!(validate::checkpoint(&repo.root, &stale, Some(&old))
        .unwrap_err()
        .contains("revision-zero"));
    let wrong = support::request(next, 7);
    assert!(validate::checkpoint(&repo.root, &wrong, Some(&old))
        .unwrap_err()
        .contains("stale state revision"));
}

#[test]
fn rejects_state_revision_overflow() {
    let repo = support::repository("overflow");
    let mut old = support::state(&repo, "overflow-task");
    old.state_revision = u64::MAX;
    let next = old.clone();
    let request = support::request(next, u64::MAX);
    assert!(validate::checkpoint(&repo.root, &request, Some(&old))
        .unwrap_err()
        .contains("state revision overflow"));
}

#[test]
fn validates_artifact_hashes_and_evidence_declarations() {
    let repo = support::repository("references");
    let mut state = support::state(&repo, "reference-task");
    state.completed_actions[0].evidence = vec!["missing.txt".into()];
    assert!(validate::validate(&repo.root, &state, true)
        .unwrap_err()
        .contains("not declared"));
    state.completed_actions[0].evidence.clear();
    state.evidence_references[0].sha256 = "0".repeat(64);
    assert!(validate::validate(&repo.root, &state, true)
        .unwrap_err()
        .contains("content hash mismatch"));
}

#[test]
fn resume_context_reports_deterministic_output_truncation() {
    let repo = support::repository("context-limit");
    let mut response = ResumeContext {
        schema: "lkjscript.agent-resume-context",
        version: 1,
        state: support::state(&repo, "context-task"),
        repository_context: Vec::new(),
        revision_mismatch: None,
        omissions: Vec::new(),
        truncated: false,
    };
    let context_result = crate::model::QueryResult {
        schema: "lkjscript.repository-query".into(),
        version: 1,
        command: "context".into(),
        target: "workspace".into(),
        profile: Some("weak".into()),
        sections: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        work_used: 0,
        bytes_used: 0,
        truncated: false,
        unsupported: Vec::new(),
    };
    context::include(&mut response, context_result, "weak", 1).unwrap();
    assert!(response.truncated);
    assert!(response.repository_context.is_empty());
    assert_eq!(response.omissions.len(), 1);
}

#[test]
fn compaction_retains_failures_test_truth_commits_and_references() {
    let repo = support::repository("compaction");
    let mut state = support::state(&repo, "compact-task");
    state.completed_actions = vec![
        support::action(1, ActionOutcome::Completed, Vec::new()),
        support::action(2, ActionOutcome::Completed, vec![1]),
        support::action(3, ActionOutcome::Failed, vec![2]),
        support::action(4, ActionOutcome::Tested, Vec::new()),
        support::action(5, ActionOutcome::NotTested, Vec::new()),
    ];
    state.produced_commits.push(repo.revision.clone());
    let compacted = compact::compact(&state).unwrap().unwrap();
    let sequences: Vec<_> = compacted
        .completed_actions
        .iter()
        .map(|action| action.sequence)
        .collect();
    assert_eq!(sequences, [3, 4, 5]);
    assert_eq!(compacted.produced_commits, state.produced_commits);
    assert_eq!(compacted.evidence_references, state.evidence_references);
    assert!(compact::compact(&compacted).unwrap().is_none());
}

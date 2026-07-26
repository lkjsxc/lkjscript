use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::agent::model::{
    ActionOutcome, CheckpointRequest, CompletedAction, ContentReference, SemanticWorkContext,
    WorkState,
};

pub struct Repository {
    pub root: PathBuf,
    pub revision: String,
    pub evidence: ContentReference,
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn repository(name: &str) -> Repository {
    let root = std::env::temp_dir().join(format!(
        "lkjscript-agent-test-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("crates/sample")).unwrap();
    fs::write(
        root.join("capsule.json"),
        format!(
            r#"{{
  "schema":"lkjscript.capsule","contract":"{}","id":"workspace","root":".","kind":"workspace",
  "purpose":"test","layer":"test","concepts":[],"facade":[],"allowed_dependencies":[],
  "forbidden_dependencies":[],"tests":[],"decisions":[],
  "unsafe_boundary":{{"status":"forbidden","boundaries":[]}},
  "capability":{{"status":"none","names":[]}},
  "provenance":{{"class":"authored","source":"test"}},
  "verification":[],"context_card":{{"goal":"test","interfaces":[],"invariants":[]}}
}}
"#,
            lkjscript_contracts::CAPSULE_MANIFEST_DIGEST.to_hex()
        ),
    )
    .unwrap();
    fs::write(root.join("evidence.txt"), "evidence\n").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "user.name", "Test"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "initial"]);
    let revision = output(&root, &["rev-parse", "HEAD"]);
    let bytes = fs::read(root.join("evidence.txt")).unwrap();
    Repository {
        root,
        revision,
        evidence: ContentReference {
            path: "evidence.txt".into(),
            sha256: crate::sha256::digest(&bytes),
        },
    }
}

pub fn state(repo: &Repository, task: &str) -> WorkState {
    WorkState {
        schema: "lkjscript.agent-work-state".into(),
        contract: lkjscript_contracts::AGENT_WORK_STATE_DIGEST.to_hex(),
        task_id: task.into(),
        state_revision: 1,
        base_repository_revision: repo.revision.clone(),
        current_repository_revision: repo.revision.clone(),
        goal: "implement bounded state".into(),
        hard_constraints: vec!["no hidden state".into()],
        selected_capsule_scope: vec!["workspace".into()],
        semantic_context: SemanticWorkContext {
            session: None,
            target_entities: Vec::new(),
            completed_transactions: Vec::new(),
            diagnostics: Vec::new(),
            unresolved_holes: Vec::new(),
        },
        accepted_decisions: Vec::new(),
        completed_actions: vec![action(1, ActionOutcome::Completed, Vec::new())],
        command_results: Vec::new(),
        produced_commits: Vec::new(),
        open_defects: Vec::new(),
        risks: Vec::new(),
        external_blockers: Vec::new(),
        next_actions: vec!["verify".into()],
        invalidated_assumptions: Vec::new(),
        artifact_references: Vec::new(),
        evidence_references: vec![repo.evidence.clone()],
    }
}

pub fn request(state: WorkState, expected: u64) -> CheckpointRequest {
    CheckpointRequest {
        schema: "lkjscript.agent-work-state-checkpoint".into(),
        contract: lkjscript_contracts::AGENT_WORK_STATE_DIGEST.to_hex(),
        expected_state_revision: expected,
        state,
    }
}

pub fn action(sequence: u64, outcome: ActionOutcome, supersedes: Vec<u64>) -> CompletedAction {
    CompletedAction {
        sequence,
        action: format!("action {sequence}"),
        outcome,
        summary: format!("summary {sequence}"),
        supersedes,
        evidence: Vec::new(),
    }
}

pub fn publish(repo: &Repository, state: &WorkState) {
    let bytes = crate::agent::json::encode_state(state).unwrap();
    crate::agent::storage::write_state(&repo.root, &state.task_id, &bytes).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap()
        .success());
}

fn output(root: &Path, args: &[&str]) -> String {
    let result = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().trim().into()
}

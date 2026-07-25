use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointRequest {
    pub schema: String,
    pub version: u32,
    pub expected_state_revision: u64,
    pub state: WorkState,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkState {
    pub schema: String,
    pub version: u32,
    pub task_id: String,
    pub state_revision: u64,
    pub base_repository_revision: String,
    pub current_repository_revision: String,
    pub goal: String,
    pub hard_constraints: Vec<String>,
    pub selected_capsule_scope: Vec<String>,
    pub semantic_context: SemanticWorkContext,
    pub accepted_decisions: Vec<ContentReference>,
    pub completed_actions: Vec<CompletedAction>,
    pub command_results: Vec<CommandResult>,
    pub produced_commits: Vec<String>,
    pub open_defects: Vec<String>,
    pub risks: Vec<String>,
    pub external_blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub invalidated_assumptions: Vec<String>,
    pub artifact_references: Vec<ContentReference>,
    pub evidence_references: Vec<ContentReference>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticWorkContext {
    pub session: Option<SemanticSessionReference>,
    pub target_entities: Vec<SemanticReference>,
    pub completed_transactions: Vec<SemanticReference>,
    pub diagnostics: Vec<SemanticReference>,
    pub unresolved_holes: Vec<SemanticReference>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticSessionReference {
    pub schema: String,
    pub version: u32,
    pub identity: String,
    pub source_revision: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticReference {
    pub schema: String,
    pub version: u32,
    pub source_revision: String,
    pub identity: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReference {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedAction {
    pub sequence: u64,
    pub action: String,
    pub outcome: ActionOutcome,
    pub summary: String,
    pub supersedes: Vec<u64>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionOutcome {
    Completed,
    Failed,
    Tested,
    NotTested,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandResult {
    pub sequence: u64,
    pub command: String,
    pub exit: i32,
    pub summary: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ResumeContext {
    pub schema: &'static str,
    pub version: u32,
    pub state: WorkState,
    pub repository_context: Vec<crate::model::QueryResult>,
    pub revision_mismatch: Option<String>,
    pub omissions: Vec<String>,
    pub truncated: bool,
}

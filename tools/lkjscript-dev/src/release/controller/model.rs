use serde::{Deserialize, Serialize};

pub(super) const REPOSITORY: &str = "lkjsxc/lkjscript";
pub(super) const WORKFLOW: &str = ".github/workflows/release.yml";
pub(super) const ACCEPTANCE_JOB: &str = "Accept final candidate";
pub(super) const SELECTION_FORMAT: &str = "lkjscript-candidate-selection-1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileIdentity {
    pub(crate) name: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CandidateContent {
    pub(crate) source_commit: String,
    pub(crate) tag: String,
    pub(crate) verifier: FileIdentity,
    pub(crate) assets: Vec<FileIdentity>,
    pub(crate) acceptance_contract: String,
    pub(crate) target_policy_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Producer {
    pub(super) repository: String,
    pub(super) run_id: u64,
    pub(super) run_attempt: u64,
    pub(super) source_commit: String,
    pub(super) workflow_id: u64,
    pub(super) workflow_path: String,
    pub(super) acceptance_job_id: u64,
    pub(super) artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Artifact {
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) role: String,
    pub(super) digest: String,
    pub(super) byte_length: u64,
    pub(super) expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub(super) format: String,
    pub(super) controller_source: String,
    pub(super) consumer_run_id: u64,
    pub(super) consumer_run_attempt: u64,
    pub(super) producer: Producer,
    pub(super) content: CandidateContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    pub(super) source: String,
    pub(super) run_id: u64,
    pub(super) run_attempt: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Authority {
    pub(super) controller_source: String,
    pub(super) product_source: String,
    pub(super) main_source: String,
    pub(super) annotated_tag_object: String,
    pub(super) tag: String,
    pub(super) producer_run_id: u64,
    pub(super) producer_run_attempt: u64,
    pub(super) release_notes: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LatestState {
    Selected,
    Superseded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Published {
    pub(super) release_id: u64,
    pub(super) release_url: String,
    pub(super) immutable: bool,
    pub(super) latest: LatestState,
    pub(super) latest_tag: String,
    pub(super) latest_release_id: u64,
    pub(super) latest_source_commit: String,
}

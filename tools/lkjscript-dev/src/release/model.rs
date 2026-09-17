use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SchemaIdentity {
    pub(super) identity: String,
    pub(super) version: u32,
}

pub(super) use lkjscript::release_container::model::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EvidenceClassification {
    FreshPassed,
    Reused,
    Skipped,
    Unavailable,
    Failed,
    NotProvided,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VerificationClassification {
    pub(super) name: String,
    pub(super) classification: EvidenceClassification,
    pub(super) detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactIdentity {
    pub(super) name: String,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExternalEvidence {
    pub(super) path: String,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostedContext {
    pub(super) github_actions: Option<String>,
    pub(super) repository: Option<String>,
    pub(super) workflow: Option<String>,
    pub(super) job: Option<String>,
    pub(super) run_id: Option<String>,
    pub(super) run_attempt: Option<String>,
    pub(super) run_url: Option<String>,
    pub(super) runner_os: Option<String>,
    pub(super) runner_architecture: Option<String>,
    pub(super) runner_image_os: Option<String>,
    pub(super) runner_image_version: Option<String>,
}

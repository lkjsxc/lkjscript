use crate::evidence::{FileProof, VerificationDigest};
use crate::process::ProcessObservation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) const CHECK_CONTRACT_VERSION: u32 = 4;
pub(crate) const CACHE_CONTRACT_VERSION: u32 = 2;
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3_600);
pub(crate) const DEFAULT_MAXIMUM_STREAM_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAXIMUM_WORKERS: usize = 8;
pub(crate) const MAXIMUM_CACHE_RECORD_BYTES: u64 = 1024 * 1024;
pub(crate) const MAXIMUM_CACHE_ENTRIES_PER_GATE: usize = 16;
pub(crate) const MAXIMUM_CACHE_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const MAXIMUM_FAILURE_EXCERPT_BYTES: usize = 2_048;

#[derive(Clone, Debug)]
pub(crate) struct Gate {
    pub(crate) name: String,
    pub(crate) command: Vec<String>,
    pub(crate) identity_command: Option<Vec<String>>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) timeout: Duration,
    pub(crate) maximum_stdout_bytes: u64,
    pub(crate) maximum_stderr_bytes: u64,
    pub(crate) cacheable: bool,
    pub(crate) required_outputs: Vec<PathBuf>,
}

impl Gate {
    pub(crate) fn new(name: &str, command: Vec<String>) -> Self {
        Self {
            name: name.to_owned(),
            command,
            identity_command: None,
            dependencies: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            maximum_stdout_bytes: DEFAULT_MAXIMUM_STREAM_BYTES,
            maximum_stderr_bytes: DEFAULT_MAXIMUM_STREAM_BYTES,
            cacheable: true,
            required_outputs: Vec::new(),
        }
    }

    pub(crate) fn identity_command(&self) -> &[String] {
        self.identity_command.as_deref().unwrap_or(&self.command)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputSnapshot {
    pub(crate) digest: VerificationDigest,
    pub(crate) git_head: String,
    pub(crate) cargo_lock_digest: VerificationDigest,
    pub(crate) file_count: usize,
    pub(crate) total_bytes: u64,
    pub(crate) entries: Vec<InputEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputEntry {
    pub(crate) source: InputSource,
    pub(crate) proof: FileProof,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gitlink_head: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InputSource {
    Tracked,
    Untracked,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeIdentity {
    pub(crate) digest: VerificationDigest,
    pub(crate) rustc: String,
    pub(crate) cargo: String,
    pub(crate) platform: PlatformIdentity,
    pub(crate) environment_digest: VerificationDigest,
    pub(crate) environment_names: Vec<String>,
    pub(crate) harness: FileProof,
    pub(crate) command_executables: BTreeMap<String, FileProof>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlatformIdentity {
    pub(crate) operating_system: String,
    pub(crate) architecture: String,
    pub(crate) family: String,
    pub(crate) child_process_control: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GateStatus {
    Passed,
    Failed,
    Unavailable,
    Timeout,
    OutputExhausted,
    Signaled,
    InfrastructureFailure,
    Skipped,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionKind {
    Fresh,
    Reused,
    Skipped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GateReceipt {
    pub(crate) name: String,
    pub(crate) status: GateStatus,
    pub(crate) execution: ExecutionKind,
    pub(crate) command: Vec<String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) failed_dependencies: Vec<String>,
    pub(crate) started_unix_nanoseconds: u128,
    pub(crate) completed_unix_nanoseconds: u128,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) process: Option<ProcessObservation>,
    pub(crate) outputs: Vec<FileProof>,
    pub(crate) input_fingerprint: VerificationDigest,
    pub(crate) evidence_digest: VerificationDigest,
    pub(crate) cache: CacheObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stdout_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stderr_excerpt: Option<String>,
}

impl GateReceipt {
    pub(crate) fn passed(&self) -> bool {
        self.status == GateStatus::Passed
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CacheObservation {
    pub(crate) eligible: bool,
    pub(crate) lookup: CacheLookupStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) record: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) write: Option<CacheWriteStatus>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CacheLookupStatus {
    Hit,
    Miss,
    Bypassed,
    NotAttempted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CacheWriteStatus {
    Stored,
    Failed,
    WithheldInputChanged,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AggregateStatus {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckReceipt {
    pub(crate) contract_version: u32,
    pub(crate) status: AggregateStatus,
    pub(crate) profile: String,
    pub(crate) profile_definition_digest: VerificationDigest,
    pub(crate) started_unix_nanoseconds: u128,
    pub(crate) completed_unix_nanoseconds: u128,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) git_head: Option<String>,
    pub(crate) worktree_input_digest: Option<VerificationDigest>,
    pub(crate) final_worktree_input_digest: Option<VerificationDigest>,
    pub(crate) input_stable: bool,
    pub(crate) input_manifest: Option<String>,
    pub(crate) dag_manifest: Option<String>,
    pub(crate) runtime: Option<RuntimeIdentity>,
    pub(crate) requested_gates: Vec<String>,
    pub(crate) selected_gates: Vec<String>,
    pub(crate) passed_gates: usize,
    pub(crate) fresh_passed_gates: usize,
    pub(crate) reused_passed_gates: usize,
    pub(crate) unrun_gates: Vec<String>,
    pub(crate) maximum_workers: usize,
    pub(crate) fresh_required: bool,
    pub(crate) gates: Vec<GateReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) failure: Option<FailureSummary>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailureSummary {
    pub(crate) owner: String,
    pub(crate) status: String,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagManifest {
    pub(crate) contract_version: u32,
    pub(crate) requested: Vec<String>,
    pub(crate) selected_closure: Vec<String>,
    pub(crate) maximum_workers: usize,
    pub(crate) nodes: Vec<DagNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagNode {
    pub(crate) name: String,
    pub(crate) dependencies: Vec<String>,
    pub(crate) command: Vec<String>,
    pub(crate) cacheable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputManifest {
    pub(crate) contract_version: u32,
    pub(crate) snapshot: InputSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CacheRecord {
    pub(crate) cache_contract_version: u32,
    pub(crate) gate: String,
    pub(crate) input_fingerprint: VerificationDigest,
    pub(crate) identity_command: Vec<String>,
    pub(crate) source_elapsed_nanoseconds: u64,
    pub(crate) process: ProcessObservation,
    pub(crate) outputs: Vec<FileProof>,
    pub(crate) evidence_digest: VerificationDigest,
}

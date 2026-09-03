use crate::evidence::{FileProof, VerificationDigest};
use crate::process::{ProcessObservation, ProcessStatus};
use lkjscript::platform::contributor::CatalogInventory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const SCALE_CONTRACT_VERSION: u32 = 3;
pub(crate) const SCALE_SCHEMA: &str = "lkjscript-semantic-scale-receipt";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReceiptStatus {
    Passed,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AdmissionClassification {
    Completed,
    EnvironmentLimit,
    NotRunWithReason,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Lifecycle {
    Full,
    Capacity,
}

impl Lifecycle {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Capacity => "capacity",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceIdentity {
    pub(crate) branch: String,
    pub(crate) commit: String,
    pub(crate) tree: String,
    pub(crate) upstream: Option<String>,
    pub(crate) ahead: Option<u64>,
    pub(crate) behind: Option<u64>,
    pub(crate) worktree_clean: bool,
    pub(crate) worktree_status_bytes: u64,
    pub(crate) worktree_status_digest: VerificationDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolchainIdentity {
    pub(crate) rustc: String,
    pub(crate) cargo: String,
    pub(crate) channel_file: FileProof,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CandidateIdentity {
    pub(crate) configured_path: String,
    pub(crate) executed_path: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
    pub(crate) verification: FileProof,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapabilityIdentity {
    pub(crate) product_name: String,
    pub(crate) product_version: String,
    pub(crate) digest: String,
    pub(crate) sections: BTreeMap<String, CapabilitySection>,
    pub(crate) operations: Vec<OperationIdentity>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OperationIdentity {
    pub(crate) name: String,
    pub(crate) usage: String,
    pub(crate) request_model: String,
    pub(crate) response_model: String,
    pub(crate) authority_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapabilitySection {
    pub(crate) digest: String,
    pub(crate) records: u64,
    pub(crate) bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScenarioEvidence {
    pub(crate) topology: String,
    pub(crate) semantic_shape: String,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) requested_items: u64,
    pub(crate) batch_size: u64,
    pub(crate) requested_modules: Option<u64>,
    pub(crate) requested: SemanticCounts,
    pub(crate) maximum_wall_seconds: u64,
    pub(crate) maximum_run_bytes: u64,
    pub(crate) minimum_available_memory_bytes: u64,
    pub(crate) minimum_available_disk_bytes: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SemanticCounts {
    pub(crate) owners: u64,
    pub(crate) modules: u64,
    pub(crate) functions: u64,
    pub(crate) relations: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScaleReceipt {
    pub(crate) schema: String,
    pub(crate) contract_version: u32,
    pub(crate) status: ReceiptStatus,
    pub(crate) admission: AdmissionClassification,
    pub(crate) source: Option<SourceIdentity>,
    pub(crate) toolchain: Option<ToolchainIdentity>,
    pub(crate) candidate: Option<CandidateIdentity>,
    pub(crate) capabilities: Option<CapabilityIdentity>,
    pub(crate) scenario: ScenarioEvidence,
    pub(crate) logical: LogicalEvidence,
    pub(crate) commands: Vec<CommandEvidence>,
    pub(crate) observations: ObservationEvidence,
    pub(crate) cleanup: CleanupEvidence,
    pub(crate) limitations: Vec<String>,
    pub(crate) failure: Option<FailureEvidence>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogicalEvidence {
    pub(crate) starting_revision: Option<String>,
    pub(crate) final_revision: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) package: Option<String>,
    pub(crate) semantic_state: Option<String>,
    pub(crate) construction_batches: u64,
    pub(crate) plan_batches: u64,
    pub(crate) apply_batches: u64,
    pub(crate) accepted_revisions: Vec<String>,
    pub(crate) batches: Vec<BatchEvidence>,
    pub(crate) rename: Option<RenameEvidence>,
    pub(crate) public_reads: Option<PublicReadEvidence>,
    pub(crate) check: Option<CheckEvidence>,
    pub(crate) cache_reset_before_clean_build: Option<bool>,
    pub(crate) builds: Vec<BuildEvidence>,
    pub(crate) clean_exact_artifacts_equal: Option<bool>,
    pub(crate) oracle: Option<OracleEvidence>,
    pub(crate) catalog: Option<CatalogInventory>,
    pub(crate) observed: Option<SemanticCounts>,
    pub(crate) public_oracle_equal: Option<bool>,
    pub(crate) shape_digest: Option<VerificationDigest>,
    pub(crate) complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BatchEvidence {
    pub(crate) kind: String,
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) logical_items: u64,
    pub(crate) request_records: u64,
    pub(crate) request_bytes: u64,
    pub(crate) request_digest: VerificationDigest,
    pub(crate) request_path: String,
    pub(crate) plan_command: u64,
    pub(crate) apply_command: u64,
    pub(crate) base_revision: String,
    pub(crate) result_revision: String,
    pub(crate) plan_token: String,
    pub(crate) allocated_identities: u64,
    pub(crate) identity_digest: VerificationDigest,
    pub(crate) compiler_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenameEvidence {
    pub(crate) owner: String,
    pub(crate) before: String,
    pub(crate) after: String,
    pub(crate) base_revision: String,
    pub(crate) result_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicReadEvidence {
    pub(crate) revision: String,
    pub(crate) status_owners: u64,
    pub(crate) inspected_owner: String,
    pub(crate) inspected_kind: String,
    pub(crate) owner_query_returned: u64,
    pub(crate) owner_query_visited: u64,
    pub(crate) owner_query_truncated: bool,
    pub(crate) name_query_owner: String,
    pub(crate) context_owner: String,
    pub(crate) context_returned: u64,
    pub(crate) context_total_owners: u64,
    pub(crate) context_total_relations: u64,
    pub(crate) context_truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompilationEvidence {
    pub(crate) cache: String,
    pub(crate) manifest: String,
    pub(crate) compiled: u64,
    pub(crate) reused: u64,
    pub(crate) removed: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactEvidence {
    pub(crate) manifest: String,
    pub(crate) bundle: String,
    pub(crate) bytes: u64,
    pub(crate) packages: u64,
    pub(crate) closure_objects: u64,
    pub(crate) compiler_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckEvidence {
    pub(crate) command: u64,
    pub(crate) revision: String,
    pub(crate) compilation: CompilationEvidence,
    pub(crate) artifact: ArtifactEvidence,
    pub(crate) tests_passed: u64,
    pub(crate) tests_failed: u64,
    pub(crate) differential: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BuildEvidence {
    pub(crate) mode: String,
    pub(crate) command: u64,
    pub(crate) revision: String,
    pub(crate) compilation: CompilationEvidence,
    pub(crate) artifact: ArtifactEvidence,
    pub(crate) output: FileProof,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OracleEvidence {
    pub(crate) revision: String,
    pub(crate) owners: u64,
    pub(crate) modules: u64,
    pub(crate) functions: u64,
    pub(crate) relations: u64,
    pub(crate) types: u64,
    pub(crate) dependencies: u64,
    pub(crate) retirements: u64,
    pub(crate) owner_kinds: BTreeMap<String, u64>,
    pub(crate) owner_identity_digest: VerificationDigest,
    pub(crate) relation_digest: VerificationDigest,
    pub(crate) validation_owner_records: u64,
    pub(crate) validation_type_objects: u64,
    pub(crate) validation_expression_records: u64,
    pub(crate) validation_relation_edges: u64,
    pub(crate) validation_work: u64,
    pub(crate) map_pages_read: u64,
    pub(crate) map_bytes_read: u64,
    pub(crate) store_objects_read: u64,
    pub(crate) store_bytes_read: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandEvidence {
    pub(crate) ordinal: u64,
    pub(crate) name: String,
    pub(crate) command: Vec<String>,
    pub(crate) classification: ProcessStatus,
    pub(crate) response_digest: Option<VerificationDigest>,
    pub(crate) response_records: Option<u64>,
    pub(crate) process: ProcessObservation,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileAreaEvidence {
    pub(crate) files: u64,
    pub(crate) bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostObservation {
    pub(crate) operating_system: String,
    pub(crate) architecture: String,
    pub(crate) kernel: Option<String>,
    pub(crate) logical_cpus: Option<u64>,
    pub(crate) memory_total_bytes: Option<u64>,
    pub(crate) memory_available_bytes: Option<u64>,
    pub(crate) filesystem: Option<String>,
    pub(crate) disk_total_bytes: Option<u64>,
    pub(crate) disk_available_bytes: Option<u64>,
    pub(crate) unavailable_dimensions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObservationEvidence {
    pub(crate) started_unix_nanoseconds: u128,
    pub(crate) completed_unix_nanoseconds: u128,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) host: HostObservation,
    pub(crate) child_cpu_nanoseconds: Option<u64>,
    pub(crate) maximum_child_peak_rss_kib: Option<u64>,
    pub(crate) harness_peak_rss_kib: Option<u64>,
    pub(crate) repository: Option<FileAreaEvidence>,
    pub(crate) derived: Option<FileAreaEvidence>,
    pub(crate) artifacts: Option<FileAreaEvidence>,
    pub(crate) total_run_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CleanupStatus {
    Removed,
    Retained,
    NotCreated,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CleanupEvidence {
    pub(crate) status: CleanupStatus,
    pub(crate) project_path: String,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailureEvidence {
    pub(crate) class: String,
    pub(crate) message: String,
}

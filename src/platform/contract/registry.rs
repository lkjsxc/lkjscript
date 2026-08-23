use super::super::artifact::{ARTIFACT_CONTRACT_VERSION, PACKAGE_ARTIFACT_CONTRACT_VERSION};
use super::super::bootstrap::{BOOTSTRAP_CONTRACT_VERSION, ProjectTemplate};
use super::super::configuration::CONFIGURATION_ADAPTER_CONTRACT_VERSION;
use super::super::database::POSTGRES_ADAPTER_CONTRACT_VERSION;
use super::super::deployment::DEPLOYMENT_CONTRACT_VERSION;
use super::super::diagnostic::DiagnosticClass;
use super::super::execution::CAPABILITY_GRANT_CONTRACT_VERSION;
use super::super::graph::{ROOT_STORAGE_CONTRACT_IDENTITY, ROOT_STORAGE_CONTRACT_VERSION};
use super::super::http::HTTP_ADAPTER_CONTRACT_VERSION;
use super::super::json::JSON_CONTRACT_VERSION;
use super::super::meaning::{GRAPH_CONTRACT_IDENTITY, GRAPH_CONTRACT_VERSION, RelationRole};
use super::super::object::OBJECT_ADAPTER_CONTRACT_VERSION;
use super::super::package::{PACKAGE_CONTRACT_VERSION, RunnerKind};
use super::super::queue::DURABLE_QUEUE_CONTRACT_VERSION;
use super::super::repository::{
    BACKUP_CONTRACT_VERSION, BACKUP_SEGMENT_ENTRY_LIMIT, MAXIMUM_BACKUP_MANIFEST_BYTES,
    MAXIMUM_BACKUP_SEGMENT_BYTES, RETENTION_CONTRACT_VERSION,
};
use super::super::revision::{RECEIPT_CONTRACT_VERSION, REVISION_CONTRACT_VERSION};
use super::super::runtime::RESIDENT_RUNTIME_CONTRACT_VERSION;
use super::super::secrets::{SECRET_CATALOG_CONTRACT_VERSION, SECRET_VERIFIER_CONTRACT_VERSION};
use super::super::security::SECURITY_ADAPTER_CONTRACT_VERSION;
use super::super::semantic_change::{
    CHANGE_CONTRACT_VERSION, ChangeKind, ExpressionFormKind, TypeFormKind,
};
use super::super::semantic_diff::SEMANTIC_DIFF_CONTRACT_VERSION;
use super::super::semantic_draft::DRAFT_CONTRACT_VERSION;
use super::super::semantic_fact::{
    SEMANTIC_FACT_CONTRACT_IDENTITY, SEMANTIC_FACT_CONTRACT_VERSION,
};
use super::super::semantic_merge::SEMANTIC_MERGE_CONTRACT_VERSION;
use super::super::semantic_projection::REVIEW_PROJECTION_CONTRACT_VERSION;
use super::super::semantic_query::{
    MAXIMUM_BYTE_LIMIT, MAXIMUM_ITEM_LIMIT, MAXIMUM_QUERY_DEPTH, MAXIMUM_QUERY_FANOUT,
    MAXIMUM_WORK_LIMIT, OwnerKind, QUERY_CONTRACT_VERSION, QUERY_INDEX_CONTRACT_VERSION,
};
use super::super::semantic_summary::{
    SEMANTIC_SUMMARY_CONTRACT_IDENTITY, SEMANTIC_SUMMARY_CONTRACT_VERSION,
    SEMANTIC_VALIDATOR_CONTRACT_IDENTITY,
};
use super::super::semantic_transaction::{
    MAXIMUM_TRANSACTION_AFFECTED_OWNERS, MAXIMUM_TRANSACTION_OPERATIONS, MAXIMUM_TRANSACTION_WORK,
    TRANSACTION_CONTRACT_VERSION,
};
use super::super::stream::STREAM_CONTRACT_VERSION;
use super::super::worker::WORKER_RUNNER_CONTRACT_VERSION;
use super::super::workspace::WORKSPACE_CONTRACT_VERSION;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const REGISTRY_CONTRACT_IDENTITY: &str = "lkjscript-contract-registry-1";
pub const REGISTRY_CONTRACT_VERSION: u16 = 1;
pub const CLI_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_CLI_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_TRANSACTION_REQUEST_BYTES: usize = 16 * 1_048_576;

const REGISTRY_DIGEST_DOMAIN: &str = "lkjscript.contract-registry.v1";
const REGISTRY_SECTION_DIGEST_DOMAIN: &str = "lkjscript.contract-registry-section.v1";
pub(crate) const PROTOCOL_SCHEMA_DIGEST_DOMAIN: &str = "lkjscript.protocol-schema.v1";

pub(crate) const MODULE_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.module-object.v2";
pub(crate) const ROOT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.root-object.v2";
pub(crate) const SEMANTIC_DIFF_DIGEST_DOMAIN: &str = "lkjscript.semantic-diff.v1";
pub(crate) const RECEIPT_DIGEST_DOMAIN: &str = "lkjscript.receipt.v1";
pub(crate) const TRANSACTION_DIGEST_DOMAIN: &str = "lkjscript.transaction.v3";
pub(crate) const INDEX_DIGEST_DOMAIN: &str = "lkjscript.index.v2";
pub(crate) const BACKUP_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.backup.v2";
pub(crate) const ARTIFACT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.artifact.v3";
pub(crate) const REVISION_RECORD_DIGEST_DOMAIN: &str = "lkjscript.revision-record.v2";
pub(crate) const CLEANUP_PLAN_DIGEST_DOMAIN: &str = "lkjscript.cleanup-plan.v1";
pub(crate) const SEMANTIC_REVISION_DIGEST_DOMAIN: &str = "lkjscript.semantic-revision.v1";
pub(crate) const PACKAGE_REVISION_DIGEST_DOMAIN: &str = "lkjscript.package-revision.v1";
pub(crate) const IDENTITY_MIGRATION_DIGEST_DOMAIN: &str =
    "lkjscript.semantic-identity.migration.v1";
pub(crate) const REQUEST_LOCAL_IDENTITY_DIGEST_DOMAIN: &str =
    "lkjscript.semantic-identity.request-local-allocation.v1";
pub(crate) const REVIEW_PROJECTION_DIGEST_DOMAIN: &str = "lkjscript.semantic-review-projection.v1";
pub(crate) const LOCAL_INDEX_BUCKET_DIGEST_DOMAIN: &str =
    "lkjscript.semantic-local-index-bucket.v1";
pub(crate) const CONTINUATION_DIGEST_DOMAIN: &str = "lkjscript.semantic-continuation.v1";
pub(crate) const QUERY_DIGEST_DOMAIN: &str = "lkjscript.semantic-query.v1";
pub(crate) const CLEANUP_CANDIDATE_DIGEST_DOMAIN: &str = "lkjscript.cleanup-candidate.v1";

const fn magic_bytes(value: &str) -> [u8; 8] {
    let bytes = value.as_bytes();
    [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]
}

const ARTIFACT_MAGIC_TEXT: &str = "LKJART04";
pub(crate) const ARTIFACT_MAGIC: [u8; 8] = magic_bytes(ARTIFACT_MAGIC_TEXT);
pub(crate) const ARTIFACT_DOMAIN: &str = "lkjscript.graph-artifact-bundle.v4";
const PACKAGE_ARTIFACT_MAGIC_TEXT: &str = "LKJPKG03";
pub(crate) const PACKAGE_ARTIFACT_MAGIC: [u8; 8] = magic_bytes(PACKAGE_ARTIFACT_MAGIC_TEXT);
pub(crate) const PACKAGE_ARTIFACT_DOMAIN: &str = "lkjscript.graph-package-artifact.v3";
const LOGICAL_ROOT_MAGIC_TEXT: &str = "LKJGRF04";
pub(crate) const LOGICAL_ROOT_MAGIC: [u8; 8] = magic_bytes(LOGICAL_ROOT_MAGIC_TEXT);
pub(crate) const LOGICAL_ROOT_DIGEST_DOMAIN: &str = "lkjscript.logical-graph-root.v4";
const STORED_ROOT_MAGIC_TEXT: &str = "LKJROOT3";
pub(crate) const STORED_ROOT_MAGIC: [u8; 8] = magic_bytes(STORED_ROOT_MAGIC_TEXT);
pub(crate) const STORED_ROOT_DIGEST_DOMAIN: &str = "lkjscript.persistent-root-object.v2";
const MODULE_MAGIC_TEXT: &str = "LKJMOD04";
pub(crate) const MODULE_MAGIC: [u8; 8] = magic_bytes(MODULE_MAGIC_TEXT);
pub(crate) const MODULE_DIGEST_DOMAIN: &str = "lkjscript.semantic-module-object.v4";
const MAP_PAGE_MAGIC_TEXT: &str = "LKJPMAP1";
pub(crate) const MAP_PAGE_MAGIC: [u8; 8] = magic_bytes(MAP_PAGE_MAGIC_TEXT);
pub(crate) const MAP_PAGE_CONTRACT_VERSION: u16 = 1;
pub(crate) const MAP_PAGE_DIGEST_DOMAIN: &str = "lkjscript.persistent-map-page.v1";
pub(crate) const MAP_PAGE_CHECKSUM_DOMAIN: &str = "lkjscript.persistent-map-checksum.v1";
const REVISION_MAGIC_TEXT: &str = "LKJREV04";
pub(crate) const REVISION_MAGIC: [u8; 8] = magic_bytes(REVISION_MAGIC_TEXT);
pub(crate) const REVISION_DOMAIN: &str = "lkjscript.revision-record-envelope.v4";
const RECEIPT_MAGIC_TEXT: &str = "LKJRCPT3";
pub(crate) const RECEIPT_MAGIC: [u8; 8] = magic_bytes(RECEIPT_MAGIC_TEXT);
pub(crate) const RECEIPT_DOMAIN: &str = "lkjscript.transaction-receipt-envelope.v3";
const HEAD_MAGIC_TEXT: &str = "LKJHEAD4";
pub(crate) const HEAD_MAGIC: [u8; 8] = magic_bytes(HEAD_MAGIC_TEXT);
pub(crate) const HEAD_DOMAIN: &str = "lkjscript.semantic-head-envelope.v4";
const BACKUP_MAGIC_TEXT: &str = "LKJBKP04";
pub(crate) const BACKUP_MAGIC: [u8; 8] = magic_bytes(BACKUP_MAGIC_TEXT);
pub(crate) const BACKUP_DIGEST_DOMAIN: &str = "lkjscript.semantic-backup.v4";
const BACKUP_SEGMENT_MAGIC_TEXT: &str = "LKJBKS04";
pub(crate) const BACKUP_SEGMENT_MAGIC: [u8; 8] = magic_bytes(BACKUP_SEGMENT_MAGIC_TEXT);
pub(crate) const BACKUP_SEGMENT_DIGEST_DOMAIN: &str = "lkjscript.semantic-backup-segment.v4";
pub(crate) const BACKUP_SEGMENT_REFERENCE_DIGEST_DOMAIN: &str =
    "lkjscript.semantic-backup-segment-reference.v4";
pub(crate) const BACKUP_ENTRY_DIGEST_DOMAIN: &str = "lkjscript.semantic-backup-entry.v4";
const DRAFT_MAGIC_TEXT: &str = "LKJDRF04";
pub(crate) const DRAFT_MAGIC: [u8; 8] = magic_bytes(DRAFT_MAGIC_TEXT);
pub(crate) const DRAFT_DIGEST_DOMAIN: &str = "lkjscript.semantic-draft.v4";
const FACT_MANIFEST_MAGIC_TEXT: &str = "LKJSFI03";
pub(crate) const FACT_MANIFEST_MAGIC: [u8; 8] = magic_bytes(FACT_MANIFEST_MAGIC_TEXT);
pub(crate) const FACT_MANIFEST_DOMAIN: &str = "lkjscript.semantic-fact-manifest.v3";
pub(crate) const SEMANTIC_CERTIFICATE_DOMAIN: &str = "lkjscript.semantic-certificate.v3";
const QUERY_INDEX_MAGIC_TEXT: &str = "LKJIDX02";
pub(crate) const QUERY_INDEX_MAGIC: [u8; 8] = magic_bytes(QUERY_INDEX_MAGIC_TEXT);
pub(crate) const QUERY_INDEX_DOMAIN: &str = "lkjscript.semantic-query-index.v2";
pub(crate) const LOCAL_INDEX_CONTRACT_VERSION: u16 = 3;
const LOCAL_MANIFEST_MAGIC_TEXT: &str = "LKJIXM03";
pub(crate) const LOCAL_MANIFEST_MAGIC: [u8; 8] = magic_bytes(LOCAL_MANIFEST_MAGIC_TEXT);
const LOCAL_OWNER_MAGIC_TEXT: &str = "LKJIXO03";
pub(crate) const LOCAL_OWNER_MAGIC: [u8; 8] = magic_bytes(LOCAL_OWNER_MAGIC_TEXT);
const LOCAL_NAME_MAGIC_TEXT: &str = "LKJIXN03";
pub(crate) const LOCAL_NAME_MAGIC: [u8; 8] = magic_bytes(LOCAL_NAME_MAGIC_TEXT);
pub(crate) const LOCAL_MANIFEST_DOMAIN: &str = "lkjscript.semantic-local-index-manifest.v3";
pub(crate) const LOCAL_OWNER_DOMAIN: &str = "lkjscript.semantic-local-owner-index.v3";
pub(crate) const LOCAL_NAME_DOMAIN: &str = "lkjscript.semantic-local-name-index.v3";
const SUMMARY_MAGIC_TEXT: &str = "LKJSUM03";
pub(crate) const SUMMARY_MAGIC: [u8; 8] = magic_bytes(SUMMARY_MAGIC_TEXT);
pub(crate) const SUMMARY_ENVELOPE_DOMAIN: &str = "lkjscript.semantic-summary-envelope.v3";
pub(crate) const VALIDATOR_DIGEST_DOMAIN: &str = "lkjscript.semantic-validator-contract.v3";
pub(crate) const SUMMARY_INPUT_DIGEST_DOMAIN: &str = "lkjscript.semantic-summary-input.v3";
pub(crate) const PUBLIC_SIGNATURE_DIGEST_DOMAIN: &str = "lkjscript.public-signature-summary.v3";
pub(crate) const DECLARATION_SIGNATURE_DIGEST_DOMAIN: &str = "lkjscript.declaration-signature.v3";
pub(crate) const DECLARATION_IMPLEMENTATION_DIGEST_DOMAIN: &str =
    "lkjscript.declaration-implementation.v3";
pub(crate) const DECLARATION_EFFECT_DIGEST_DOMAIN: &str = "lkjscript.declaration-effect.v3";
pub(crate) const MODULE_IMPLEMENTATION_DIGEST_DOMAIN: &str = "lkjscript.module-implementation.v3";
pub(crate) const SUMMARY_DEPENDENCY_DIGEST_DOMAIN: &str =
    "lkjscript.semantic-summary-dependencies.v3";
pub(crate) const SUMMARY_RECORD_DIGEST_DOMAIN: &str = "lkjscript.semantic-summary-record.v3";
pub(crate) const CHANGE_ALLOCATION_SEED_DOMAIN: &str = "lkjscript.change-allocation-seed.v1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKey {
    Registry,
    Cli,
    MeaningGraph,
    PersistentRoot,
    Revision,
    Receipt,
    SemanticSummary,
    SemanticFacts,
    SemanticValidator,
    Change,
    Transaction,
    Query,
    QueryIndex,
    LocalQueryIndex,
    Draft,
    Diff,
    Merge,
    ReviewProjection,
    Artifact,
    PackageArtifact,
    Backup,
    Bootstrap,
    Retention,
    PackageDescriptor,
    Deployment,
    ConfigurationAdapter,
    PostgresAdapter,
    CapabilityGrant,
    HttpAdapter,
    Json,
    ObjectAdapter,
    QueueAdapter,
    ResidentRuntime,
    SecretCatalog,
    SecretVerifier,
    SecurityAdapter,
    Stream,
    WorkerRunner,
    Workspace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStability {
    Current,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractAuthority {
    CanonicalMeaning,
    AcceptedHistory,
    RequiredWitness,
    DerivedDisposable,
    PublicProtocol,
    Operational,
    Deployment,
    Runtime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PredecessorPolicy {
    Reject,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContractDescriptor {
    pub key: ContractKey,
    pub name: &'static str,
    pub identity: &'static str,
    pub version: u16,
    pub stability: ContractStability,
    pub authority: ContractAuthority,
    pub predecessor_policy: PredecessorPolicy,
    pub magic_values: &'static [&'static str],
    pub digest_domains: &'static [&'static str],
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(rename = "lkjscript.ContractDescriptorV1")]
#[serde(deny_unknown_fields)]
pub struct ContractManifestEntry {
    pub key: String,
    pub name: String,
    pub identity: String,
    pub version: u16,
    pub stability: String,
    pub authority: String,
    pub predecessor_policy: String,
    pub magic_values: Vec<String>,
    pub digest_domains: Vec<String>,
}

pub fn contract_descriptors() -> &'static [ContractDescriptor] {
    const CURRENT: ContractStability = ContractStability::Current;
    const REJECT: PredecessorPolicy = PredecessorPolicy::Reject;
    const NONE: &[&str] = &[];
    const CONTRACTS: &[ContractDescriptor] = &[
        ContractDescriptor {
            key: ContractKey::Registry,
            name: "contract registry",
            identity: REGISTRY_CONTRACT_IDENTITY,
            version: REGISTRY_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: PredecessorPolicy::NotApplicable,
            magic_values: NONE,
            digest_domains: &[
                REGISTRY_DIGEST_DOMAIN,
                REGISTRY_SECTION_DIGEST_DOMAIN,
                PROTOCOL_SCHEMA_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Cli,
            name: "command line protocol",
            identity: "lkjscript-cli-4",
            version: CLI_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: NONE,
        },
        ContractDescriptor {
            key: ContractKey::MeaningGraph,
            name: "meaning graph",
            identity: GRAPH_CONTRACT_IDENTITY,
            version: GRAPH_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::CanonicalMeaning,
            predecessor_policy: REJECT,
            magic_values: &[LOGICAL_ROOT_MAGIC_TEXT, MODULE_MAGIC_TEXT],
            digest_domains: &[
                LOGICAL_ROOT_DIGEST_DOMAIN,
                MODULE_DIGEST_DOMAIN,
                MODULE_OBJECT_DIGEST_DOMAIN,
                IDENTITY_MIGRATION_DIGEST_DOMAIN,
                REQUEST_LOCAL_IDENTITY_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::PersistentRoot,
            name: "persistent graph root",
            identity: ROOT_STORAGE_CONTRACT_IDENTITY,
            version: ROOT_STORAGE_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::CanonicalMeaning,
            predecessor_policy: REJECT,
            magic_values: &[STORED_ROOT_MAGIC_TEXT, MAP_PAGE_MAGIC_TEXT],
            digest_domains: &[
                STORED_ROOT_DIGEST_DOMAIN,
                ROOT_OBJECT_DIGEST_DOMAIN,
                MAP_PAGE_DIGEST_DOMAIN,
                MAP_PAGE_CHECKSUM_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Revision,
            name: "revision record",
            identity: "lkjscript-revision-4",
            version: REVISION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::AcceptedHistory,
            predecessor_policy: REJECT,
            magic_values: &[REVISION_MAGIC_TEXT, HEAD_MAGIC_TEXT],
            digest_domains: &[
                REVISION_DOMAIN,
                HEAD_DOMAIN,
                REVISION_RECORD_DIGEST_DOMAIN,
                SEMANTIC_REVISION_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Receipt,
            name: "transaction receipt",
            identity: "lkjscript-receipt-3",
            version: RECEIPT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::AcceptedHistory,
            predecessor_policy: REJECT,
            magic_values: &[RECEIPT_MAGIC_TEXT],
            digest_domains: &[RECEIPT_DOMAIN, RECEIPT_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::SemanticSummary,
            name: "semantic summary",
            identity: SEMANTIC_SUMMARY_CONTRACT_IDENTITY,
            version: SEMANTIC_SUMMARY_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::RequiredWitness,
            predecessor_policy: REJECT,
            magic_values: &[SUMMARY_MAGIC_TEXT],
            digest_domains: &[
                SUMMARY_ENVELOPE_DOMAIN,
                SUMMARY_INPUT_DIGEST_DOMAIN,
                PUBLIC_SIGNATURE_DIGEST_DOMAIN,
                DECLARATION_SIGNATURE_DIGEST_DOMAIN,
                DECLARATION_IMPLEMENTATION_DIGEST_DOMAIN,
                DECLARATION_EFFECT_DIGEST_DOMAIN,
                MODULE_IMPLEMENTATION_DIGEST_DOMAIN,
                SUMMARY_DEPENDENCY_DIGEST_DOMAIN,
                SUMMARY_RECORD_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::SemanticFacts,
            name: "semantic fact witness",
            identity: SEMANTIC_FACT_CONTRACT_IDENTITY,
            version: SEMANTIC_FACT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::RequiredWitness,
            predecessor_policy: REJECT,
            magic_values: &[FACT_MANIFEST_MAGIC_TEXT],
            digest_domains: &[FACT_MANIFEST_DOMAIN, SEMANTIC_CERTIFICATE_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::SemanticValidator,
            name: "semantic validator",
            identity: SEMANTIC_VALIDATOR_CONTRACT_IDENTITY,
            version: 3,
            stability: CURRENT,
            authority: ContractAuthority::RequiredWitness,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[VALIDATOR_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Change,
            name: "authored semantic change",
            identity: "lkjscript-change-3",
            version: CHANGE_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[CHANGE_ALLOCATION_SEED_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Transaction,
            name: "normalized semantic transaction",
            identity: "lkjscript-transaction-4",
            version: TRANSACTION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[TRANSACTION_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Query,
            name: "semantic query",
            identity: "lkjscript-query-2",
            version: QUERY_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[QUERY_DIGEST_DOMAIN, CONTINUATION_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::QueryIndex,
            name: "broad semantic query index",
            identity: "lkjscript-query-index-2",
            version: QUERY_INDEX_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &[QUERY_INDEX_MAGIC_TEXT],
            digest_domains: &[QUERY_INDEX_DOMAIN, INDEX_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::LocalQueryIndex,
            name: "local exact query index",
            identity: "lkjscript-local-query-index-3",
            version: 3,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &[
                LOCAL_MANIFEST_MAGIC_TEXT,
                LOCAL_OWNER_MAGIC_TEXT,
                LOCAL_NAME_MAGIC_TEXT,
            ],
            digest_domains: &[
                LOCAL_MANIFEST_DOMAIN,
                LOCAL_OWNER_DOMAIN,
                LOCAL_NAME_DOMAIN,
                LOCAL_INDEX_BUCKET_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Draft,
            name: "semantic draft",
            identity: "lkjscript-draft-4",
            version: DRAFT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Operational,
            predecessor_policy: REJECT,
            magic_values: &[DRAFT_MAGIC_TEXT],
            digest_domains: &[DRAFT_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Diff,
            name: "semantic diff",
            identity: "lkjscript-diff-2",
            version: SEMANTIC_DIFF_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[SEMANTIC_DIFF_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Merge,
            name: "semantic merge",
            identity: "lkjscript-merge-2",
            version: SEMANTIC_MERGE_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: NONE,
        },
        ContractDescriptor {
            key: ContractKey::ReviewProjection,
            name: "review projection",
            identity: "lkjscript-review-projection-2",
            version: REVIEW_PROJECTION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[REVIEW_PROJECTION_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Artifact,
            name: "graph artifact bundle",
            identity: "lkjscript-artifact-4",
            version: ARTIFACT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Runtime,
            predecessor_policy: REJECT,
            magic_values: &[ARTIFACT_MAGIC_TEXT],
            digest_domains: &[ARTIFACT_DOMAIN, ARTIFACT_OBJECT_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::PackageArtifact,
            name: "package artifact object",
            identity: "lkjscript-package-artifact-3",
            version: PACKAGE_ARTIFACT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Runtime,
            predecessor_policy: REJECT,
            magic_values: &[PACKAGE_ARTIFACT_MAGIC_TEXT],
            digest_domains: &[PACKAGE_ARTIFACT_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Backup,
            name: "repository backup",
            identity: "lkjscript-backup-4",
            version: BACKUP_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Operational,
            predecessor_policy: REJECT,
            magic_values: &[BACKUP_MAGIC_TEXT, BACKUP_SEGMENT_MAGIC_TEXT],
            digest_domains: &[
                BACKUP_DIGEST_DOMAIN,
                BACKUP_SEGMENT_DIGEST_DOMAIN,
                BACKUP_SEGMENT_REFERENCE_DIGEST_DOMAIN,
                BACKUP_ENTRY_DIGEST_DOMAIN,
                BACKUP_OBJECT_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Bootstrap,
            name: "project bootstrap",
            identity: "lkjscript-bootstrap-2",
            version: BOOTSTRAP_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: NONE,
        },
        ContractDescriptor {
            key: ContractKey::Retention,
            name: "retention preview",
            identity: "lkjscript-retention-1",
            version: RETENTION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Operational,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[CLEANUP_PLAN_DIGEST_DOMAIN, CLEANUP_CANDIDATE_DIGEST_DOMAIN],
        },
        simple_contract_with_domains(
            ContractKey::PackageDescriptor,
            "package descriptor",
            "lkjscript-package-descriptor-1",
            PACKAGE_CONTRACT_VERSION,
            ContractAuthority::PublicProtocol,
            &[PACKAGE_REVISION_DIGEST_DOMAIN],
        ),
        simple_contract(
            ContractKey::Deployment,
            "deployment descriptor",
            "lkjscript-deployment-1",
            DEPLOYMENT_CONTRACT_VERSION,
            ContractAuthority::Deployment,
        ),
        simple_contract(
            ContractKey::ConfigurationAdapter,
            "configuration adapter",
            "lkjscript-configuration-adapter-1",
            CONFIGURATION_ADAPTER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::PostgresAdapter,
            "PostgreSQL adapter",
            "lkjscript-postgres-adapter-1",
            POSTGRES_ADAPTER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::CapabilityGrant,
            "capability grant",
            "lkjscript-capability-grant-1",
            CAPABILITY_GRANT_CONTRACT_VERSION,
            ContractAuthority::Deployment,
        ),
        simple_contract(
            ContractKey::HttpAdapter,
            "HTTP adapter",
            "lkjscript-http-adapter-1",
            HTTP_ADAPTER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::Json,
            "JSON value",
            "lkjscript-json-1",
            JSON_CONTRACT_VERSION,
            ContractAuthority::PublicProtocol,
        ),
        simple_contract(
            ContractKey::ObjectAdapter,
            "object storage adapter",
            "lkjscript-object-adapter-1",
            OBJECT_ADAPTER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::QueueAdapter,
            "durable queue adapter",
            "lkjscript-queue-adapter-1",
            DURABLE_QUEUE_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::ResidentRuntime,
            "resident runtime",
            "lkjscript-resident-runtime-1",
            RESIDENT_RUNTIME_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::SecretCatalog,
            "secret catalog",
            "lkjscript-secret-catalog-1",
            SECRET_CATALOG_CONTRACT_VERSION,
            ContractAuthority::Deployment,
        ),
        simple_contract(
            ContractKey::SecretVerifier,
            "secret verifier",
            "lkjscript-secret-verifier-1",
            SECRET_VERIFIER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::SecurityAdapter,
            "security adapter",
            "lkjscript-security-adapter-1",
            SECURITY_ADAPTER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::Stream,
            "stream",
            "lkjscript-stream-1",
            STREAM_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::WorkerRunner,
            "worker runner",
            "lkjscript-worker-runner-1",
            WORKER_RUNNER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::Workspace,
            "semantic workspace",
            "lkjscript-workspace-3",
            WORKSPACE_CONTRACT_VERSION,
            ContractAuthority::PublicProtocol,
        ),
    ];
    CONTRACTS
}

const fn simple_contract(
    key: ContractKey,
    name: &'static str,
    identity: &'static str,
    version: u16,
    authority: ContractAuthority,
) -> ContractDescriptor {
    ContractDescriptor {
        key,
        name,
        identity,
        version,
        stability: ContractStability::Current,
        authority,
        predecessor_policy: PredecessorPolicy::Reject,
        magic_values: &[],
        digest_domains: &[],
    }
}

const fn simple_contract_with_domains(
    key: ContractKey,
    name: &'static str,
    identity: &'static str,
    version: u16,
    authority: ContractAuthority,
    digest_domains: &'static [&'static str],
) -> ContractDescriptor {
    ContractDescriptor {
        key,
        name,
        identity,
        version,
        stability: ContractStability::Current,
        authority,
        predecessor_policy: PredecessorPolicy::Reject,
        magic_values: &[],
        digest_domains,
    }
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[schemars(rename = "lkjscript.PublicOperationV1")]
#[serde(rename_all = "snake_case")]
pub enum PublicOperation {
    Capabilities,
    New,
    Inspect,
    Query,
    Change,
    Draft,
    History,
    Package,
    Check,
    Build,
    Run,
    Serve,
    Worker,
    Review,
    Backup,
    Restore,
    Doctor,
}

impl PublicOperation {
    pub const ALL: [Self; 17] = [
        Self::Capabilities,
        Self::New,
        Self::Inspect,
        Self::Query,
        Self::Change,
        Self::Draft,
        Self::History,
        Self::Package,
        Self::Check,
        Self::Build,
        Self::Run,
        Self::Serve,
        Self::Worker,
        Self::Review,
        Self::Backup,
        Self::Restore,
        Self::Doctor,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::New => "new",
            Self::Inspect => "inspect",
            Self::Query => "query",
            Self::Change => "change",
            Self::Draft => "draft",
            Self::History => "history",
            Self::Package => "package",
            Self::Check => "check",
            Self::Build => "build",
            Self::Run => "run",
            Self::Serve => "serve",
            Self::Worker => "worker",
            Self::Review => "review",
            Self::Backup => "backup",
            Self::Restore => "restore",
            Self::Doctor => "doctor",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operation| operation.name() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.AuthorityEffectV1")]
#[serde(rename_all = "snake_case")]
pub enum AuthorityEffect {
    None,
    Accepted,
    AcceptedOnCommit,
    DraftOrAccepted,
    Operational,
    ExternalOutput,
    ExternalRuntime,
    OperationalIndexesOnly,
    OptionalExternalOutput,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.ProjectRequirementV1")]
#[serde(rename_all = "snake_case")]
pub enum ProjectRequirement {
    None,
    Destination,
    Required,
    RequiredByAction,
    RequiredForStage,
    DescriptorBound,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.BudgetProfileV1")]
#[serde(rename_all = "snake_case")]
pub enum BudgetProfile {
    Discovery,
    BoundedRead,
    SemanticChange,
    Build,
    Runtime,
    Maintenance,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.SchemaIdV1")]
#[serde(rename_all = "snake_case")]
pub enum SchemaId {
    CapabilitiesRequest,
    NewRequest,
    InspectRequest,
    QueryRequest,
    ChangeRequest,
    DraftRequest,
    HistoryRequest,
    PackageRequest,
    CheckRequest,
    BuildRequest,
    RunRequest,
    ServeRequest,
    WorkerRequest,
    ReviewRequest,
    BackupRequest,
    RestoreRequest,
    DoctorRequest,
    CliSuccess,
    RuntimeEvent,
}

impl SchemaId {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CapabilitiesRequest => "capabilities_request",
            Self::NewRequest => "new_request",
            Self::InspectRequest => "inspect_request",
            Self::QueryRequest => "query_request",
            Self::ChangeRequest => "change_request",
            Self::DraftRequest => "draft_request",
            Self::HistoryRequest => "history_request",
            Self::PackageRequest => "package_request",
            Self::CheckRequest => "check_request",
            Self::BuildRequest => "build_request",
            Self::RunRequest => "run_request",
            Self::ServeRequest => "serve_request",
            Self::WorkerRequest => "worker_request",
            Self::ReviewRequest => "review_request",
            Self::BackupRequest => "backup_request",
            Self::RestoreRequest => "restore_request",
            Self::DoctorRequest => "doctor_request",
            Self::CliSuccess => "cli_success",
            Self::RuntimeEvent => "runtime_event",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDescriptor {
    pub operation: PublicOperation,
    pub purpose: &'static str,
    pub usage: &'static str,
    pub request_schema: SchemaId,
    pub response_schema: SchemaId,
    pub authority_effect: AuthorityEffect,
    pub project_requirement: ProjectRequirement,
    pub default_budget: BudgetProfile,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(rename = "lkjscript.OperationDescriptorV1")]
#[serde(deny_unknown_fields)]
pub struct OperationManifestEntry {
    pub name: String,
    pub purpose: String,
    pub usage: String,
    pub request_schema: String,
    pub response_schema: String,
    pub authority_effect: AuthorityEffect,
    pub project_requirement: ProjectRequirement,
    pub default_budget: BudgetProfile,
}

pub fn operation_descriptors() -> &'static [OperationDescriptor] {
    const OPERATIONS: &[OperationDescriptor] = &[
        operation(
            PublicOperation::Capabilities,
            "Discover exact executable contracts and changed schema sections.",
            "capabilities [COMMAND] [--known-schema DIGEST] [--section SECTION] [--known-section SECTION=DIGEST] [--output PATH] [--generate-docs DIR] [--verify-generated DIR]",
            SchemaId::CapabilitiesRequest,
            AuthorityEffect::OptionalExternalOutput,
            ProjectRequirement::None,
            BudgetProfile::Discovery,
        ),
        operation(
            PublicOperation::New,
            "Create fresh canonical authority in an empty destination.",
            "new DEST [--template minimal|command] [--name NAME]",
            SchemaId::NewRequest,
            AuthorityEffect::Accepted,
            ProjectRequirement::Destination,
            BudgetProfile::SemanticChange,
        ),
        operation(
            PublicOperation::Inspect,
            "Inspect project, owner, target, revision, artifact, or deployment state.",
            "inspect status|project|owner|targets|revision|artifact|deployment ...",
            SchemaId::InspectRequest,
            AuthorityEffect::None,
            ProjectRequirement::RequiredByAction,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Query,
            "Select bounded owners, relations, context, and impact.",
            "query owners|find|relations|callers|callees|types|capabilities|context|impact|request ...",
            SchemaId::QueryRequest,
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Change,
            "Normalize and validate or atomically commit one semantic change.",
            "change (--request JSON | --request-file PATH) [--dry-run|--commit]",
            SchemaId::ChangeRequest,
            AuthorityEffect::AcceptedOnCommit,
            ProjectRequirement::Required,
            BudgetProfile::SemanticChange,
        ),
        operation(
            PublicOperation::Draft,
            "Create, inspect, append, rebase, publish, or drop non-executable work.",
            "draft create|status|append|rebase|publish|drop ...",
            SchemaId::DraftRequest,
            AuthorityEffect::DraftOrAccepted,
            ProjectRequirement::Required,
            BudgetProfile::SemanticChange,
        ),
        operation(
            PublicOperation::History,
            "List, inspect, diff, or merge exact revisions.",
            "history list|show|diff|merge ...",
            SchemaId::HistoryRequest,
            AuthorityEffect::AcceptedOnCommit,
            ProjectRequirement::Required,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Package,
            "Stage an exact dependency or inspect/export a built-in package.",
            "package stage PATH | package builtin inspect|export ...",
            SchemaId::PackageRequest,
            AuthorityEffect::Operational,
            ProjectRequirement::RequiredForStage,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Check,
            "Run graph-owned tests through production and independent execution.",
            "check",
            SchemaId::CheckRequest,
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::Runtime,
        ),
        operation(
            PublicOperation::Build,
            "Build a deterministic graph-native artifact.",
            "build [--output PATH]",
            SchemaId::BuildRequest,
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::Build,
        ),
        operation(
            PublicOperation::Run,
            "Run a pure command, batch, or test target through both execution tiers.",
            "run TARGET [--arguments JSON]",
            SchemaId::RunRequest,
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::Runtime,
        ),
        runtime_operation(
            PublicOperation::Serve,
            "Run one plaintext HTTP deployment with bounded shutdown.",
            "serve --deployment DESCRIPTOR",
            SchemaId::ServeRequest,
        ),
        runtime_operation(
            PublicOperation::Worker,
            "Run one bounded worker deployment.",
            "worker --deployment DESCRIPTOR",
            SchemaId::WorkerRequest,
        ),
        operation(
            PublicOperation::Review,
            "Write a deterministic non-authoritative review projection.",
            "review [--revision REV] [--output PATH]",
            SchemaId::ReviewRequest,
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Backup,
            "Capture one exact reachable canonical authority bundle.",
            "backup [--output PATH]",
            SchemaId::BackupRequest,
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Restore,
            "Verify and atomically restore a canonical authority bundle.",
            "restore --backup PATH (--output PROJECT | --project PROJECT)",
            SchemaId::RestoreRequest,
            AuthorityEffect::Accepted,
            ProjectRequirement::Destination,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Doctor,
            "Validate authority or preview exact retained/reclaimable storage.",
            "doctor [--deep] | doctor cleanup",
            SchemaId::DoctorRequest,
            AuthorityEffect::OperationalIndexesOnly,
            ProjectRequirement::Required,
            BudgetProfile::Maintenance,
        ),
    ];
    OPERATIONS
}

const fn operation(
    operation: PublicOperation,
    purpose: &'static str,
    usage: &'static str,
    request_schema: SchemaId,
    authority_effect: AuthorityEffect,
    project_requirement: ProjectRequirement,
    default_budget: BudgetProfile,
) -> OperationDescriptor {
    OperationDescriptor {
        operation,
        purpose,
        usage,
        request_schema,
        response_schema: SchemaId::CliSuccess,
        authority_effect,
        project_requirement,
        default_budget,
    }
}

const fn runtime_operation(
    operation: PublicOperation,
    purpose: &'static str,
    usage: &'static str,
    request_schema: SchemaId,
) -> OperationDescriptor {
    OperationDescriptor {
        operation,
        purpose,
        usage,
        request_schema,
        response_schema: SchemaId::RuntimeEvent,
        authority_effect: AuthorityEffect::ExternalRuntime,
        project_requirement: ProjectRequirement::DescriptorBound,
        default_budget: BudgetProfile::Runtime,
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.LimitClassV1")]
#[serde(rename_all = "snake_case")]
pub enum LimitClass {
    HostileDecoderSafety,
    ExplicitRequestBudget,
    DefaultPagination,
    ImplementationLimitation,
    DeploymentResourcePolicy,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.LimitUnitV1")]
#[serde(rename_all = "snake_case")]
pub enum LimitUnit {
    Bytes,
    Items,
    Work,
    Depth,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.OverridePolicyV1")]
#[serde(rename_all = "snake_case")]
pub enum OverridePolicy {
    Fixed,
    RequestUpToMaximum,
    DeploymentUpToMaximum,
}

#[derive(Clone, Copy, Debug, JsonSchema, Serialize)]
#[schemars(rename = "lkjscript.LimitDescriptorV1")]
#[serde(deny_unknown_fields)]
pub struct LimitDescriptor {
    pub name: &'static str,
    pub value: u64,
    pub class: LimitClass,
    pub unit: LimitUnit,
    pub override_policy: OverridePolicy,
}

pub fn limit_descriptors() -> &'static [LimitDescriptor] {
    const LIMITS: &[LimitDescriptor] = &[
        limit(
            "cli_response_bytes",
            MAXIMUM_CLI_RESPONSE_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "change_request_bytes",
            MAXIMUM_TRANSACTION_REQUEST_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_items",
            MAXIMUM_ITEM_LIMIT,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Items,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_bytes",
            MAXIMUM_BYTE_LIMIT,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Bytes,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_work",
            MAXIMUM_WORK_LIMIT,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Work,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_depth",
            MAXIMUM_QUERY_DEPTH,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Depth,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_fanout",
            MAXIMUM_QUERY_FANOUT,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Items,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "transaction_operations",
            MAXIMUM_TRANSACTION_OPERATIONS,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Items,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "transaction_work",
            MAXIMUM_TRANSACTION_WORK,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Work,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "transaction_affected_owners",
            MAXIMUM_TRANSACTION_AFFECTED_OWNERS,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Items,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "backup_manifest_bytes",
            MAXIMUM_BACKUP_MANIFEST_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "backup_segment_bytes",
            MAXIMUM_BACKUP_SEGMENT_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "backup_segment_entries",
            BACKUP_SEGMENT_ENTRY_LIMIT,
            LimitClass::ImplementationLimitation,
            LimitUnit::Items,
            OverridePolicy::Fixed,
        ),
    ];
    LIMITS
}

const fn limit(
    name: &'static str,
    value: usize,
    class: LimitClass,
    unit: LimitUnit,
    override_policy: OverridePolicy,
) -> LimitDescriptor {
    LimitDescriptor {
        name,
        value: value as u64,
        class,
        unit,
        override_policy,
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticDescriptor {
    pub code: &'static str,
    pub class: DiagnosticClass,
    pub meaning: &'static str,
    pub retry: &'static str,
}

pub fn diagnostic_descriptors() -> &'static [DiagnosticDescriptor] {
    const DIAGNOSTICS: &[DiagnosticDescriptor] = &[
        DiagnosticDescriptor {
            code: "cli_usage",
            class: DiagnosticClass::Source,
            meaning: "The command, option, or argument grammar is invalid.",
            retry: "Correct the request using capabilities output.",
        },
        DiagnosticDescriptor {
            code: "contract_registry_invalid",
            class: DiagnosticClass::Corrupt,
            meaning: "Executable contract descriptors violate uniqueness or completeness.",
            retry: "Use a verified executable build.",
        },
        DiagnosticDescriptor {
            code: "contract_section",
            class: DiagnosticClass::Source,
            meaning: "A requested registry section is unknown or malformed.",
            retry: "Select one advertised section.",
        },
        DiagnosticDescriptor {
            code: "contract_generated_output",
            class: DiagnosticClass::Infrastructure,
            meaning: "Generated contract output could not be published.",
            retry: "Correct the destination permissions or path.",
        },
        DiagnosticDescriptor {
            code: "contract_generated_drift",
            class: DiagnosticClass::Source,
            meaning: "Checked-in generated contract bytes differ from executable truth.",
            retry: "Regenerate with the exact command reported by the diagnostic.",
        },
        DiagnosticDescriptor {
            code: "predecessor_contract",
            class: DiagnosticClass::Source,
            meaning: "Input uses a predecessor contract rejected by direct cutover.",
            retry: "Recreate the request or authority under the advertised current contract.",
        },
    ];
    DIAGNOSTICS
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExitStatusDescriptor {
    pub status: u8,
    pub meaning: &'static str,
}

pub fn exit_status_descriptors() -> &'static [ExitStatusDescriptor] {
    const STATUSES: &[ExitStatusDescriptor] = &[
        ExitStatusDescriptor {
            status: 0,
            meaning: "successful or domain-classified nonfailure response",
        },
        ExitStatusDescriptor {
            status: 2,
            meaning: "usage, source, semantic, precondition, or conflict failure",
        },
        ExitStatusDescriptor {
            status: 3,
            meaning: "capability or cancellation failure",
        },
        ExitStatusDescriptor {
            status: 4,
            meaning: "resource exhaustion",
        },
        ExitStatusDescriptor {
            status: 5,
            meaning: "corrupt authority or derived state",
        },
        ExitStatusDescriptor {
            status: 6,
            meaning: "host infrastructure failure",
        },
        ExitStatusDescriptor {
            status: 7,
            meaning: "stale accepted base",
        },
        ExitStatusDescriptor {
            status: 8,
            meaning: "invalid candidate meaning",
        },
    ];
    STATUSES
}

pub const fn exit_status_for(class: DiagnosticClass) -> u8 {
    match class {
        DiagnosticClass::Source | DiagnosticClass::Semantic => 2,
        DiagnosticClass::Capability | DiagnosticClass::Cancelled => 3,
        DiagnosticClass::Resource => 4,
        DiagnosticClass::Corrupt => 5,
        DiagnosticClass::Infrastructure => 6,
    }
}

pub fn outcome_exit_status(status: &str) -> u8 {
    match status {
        "stale_base" | "stale_head" => 7,
        "invalid_graph" => 8,
        "resource_exhausted" => 4,
        "precondition_failed" | "foreign_identity" | "conflicted" => 2,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateDescriptor {
    pub name: &'static str,
    pub purpose: &'static str,
    pub creates_command_target: bool,
}

pub fn template_descriptors() -> &'static [TemplateDescriptor] {
    const TEMPLATES: &[TemplateDescriptor] = &[
        TemplateDescriptor {
            name: "minimal",
            purpose: "Create the smallest accepted project authority.",
            creates_command_target: false,
        },
        TemplateDescriptor {
            name: "command",
            purpose: "Create one offline command target and graph-owned test.",
            creates_command_target: true,
        },
    ];
    TEMPLATES
}

pub fn nonclaims() -> &'static [&'static str] {
    &[
        "no TLS implementation or encrypted transport guarantee",
        "no hostile-code or hostile-multi-tenant sandbox",
        "no artifact signature or authenticated provenance",
        "no distributed consensus or multi-node publication",
        "no portability claim beyond retained Linux x86-64 evidence",
        "no provider-token or monetary-cost claim without external telemetry",
    ]
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[schemars(rename = "lkjscript.RegistrySectionV1")]
#[serde(rename_all = "snake_case")]
pub enum RegistrySection {
    Contracts,
    Operations,
    Change,
    Type,
    Expression,
    Owners,
    Relations,
    Limits,
    Diagnostics,
    Templates,
    Runners,
    Security,
}

impl RegistrySection {
    pub const ALL: [Self; 12] = [
        Self::Contracts,
        Self::Operations,
        Self::Change,
        Self::Type,
        Self::Expression,
        Self::Owners,
        Self::Relations,
        Self::Limits,
        Self::Diagnostics,
        Self::Templates,
        Self::Runners,
        Self::Security,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Contracts => "contracts",
            Self::Operations => "operations",
            Self::Change => "change",
            Self::Type => "type",
            Self::Expression => "expression",
            Self::Owners => "owners",
            Self::Relations => "relations",
            Self::Limits => "limits",
            Self::Diagnostics => "diagnostics",
            Self::Templates => "templates",
            Self::Runners => "runners",
            Self::Security => "security",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|section| section.name() == value)
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(rename = "lkjscript.RegistryManifestV1")]
#[serde(deny_unknown_fields)]
pub struct RegistryManifest {
    pub contract: String,
    pub version: u16,
    pub graph_contract: String,
    pub cli_contract_version: u16,
    pub contracts: Vec<ContractManifestEntry>,
    pub operations: Vec<OperationManifestEntry>,
    pub sections: BTreeMap<String, String>,
    pub schema_digest: String,
}

#[derive(Clone, Debug)]
pub struct RegistrySnapshot {
    pub manifest: RegistryManifest,
    pub digest: String,
    pub section_values: BTreeMap<RegistrySection, Value>,
}

pub fn registry_snapshot(schema_digest: &str) -> Result<RegistrySnapshot, String> {
    validate_registry()?;
    let section_values = RegistrySection::ALL
        .into_iter()
        .map(|section| (section, section_value(section)))
        .collect::<BTreeMap<_, _>>();
    let section_digests = section_values
        .iter()
        .map(|(section, value)| {
            canonical_json(value)
                .map(|bytes| (section.name().to_owned(), section_digest(*section, &bytes)))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let manifest = RegistryManifest {
        contract: REGISTRY_CONTRACT_IDENTITY.to_owned(),
        version: REGISTRY_CONTRACT_VERSION,
        graph_contract: GRAPH_CONTRACT_IDENTITY.to_owned(),
        cli_contract_version: CLI_CONTRACT_VERSION,
        contracts: contract_descriptors()
            .iter()
            .map(contract_manifest_entry)
            .collect(),
        operations: operation_descriptors()
            .iter()
            .map(operation_manifest_entry)
            .collect(),
        sections: section_digests,
        schema_digest: schema_digest.to_owned(),
    };
    let bytes = canonical_json(&manifest)?;
    let digest = digest(REGISTRY_DIGEST_DOMAIN, &bytes);
    Ok(RegistrySnapshot {
        manifest,
        digest,
        section_values,
    })
}

fn contract_manifest_entry(descriptor: &ContractDescriptor) -> ContractManifestEntry {
    ContractManifestEntry {
        key: serde_json::to_value(descriptor.key)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default(),
        name: descriptor.name.to_owned(),
        identity: descriptor.identity.to_owned(),
        version: descriptor.version,
        stability: enum_name(descriptor.stability),
        authority: enum_name(descriptor.authority),
        predecessor_policy: enum_name(descriptor.predecessor_policy),
        magic_values: descriptor
            .magic_values
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        digest_domains: descriptor
            .digest_domains
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

fn operation_manifest_entry(descriptor: &OperationDescriptor) -> OperationManifestEntry {
    OperationManifestEntry {
        name: descriptor.operation.name().to_owned(),
        purpose: descriptor.purpose.to_owned(),
        usage: descriptor.usage.to_owned(),
        request_schema: descriptor.request_schema.name().to_owned(),
        response_schema: descriptor.response_schema.name().to_owned(),
        authority_effect: descriptor.authority_effect,
        project_requirement: descriptor.project_requirement,
        default_budget: descriptor.default_budget,
    }
}

pub(crate) fn enum_name(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn section_value(section: RegistrySection) -> Value {
    match section {
        RegistrySection::Contracts => json!(contract_descriptors()),
        RegistrySection::Operations => json!(operation_descriptors()),
        RegistrySection::Change => json!({
            "contract": format!("lkjscript-change-{CHANGE_CONTRACT_VERSION}"),
            "forms": ChangeKind::ALL.map(ChangeKind::name),
            "request_schema": SchemaId::ChangeRequest.name(),
            "reference_forms": [
                "request_local_symbol",
                "local_declaration_id",
                "exact_package_module_declaration"
            ],
            "reference_syntax": {
                "request_local_symbol": "$NAME",
                "local_declaration_id": "decl_HEX",
                "exact_package_module_declaration": "exact:PACKAGE_HEX/mod_HEX/decl_HEX"
            }
        }),
        RegistrySection::Type => json!({"forms": TypeFormKind::ALL.map(TypeFormKind::name)}),
        RegistrySection::Expression => {
            json!({"forms": ExpressionFormKind::ALL.map(ExpressionFormKind::name)})
        }
        RegistrySection::Owners => json!({"kinds": OwnerKind::ALL.map(OwnerKind::name)}),
        RegistrySection::Relations => json!({"roles": RelationRole::ALL.map(RelationRole::name)}),
        RegistrySection::Limits => json!(limit_descriptors()),
        RegistrySection::Diagnostics => json!({
            "codes": diagnostic_descriptors(),
            "exit_statuses": exit_status_descriptors()
        }),
        RegistrySection::Templates => json!(template_descriptors()),
        RegistrySection::Runners => json!({"kinds": RunnerKind::ALL.map(RunnerKind::name)}),
        RegistrySection::Security => json!({"nonclaims": nonclaims()}),
    }
}

fn validate_registry() -> Result<(), String> {
    unique(
        contract_descriptors().iter().map(|value| value.identity),
        "contract identity",
    )?;
    unique(
        contract_descriptors()
            .iter()
            .flat_map(|value| value.magic_values.iter().copied()),
        "magic value",
    )?;
    unique(
        contract_descriptors()
            .iter()
            .flat_map(|value| value.digest_domains.iter().copied()),
        "digest domain",
    )?;
    unique(
        operation_descriptors()
            .iter()
            .map(|value| value.operation.name()),
        "operation",
    )?;
    unique(
        diagnostic_descriptors().iter().map(|value| value.code),
        "diagnostic code",
    )?;
    unique(limit_descriptors().iter().map(|value| value.name), "limit")?;
    unique(
        template_descriptors().iter().map(|value| value.name),
        "template",
    )?;
    if operation_descriptors().len() != PublicOperation::ALL.len() {
        return Err("every public operation must have exactly one descriptor".to_owned());
    }
    if template_descriptors().len() != ProjectTemplate::ALL.len() {
        return Err("every project template must have exactly one descriptor".to_owned());
    }
    Ok(())
}

fn unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.is_empty() || !seen.insert(value) {
            return Err(format!("{label} '{value}' is empty or duplicated"));
        }
    }
    Ok(())
}

fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("registry JSON encoding failed: {error}"))
}

fn section_digest(section: RegistrySection, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(REGISTRY_SECTION_DIGEST_DOMAIN);
    hasher.update(&(section.name().len() as u64).to_be_bytes());
    hasher.update(section.name().as_bytes());
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

fn digest(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_unique_complete_and_deterministic() {
        let first = registry_snapshot("schema").expect("valid registry");
        let second = registry_snapshot("schema").expect("valid registry");
        assert_eq!(first.digest, second.digest);
        assert_eq!(first.manifest.sections, second.manifest.sections);
        assert_eq!(operation_descriptors().len(), PublicOperation::ALL.len());
        assert_eq!(RegistrySection::ALL.len(), first.section_values.len());
    }

    #[test]
    fn owner_parser_accepts_only_registry_names() {
        for kind in OwnerKind::ALL {
            assert_eq!(
                OwnerKind::parse(kind.name()).expect("registered kind"),
                kind
            );
        }
        assert!(OwnerKind::parse("function").is_err());
        assert!(OwnerKind::parse("task").is_err());
    }
}

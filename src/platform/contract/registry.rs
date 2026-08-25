use super::super::artifact::{ARTIFACT_CONTRACT_VERSION, PACKAGE_ARTIFACT_CONTRACT_VERSION};
use super::super::bootstrap::BOOTSTRAP_CONTRACT_VERSION;
use super::super::configuration::CONFIGURATION_ADAPTER_CONTRACT_VERSION;
use super::super::control::{
    CHANGE_PLAN_DIGEST_DOMAIN, COMPACT_CHANGE_CONTRACT_IDENTITY, COMPACT_CHANGE_OPERATIONS,
    COMPACT_EXPRESSION_FORMS, COMPACT_TYPE_FORMS, MAXIMUM_COMPACT_INPUT_BYTES, render_record,
};
use super::super::database::POSTGRES_ADAPTER_CONTRACT_VERSION;
use super::super::deployment::DEPLOYMENT_CONTRACT_VERSION;
use super::super::diagnostic::DiagnosticClass;
use super::super::execution::CAPABILITY_GRANT_CONTRACT_VERSION;
use super::super::graph::{ROOT_STORAGE_CONTRACT_IDENTITY, ROOT_STORAGE_CONTRACT_VERSION};
use super::super::http::HTTP_ADAPTER_CONTRACT_VERSION;
use super::super::json::JSON_CONTRACT_VERSION;
use super::super::kernel::OwnerKind;
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
use super::super::semantic_diff::SEMANTIC_DIFF_CONTRACT_VERSION;
use super::super::semantic_draft::DRAFT_CONTRACT_VERSION;
use super::super::semantic_fact::{
    SEMANTIC_FACT_CONTRACT_IDENTITY, SEMANTIC_FACT_CONTRACT_VERSION,
};
use super::super::semantic_merge::SEMANTIC_MERGE_CONTRACT_VERSION;
use super::super::semantic_projection::REVIEW_PROJECTION_CONTRACT_VERSION;
use super::super::semantic_query::{
    MAXIMUM_BYTE_LIMIT, MAXIMUM_ITEM_LIMIT, MAXIMUM_QUERY_DEPTH, MAXIMUM_QUERY_FANOUT,
    MAXIMUM_WORK_LIMIT, QUERY_CONTRACT_VERSION, QUERY_INDEX_CONTRACT_VERSION,
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
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const REGISTRY_CONTRACT_IDENTITY: &str = "lkjscript-contract-registry-3";
pub const REGISTRY_CONTRACT_VERSION: u16 = 3;
pub const CLI_CONTRACT_VERSION: u16 = 5;
pub const MAXIMUM_CLI_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_CLI_RESPONSE_RECORDS: usize = 10_000;
pub const MAXIMUM_TRANSACTION_REQUEST_BYTES: usize = 16 * 1_048_576;

const REGISTRY_DIGEST_DOMAIN: &str = "lkjscript.contract-registry.v3";
const REGISTRY_SECTION_DIGEST_DOMAIN: &str = "lkjscript.contract-registry-section.v3";

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
const STORED_ROOT_MAGIC_TEXT: &str = "LKJROOT4";
pub(crate) const STORED_ROOT_MAGIC: [u8; 8] = magic_bytes(STORED_ROOT_MAGIC_TEXT);
pub(crate) const STORED_ROOT_DIGEST_DOMAIN: &str = "lkjscript.persistent-root-object.v3";
const MODULE_MAGIC_TEXT: &str = "LKJMOD04";
pub(crate) const MODULE_MAGIC: [u8; 8] = magic_bytes(MODULE_MAGIC_TEXT);
pub(crate) const MODULE_DIGEST_DOMAIN: &str = "lkjscript.semantic-module-object.v4";
const MAP_PAGE_MAGIC_TEXT: &str = "LKJPMAP2";
pub(crate) const MAP_PAGE_MAGIC: [u8; 8] = magic_bytes(MAP_PAGE_MAGIC_TEXT);
pub(crate) const MAP_PAGE_CONTRACT_VERSION: u16 = 2;
pub(crate) const MAP_PAGE_DIGEST_DOMAIN: &str = "lkjscript.persistent-map-page.v2";
pub(crate) const MAP_PAGE_CHECKSUM_DOMAIN: &str = "lkjscript.persistent-map-checksum.v2";
pub(crate) const MAP_CONTENT_DIGEST_DOMAIN: &str = "lkjscript.persistent-map-content.v1";
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
pub(crate) const CHANGE_ALLOCATION_SEED_DOMAIN: &str = "lkjscript.change-allocation-seed.v2";

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

impl ContractKey {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::Cli => "cli",
            Self::MeaningGraph => "meaning_graph",
            Self::PersistentRoot => "persistent_root",
            Self::Revision => "revision",
            Self::Receipt => "receipt",
            Self::SemanticSummary => "semantic_summary",
            Self::SemanticFacts => "semantic_facts",
            Self::SemanticValidator => "semantic_validator",
            Self::Change => "change",
            Self::Transaction => "transaction",
            Self::Query => "query",
            Self::QueryIndex => "query_index",
            Self::LocalQueryIndex => "local_query_index",
            Self::Draft => "draft",
            Self::Diff => "diff",
            Self::Merge => "merge",
            Self::ReviewProjection => "review_projection",
            Self::Artifact => "artifact",
            Self::PackageArtifact => "package_artifact",
            Self::Backup => "backup",
            Self::Bootstrap => "bootstrap",
            Self::Retention => "retention",
            Self::PackageDescriptor => "package_descriptor",
            Self::Deployment => "deployment",
            Self::ConfigurationAdapter => "configuration_adapter",
            Self::PostgresAdapter => "postgres_adapter",
            Self::CapabilityGrant => "capability_grant",
            Self::HttpAdapter => "http_adapter",
            Self::Json => "json",
            Self::ObjectAdapter => "object_adapter",
            Self::QueueAdapter => "queue_adapter",
            Self::ResidentRuntime => "resident_runtime",
            Self::SecretCatalog => "secret_catalog",
            Self::SecretVerifier => "secret_verifier",
            Self::SecurityAdapter => "security_adapter",
            Self::Stream => "stream",
            Self::WorkerRunner => "worker_runner",
            Self::Workspace => "workspace",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStability {
    Current,
}

impl ContractStability {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Current => "current",
        }
    }
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

impl ContractAuthority {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CanonicalMeaning => "canonical_meaning",
            Self::AcceptedHistory => "accepted_history",
            Self::RequiredWitness => "required_witness",
            Self::DerivedDisposable => "derived_disposable",
            Self::PublicProtocol => "public_protocol",
            Self::Operational => "operational",
            Self::Deployment => "deployment",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PredecessorPolicy {
    Reject,
    NotApplicable,
}

impl PredecessorPolicy {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::NotApplicable => "not_applicable",
        }
    }
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
            digest_domains: &[REGISTRY_DIGEST_DOMAIN, REGISTRY_SECTION_DIGEST_DOMAIN],
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
                MAP_CONTENT_DIGEST_DOMAIN,
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
            identity: COMPACT_CHANGE_CONTRACT_IDENTITY,
            version: 1,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[CHANGE_ALLOCATION_SEED_DOMAIN, CHANGE_PLAN_DIGEST_DOMAIN],
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicOperation {
    Capabilities,
    New,
    Status,
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
    pub const ALL: [Self; 18] = [
        Self::Capabilities,
        Self::New,
        Self::Status,
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
            Self::Status => "status",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

impl AuthorityEffect {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Accepted => "accepted",
            Self::AcceptedOnCommit => "accepted_on_commit",
            Self::DraftOrAccepted => "draft_or_accepted",
            Self::Operational => "operational",
            Self::ExternalOutput => "external_output",
            Self::ExternalRuntime => "external_runtime",
            Self::OperationalIndexesOnly => "operational_indexes_only",
            Self::OptionalExternalOutput => "optional_external_output",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRequirement {
    None,
    Destination,
    Required,
    RequiredByAction,
    RequiredForStage,
    DescriptorBound,
}

impl ProjectRequirement {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Destination => "destination",
            Self::Required => "required",
            Self::RequiredByAction => "required_by_action",
            Self::RequiredForStage => "required_for_stage",
            Self::DescriptorBound => "descriptor_bound",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetProfile {
    Discovery,
    BoundedRead,
    SemanticChange,
    Build,
    Runtime,
    Maintenance,
}

impl BudgetProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::BoundedRead => "bounded_read",
            Self::SemanticChange => "semantic_change",
            Self::Build => "build",
            Self::Runtime => "runtime",
            Self::Maintenance => "maintenance",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlModel {
    CapabilitiesRequest,
    CapabilitiesResult,
    NewRequest,
    NewResult,
    StatusRequest,
    StatusResult,
    InspectRequest,
    InspectResult,
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

impl ControlModel {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CapabilitiesRequest => "capabilities_request",
            Self::CapabilitiesResult => "capabilities_result",
            Self::NewRequest => "new_request",
            Self::NewResult => "new_result",
            Self::StatusRequest => "status_request",
            Self::StatusResult => "status_result",
            Self::InspectRequest => "inspect_request",
            Self::InspectResult => "inspect_result",
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
    pub request_model: ControlModel,
    pub response_model: ControlModel,
    pub authority_effect: AuthorityEffect,
    pub project_requirement: ProjectRequirement,
    pub default_budget: BudgetProfile,
}

pub fn operation_descriptors() -> &'static [OperationDescriptor] {
    const OPERATIONS: &[OperationDescriptor] = &[
        capabilities_operation(
            "Discover exact executable contracts and changed registry sections.",
            "capabilities [COMMAND] [--known-registry DIGEST] [--section SECTION] [--known-section SECTION=DIGEST] [--output PATH] [--generate-docs DIR] [--verify-generated DIR]",
        ),
        new_operation(
            "Create fresh normalized semantic authority atomically in an absent or empty safe destination.",
            "new DEST [--template minimal] [--name NAME]",
        ),
        status_operation(
            "Report the exact current semantic authority and its durable acceptance evidence.",
            "status",
        ),
        inspect_operation(
            "Inspect a compact summary of one exact owner at the observed accepted revision.",
            "inspect owner KIND ID [--package PACKAGE]",
        ),
        operation(
            PublicOperation::Query,
            "Select bounded owners, relations, context, and impact.",
            "query owners|find|relations|callers|callees|types|capabilities|context|impact|request ...",
            ControlModel::QueryRequest,
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Change,
            "Plan or atomically apply one compact typed semantic change.",
            "change plan (--input RECORDS | --input-file PATH) | change apply (--input RECORDS | --input-file PATH) --plan DIGEST",
            ControlModel::ChangeRequest,
            AuthorityEffect::AcceptedOnCommit,
            ProjectRequirement::Required,
            BudgetProfile::SemanticChange,
        ),
        operation(
            PublicOperation::Draft,
            "Create, inspect, append, rebase, publish, or drop non-executable work.",
            "draft create|status|append|rebase|publish|drop ...",
            ControlModel::DraftRequest,
            AuthorityEffect::DraftOrAccepted,
            ProjectRequirement::Required,
            BudgetProfile::SemanticChange,
        ),
        operation(
            PublicOperation::History,
            "List, inspect, diff, or merge exact revisions.",
            "history list|show|diff|merge ...",
            ControlModel::HistoryRequest,
            AuthorityEffect::AcceptedOnCommit,
            ProjectRequirement::Required,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Package,
            "Stage an exact dependency or inspect/export a built-in package.",
            "package stage PATH | package builtin inspect|export ...",
            ControlModel::PackageRequest,
            AuthorityEffect::Operational,
            ProjectRequirement::RequiredForStage,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Check,
            "Run graph-owned tests through production and independent execution.",
            "check",
            ControlModel::CheckRequest,
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::Runtime,
        ),
        operation(
            PublicOperation::Build,
            "Build a deterministic graph-native artifact.",
            "build [--output PATH]",
            ControlModel::BuildRequest,
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::Build,
        ),
        operation(
            PublicOperation::Run,
            "Run a pure command, batch, or test target through both execution tiers.",
            "run TARGET [--arguments JSON]",
            ControlModel::RunRequest,
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::Runtime,
        ),
        runtime_operation(
            PublicOperation::Serve,
            "Run one plaintext HTTP deployment with bounded shutdown.",
            "serve --deployment DESCRIPTOR",
            ControlModel::ServeRequest,
        ),
        runtime_operation(
            PublicOperation::Worker,
            "Run one bounded worker deployment.",
            "worker --deployment DESCRIPTOR",
            ControlModel::WorkerRequest,
        ),
        operation(
            PublicOperation::Review,
            "Write a deterministic non-authoritative review projection.",
            "review [--revision REV] [--output PATH]",
            ControlModel::ReviewRequest,
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::BoundedRead,
        ),
        operation(
            PublicOperation::Backup,
            "Capture one exact reachable canonical authority bundle.",
            "backup [--output PATH]",
            ControlModel::BackupRequest,
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Restore,
            "Verify and atomically restore a canonical authority bundle.",
            "restore --backup PATH (--output PROJECT | --project PROJECT)",
            ControlModel::RestoreRequest,
            AuthorityEffect::Accepted,
            ProjectRequirement::Destination,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Doctor,
            "Validate authority or preview exact retained/reclaimable storage.",
            "doctor [--deep] | doctor cleanup",
            ControlModel::DoctorRequest,
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
    request_model: ControlModel,
    authority_effect: AuthorityEffect,
    project_requirement: ProjectRequirement,
    default_budget: BudgetProfile,
) -> OperationDescriptor {
    OperationDescriptor {
        operation,
        purpose,
        usage,
        request_model,
        response_model: ControlModel::CliSuccess,
        authority_effect,
        project_requirement,
        default_budget,
    }
}

const fn capabilities_operation(purpose: &'static str, usage: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        operation: PublicOperation::Capabilities,
        purpose,
        usage,
        request_model: ControlModel::CapabilitiesRequest,
        response_model: ControlModel::CapabilitiesResult,
        authority_effect: AuthorityEffect::OptionalExternalOutput,
        project_requirement: ProjectRequirement::None,
        default_budget: BudgetProfile::Discovery,
    }
}

const fn new_operation(purpose: &'static str, usage: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        operation: PublicOperation::New,
        purpose,
        usage,
        request_model: ControlModel::NewRequest,
        response_model: ControlModel::NewResult,
        authority_effect: AuthorityEffect::Accepted,
        project_requirement: ProjectRequirement::Destination,
        default_budget: BudgetProfile::SemanticChange,
    }
}

const fn status_operation(purpose: &'static str, usage: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        operation: PublicOperation::Status,
        purpose,
        usage,
        request_model: ControlModel::StatusRequest,
        response_model: ControlModel::StatusResult,
        authority_effect: AuthorityEffect::None,
        project_requirement: ProjectRequirement::Required,
        default_budget: BudgetProfile::BoundedRead,
    }
}

const fn inspect_operation(purpose: &'static str, usage: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        operation: PublicOperation::Inspect,
        purpose,
        usage,
        request_model: ControlModel::InspectRequest,
        response_model: ControlModel::InspectResult,
        authority_effect: AuthorityEffect::None,
        project_requirement: ProjectRequirement::Required,
        default_budget: BudgetProfile::BoundedRead,
    }
}

const fn runtime_operation(
    operation: PublicOperation,
    purpose: &'static str,
    usage: &'static str,
    request_model: ControlModel,
) -> OperationDescriptor {
    OperationDescriptor {
        operation,
        purpose,
        usage,
        request_model,
        response_model: ControlModel::RuntimeEvent,
        authority_effect: AuthorityEffect::ExternalRuntime,
        project_requirement: ProjectRequirement::DescriptorBound,
        default_budget: BudgetProfile::Runtime,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitClass {
    HostileDecoderSafety,
    DeterministicOperationBudget,
    ExplicitRequestBudget,
    DefaultPagination,
    ImplementationLimitation,
    DeploymentResourcePolicy,
}

impl LimitClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::HostileDecoderSafety => "hostile_decoder_safety",
            Self::DeterministicOperationBudget => "deterministic_operation_budget",
            Self::ExplicitRequestBudget => "explicit_request_budget",
            Self::DefaultPagination => "default_pagination",
            Self::ImplementationLimitation => "implementation_limitation",
            Self::DeploymentResourcePolicy => "deployment_resource_policy",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitUnit {
    Bytes,
    Records,
    Items,
    Work,
    Depth,
}

impl LimitUnit {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::Records => "records",
            Self::Items => "items",
            Self::Work => "work",
            Self::Depth => "depth",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverridePolicy {
    Fixed,
    RequestUpToMaximum,
    DeploymentUpToMaximum,
}

impl OverridePolicy {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::RequestUpToMaximum => "request_up_to_maximum",
            Self::DeploymentUpToMaximum => "deployment_up_to_maximum",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
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
            LimitClass::DeterministicOperationBudget,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "cli_response_records",
            MAXIMUM_CLI_RESPONSE_RECORDS,
            LimitClass::DeterministicOperationBudget,
            LimitUnit::Records,
            OverridePolicy::Fixed,
        ),
        limit(
            "change_request_bytes",
            MAXIMUM_COMPACT_INPUT_BYTES,
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

const fn diagnostic(
    code: &'static str,
    class: DiagnosticClass,
    meaning: &'static str,
    retry: &'static str,
) -> DiagnosticDescriptor {
    DiagnosticDescriptor {
        code,
        class,
        meaning,
        retry,
    }
}

pub fn diagnostic_descriptors() -> &'static [DiagnosticDescriptor] {
    const DIAGNOSTICS: &[DiagnosticDescriptor] = &[
        DiagnosticDescriptor {
            code: "cli_usage",
            class: DiagnosticClass::Source,
            meaning: "The command, option, or argument grammar is invalid.",
            retry: "Correct the request using capabilities output.",
        },
        diagnostic(
            "control_input_bytes",
            DiagnosticClass::Resource,
            "Compact input exceeds its deterministic byte bound.",
            "Reduce the request or move large values to an advertised external input.",
        ),
        diagnostic(
            "control_utf8",
            DiagnosticClass::Source,
            "Compact input is not valid UTF-8.",
            "Encode the request as valid UTF-8 records.",
        ),
        diagnostic(
            "control_record_count",
            DiagnosticClass::Source,
            "Compact input exceeds its record-count format bound.",
            "Split the semantic work into smaller exact-base requests.",
        ),
        diagnostic(
            "control_record_bytes",
            DiagnosticClass::Source,
            "One compact physical record exceeds its byte bound.",
            "Use flat fragments or an advertised external value input.",
        ),
        diagnostic(
            "control_operation",
            DiagnosticClass::Source,
            "A compact record has a malformed or unknown operation token.",
            "Select an operation reported by the focused capabilities section.",
        ),
        diagnostic(
            "control_field_separator",
            DiagnosticClass::Source,
            "Compact fields are not separated by ASCII whitespace.",
            "Separate each field assignment with ASCII whitespace.",
        ),
        diagnostic(
            "control_field_count",
            DiagnosticClass::Source,
            "One compact record exceeds its field-count format bound.",
            "Use additional flat records for repeated or nested facts.",
        ),
        diagnostic(
            "control_field_name",
            DiagnosticClass::Source,
            "A compact field name is malformed.",
            "Use the closed lowercase field name reported by capabilities.",
        ),
        diagnostic(
            "control_duplicate_field",
            DiagnosticClass::Source,
            "One compact record repeats a field name.",
            "Supply each closed field exactly once.",
        ),
        diagnostic(
            "control_field_equals",
            DiagnosticClass::Source,
            "A compact field is missing its immediate equals delimiter.",
            "Write each assignment as field=value.",
        ),
        diagnostic(
            "control_value_missing",
            DiagnosticClass::Source,
            "A compact field has no value.",
            "Supply a bare or quoted explicit value.",
        ),
        diagnostic(
            "control_bare_value",
            DiagnosticClass::Source,
            "A bare compact value contains a byte that requires quoting.",
            "Quote the value using the advertised deterministic escaping rule.",
        ),
        diagnostic(
            "control_quote_unclosed",
            DiagnosticClass::Source,
            "A quoted compact value is not closed on its physical record.",
            "Close the quote on the same record.",
        ),
        diagnostic(
            "control_escape_truncated",
            DiagnosticClass::Source,
            "A quoted compact escape ends before its payload.",
            "Complete the escape sequence.",
        ),
        diagnostic(
            "control_escape_unknown",
            DiagnosticClass::Source,
            "A quoted compact value uses an unknown escape.",
            "Use one escape reported by the compact record grammar.",
        ),
        diagnostic(
            "control_unicode_escape",
            DiagnosticClass::Source,
            "A Unicode compact escape is malformed.",
            "Use a canonical hexadecimal Unicode scalar escape.",
        ),
        diagnostic(
            "control_unicode_scalar",
            DiagnosticClass::Source,
            "A Unicode compact escape does not identify a scalar value.",
            "Use a valid Unicode scalar value.",
        ),
        diagnostic(
            "control_quoted_control",
            DiagnosticClass::Source,
            "A quoted compact value contains an unescaped control character.",
            "Escape control characters using the compact grammar.",
        ),
        diagnostic(
            "control_value_bytes",
            DiagnosticClass::Source,
            "One decoded compact value exceeds its byte bound.",
            "Use an advertised external value input or reduce the value.",
        ),
        diagnostic(
            "change_request_missing",
            DiagnosticClass::Source,
            "A compact change has no request record.",
            "Add exactly one request record with an exact base revision.",
        ),
        diagnostic(
            "change_request_duplicate",
            DiagnosticClass::Source,
            "A compact change has more than one request record.",
            "Retain one request record and one exact base.",
        ),
        diagnostic(
            "change_operations_missing",
            DiagnosticClass::Source,
            "A compact change has no semantic operation.",
            "Add at least one operation reported by capabilities --section change.",
        ),
        diagnostic(
            "change_operation_unknown",
            DiagnosticClass::Source,
            "A compact change record is not registered.",
            "Select a record reported by capabilities --section change.",
        ),
        diagnostic(
            "change_field_unknown",
            DiagnosticClass::Source,
            "A compact semantic record contains an unknown field.",
            "Remove the field or use the focused operation grammar.",
        ),
        diagnostic(
            "change_field_missing",
            DiagnosticClass::Source,
            "A compact semantic record omits a required field.",
            "Supply the exact required field reported by focused discovery.",
        ),
        diagnostic(
            "change_field_value",
            DiagnosticClass::Source,
            "A compact semantic field has an invalid typed value.",
            "Correct the value using the field diagnostic and focused grammar.",
        ),
        diagnostic(
            "change_local_label",
            DiagnosticClass::Source,
            "A request-local symbol or fragment label is malformed.",
            "Use a portable $ semantic label or @ type label.",
        ),
        diagnostic(
            "change_idempotency",
            DiagnosticClass::Source,
            "An idempotency key violates its portable byte contract.",
            "Use 1 through 128 portable identifier bytes.",
        ),
        diagnostic(
            "change_intent_bytes",
            DiagnosticClass::Source,
            "Nonsemantic intent exceeds its byte bound.",
            "Shorten or omit the intent field.",
        ),
        diagnostic(
            "change_boolean",
            DiagnosticClass::Source,
            "A compact boolean is not exactly true or false.",
            "Use true or false.",
        ),
        diagnostic(
            "change_visibility",
            DiagnosticClass::Source,
            "A declaration visibility value is unknown.",
            "Use private, package, or public.",
        ),
        diagnostic(
            "change_declaration_selector",
            DiagnosticClass::Source,
            "A declaration selector has no supported exact form.",
            "Use $symbol, decl_ID, or MODULE/NAME.",
        ),
        diagnostic(
            "change_local_reference",
            DiagnosticClass::Source,
            "A local-value reference has a foreign or malformed identity domain.",
            "Use a compatible request symbol, parameter ID, or binding ID.",
        ),
        diagnostic(
            "change_edge_index_duplicate",
            DiagnosticClass::Source,
            "A flat fragment repeats one child index.",
            "Use each zero-based child index once.",
        ),
        diagnostic(
            "change_edge_index_order",
            DiagnosticClass::Source,
            "A flat fragment's child indexes are not contiguous from zero.",
            "Provide the missing index or renumber the fragment edges.",
        ),
        diagnostic(
            "change_edge_parent",
            DiagnosticClass::Source,
            "A flat child edge names no matching parent fragment.",
            "Define the exact expression or type parent label.",
        ),
        diagnostic(
            "change_type_duplicate",
            DiagnosticClass::Source,
            "A compact type label is defined more than once.",
            "Define each @ type label once.",
        ),
        diagnostic(
            "change_type_reference",
            DiagnosticClass::Source,
            "A compact type reference is neither primitive nor an @ label.",
            "Use an advertised primitive or defined @ type label.",
        ),
        diagnostic(
            "change_type_undefined",
            DiagnosticClass::Source,
            "A compact type label is referenced but not defined.",
            "Define the referenced @ type fragment.",
        ),
        diagnostic(
            "change_type_cycle",
            DiagnosticClass::Semantic,
            "Compact type fragments contain an ill-founded cycle.",
            "Use an exact named declaration reference at recursive boundaries.",
        ),
        diagnostic(
            "change_type_form_unknown",
            DiagnosticClass::Source,
            "A compact type fragment uses an unknown form.",
            "Select a form reported by capabilities --section type.",
        ),
        diagnostic(
            "change_expression_duplicate",
            DiagnosticClass::Source,
            "A compact expression label is defined more than once.",
            "Define each $ expression label once.",
        ),
        diagnostic(
            "change_expression_reference",
            DiagnosticClass::Source,
            "An expression reference is not a request-local $ label.",
            "Use a defined $ expression label.",
        ),
        diagnostic(
            "change_expression_undefined",
            DiagnosticClass::Source,
            "An expression label is referenced but not defined.",
            "Define the referenced $ expression fragment.",
        ),
        diagnostic(
            "change_expression_cycle",
            DiagnosticClass::Semantic,
            "Compact expression fragments contain an ownership cycle.",
            "Make expression fragments one acyclic owned tree.",
        ),
        diagnostic(
            "change_expression_unused",
            DiagnosticClass::Source,
            "A compact expression fragment is unreachable from every operation.",
            "Remove the fragment or attach it to one semantic operation.",
        ),
        diagnostic(
            "change_expression_shared",
            DiagnosticClass::Source,
            "A compact expression fragment has more than one owner.",
            "Define a distinct fragment for each owned tree position.",
        ),
        diagnostic(
            "change_expression_form_unknown",
            DiagnosticClass::Source,
            "A compact expression fragment uses an unknown form.",
            "Select a form reported by capabilities --section expression.",
        ),
        diagnostic(
            "change_effect_unsupported",
            DiagnosticClass::Source,
            "The compact operation does not expose the requested function effect.",
            "Use the advertised effect or wait for the complete typed form cutover.",
        ),
        diagnostic(
            "change_plan_domain",
            DiagnosticClass::Source,
            "A reviewed plan has the wrong typed digest prefix.",
            "Use the exact plan_ digest returned by change plan.",
        ),
        diagnostic(
            "change_plan_length",
            DiagnosticClass::Source,
            "A reviewed plan has the wrong digest length.",
            "Use the complete plan_ digest returned by change plan.",
        ),
        diagnostic(
            "change_plan_field_length",
            DiagnosticClass::Resource,
            "A normalized plan field exceeds its digest length domain.",
            "Reduce the request within the advertised compact input bounds.",
        ),
        diagnostic(
            "change_plan_hex",
            DiagnosticClass::Source,
            "A reviewed plan has noncanonical hexadecimal bytes.",
            "Use the lowercase plan_ digest returned by change plan.",
        ),
        diagnostic(
            "change_plan_mismatch",
            DiagnosticClass::Semantic,
            "Reviewed plan identity differs from the normalized input.",
            "Re-run change plan for the exact input and review the new result.",
        ),
        diagnostic(
            "change_authored_stale_base",
            DiagnosticClass::Semantic,
            "The request base is not the currently observed accepted revision.",
            "Refresh status and rebuild the request against the observed revision.",
        ),
        diagnostic(
            "change_stale_base",
            DiagnosticClass::Semantic,
            "HEAD changed after preparation and before publication.",
            "Refresh status, re-plan the request, and review its new plan digest.",
        ),
        diagnostic(
            "change_expression_inventory",
            DiagnosticClass::Infrastructure,
            "Compact expression inventory disagrees with decoded definitions.",
            "Use a verified executable and retain the failing request.",
        ),
        diagnostic(
            "change_prepared_base",
            DiagnosticClass::Corrupt,
            "Prepared publication does not bind one exact accepted base.",
            "Preserve the repository and run deep verification.",
        ),
        DiagnosticDescriptor {
            code: "control_response_byte_budget",
            class: DiagnosticClass::Resource,
            meaning: "A finite compact response exceeds its deterministic byte budget.",
            retry: "Request a focused section or export the complete material to a file.",
        },
        DiagnosticDescriptor {
            code: "control_response_record_budget",
            class: DiagnosticClass::Resource,
            meaning: "A finite compact response exceeds its deterministic record budget.",
            retry: "Request a focused section or export the complete material to a file.",
        },
        DiagnosticDescriptor {
            code: "control_response_allocation",
            class: DiagnosticClass::Resource,
            meaning: "The bounded compact response buffer could not be allocated.",
            retry: "Request less output or retry when host memory is available.",
        },
        DiagnosticDescriptor {
            code: "control_response_limits",
            class: DiagnosticClass::Infrastructure,
            meaning: "Configured compact response limits are zero or not addressable.",
            retry: "Use a verified executable build with a valid response budget.",
        },
        DiagnosticDescriptor {
            code: "control_response_records_newline",
            class: DiagnosticClass::Infrastructure,
            meaning: "Executable registry records are not newline-complete.",
            retry: "Use a verified executable build whose registry passes conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_response_records_blank",
            class: DiagnosticClass::Infrastructure,
            meaning: "Executable registry material contains a blank physical record.",
            retry: "Use a verified executable build whose registry passes conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_response_records_invalid",
            class: DiagnosticClass::Infrastructure,
            meaning: "Executable registry material contains an invalid compact record.",
            retry: "Use a verified executable build whose registry passes conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_render_operation",
            class: DiagnosticClass::Infrastructure,
            meaning: "A compact response producer supplied an invalid operation name.",
            retry: "Use a verified executable build whose registry passes conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_render_fields",
            class: DiagnosticClass::Resource,
            meaning: "One compact response record exceeds the format field-count bound.",
            retry: "Request a focused result or export the complete material to a file.",
        },
        DiagnosticDescriptor {
            code: "control_render_field",
            class: DiagnosticClass::Infrastructure,
            meaning: "A compact response producer supplied an invalid field name.",
            retry: "Use a verified executable build whose registry passes conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_render_duplicate_field",
            class: DiagnosticClass::Infrastructure,
            meaning: "A compact response producer supplied a duplicate field name.",
            retry: "Use a verified executable build whose registry passes conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_render_value_bytes",
            class: DiagnosticClass::Resource,
            meaning: "One compact response field exceeds the format byte bound.",
            retry: "Request a focused result or export the complete value to a file.",
        },
        DiagnosticDescriptor {
            code: "control_render_record_bytes",
            class: DiagnosticClass::Resource,
            meaning: "One compact response record exceeds the format byte bound.",
            retry: "Request a focused result or export the complete material to a file.",
        },
        DiagnosticDescriptor {
            code: "control_render_allocation",
            class: DiagnosticClass::Resource,
            meaning: "The bounded compact record buffer could not be allocated.",
            retry: "Request less output or retry when host memory is available.",
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
        DiagnosticDescriptor {
            code: "owner_selector_kind",
            class: DiagnosticClass::Source,
            meaning: "An exact owner selector names an unknown or nonpublic semantic owner kind.",
            retry: "Select one owner kind reported by capabilities --section owners.",
        },
        DiagnosticDescriptor {
            code: "owner_selector_identity",
            class: DiagnosticClass::Source,
            meaning: "An exact owner selector contains a malformed or unknown identity domain.",
            retry: "Use an exact identity returned by a current project query or change receipt.",
        },
        DiagnosticDescriptor {
            code: "owner_foreign_package",
            class: DiagnosticClass::Semantic,
            meaning: "An exact owner selector belongs to another package authority.",
            retry: "Open the selected package or use the current project's package identity.",
        },
        DiagnosticDescriptor {
            code: "owner_wrong_kind",
            class: DiagnosticClass::Semantic,
            meaning: "An exact owner identity does not have the requested semantic kind.",
            retry: "Refresh the exact owner kind and retry without changing the identity.",
        },
        DiagnosticDescriptor {
            code: "owner_not_found",
            class: DiagnosticClass::Semantic,
            meaning: "The exact owner identity is not live at the observed accepted revision.",
            retry: "Refresh the owner identity at the reported revision.",
        },
        DiagnosticDescriptor {
            code: "publication_summary_binding",
            class: DiagnosticClass::Corrupt,
            meaning: "Accepted owner authority and its validation-summary binding disagree.",
            retry: "Preserve the repository and run deep authority verification.",
        },
        DiagnosticDescriptor {
            code: "project_not_found",
            class: DiagnosticClass::Source,
            meaning: "No current semantic repository exists at or above the selected directory.",
            retry: "Select a current project directory or create one with new.",
        },
        DiagnosticDescriptor {
            code: "project_path",
            class: DiagnosticClass::Source,
            meaning: "The project discovery path is missing, non-directory, traversing, or symbolic.",
            retry: "Select an existing ordinary directory without symbolic-link or parent traversal components.",
        },
        DiagnosticDescriptor {
            code: "project_marker",
            class: DiagnosticClass::Corrupt,
            meaning: "A normalized repository marker is not an ordinary file.",
            retry: "Preserve the authority and inspect the reported repository path.",
        },
        DiagnosticDescriptor {
            code: "project_io",
            class: DiagnosticClass::Infrastructure,
            meaning: "Project discovery could not inspect the current directory, path, or marker.",
            retry: "Correct host path availability or permissions and retry.",
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
    const TEMPLATES: &[TemplateDescriptor] = &[TemplateDescriptor {
        name: "minimal",
        purpose: "Create the smallest normalized accepted project authority.",
        creates_command_target: false,
    }];
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrySectionSnapshot {
    pub section: RegistrySection,
    pub digest: String,
    pub records: usize,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrySnapshot {
    pub contract: &'static str,
    pub version: u16,
    pub graph_contract: &'static str,
    pub cli_contract_version: u16,
    pub digest: String,
    pub bytes: Vec<u8>,
    pub sections: BTreeMap<RegistrySection, RegistrySectionSnapshot>,
}

impl RegistrySnapshot {
    pub fn section(&self, section: RegistrySection) -> Option<&RegistrySectionSnapshot> {
        self.sections.get(&section)
    }
}

pub fn registry_snapshot() -> Result<RegistrySnapshot, String> {
    validate_registry()?;
    let mut sections = BTreeMap::new();
    for section in RegistrySection::ALL {
        let records = section_records(section)?;
        let bytes = records.concat().into_bytes();
        let snapshot = RegistrySectionSnapshot {
            section,
            digest: section_digest(section, &bytes),
            records: records.len(),
            bytes,
        };
        sections.insert(section, snapshot);
    }

    let mut bytes = compact_record(
        "registry",
        &[
            ("contract", REGISTRY_CONTRACT_IDENTITY.to_owned()),
            ("version", REGISTRY_CONTRACT_VERSION.to_string()),
            ("graph", GRAPH_CONTRACT_IDENTITY.to_owned()),
            ("cli", CLI_CONTRACT_VERSION.to_string()),
        ],
    )?
    .into_bytes();
    for section in RegistrySection::ALL {
        let snapshot = sections
            .get(&section)
            .ok_or_else(|| format!("registry section '{}' is missing", section.name()))?;
        bytes.extend_from_slice(
            compact_record(
                "section",
                &[
                    ("name", section.name().to_owned()),
                    ("digest", snapshot.digest.clone()),
                    ("records", snapshot.records.to_string()),
                    ("bytes", snapshot.bytes.len().to_string()),
                ],
            )?
            .as_bytes(),
        );
        bytes.extend_from_slice(&snapshot.bytes);
    }
    let digest = digest(REGISTRY_DIGEST_DOMAIN, &bytes);
    Ok(RegistrySnapshot {
        contract: REGISTRY_CONTRACT_IDENTITY,
        version: REGISTRY_CONTRACT_VERSION,
        graph_contract: GRAPH_CONTRACT_IDENTITY,
        cli_contract_version: CLI_CONTRACT_VERSION,
        digest,
        bytes,
        sections,
    })
}

fn section_records(section: RegistrySection) -> Result<Vec<String>, String> {
    let mut records = Vec::new();
    match section {
        RegistrySection::Contracts => {
            for descriptor in contract_descriptors() {
                records.push(compact_record(
                    "contract",
                    &[
                        ("key", descriptor.key.name().to_owned()),
                        ("name", descriptor.name.to_owned()),
                        ("identity", descriptor.identity.to_owned()),
                        ("version", descriptor.version.to_string()),
                        ("stability", descriptor.stability.name().to_owned()),
                        ("authority", descriptor.authority.name().to_owned()),
                        (
                            "predecessor",
                            descriptor.predecessor_policy.name().to_owned(),
                        ),
                        ("magic-count", descriptor.magic_values.len().to_string()),
                        ("digest-count", descriptor.digest_domains.len().to_string()),
                    ],
                )?);
                for magic in descriptor.magic_values {
                    records.push(compact_record(
                        "contract.magic",
                        &[
                            ("contract", descriptor.identity.to_owned()),
                            ("value", (*magic).to_owned()),
                        ],
                    )?);
                }
                for domain in descriptor.digest_domains {
                    records.push(compact_record(
                        "contract.digest",
                        &[
                            ("contract", descriptor.identity.to_owned()),
                            ("domain", (*domain).to_owned()),
                        ],
                    )?);
                }
            }
        }
        RegistrySection::Operations => {
            for descriptor in operation_descriptors() {
                records.push(operation_record(descriptor)?);
            }
        }
        RegistrySection::Change => {
            records.push(compact_record(
                "change",
                &[
                    ("contract", COMPACT_CHANGE_CONTRACT_IDENTITY.to_owned()),
                    (
                        "request-model",
                        ControlModel::ChangeRequest.name().to_owned(),
                    ),
                    ("request-record", "request".to_owned()),
                    ("plan-prefix", "plan_".to_owned()),
                ],
            )?);
            for operation in COMPACT_CHANGE_OPERATIONS {
                records.push(compact_record(
                    "change.operation",
                    &[("name", (*operation).to_owned())],
                )?);
            }
            for (name, parent, child) in [
                ("expression.argument", "expression", "expression"),
                ("type.argument", "type", "type"),
            ] {
                records.push(compact_record(
                    "change.edge",
                    &[
                        ("name", name.to_owned()),
                        ("parent", parent.to_owned()),
                        ("child", child.to_owned()),
                        ("order", "zero-based-contiguous-index".to_owned()),
                    ],
                )?);
            }
            for (name, syntax) in [
                ("request_local_symbol", "$NAME"),
                ("request_local_type", "@NAME"),
                ("exact_owner", "DOMAIN_HEX"),
                ("qualified_declaration", "MODULE/NAME"),
                ("exact_package_declaration", "pkg_HEX/decl_HEX"),
            ] {
                records.push(compact_record(
                    "change.reference",
                    &[("name", name.to_owned()), ("syntax", syntax.to_owned())],
                )?);
            }
        }
        RegistrySection::Type => {
            for form in COMPACT_TYPE_FORMS {
                records.push(compact_record(
                    "type.form",
                    &[("name", (*form).to_owned())],
                )?);
            }
        }
        RegistrySection::Expression => {
            for form in COMPACT_EXPRESSION_FORMS {
                records.push(compact_record(
                    "expression.form",
                    &[("name", (*form).to_owned())],
                )?);
            }
        }
        RegistrySection::Owners => {
            for kind in OwnerKind::PUBLIC_EXACT {
                records.push(compact_record(
                    "owner.kind",
                    &[("name", kind.name().to_owned())],
                )?);
            }
        }
        RegistrySection::Relations => {
            for role in RelationRole::ALL {
                records.push(compact_record(
                    "relation.role",
                    &[("name", role.name().to_owned())],
                )?);
            }
        }
        RegistrySection::Limits => {
            for descriptor in limit_descriptors() {
                records.push(compact_record(
                    "limit",
                    &[
                        ("name", descriptor.name.to_owned()),
                        ("value", descriptor.value.to_string()),
                        ("class", descriptor.class.name().to_owned()),
                        ("unit", descriptor.unit.name().to_owned()),
                        ("override", descriptor.override_policy.name().to_owned()),
                    ],
                )?);
            }
        }
        RegistrySection::Diagnostics => {
            for descriptor in diagnostic_descriptors() {
                records.push(compact_record(
                    "diagnostic",
                    &[
                        ("code", descriptor.code.to_owned()),
                        ("class", diagnostic_class_name(descriptor.class).to_owned()),
                        ("meaning", descriptor.meaning.to_owned()),
                        ("retry", descriptor.retry.to_owned()),
                    ],
                )?);
            }
            for descriptor in exit_status_descriptors() {
                records.push(compact_record(
                    "exit-status",
                    &[
                        ("status", descriptor.status.to_string()),
                        ("meaning", descriptor.meaning.to_owned()),
                    ],
                )?);
            }
        }
        RegistrySection::Templates => {
            for descriptor in template_descriptors() {
                records.push(compact_record(
                    "template",
                    &[
                        ("name", descriptor.name.to_owned()),
                        ("purpose", descriptor.purpose.to_owned()),
                        (
                            "command-target",
                            descriptor.creates_command_target.to_string(),
                        ),
                    ],
                )?);
            }
        }
        RegistrySection::Runners => {
            for kind in RunnerKind::ALL {
                records.push(compact_record(
                    "runner.kind",
                    &[("name", kind.name().to_owned())],
                )?);
            }
        }
        RegistrySection::Security => {
            for (index, statement) in nonclaims().iter().enumerate() {
                records.push(compact_record(
                    "security.nonclaim",
                    &[
                        ("ordinal", index.saturating_add(1).to_string()),
                        ("statement", (*statement).to_owned()),
                    ],
                )?);
            }
        }
    }
    Ok(records)
}

pub fn operation_record(descriptor: &OperationDescriptor) -> Result<String, String> {
    compact_record(
        "operation",
        &[
            ("name", descriptor.operation.name().to_owned()),
            ("purpose", descriptor.purpose.to_owned()),
            ("usage", descriptor.usage.to_owned()),
            ("request-model", descriptor.request_model.name().to_owned()),
            (
                "response-model",
                descriptor.response_model.name().to_owned(),
            ),
            (
                "authority-effect",
                descriptor.authority_effect.name().to_owned(),
            ),
            ("project", descriptor.project_requirement.name().to_owned()),
            ("budget", descriptor.default_budget.name().to_owned()),
        ],
    )
}

fn compact_record(operation: &str, fields: &[(&str, String)]) -> Result<String, String> {
    let borrowed = fields
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    render_record(operation, &borrowed).map_err(|error| {
        format!(
            "compact registry rendering failed: {}: {}",
            error.code, error.message
        )
    })
}

pub const fn diagnostic_class_name(class: DiagnosticClass) -> &'static str {
    match class {
        DiagnosticClass::Source => "source",
        DiagnosticClass::Semantic => "semantic",
        DiagnosticClass::Capability => "capability",
        DiagnosticClass::Resource => "resource",
        DiagnosticClass::Cancelled => "cancelled",
        DiagnosticClass::Corrupt => "corrupt",
        DiagnosticClass::Infrastructure => "infrastructure",
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
        let first = registry_snapshot().expect("valid registry");
        let second = registry_snapshot().expect("valid registry");
        assert_eq!(first.digest, second.digest);
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.sections, second.sections);
        assert_eq!(operation_descriptors().len(), PublicOperation::ALL.len());
        assert_eq!(RegistrySection::ALL.len(), first.sections.len());
        assert!(first.bytes.starts_with(b"registry "));
        assert!(!first.bytes.starts_with(b"{"));
    }

    #[test]
    fn cli_response_budgets_and_writer_failures_are_advertised() {
        let bytes = limit_descriptors()
            .iter()
            .find(|descriptor| descriptor.name == "cli_response_bytes")
            .expect("CLI response byte descriptor");
        assert_eq!(bytes.value, MAXIMUM_CLI_RESPONSE_BYTES as u64);
        assert_eq!(bytes.class, LimitClass::DeterministicOperationBudget);
        assert_eq!(bytes.unit, LimitUnit::Bytes);
        assert_eq!(bytes.override_policy, OverridePolicy::Fixed);

        let records = limit_descriptors()
            .iter()
            .find(|descriptor| descriptor.name == "cli_response_records")
            .expect("CLI response record descriptor");
        assert_eq!(records.value, MAXIMUM_CLI_RESPONSE_RECORDS as u64);
        assert_eq!(records.class, LimitClass::DeterministicOperationBudget);
        assert_eq!(records.unit, LimitUnit::Records);
        assert_eq!(records.override_policy, OverridePolicy::Fixed);

        for code in [
            "control_response_byte_budget",
            "control_response_record_budget",
            "control_response_allocation",
            "control_response_limits",
            "control_response_records_newline",
            "control_response_records_blank",
            "control_response_records_invalid",
            "control_render_operation",
            "control_render_fields",
            "control_render_field",
            "control_render_duplicate_field",
            "control_render_value_bytes",
            "control_render_record_bytes",
            "control_render_allocation",
        ] {
            assert!(
                diagnostic_descriptors()
                    .iter()
                    .any(|descriptor| descriptor.code == code),
                "missing compact response diagnostic descriptor {code}"
            );
        }
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
        let owners = section_records(RegistrySection::Owners).expect("owner records");
        assert_eq!(owners.len(), OwnerKind::PUBLIC_EXACT.len());
        assert!(owners.iter().all(|record| !record.contains("name=field")));
        assert!(
            owners
                .iter()
                .all(|record| !record.contains("name=expression"))
        );
    }
}

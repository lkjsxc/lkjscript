use super::super::builtin_discovery::{
    BUILTIN_CONTINUATION_MAXIMUM_BYTES, BUILTIN_QUERY_DEFAULT_BYTES, BUILTIN_QUERY_DEFAULT_ITEMS,
    BUILTIN_QUERY_MAXIMUM_BYTES, BUILTIN_QUERY_MAXIMUM_ITEMS, BUILTIN_QUERY_MINIMUM_BYTES,
};
use super::super::compiler::{
    ARTIFACT_BUNDLE_CHECKSUM_DOMAIN, ARTIFACT_BUNDLE_CONTRACT_IDENTITY,
    ARTIFACT_BUNDLE_DIGEST_DOMAIN, ARTIFACT_CLOSURE_DIGEST_DOMAIN, ARTIFACT_CONTRACT_VERSION,
    ARTIFACT_MANIFEST_CONTRACT_IDENTITY, ARTIFACT_MANIFEST_ENVELOPE_DOMAIN,
    BYTECODE_CONTRACT_IDENTITY, BYTECODE_CONTRACT_VERSION, COMPILATION_MANIFEST_CONTRACT_IDENTITY,
    COMPILATION_MANIFEST_CONTRACT_VERSION, COMPILATION_MANIFEST_ENVELOPE_DOMAIN,
    COMPILER_UNIT_CONTRACT_IDENTITY, COMPILER_UNIT_CONTRACT_VERSION, COMPILER_UNIT_ENVELOPE_DOMAIN,
    COMPILER_UNIT_KEY_DOMAIN,
};
use super::super::configuration::CONFIGURATION_ADAPTER_CONTRACT_VERSION;
use super::super::control::{
    AUTHORED_CHANGE_CODEC_IDENTITY, AUTHORED_CHANGE_CODEC_VERSION,
    CHANGE_REQUEST_COMMITMENT_DOMAIN, COMPACT_CHANGE_CONTRACT_IDENTITY,
    COMPACT_CHANGE_CONTRACT_VERSION, COMPACT_CHANGE_EDGE_DESCRIPTORS,
    COMPACT_CHANGE_OPERATION_DESCRIPTORS, COMPACT_CHANGE_PRECONDITION_FIELDS,
    COMPACT_CHANGE_PRECONDITIONS, COMPACT_DECLARATION_VISIBILITIES, COMPACT_DELETE_POLICIES,
    COMPACT_EXPRESSION_FORM_FIELDS, COMPACT_EXPRESSION_FORMS, COMPACT_FUNCTION_EFFECTS,
    COMPACT_NAMESPACE_CLASSES, COMPACT_TYPE_FORM_FIELDS, COMPACT_TYPE_FORMS,
    CompactChangeFieldForm, CompactChangeOperation, LOGICAL_CHANGE_PLAN_CONTRACT_IDENTITY,
    LOGICAL_CHANGE_PLAN_CONTRACT_VERSION, LOGICAL_PLAN_RECORD_DESCRIPTORS,
    MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_LOGICAL_PLAN_BYTES, MAXIMUM_LOGICAL_PLAN_RECORDS,
    PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN, render_record,
};
use super::super::data::{
    DATA_BACKUP_CONTRACT_IDENTITY, DATA_BACKUP_CONTRACT_VERSION, DATA_STORE_CONTRACT_IDENTITY,
    DATA_STORE_CONTRACT_VERSION, MAXIMUM_DATA_BACKUP_BYTES, MAXIMUM_DATA_HISTORY_REVISIONS,
    MAXIMUM_DATA_KEY_BYTES, MAXIMUM_DATA_KEY_PARTS, MAXIMUM_DATA_LIVE_TRANSACTIONS,
    MAXIMUM_DATA_NAMESPACE_BYTES, MAXIMUM_DATA_REVISION_BYTES, MAXIMUM_DATA_SCAN_BYTES,
    MAXIMUM_DATA_SCAN_ITEMS, MAXIMUM_DATA_SCAN_WORK, MAXIMUM_DATA_SPACE_NAME_BYTES,
    MAXIMUM_DATA_STORE_OBJECTS, MAXIMUM_DATA_TRANSACTION_BYTES, MAXIMUM_DATA_TRANSACTION_MUTATIONS,
    MAXIMUM_DATA_VALUE_BYTES,
};
use super::super::deployment::{
    DEPLOYMENT_ADAPTER_SCHEMAS, DEPLOYMENT_CONTRACT_VERSION, DEPLOYMENT_SCHEMA_FIELDS,
    MAXIMUM_DEPLOYMENT_BYTES, MAXIMUM_DEPLOYMENT_GRANTS,
};
use super::super::diagnostic::DiagnosticClass;
use super::super::execution::normalized::CAPABILITY_GRANT_CONTRACT_VERSION;
use super::super::http::HTTP_ADAPTER_CONTRACT_VERSION;
use super::super::http_client::HTTP_CLIENT_ADAPTER_CONTRACT_VERSION;
use super::super::json::JSON_CONTRACT_VERSION;
use super::super::kernel::contract::{GRAPH_CONTRACT_IDENTITY, GRAPH_CONTRACT_VERSION};
use super::super::kernel::{NamespaceClass, OwnerKind, RelationKind};
use super::super::normalized_query::{
    ContextDirection, DEFAULT_QUERY_ITEMS, DEFAULT_QUERY_OUTPUT_BYTES, MAXIMUM_CONTEXT_DEPTH,
    MAXIMUM_CONTEXT_MAP_BYTES, MAXIMUM_CONTEXT_MAP_ENTRIES, MAXIMUM_CONTEXT_MAP_PAGES,
    MAXIMUM_CONTEXT_OWNERS, MAXIMUM_CONTEXT_RELATION_WITNESSES, MAXIMUM_CONTEXT_RELATIONS,
    MAXIMUM_CONTEXT_STORE_BYTES, MAXIMUM_CONTEXT_STORE_OBJECTS, MAXIMUM_QUERY_CONTINUATION_BYTES,
    MAXIMUM_QUERY_ITEMS, MAXIMUM_QUERY_OUTPUT_BYTES, MINIMUM_CONTEXT_DEPTH,
    MINIMUM_QUERY_OUTPUT_BYTES, QUERY_CONTINUATION_INTEGRITY_DOMAIN, QUERY_CONTINUATION_MAGIC_TEXT,
    QUERY_CONTRACT_IDENTITY, QUERY_CONTRACT_VERSION, QUERY_OPERATION_DESCRIPTORS,
    QUERY_RESPONSE_FIELDS, QUERY_SELECTOR_DIGEST_DOMAIN, QUERY_SELECTOR_FIELDS, QueryDirection,
};
use super::super::object::OBJECT_ADAPTER_CONTRACT_VERSION;
use super::super::package::RunnerKind;
use super::super::package_interface::{
    PACKAGE_INTERFACE_CONTRACT_IDENTITY, PACKAGE_INTERFACE_CONTRACT_VERSION,
    PACKAGE_INTERFACE_ENVELOPE_DOMAIN,
};
use super::super::package_transport::{
    PACKAGE_REVISION_CONTRACT_IDENTITY, PACKAGE_REVISION_CONTRACT_VERSION,
    PACKAGE_REVISION_ENVELOPE_DOMAIN, PACKAGE_TRANSPORT_CONTRACT_IDENTITY,
    PACKAGE_TRANSPORT_CONTRACT_VERSION, PACKAGE_TRANSPORT_ENVELOPE_DOMAIN,
    PACKAGE_TRANSPORT_SELECTION_CONTRACT_IDENTITY, PACKAGE_TRANSPORT_SELECTION_CONTRACT_VERSION,
    PACKAGE_TRANSPORT_SELECTION_ENVELOPE_DOMAIN,
};
use super::super::project_creation::{
    PROJECT_CREATION_CONTRACT_IDENTITY, PROJECT_CREATION_CONTRACT_VERSION, ProjectTemplate,
};
use super::super::publication::contract::{
    RECEIPT_CONTRACT_IDENTITY, RECEIPT_CONTRACT_VERSION, RECEIPT_ENVELOPE_DOMAIN,
    REVISION_CONTRACT_IDENTITY, REVISION_CONTRACT_VERSION, REVISION_ENVELOPE_DOMAIN,
    REVISION_IDENTITY_DIGEST_DOMAIN, SEMANTIC_DIFF_CONTRACT_IDENTITY,
    SEMANTIC_DIFF_CONTRACT_VERSION, SEMANTIC_DIFF_ENVELOPE_DOMAIN, TRANSACTION_CONTRACT_IDENTITY,
    TRANSACTION_CONTRACT_VERSION, TRANSACTION_ENVELOPE_DOMAIN,
};
use super::super::queue::DURABLE_QUEUE_CONTRACT_VERSION;
use super::super::runtime::RESIDENT_RUNTIME_CONTRACT_VERSION;
use super::super::secrets::{SECRET_CATALOG_CONTRACT_VERSION, SECRET_VERIFIER_CONTRACT_VERSION};
use super::super::security::SECURITY_ADAPTER_CONTRACT_VERSION;
use super::super::storage::contract as storage_contract;
use super::super::stream::STREAM_CONTRACT_VERSION;
use super::super::witness::contract as witness_contract;
use super::super::worker::WORKER_RUNNER_CONTRACT_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const REGISTRY_CONTRACT_IDENTITY: &str = "lkjscript-contract-registry-5";
pub const REGISTRY_CONTRACT_VERSION: u16 = 5;
pub const CLI_CONTRACT_VERSION: u16 = 17;
pub const MAXIMUM_CLI_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_CLI_RESPONSE_RECORDS: usize = 10_000;
pub const MAXIMUM_TRANSACTION_REQUEST_BYTES: usize = 16 * 1_048_576;

const REGISTRY_DIGEST_DOMAIN: &str = "lkjscript.contract-registry.v5";
const REGISTRY_SECTION_DIGEST_DOMAIN: &str = "lkjscript.contract-registry-section.v5";
const CAPABILITIES_DIGEST_DOMAIN: &str = "lkjscript.public-capabilities";

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

const fn magic_bytes(value: &str) -> [u8; 8] {
    let bytes = value.as_bytes();
    [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]
}

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
const FACT_MANIFEST_MAGIC_TEXT: &str = "LKJSFI03";
pub(crate) const FACT_MANIFEST_MAGIC: [u8; 8] = magic_bytes(FACT_MANIFEST_MAGIC_TEXT);
pub(crate) const FACT_MANIFEST_DOMAIN: &str = "lkjscript.semantic-fact-manifest.v3";
pub(crate) const SEMANTIC_CERTIFICATE_DOMAIN: &str = "lkjscript.semantic-certificate.v3";
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
pub(crate) const CHANGE_ALLOCATION_SEED_DOMAIN: &str = "lkjscript.change-allocation-seed.v5";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKey {
    Registry,
    Cli,
    MeaningGraph,
    ImmutableObjectStore,
    ImmutablePack,
    ObjectCatalog,
    Revision,
    Receipt,
    SemanticSummary,
    SemanticFacts,
    SemanticValidator,
    Change,
    AuthoredChangeCodec,
    LogicalChangePlan,
    Transaction,
    Query,
    Diff,
    ArtifactManifest,
    ArtifactBundle,
    ProjectCreation,
    PackageRevision,
    PackageInterface,
    PackageTransport,
    PackageTransportSelection,
    CompilationManifest,
    CompilerUnit,
    Bytecode,
    Deployment,
    DataStore,
    DataBackup,
    ConfigurationAdapter,
    CapabilityGrant,
    HttpAdapter,
    HttpClientAdapter,
    Json,
    ObjectAdapter,
    QueueAdapter,
    ResidentRuntime,
    SecretCatalog,
    SecretVerifier,
    SecurityAdapter,
    Stream,
    WorkerRunner,
}

impl ContractKey {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::Cli => "cli",
            Self::MeaningGraph => "meaning_graph",
            Self::ImmutableObjectStore => "immutable_object_store",
            Self::ImmutablePack => "immutable_pack",
            Self::ObjectCatalog => "object_catalog",
            Self::Revision => "revision",
            Self::Receipt => "receipt",
            Self::SemanticSummary => "semantic_summary",
            Self::SemanticFacts => "semantic_facts",
            Self::SemanticValidator => "semantic_validator",
            Self::Change => "change",
            Self::AuthoredChangeCodec => "authored_change_codec",
            Self::LogicalChangePlan => "logical_change_plan",
            Self::Transaction => "transaction",
            Self::Query => "query",
            Self::Diff => "diff",
            Self::ArtifactManifest => "artifact_manifest",
            Self::ArtifactBundle => "artifact_bundle",
            Self::ProjectCreation => "project_creation",
            Self::PackageRevision => "package_revision",
            Self::PackageInterface => "package_interface",
            Self::PackageTransport => "package_transport",
            Self::PackageTransportSelection => "package_transport_selection",
            Self::CompilationManifest => "compilation_manifest",
            Self::CompilerUnit => "compiler_unit",
            Self::Bytecode => "bytecode",
            Self::Deployment => "deployment",
            Self::DataStore => "data_store",
            Self::DataBackup => "data_backup",
            Self::ConfigurationAdapter => "configuration_adapter",
            Self::CapabilityGrant => "capability_grant",
            Self::HttpAdapter => "http_adapter",
            Self::HttpClientAdapter => "http_client_adapter",
            Self::Json => "json",
            Self::ObjectAdapter => "object_adapter",
            Self::QueueAdapter => "queue_adapter",
            Self::ResidentRuntime => "resident_runtime",
            Self::SecretCatalog => "secret_catalog",
            Self::SecretVerifier => "secret_verifier",
            Self::SecurityAdapter => "security_adapter",
            Self::Stream => "stream",
            Self::WorkerRunner => "worker_runner",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStability {
    Current,
    Frozen,
}

impl ContractStability {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Frozen => "frozen",
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
            name: "normalized command line protocol",
            identity: "lkjscript-cli-13",
            version: CLI_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: NONE,
        },
        ContractDescriptor {
            key: ContractKey::MeaningGraph,
            name: "normalized meaning graph",
            identity: GRAPH_CONTRACT_IDENTITY,
            version: GRAPH_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::CanonicalMeaning,
            predecessor_policy: REJECT,
            magic_values: &["LKJOWN06", "LKJTYP06", "LKJSMR01", "LKJDEP06", "LKJRET06"],
            digest_domains: &[
                super::super::kernel::contract::OWNER_ENVELOPE_DOMAIN,
                super::super::kernel::contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
                super::super::kernel::contract::ROOT_ENVELOPE_DOMAIN,
                super::super::kernel::contract::DEPENDENCY_ENVELOPE_DOMAIN,
                super::super::kernel::contract::RETIREMENT_ENVELOPE_DOMAIN,
                super::super::kernel::contract::OWNER_OBJECT_DIGEST_DOMAIN,
                super::super::kernel::contract::TYPE_OBJECT_DIGEST_DOMAIN,
                super::super::kernel::contract::BLOB_OBJECT_DIGEST_DOMAIN,
                super::super::kernel::contract::SEQUENCE_OBJECT_DIGEST_DOMAIN,
                super::super::kernel::contract::SEMANTIC_ROOT_DIGEST_DOMAIN,
                super::super::kernel::contract::SEMANTIC_STATE_DIGEST_DOMAIN,
                super::super::kernel::contract::DEPENDENCY_OBJECT_DIGEST_DOMAIN,
                super::super::kernel::contract::RETIREMENT_OBJECT_DIGEST_DOMAIN,
                super::super::kernel::contract::CHANGE_DIGEST_DOMAIN,
                super::super::kernel::contract::PACKAGE_ID_MIGRATION_DOMAIN,
            ],
        },
        simple_contract(
            ContractKey::ImmutableObjectStore,
            "immutable object store",
            storage_contract::OBJECT_STORE_CONTRACT_IDENTITY,
            1,
            ContractAuthority::CanonicalMeaning,
        ),
        ContractDescriptor {
            key: ContractKey::ImmutablePack,
            name: "immutable object pack",
            identity: storage_contract::PACK_CONTRACT_IDENTITY,
            version: storage_contract::PACK_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::CanonicalMeaning,
            predecessor_policy: REJECT,
            magic_values: &["LKJPAK01", "LKJIDX01", "LKJEND01"],
            digest_domains: &[
                storage_contract::PACK_NONCE_DOMAIN,
                storage_contract::PACK_ID_DOMAIN,
                storage_contract::PACK_ENTRY_CHECKSUM_DOMAIN,
                storage_contract::PACK_INDEX_CHECKSUM_DOMAIN,
                storage_contract::PACK_CHECKSUM_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::ObjectCatalog,
            name: "rebuildable object catalog",
            identity: storage_contract::CATALOG_CONTRACT_IDENTITY,
            version: storage_contract::CATALOG_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &["LKJCAT01", "LKJCEND1"],
            digest_domains: &[
                storage_contract::CATALOG_CHECKSUM_DOMAIN,
                storage_contract::CATALOG_GENERATION_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Revision,
            name: "accepted revision",
            identity: REVISION_CONTRACT_IDENTITY,
            version: REVISION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::AcceptedHistory,
            predecessor_policy: REJECT,
            magic_values: &["LKJREV07", "LKJHEAD7"],
            digest_domains: &[
                REVISION_ENVELOPE_DOMAIN,
                super::super::publication::contract::HEAD_ENVELOPE_DOMAIN,
                REVISION_IDENTITY_DIGEST_DOMAIN,
                storage_contract::REVISION_OBJECT_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Receipt,
            name: "publication receipt",
            identity: RECEIPT_CONTRACT_IDENTITY,
            version: RECEIPT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::AcceptedHistory,
            predecessor_policy: REJECT,
            magic_values: &["LKJRCPT5"],
            digest_domains: &[
                RECEIPT_ENVELOPE_DOMAIN,
                storage_contract::RECEIPT_OBJECT_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Transaction,
            name: "normalized semantic transaction",
            identity: TRANSACTION_CONTRACT_IDENTITY,
            version: TRANSACTION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::AcceptedHistory,
            predecessor_policy: REJECT,
            magic_values: &["LKJTXN05"],
            digest_domains: &[
                TRANSACTION_ENVELOPE_DOMAIN,
                storage_contract::TRANSACTION_OBJECT_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::Diff,
            name: "accepted semantic diff",
            identity: SEMANTIC_DIFF_CONTRACT_IDENTITY,
            version: SEMANTIC_DIFF_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::AcceptedHistory,
            predecessor_policy: REJECT,
            magic_values: &["LKJDIFF3"],
            digest_domains: &[
                SEMANTIC_DIFF_ENVELOPE_DOMAIN,
                storage_contract::SEMANTIC_DIFF_OBJECT_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::SemanticSummary,
            name: "owner validation summary",
            identity: witness_contract::OWNER_SUMMARY_CONTRACT_IDENTITY,
            version: witness_contract::OWNER_SUMMARY_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::RequiredWitness,
            predecessor_policy: REJECT,
            magic_values: &["LKJSUM06"],
            digest_domains: &[
                witness_contract::OWNER_SUMMARY_ENVELOPE_DOMAIN,
                witness_contract::OWNER_SUMMARY_DIGEST_DOMAIN,
                witness_contract::INTERFACE_DIGEST_DOMAIN,
                witness_contract::IMPLEMENTATION_DIGEST_DOMAIN,
                witness_contract::TYPE_DIGEST_DOMAIN,
                witness_contract::EFFECT_DIGEST_DOMAIN,
                witness_contract::CAPABILITY_DIGEST_DOMAIN,
                witness_contract::RELATION_DIGEST_DOMAIN,
                witness_contract::PRESENTATION_DIGEST_DOMAIN,
                witness_contract::TEST_DIGEST_DOMAIN,
                witness_contract::VALIDATION_DEPENDENCY_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::SemanticFacts,
            name: "validation witness",
            identity: witness_contract::WITNESS_CONTRACT_IDENTITY,
            version: witness_contract::WITNESS_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::RequiredWitness,
            predecessor_policy: REJECT,
            magic_values: &["LKJWIT03"],
            digest_domains: &[
                witness_contract::WITNESS_ENVELOPE_DOMAIN,
                witness_contract::VALIDATION_WITNESS_DIGEST_DOMAIN,
                witness_contract::VALIDATION_CERTIFICATE_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::SemanticValidator,
            name: "semantic validator",
            identity: witness_contract::VALIDATOR_CONTRACT_IDENTITY,
            version: 6,
            stability: CURRENT,
            authority: ContractAuthority::RequiredWitness,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[witness_contract::VALIDATOR_CONTRACT_DIGEST_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Change,
            name: "authored semantic change",
            identity: COMPACT_CHANGE_CONTRACT_IDENTITY,
            version: COMPACT_CHANGE_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: NONE,
        },
        ContractDescriptor {
            key: ContractKey::AuthoredChangeCodec,
            name: "normalized authored change codec",
            identity: AUTHORED_CHANGE_CODEC_IDENTITY,
            version: AUTHORED_CHANGE_CODEC_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: &["LKJACR06", "LKJABG01"],
            digest_domains: &[
                CHANGE_ALLOCATION_SEED_DOMAIN,
                CHANGE_REQUEST_COMMITMENT_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::LogicalChangePlan,
            name: "logical change plan",
            identity: LOGICAL_CHANGE_PLAN_CONTRACT_IDENTITY,
            version: LOGICAL_CHANGE_PLAN_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: &[PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::Query,
            name: "normalized semantic query",
            identity: QUERY_CONTRACT_IDENTITY,
            version: QUERY_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: &[QUERY_CONTINUATION_MAGIC_TEXT],
            digest_domains: &[
                QUERY_SELECTOR_DIGEST_DOMAIN,
                QUERY_CONTINUATION_INTEGRITY_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::ProjectCreation,
            name: "typed project creation recipe",
            identity: PROJECT_CREATION_CONTRACT_IDENTITY,
            version: PROJECT_CREATION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::PublicProtocol,
            predecessor_policy: REJECT,
            magic_values: NONE,
            digest_domains: NONE,
        },
        ContractDescriptor {
            key: ContractKey::PackageRevision,
            name: "package revision",
            identity: PACKAGE_REVISION_CONTRACT_IDENTITY,
            version: PACKAGE_REVISION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &["LKJPKR01"],
            digest_domains: &[
                PACKAGE_REVISION_ENVELOPE_DOMAIN,
                super::super::kernel::contract::PACKAGE_REVISION_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::PackageInterface,
            name: "package interface owner",
            identity: PACKAGE_INTERFACE_CONTRACT_IDENTITY,
            version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &["LKJPIF04"],
            digest_domains: &[
                PACKAGE_INTERFACE_ENVELOPE_DOMAIN,
                super::super::kernel::contract::PACKAGE_INTERFACE_DIGEST_DOMAIN,
                storage_contract::PACKAGE_INTERFACE_OWNER_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::PackageTransport,
            name: "package transport",
            identity: PACKAGE_TRANSPORT_CONTRACT_IDENTITY,
            version: PACKAGE_TRANSPORT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &["LKJPKT01"],
            digest_domains: &[
                PACKAGE_TRANSPORT_ENVELOPE_DOMAIN,
                super::super::kernel::contract::PACKAGE_TRANSPORT_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::PackageTransportSelection,
            name: "repository package transport selection",
            identity: PACKAGE_TRANSPORT_SELECTION_CONTRACT_IDENTITY,
            version: PACKAGE_TRANSPORT_SELECTION_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Operational,
            predecessor_policy: REJECT,
            magic_values: &["LKJPTS01"],
            digest_domains: &[PACKAGE_TRANSPORT_SELECTION_ENVELOPE_DOMAIN],
        },
        ContractDescriptor {
            key: ContractKey::CompilationManifest,
            name: "revision-bound compilation manifest",
            identity: COMPILATION_MANIFEST_CONTRACT_IDENTITY,
            version: COMPILATION_MANIFEST_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &["LKJCMF03"],
            digest_domains: &[
                COMPILATION_MANIFEST_ENVELOPE_DOMAIN,
                storage_contract::COMPILATION_MANIFEST_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::CompilerUnit,
            name: "normalized compiler unit",
            identity: COMPILER_UNIT_CONTRACT_IDENTITY,
            version: COMPILER_UNIT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::DerivedDisposable,
            predecessor_policy: REJECT,
            magic_values: &["LKJCUN02"],
            digest_domains: &[
                COMPILER_UNIT_ENVELOPE_DOMAIN,
                COMPILER_UNIT_KEY_DOMAIN,
                storage_contract::COMPILER_UNIT_DIGEST_DOMAIN,
            ],
        },
        simple_contract(
            ContractKey::Bytecode,
            "normalized typed bytecode",
            BYTECODE_CONTRACT_IDENTITY,
            BYTECODE_CONTRACT_VERSION,
            ContractAuthority::DerivedDisposable,
        ),
        ContractDescriptor {
            key: ContractKey::ArtifactManifest,
            name: "normalized artifact manifest",
            identity: ARTIFACT_MANIFEST_CONTRACT_IDENTITY,
            version: ARTIFACT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Runtime,
            predecessor_policy: REJECT,
            magic_values: &["LKJAMF11"],
            digest_domains: &[
                ARTIFACT_MANIFEST_ENVELOPE_DOMAIN,
                storage_contract::ARTIFACT_MANIFEST_DIGEST_DOMAIN,
            ],
        },
        ContractDescriptor {
            key: ContractKey::ArtifactBundle,
            name: "normalized artifact bundle",
            identity: ARTIFACT_BUNDLE_CONTRACT_IDENTITY,
            version: ARTIFACT_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Runtime,
            predecessor_policy: REJECT,
            magic_values: &["LKJART11", "LKJAEN11"],
            digest_domains: &[
                ARTIFACT_BUNDLE_DIGEST_DOMAIN,
                ARTIFACT_BUNDLE_CHECKSUM_DOMAIN,
                ARTIFACT_CLOSURE_DIGEST_DOMAIN,
            ],
        },
        simple_contract(
            ContractKey::Deployment,
            "standalone artifact deployment descriptor",
            "lkjscript-deployment-1",
            DEPLOYMENT_CONTRACT_VERSION,
            ContractAuthority::Deployment,
        ),
        ContractDescriptor {
            key: ContractKey::DataStore,
            name: "first-party ordered application-data store",
            identity: DATA_STORE_CONTRACT_IDENTITY,
            version: DATA_STORE_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Operational,
            predecessor_policy: REJECT,
            magic_values: &["LKJDATA1", "LKJDREV1", "LKJDHEAD"],
            digest_domains: &[
                "lkjscript.data.format.v1",
                "lkjscript.data.revision.v1",
                "lkjscript.data.revision-envelope.v1",
                "lkjscript.data.head-envelope.v1",
            ],
        },
        ContractDescriptor {
            key: ContractKey::DataBackup,
            name: "canonical logical application-data backup",
            identity: DATA_BACKUP_CONTRACT_IDENTITY,
            version: DATA_BACKUP_CONTRACT_VERSION,
            stability: CURRENT,
            authority: ContractAuthority::Operational,
            predecessor_policy: REJECT,
            magic_values: &["LKJDBAK1"],
            digest_domains: &["lkjscript.data.backup-envelope.v1"],
        },
        simple_contract(
            ContractKey::ConfigurationAdapter,
            "configuration adapter",
            "lkjscript-configuration-adapter-1",
            CONFIGURATION_ADAPTER_CONTRACT_VERSION,
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
            ContractKey::HttpClientAdapter,
            "outbound HTTP client adapter",
            "lkjscript-http-client-adapter-1",
            HTTP_CLIENT_ADAPTER_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::Json,
            "bounded JSON value adapter",
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
            "lkjscript-queue-adapter-2",
            DURABLE_QUEUE_CONTRACT_VERSION,
            ContractAuthority::Runtime,
        ),
        simple_contract(
            ContractKey::ResidentRuntime,
            "normalized artifact resident runtime",
            "lkjscript-resident-runtime-2",
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicOperation {
    Capabilities,
    Data,
    New,
    Status,
    Inspect,
    Query,
    Change,
    Package,
    Check,
    Build,
    Run,
    Serve,
    Worker,
}

impl PublicOperation {
    pub const ALL: [Self; 13] = [
        Self::Capabilities,
        Self::Data,
        Self::New,
        Self::Status,
        Self::Inspect,
        Self::Query,
        Self::Change,
        Self::Package,
        Self::Check,
        Self::Build,
        Self::Run,
        Self::Serve,
        Self::Worker,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Data => "data",
            Self::New => "new",
            Self::Status => "status",
            Self::Inspect => "inspect",
            Self::Query => "query",
            Self::Change => "change",
            Self::Package => "package",
            Self::Check => "check",
            Self::Build => "build",
            Self::Run => "run",
            Self::Serve => "serve",
            Self::Worker => "worker",
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
    ExternalOutput,
    ExternalRuntime,
    OptionalExternalOutput,
}

impl AuthorityEffect {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Accepted => "accepted",
            Self::AcceptedOnCommit => "accepted_on_commit",
            Self::ExternalOutput => "external_output",
            Self::ExternalRuntime => "external_runtime",
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
    DescriptorBound,
}

impl ProjectRequirement {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Destination => "destination",
            Self::Required => "required",
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
    DataRequest,
    DataResult,
    NewRequest,
    NewResult,
    StatusRequest,
    StatusResult,
    InspectRequest,
    InspectResult,
    QueryRequest,
    QueryResult,
    ChangeRequest,
    PackageRequest,
    PackageResult,
    CheckRequest,
    CheckResult,
    BuildRequest,
    BuildResult,
    RunRequest,
    RunResult,
    ServeRequest,
    WorkerRequest,
    CompactResult,
    RuntimeEvent,
}

impl ControlModel {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CapabilitiesRequest => "capabilities_request",
            Self::CapabilitiesResult => "capabilities_result",
            Self::DataRequest => "data_request",
            Self::DataResult => "data_result",
            Self::NewRequest => "new_request",
            Self::NewResult => "new_result",
            Self::StatusRequest => "status_request",
            Self::StatusResult => "status_result",
            Self::InspectRequest => "inspect_request",
            Self::InspectResult => "inspect_result",
            Self::QueryRequest => "query_request",
            Self::QueryResult => "query_result",
            Self::ChangeRequest => "change_request",
            Self::PackageRequest => "package_request",
            Self::PackageResult => "package_result",
            Self::CheckRequest => "check_request",
            Self::CheckResult => "check_result",
            Self::BuildRequest => "build_request",
            Self::BuildResult => "build_result",
            Self::RunRequest => "run_request",
            Self::RunResult => "run_result",
            Self::ServeRequest => "serve_request",
            Self::WorkerRequest => "worker_request",
            Self::CompactResult => "compact_result",
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
            "Discover product operations, grammar, limits, diagnostics, effects, and changed capability sections.",
            "capabilities [COMMAND] [--known-capabilities DIGEST] [--section SECTION] [--known-section SECTION=DIGEST] [--output PATH] [--generate-docs DIR] [--verify-generated DIR]",
        ),
        operation(
            PublicOperation::Data,
            "Initialize, verify, back up, or restore one first-party ordered application-data root.",
            "data initialize --root PATH | data verify --root PATH | data backup --root PATH --output PATH | data restore --backup PATH --root PATH",
            (ControlModel::DataRequest, ControlModel::DataResult),
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::None,
            BudgetProfile::Maintenance,
        ),
        new_operation(
            "Create fresh normalized semantic authority atomically at one absent safe destination.",
            "new DEST [--template minimal|command|http|nostr-relay-info] [--name NAME] [--relay-url URL]",
        ),
        status_operation(
            "Report the exact current semantic authority and its durable acceptance evidence.",
            "status",
        ),
        inspect_operation(
            "Inspect a compact summary of one exact owner at the observed accepted revision.",
            "inspect owner KIND ID [--package PACKAGE]",
        ),
        query_operation(
            "Enumerate live owners, resolve one exact namespace, inspect one relation prefix, or traverse one bounded local context at the current normalized revision.",
            "query owners [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN] | query find CLASS NAME [--parent OWNER] | query relations OWNER|package --direction incoming|outgoing [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN] | query context OWNER --direction incoming|outgoing|both --depth N [--limit N] [--bytes N] [--continuation TOKEN]",
        ),
        operation(
            PublicOperation::Change,
            "Prepare, optionally export, or atomically apply one review-bound logical semantic change plan.",
            "change plan ((--input RECORDS | --input-file PATH) | rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT]) [--output PATH] | change apply ((--input RECORDS | --input-file PATH) | rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT]) --plan TOKEN",
            (ControlModel::ChangeRequest, ControlModel::CompactResult),
            AuthorityEffect::AcceptedOnCommit,
            ProjectRequirement::Required,
            BudgetProfile::SemanticChange,
        ),
        operation(
            PublicOperation::Package,
            "Inspect the built-in standard, export one current package transport, or stage one exact dependency transport without editing meaning.",
            "package builtin inspect | package builtin inspect owner KIND ID | package builtin query owners [--kind KIND] [--name NAME] [--parent OWNER] [--limit N] [--bytes N] [--continuation TOKEN] | package builtin export --kind transport|artifact --output PATH | package current export --kind transport --output PATH | package dependency stage --transport DIGEST --input-file PATH",
            (ControlModel::PackageRequest, ControlModel::PackageResult),
            AuthorityEffect::OptionalExternalOutput,
            ProjectRequirement::None,
            BudgetProfile::Maintenance,
        ),
        operation(
            PublicOperation::Check,
            "Run graph-owned tests through production and independent execution.",
            "check",
            (ControlModel::CheckRequest, ControlModel::CheckResult),
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::Runtime,
        ),
        operation(
            PublicOperation::Build,
            "Build a deterministic graph-native artifact.",
            "build --output PATH",
            (ControlModel::BuildRequest, ControlModel::BuildResult),
            AuthorityEffect::ExternalOutput,
            ProjectRequirement::Required,
            BudgetProfile::Build,
        ),
        operation(
            PublicOperation::Run,
            "Run a pure command target through production and independent execution.",
            "run TARGET [--arguments JSON]",
            (ControlModel::RunRequest, ControlModel::RunResult),
            AuthorityEffect::None,
            ProjectRequirement::Required,
            BudgetProfile::Runtime,
        ),
        runtime_operation(
            PublicOperation::Serve,
            "Run one plaintext HTTP deployment from a standalone normalized artifact bundle.",
            "serve --deployment DESCRIPTOR",
            ControlModel::ServeRequest,
        ),
        runtime_operation(
            PublicOperation::Worker,
            "Run one bounded worker deployment from a standalone normalized artifact bundle.",
            "worker --deployment DESCRIPTOR",
            ControlModel::WorkerRequest,
        ),
    ];
    OPERATIONS
}

const fn operation(
    operation: PublicOperation,
    purpose: &'static str,
    usage: &'static str,
    models: (ControlModel, ControlModel),
    authority_effect: AuthorityEffect,
    project_requirement: ProjectRequirement,
    default_budget: BudgetProfile,
) -> OperationDescriptor {
    OperationDescriptor {
        operation,
        purpose,
        usage,
        request_model: models.0,
        response_model: models.1,
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

const fn query_operation(purpose: &'static str, usage: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        operation: PublicOperation::Query,
        purpose,
        usage,
        request_model: ControlModel::QueryRequest,
        response_model: ControlModel::QueryResult,
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
            "logical_change_plan_bytes",
            MAXIMUM_LOGICAL_PLAN_BYTES as usize,
            LimitClass::DeterministicOperationBudget,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "logical_change_plan_records",
            MAXIMUM_LOGICAL_PLAN_RECORDS as usize,
            LimitClass::DeterministicOperationBudget,
            LimitUnit::Records,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_default_items",
            DEFAULT_QUERY_ITEMS as usize,
            LimitClass::DefaultPagination,
            LimitUnit::Items,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_items",
            MAXIMUM_QUERY_ITEMS as usize,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Items,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_default_bytes",
            DEFAULT_QUERY_OUTPUT_BYTES,
            LimitClass::DefaultPagination,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_minimum_bytes",
            MINIMUM_QUERY_OUTPUT_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_bytes",
            MAXIMUM_QUERY_OUTPUT_BYTES,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Bytes,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_continuation_bytes",
            MAXIMUM_QUERY_CONTINUATION_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_context_depth",
            MAXIMUM_CONTEXT_DEPTH as usize,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Depth,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "query_context_owners",
            MAXIMUM_CONTEXT_OWNERS as usize,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Items,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_context_relations",
            MAXIMUM_CONTEXT_RELATIONS as usize,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Items,
            OverridePolicy::Fixed,
        ),
        limit(
            "query_context_relation_witnesses",
            MAXIMUM_CONTEXT_RELATION_WITNESSES as usize,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Records,
            OverridePolicy::Fixed,
        ),
        limit(
            "builtin_query_default_items",
            BUILTIN_QUERY_DEFAULT_ITEMS,
            LimitClass::DefaultPagination,
            LimitUnit::Items,
            OverridePolicy::Fixed,
        ),
        limit(
            "builtin_query_items",
            BUILTIN_QUERY_MAXIMUM_ITEMS,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Items,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "builtin_query_default_bytes",
            BUILTIN_QUERY_DEFAULT_BYTES,
            LimitClass::DefaultPagination,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "builtin_query_minimum_bytes",
            BUILTIN_QUERY_MINIMUM_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "builtin_query_bytes",
            BUILTIN_QUERY_MAXIMUM_BYTES,
            LimitClass::ExplicitRequestBudget,
            LimitUnit::Bytes,
            OverridePolicy::RequestUpToMaximum,
        ),
        limit(
            "builtin_continuation_bytes",
            BUILTIN_CONTINUATION_MAXIMUM_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "data_namespace_bytes",
            MAXIMUM_DATA_NAMESPACE_BYTES,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Bytes,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_space_name_bytes",
            MAXIMUM_DATA_SPACE_NAME_BYTES,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Bytes,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_key_parts",
            MAXIMUM_DATA_KEY_PARTS,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Items,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_key_bytes",
            MAXIMUM_DATA_KEY_BYTES,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Bytes,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_value_bytes",
            MAXIMUM_DATA_VALUE_BYTES,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Bytes,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_transaction_mutations",
            MAXIMUM_DATA_TRANSACTION_MUTATIONS,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Records,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_transaction_bytes",
            MAXIMUM_DATA_TRANSACTION_BYTES,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Bytes,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_scan_items",
            MAXIMUM_DATA_SCAN_ITEMS,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Items,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_scan_bytes",
            MAXIMUM_DATA_SCAN_BYTES,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Bytes,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_scan_work",
            MAXIMUM_DATA_SCAN_WORK,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Work,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_live_transactions",
            MAXIMUM_DATA_LIVE_TRANSACTIONS,
            LimitClass::DeploymentResourcePolicy,
            LimitUnit::Items,
            OverridePolicy::DeploymentUpToMaximum,
        ),
        limit(
            "data_revision_bytes",
            MAXIMUM_DATA_REVISION_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "data_backup_bytes",
            MAXIMUM_DATA_BACKUP_BYTES,
            LimitClass::HostileDecoderSafety,
            LimitUnit::Bytes,
            OverridePolicy::Fixed,
        ),
        limit(
            "data_history_revisions",
            MAXIMUM_DATA_HISTORY_REVISIONS,
            LimitClass::ImplementationLimitation,
            LimitUnit::Records,
            OverridePolicy::Fixed,
        ),
        limit(
            "data_store_objects",
            MAXIMUM_DATA_STORE_OBJECTS,
            LimitClass::HostileDecoderSafety,
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
            "data_root_absent",
            DiagnosticClass::Source,
            "A selected first-party data root does not exist.",
            "Initialize the root or restore a strict logical backup into an absent destination.",
        ),
        diagnostic(
            "data_root_type",
            DiagnosticClass::Source,
            "A selected first-party data root is not an ordinary directory.",
            "Select an ordinary local directory that does not traverse symbolic links.",
        ),
        diagnostic(
            "data_path_symlink",
            DiagnosticClass::Source,
            "A data lifecycle path traverses a symbolic link.",
            "Select an ordinary path with no symbolic-link component.",
        ),
        diagnostic(
            "data_path_traversal",
            DiagnosticClass::Source,
            "A data lifecycle path contains a parent-traversal component.",
            "Select a confined path without '..' components.",
        ),
        diagnostic(
            "data_root_inventory",
            DiagnosticClass::Corrupt,
            "A data root is incomplete or contains foreign authoritative entries.",
            "Preserve the root; restore a verified backup into a new absent destination.",
        ),
        diagnostic(
            "data_store_identity_changed",
            DiagnosticClass::Corrupt,
            "The physical data-store identity changed while the root was open.",
            "Stop using the root and restore a verified backup into a new absent destination.",
        ),
        diagnostic(
            "data_head_checksum",
            DiagnosticClass::Corrupt,
            "The accepted data-head envelope fails canonical integrity validation.",
            "Preserve the root; restore a verified backup into a new absent destination.",
        ),
        diagnostic(
            "data_revision_checksum",
            DiagnosticClass::Corrupt,
            "An immutable accepted data revision fails canonical integrity validation.",
            "Preserve the root; restore a verified backup into a new absent destination.",
        ),
        diagnostic(
            "data_backup_checksum",
            DiagnosticClass::Corrupt,
            "A logical data backup fails canonical integrity validation.",
            "Use the complete backup bytes and verify their separately retained digest.",
        ),
        diagnostic(
            "data_restore_destination_exists",
            DiagnosticClass::Source,
            "A data restore destination already exists.",
            "Choose an absent destination; restore never overwrites a root.",
        ),
        diagnostic(
            "data_output_exists",
            DiagnosticClass::Source,
            "A data backup output path already exists.",
            "Choose an absent output path; backup never overwrites a file.",
        ),
        diagnostic(
            "data_limit",
            DiagnosticClass::Source,
            "A deployment-selected data limit is zero or above its executable maximum.",
            "Select independent limits within the deployment and global capability bounds.",
        ),
        diagnostic(
            "data_verify_bytes",
            DiagnosticClass::Resource,
            "Complete data-root verification exceeds its independent byte accounting domain.",
            "Preserve the root and use a verified executable with sufficient admitted resources.",
        ),
        diagnostic(
            "data_head_visibility_unknown",
            DiagnosticClass::Infrastructure,
            "A data-head visibility change failed with an indeterminate publication result.",
            "Reopen and verify the exact root before deciding whether any logical retry is safe.",
        ),
        diagnostic(
            "data_head_durability_unknown",
            DiagnosticClass::Infrastructure,
            "A visible data head could not be confirmed durable at the root boundary.",
            "Reopen and verify the exact root before deciding whether any logical retry is safe.",
        ),
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
            "change_port_reference",
            DiagnosticClass::Source,
            "A port reference is neither a request-local symbol nor an exact package/port identity.",
            "Use $symbol or pkg_ID/port_ID as reported by focused change discovery.",
        ),
        diagnostic(
            "change_runner_kind",
            DiagnosticClass::Source,
            "A target runner is outside the public topology slice.",
            "Use command or http as reported by focused change discovery.",
        ),
        diagnostic(
            "change_dependency_binding_unsupported",
            DiagnosticClass::Source,
            "Dependency addition does not name the exact executable-owned built-in binding.",
            "Discover the current built-in package, semantic revision, and package revision and stage its exact exported transport.",
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
            "change_precondition_unknown",
            DiagnosticClass::Source,
            "A compact request uses an unknown semantic precondition.",
            "Select a precondition reported by capabilities --section change.",
        ),
        diagnostic(
            "change_precondition_namespace_class",
            DiagnosticClass::Source,
            "A namespace precondition uses an unknown namespace class.",
            "Select a change.namespace-class value from focused change discovery.",
        ),
        diagnostic(
            "change_precondition_owner_missing",
            DiagnosticClass::Semantic,
            "An owner-exists precondition observed no live exact owner.",
            "Refresh the exact owner at the observed base and rebuild the request.",
        ),
        diagnostic(
            "change_precondition_owner_present",
            DiagnosticClass::Semantic,
            "An owner-absent precondition observed a live exact owner.",
            "Refresh the exact owner at the observed base and rebuild the request.",
        ),
        diagnostic(
            "change_precondition_owner_name",
            DiagnosticClass::Semantic,
            "An exact owner no longer has the expected name.",
            "Inspect that owner at the observed base and rebuild the request.",
        ),
        diagnostic(
            "change_precondition_owner_parent",
            DiagnosticClass::Semantic,
            "An exact owner no longer has the expected semantic parent.",
            "Inspect that owner at the observed base and rebuild the request.",
        ),
        diagnostic(
            "change_precondition_namespace_present",
            DiagnosticClass::Semantic,
            "A namespace expected to be free points to a live owner.",
            "Inspect the exact namespace owner and choose another name or base.",
        ),
        diagnostic(
            "change_precondition_namespace_owner",
            DiagnosticClass::Semantic,
            "A namespace no longer points to the expected exact owner.",
            "Refresh the namespace and exact owner at the observed base.",
        ),
        diagnostic(
            "change_precondition_namespace_witness",
            DiagnosticClass::Corrupt,
            "A derived namespace entry disagrees with its canonical owner record.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_precondition_dependency_binding",
            DiagnosticClass::Semantic,
            "A package dependency no longer has the expected semantic and package revisions.",
            "Inspect the exact dependency binding at the observed base and rebuild the request.",
        ),
        diagnostic(
            "change_delete_policy",
            DiagnosticClass::Source,
            "A compact deletion requests an unsupported policy.",
            "Use exactly policy=reject or policy=owned-closure.",
        ),
        diagnostic(
            "change_delete_owned_children",
            DiagnosticClass::Semantic,
            "Reject deletion selected an owner with owned identities.",
            "Delete a leaf owner with reject or review the exact policy=owned-closure plan.",
        ),
        diagnostic(
            "change_delete_live_reference",
            DiagnosticClass::Semantic,
            "A live semantic relation still targets the selected owner.",
            "Repair every exact consumer in the same request or retain the owner.",
        ),
        diagnostic(
            "change_delete_created_owner",
            DiagnosticClass::Semantic,
            "One request creates and deletes the same owner.",
            "Remove both operations instead of publishing an identity-only no-op.",
        ),
        diagnostic(
            "change_delete_duplicate",
            DiagnosticClass::Semantic,
            "One request selects the same owner for deletion more than once.",
            "Retain one exact delete operation.",
        ),
        diagnostic(
            "change_delete_expression_parent",
            DiagnosticClass::Semantic,
            "A binding or expression was selected as an independent deletion root.",
            "Replace its exact owning body or parent semantic field.",
        ),
        diagnostic(
            "change_delete_parent_membership",
            DiagnosticClass::Semantic,
            "A selected member is not attached exactly once to its expected parent.",
            "Refresh exact owner context and re-plan against the observed base.",
        ),
        diagnostic(
            "change_delete_parent_kind",
            DiagnosticClass::Semantic,
            "A selected member is incompatible with its recorded parent kind.",
            "Refresh exact owner context and select the correct semantic parent.",
        ),
        diagnostic(
            "change_delete_owner_cache",
            DiagnosticClass::Corrupt,
            "Deletion preparation lost a selected canonical owner.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_delete_ownership_endpoint",
            DiagnosticClass::Corrupt,
            "An ownership relation has an invalid endpoint domain.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_delete_ownership_missing",
            DiagnosticClass::Corrupt,
            "An accepted owner has no exact ownership witness.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_delete_ownership_disagreement",
            DiagnosticClass::Corrupt,
            "An ownership witness is not reproduced by accepted canonical meaning.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_delete_ownership_package",
            DiagnosticClass::Corrupt,
            "An ownership relation crosses a foreign package boundary.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_delete_relation_disagreement",
            DiagnosticClass::Corrupt,
            "An incoming relation witness is not reproduced by accepted canonical meaning.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_delete_revision",
            DiagnosticClass::Corrupt,
            "Deletion preparation has no exact accepted base revision.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_plan_domain",
            DiagnosticClass::Source,
            "A reviewed plan token has the wrong typed prefix.",
            "Use the exact plan_ token returned by change plan.",
        ),
        diagnostic(
            "change_plan_length",
            DiagnosticClass::Source,
            "A reviewed plan token has the wrong two-component length.",
            "Use the complete 128-hex-character plan_ token returned by change plan.",
        ),
        diagnostic(
            "change_request_commitment_field_length",
            DiagnosticClass::Resource,
            "A normalized request field exceeds its commitment length domain.",
            "Reduce the request within the advertised compact input bounds.",
        ),
        diagnostic(
            "change_plan_hex",
            DiagnosticClass::Source,
            "A reviewed plan token has noncanonical hexadecimal bytes.",
            "Use the lowercase plan_ token returned by change plan.",
        ),
        diagnostic(
            "change_request_commitment_mismatch",
            DiagnosticClass::Semantic,
            "The reviewed request commitment differs from normalized input.",
            "Re-run change plan for the exact input before project discovery.",
        ),
        diagnostic(
            "change_prepared_plan_mismatch",
            DiagnosticClass::Semantic,
            "The reviewed prepared-plan commitment differs from the reprepared logical plan.",
            "Re-run change plan against the current exact base and review the complete logical plan.",
        ),
        diagnostic(
            "change_plan_output_project_path",
            DiagnosticClass::Source,
            "A logical plan output target is at or below normalized project authority.",
            "Choose an explicit review-file path outside the normalized project root.",
        ),
        diagnostic(
            "change_plan_output_type",
            DiagnosticClass::Source,
            "A logical plan output target is a symlink or is not a regular file.",
            "Choose an absent path or an existing ordinary regular file.",
        ),
        diagnostic(
            "change_plan_output_existing_bytes",
            DiagnosticClass::Resource,
            "An existing logical plan output exceeds bounded comparison admission.",
            "Remove the unrelated oversized file or choose another explicit output path.",
        ),
        diagnostic(
            "change_plan_output_byte_budget",
            DiagnosticClass::Resource,
            "A complete logical plan exceeds its independent file-byte admission.",
            "Reduce the semantic change; logical plan output is never truncated.",
        ),
        diagnostic(
            "change_plan_output_record_budget",
            DiagnosticClass::Resource,
            "A complete logical plan exceeds its independent file-record admission.",
            "Reduce the semantic change; logical plan output is never truncated.",
        ),
        diagnostic(
            "change_plan_output_parent",
            DiagnosticClass::Infrastructure,
            "The logical plan output parent cannot be resolved.",
            "Create and authorize the parent directory, then retry planning.",
        ),
        diagnostic(
            "change_plan_output_project",
            DiagnosticClass::Infrastructure,
            "The normalized project root cannot be resolved for output isolation.",
            "Preserve the project and repair its path before retrying.",
        ),
        diagnostic(
            "change_plan_output_metadata",
            DiagnosticClass::Infrastructure,
            "Logical plan output target metadata cannot be inspected safely.",
            "Repair target permissions or choose another path.",
        ),
        diagnostic(
            "change_plan_output_create",
            DiagnosticClass::Infrastructure,
            "A unique private logical plan output stage cannot be created.",
            "Repair parent permissions and retry; accepted authority was not changed.",
        ),
        diagnostic(
            "change_plan_output_write",
            DiagnosticClass::Infrastructure,
            "Canonical logical plan records cannot be written to the private stage.",
            "Repair filesystem capacity or permissions and retry planning.",
        ),
        diagnostic(
            "change_plan_output_sync",
            DiagnosticClass::Infrastructure,
            "The complete private logical plan stage cannot be synchronized.",
            "Repair the filesystem and retry; the target was not published.",
        ),
        diagnostic(
            "change_plan_output_stage_remove",
            DiagnosticClass::Infrastructure,
            "An equal private logical plan stage cannot be removed.",
            "Inspect the reported sibling stage and parent permissions before retrying.",
        ),
        diagnostic(
            "change_plan_output_publish",
            DiagnosticClass::Infrastructure,
            "The synchronized logical plan stage cannot be atomically renamed.",
            "Repair the target filesystem boundary and retry planning.",
        ),
        diagnostic(
            "change_plan_output_parent_sync",
            DiagnosticClass::Infrastructure,
            "The logical plan output parent cannot be synchronized after publication.",
            "Inspect the complete target file and filesystem durability before retrying.",
        ),
        diagnostic(
            "change_plan_output_compare_metadata",
            DiagnosticClass::Infrastructure,
            "Logical plan stage or target metadata changed during bounded comparison.",
            "Remove the external race and retry planning.",
        ),
        diagnostic(
            "change_plan_output_compare_open",
            DiagnosticClass::Infrastructure,
            "A logical plan stage or target cannot be opened for bounded comparison.",
            "Repair target permissions and retry planning.",
        ),
        diagnostic(
            "change_plan_output_compare_read",
            DiagnosticClass::Infrastructure,
            "A logical plan stage or target cannot be read during bounded comparison.",
            "Repair the filesystem and retry planning.",
        ),
        diagnostic(
            "change_reviewed_plan_missing",
            DiagnosticClass::Infrastructure,
            "Validated apply dispatch lost its reviewed plan token.",
            "Retain the failing request and use a verified executable.",
        ),
        diagnostic(
            "change_logical_plan_authority",
            DiagnosticClass::Corrupt,
            "Prepared authority, semantic diff, receipt, and candidate identities disagree.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_value_sets",
            DiagnosticClass::Corrupt,
            "Logical dependency or retirement values disagree with the prepared semantic diff.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_validation_counts",
            DiagnosticClass::Corrupt,
            "Logical validation or test membership disagrees with receipt counts.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_base",
            DiagnosticClass::Corrupt,
            "Prepared authored change does not bind one exact base.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_diff",
            DiagnosticClass::Corrupt,
            "Prepared authored change does not contain a current semantic change diff.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_allocations",
            DiagnosticClass::Corrupt,
            "Logical allocations are duplicated, foreign, or noncanonical.",
            "Retain the request and use a verified executable.",
        ),
        diagnostic(
            "change_logical_plan_reasons",
            DiagnosticClass::Corrupt,
            "Logical impact reasons are duplicated or noncanonical.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_relations",
            DiagnosticClass::Corrupt,
            "A logical semantic relation is both removed and added.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_order",
            DiagnosticClass::Corrupt,
            "Logical owner or relation facts do not use canonical contract order.",
            "Retain the request and use a verified executable.",
        ),
        diagnostic(
            "change_logical_plan_dependency",
            DiagnosticClass::Corrupt,
            "A logical dependency value disagrees with its exact object identity.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_dependency_before",
            DiagnosticClass::Corrupt,
            "Authored lowering retained an unrelated dependency base value.",
            "Retain the request and use a verified executable.",
        ),
        diagnostic(
            "change_logical_plan_retirement",
            DiagnosticClass::Corrupt,
            "A logical retirement value disagrees with its exact object identity.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_retirement_before",
            DiagnosticClass::Corrupt,
            "Authored preparation lacks a required logical retirement base value.",
            "Retain the request and use a verified executable.",
        ),
        diagnostic(
            "change_logical_plan_validation",
            DiagnosticClass::Corrupt,
            "Prepared validation membership disagrees with its impact plan.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_logical_plan_tests",
            DiagnosticClass::Corrupt,
            "Prepared selected tests disagree with validation evidence.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_plan_file_read",
            DiagnosticClass::Source,
            "A logical plan file stream cannot be read completely.",
            "Supply a readable complete canonical plan file.",
        ),
        diagnostic(
            "change_plan_file_truncated",
            DiagnosticClass::Source,
            "A logical plan file ends inside a physical record.",
            "Regenerate or recopy the complete canonical plan file.",
        ),
        diagnostic(
            "change_plan_file_record_bytes",
            DiagnosticClass::Resource,
            "One logical plan record exceeds the compact-record byte bound.",
            "Reject the file and regenerate it with the current executable.",
        ),
        diagnostic(
            "change_plan_file_byte_budget",
            DiagnosticClass::Resource,
            "A logical plan file exceeds its complete decoder byte admission.",
            "Reject the file; logical plan files are never truncated.",
        ),
        diagnostic(
            "change_plan_file_record_budget",
            DiagnosticClass::Resource,
            "A logical plan file exceeds its complete decoder record admission.",
            "Reject the file; logical plan files are never truncated.",
        ),
        diagnostic(
            "change_plan_file_record_unknown",
            DiagnosticClass::Source,
            "A logical plan file contains an unknown or operational record.",
            "Regenerate the file using the current executable.",
        ),
        diagnostic(
            "change_plan_file_fields",
            DiagnosticClass::Source,
            "A logical plan record has unknown, missing, duplicate, or misordered fields.",
            "Regenerate the file using the current executable.",
        ),
        diagnostic(
            "change_plan_file_order",
            DiagnosticClass::Source,
            "Logical plan records are not in canonical order.",
            "Regenerate the file; reordering review facts is noncanonical.",
        ),
        diagnostic(
            "change_plan_file_singleton_duplicate",
            DiagnosticClass::Source,
            "A singleton logical plan record is duplicated.",
            "Remove the duplicate by regenerating the canonical plan file.",
        ),
        diagnostic(
            "change_plan_file_canonical",
            DiagnosticClass::Source,
            "A logical plan record is not canonically escaped or framed.",
            "Regenerate the plan file rather than editing its compact bytes.",
        ),
        diagnostic(
            "change_plan_file_product",
            DiagnosticClass::Source,
            "A logical plan file names a predecessor or foreign product.",
            "Regenerate the plan with the current executable.",
        ),
        diagnostic(
            "change_plan_file_capabilities",
            DiagnosticClass::Source,
            "A logical plan file binds a predecessor or foreign capability projection.",
            "Regenerate the plan with the current executable.",
        ),
        diagnostic(
            "change_plan_file_counts",
            DiagnosticClass::Source,
            "Logical plan summary counts disagree with exact preceding records.",
            "Reject the file and regenerate it from the authored request.",
        ),
        diagnostic(
            "change_plan_file_digest",
            DiagnosticClass::Source,
            "Logical plan body, commitments, and token disagree.",
            "Reject the mutated file and regenerate it from the authored request.",
        ),
        diagnostic(
            "change_plan_file_trailing",
            DiagnosticClass::Source,
            "A logical plan file contains records after its digest trailer.",
            "Reject the file and retain only the complete canonical plan through its trailer.",
        ),
        diagnostic(
            "change_plan_file_owner_order",
            DiagnosticClass::Source,
            "Logical owner facts are duplicated or out of canonical typed order.",
            "Regenerate the plan file without editing exact owner records.",
        ),
        diagnostic(
            "change_plan_file_relation_order",
            DiagnosticClass::Source,
            "Logical relation facts are duplicated or out of canonical order.",
            "Regenerate the plan file without editing exact relation records.",
        ),
        diagnostic(
            "change_plan_file_owner_consistency",
            DiagnosticClass::Source,
            "Logical owner classes or dimensions disagree with the exact binding change.",
            "Reject the file and regenerate it from the prepared candidate.",
        ),
        diagnostic(
            "change_plan_file_dependency_object",
            DiagnosticClass::Source,
            "A logical dependency value disagrees with its object identity.",
            "Reject the file and regenerate it from the prepared candidate.",
        ),
        diagnostic(
            "change_plan_file_retirement_object",
            DiagnosticClass::Source,
            "A logical retirement value disagrees with its object identity.",
            "Reject the file and regenerate it from the prepared candidate.",
        ),
        diagnostic(
            "change_authored_stale_base",
            DiagnosticClass::Semantic,
            "The request base is not the currently observed accepted revision.",
            "Refresh status and rebuild the request against the observed revision.",
        ),
        diagnostic(
            "change_authored_allocation_records",
            DiagnosticClass::Resource,
            "Logical allocation-record reservation failed within declared admission.",
            "Reduce allocated identities or available memory and retry planning.",
        ),
        diagnostic(
            "change_authored_allocation_projection",
            DiagnosticClass::Corrupt,
            "Authored lowering lost a normalized request-local allocation.",
            "Retain the request and use a verified executable.",
        ),
        diagnostic(
            "change_authored_dependency_before",
            DiagnosticClass::Corrupt,
            "Authored dependency lowering lost its exact base binding.",
            "Preserve the repository and report the exact diagnostic; do not retry a write.",
        ),
        diagnostic(
            "change_stale_base",
            DiagnosticClass::Semantic,
            "HEAD changed after preparation and before publication.",
            "Refresh status, re-plan the request, and review its new plan token.",
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
        diagnostic(
            "kernel_full_target_port_owner",
            DiagnosticClass::Semantic,
            "A target selects a port that is not owned by its exact component.",
            "Select a port added to the target component or correct the component reference.",
        ),
        diagnostic(
            "kernel_type_target_http_runner",
            DiagnosticClass::Semantic,
            "An HTTP target port does not have the exact semantic HTTP function shape.",
            "Use the discovered structural HTTP request/response function type and implementation.",
        ),
        diagnostic(
            "kernel_type_target_command_runner",
            DiagnosticClass::Semantic,
            "A command target selects the semantic HTTP port shape.",
            "Use runner=http or select a pure command-shaped port.",
        ),
        diagnostic(
            "kernel_type_task_requirement",
            DiagnosticClass::Semantic,
            "A task function requires a capability absent from its component closure.",
            "Add a matching component requirement with every operation used by the function.",
        ),
        diagnostic(
            "kernel_affine_resource_parameter_use",
            DiagnosticClass::Semantic,
            "A direct capability-resource operation parameter is not explicitly borrowed or consumed.",
            "Set use=borrow for observation or use=consume for an ownership-ending transition.",
        ),
        diagnostic(
            "kernel_affine_nonresource_parameter_use",
            DiagnosticClass::Semantic,
            "A nonresource parameter declares affine borrow or consume behavior.",
            "Use unrestricted for ordinary values or change the parameter to one direct capability-resource type.",
        ),
        diagnostic(
            "kernel_affine_resource_container",
            DiagnosticClass::Semantic,
            "A capability resource appears in a forbidden record, collection, stream, result, option, or function container.",
            "Keep it direct or in one nominal variant case.",
        ),
        diagnostic(
            "kernel_affine_function_parameter",
            DiagnosticClass::Semantic,
            "A function parameter contains a capability resource.",
            "Keep resources task-local; resource transfer across function boundaries is not supported.",
        ),
        diagnostic(
            "kernel_affine_function_result",
            DiagnosticClass::Semantic,
            "A function result contains a capability resource.",
            "Consume or drop the resource within its acquiring task instead of returning it.",
        ),
        diagnostic(
            "kernel_affine_record_field",
            DiagnosticClass::Semantic,
            "A record field contains a capability resource.",
            "Keep the resource direct or place it in the single live case of a nominal variant.",
        ),
        diagnostic(
            "kernel_affine_variant_payload",
            DiagnosticClass::Semantic,
            "A nominal variant resource payload is indirect or otherwise malformed.",
            "Use at most one case whose payload is the direct exact-interface capability-resource type.",
        ),
        diagnostic(
            "kernel_affine_resource_fabricated",
            DiagnosticClass::Semantic,
            "A resource use does not name a live owner acquired from an exact requirement call.",
            "Use the lexical binding produced by the acquiring capability operation.",
        ),
        diagnostic(
            "kernel_affine_resource_copy",
            DiagnosticClass::Semantic,
            "A live capability-resource binding is used as an ordinary copyable value.",
            "Pass it only to an explicit borrow or consume parameter, or transfer it through the supported variant match.",
        ),
        diagnostic(
            "kernel_affine_use_after_consume",
            DiagnosticClass::Semantic,
            "A capability-resource binding is used after an earlier consume transition.",
            "Remove the later use or bind the fresh resource returned by a renewing operation.",
        ),
        diagnostic(
            "kernel_affine_foreign_requirement",
            DiagnosticClass::Semantic,
            "A resource is used through a requirement different from the exact requirement that acquired it.",
            "Use the acquiring requirement for every borrow and consume transition.",
        ),
        diagnostic(
            "kernel_affine_foreign_authority",
            DiagnosticClass::Semantic,
            "A resource argument disagrees with the exact operation requirement or interface.",
            "Use a resource acquired from the same exact requirement and interface.",
        ),
        diagnostic(
            "kernel_affine_branch_join",
            DiagnosticClass::Semantic,
            "Reachable branches do not preserve identical live ownership and provenance.",
            "Consume the resource on every branch or preserve the same owner on every branch.",
        ),
        diagnostic(
            "normalized_resource_limit",
            DiagnosticClass::Resource,
            "A task cannot reserve another live runtime resource before an external effect.",
            "Reduce concurrent live handles or raise the deployment-owned task resource limit where supported.",
        ),
        diagnostic(
            "normalized_resource_foreign_scope",
            DiagnosticClass::Capability,
            "A runtime resource handle belongs to another task scope.",
            "Use only handles acquired by the current task invocation.",
        ),
        diagnostic(
            "normalized_resource_kind",
            DiagnosticClass::Capability,
            "A runtime resource handle has the wrong closed resource kind.",
            "Preserve the exact typed handle produced by the acquiring operation.",
        ),
        diagnostic(
            "normalized_resource_authority",
            DiagnosticClass::Capability,
            "A runtime resource handle belongs to another exact capability requirement.",
            "Route the handle through the requirement that acquired it.",
        ),
        diagnostic(
            "normalized_resource_interface",
            DiagnosticClass::Capability,
            "A runtime resource handle belongs to another exact capability interface.",
            "Preserve the exact interface-bound handle produced by acquisition.",
        ),
        diagnostic(
            "normalized_resource_closed",
            DiagnosticClass::Capability,
            "A runtime resource handle is absent, uncommitted, or already consumed.",
            "Do not retry the closed local right; reacquire through a new capability operation when appropriate.",
        ),
        diagnostic(
            "normalized_resource_reservation_lost",
            DiagnosticClass::Infrastructure,
            "A pre-effect queue-lease reservation disappeared before successor authority installation.",
            "Preserve the candidate and runtime evidence and use a verified executable.",
        ),
        diagnostic(
            "normalized_queue_signature",
            DiagnosticClass::Capability,
            "A durable-queue artifact operation does not have the exact current resource signature.",
            "Rebuild from the current standard package and reject predecessor queue interfaces.",
        ),
        diagnostic(
            "normalized_queue_lease_state_case",
            DiagnosticClass::Capability,
            "QueueLeaseState does not contain exactly the supported absent and live resource cases.",
            "Use the exact current built-in DurableQueue package interface.",
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
            meaning: "Executable capability records are not newline-complete.",
            retry: "Use a verified executable build whose capabilities pass conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_response_records_blank",
            class: DiagnosticClass::Infrastructure,
            meaning: "Executable capability material contains a blank physical record.",
            retry: "Use a verified executable build whose capabilities pass conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_response_records_invalid",
            class: DiagnosticClass::Infrastructure,
            meaning: "Executable capability material contains an invalid compact record.",
            retry: "Use a verified executable build whose capabilities pass conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_render_operation",
            class: DiagnosticClass::Infrastructure,
            meaning: "A compact response producer supplied an invalid operation name.",
            retry: "Use a verified executable build whose capabilities pass conformance checks.",
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
            retry: "Use a verified executable build whose capabilities pass conformance checks.",
        },
        DiagnosticDescriptor {
            code: "control_render_duplicate_field",
            class: DiagnosticClass::Infrastructure,
            meaning: "A compact response producer supplied a duplicate field name.",
            retry: "Use a verified executable build whose capabilities pass conformance checks.",
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
            code: "capabilities_projection_invalid",
            class: DiagnosticClass::Corrupt,
            meaning: "The executable could not construct its complete public capability projection.",
            retry: "Use a verified executable build.",
        },
        DiagnosticDescriptor {
            code: "capabilities_output_budget",
            class: DiagnosticClass::Resource,
            meaning: "The complete public capability projection exceeds its output bound.",
            retry: "Request one advertised capability section.",
        },
        DiagnosticDescriptor {
            code: "capabilities_generated_drift",
            class: DiagnosticClass::Source,
            meaning: "Checked-in generated public-guide bytes differ from executable truth.",
            retry: "Regenerate with the exact command reported by the diagnostic.",
        },
        DiagnosticDescriptor {
            code: "predecessor_contract",
            class: DiagnosticClass::Source,
            meaning: "Input uses a predecessor contract rejected by direct cutover.",
            retry: "Recreate the request or authority under the advertised current contract.",
        },
        diagnostic(
            "query_usage",
            DiagnosticClass::Source,
            "The normalized query action or positional grammar is incomplete.",
            "Use the exact grammar reported by capabilities query.",
        ),
        diagnostic(
            "query_unknown_action",
            DiagnosticClass::Source,
            "The query action is not one of owners, find, relations, or context.",
            "Select one action reported by capabilities query.",
        ),
        diagnostic(
            "query_unknown_option",
            DiagnosticClass::Source,
            "The selected query action does not accept this option.",
            "Use only options reported for that action by capabilities query.",
        ),
        diagnostic(
            "query_duplicate_option",
            DiagnosticClass::Source,
            "A finite query option was supplied more than once.",
            "Supply the option exactly once.",
        ),
        diagnostic(
            "query_invalid_owner_kind",
            DiagnosticClass::Source,
            "An owner-kind filter is outside the canonical owner-kind inventory.",
            "Select one query.owner-kind value from capabilities query.",
        ),
        diagnostic(
            "query_invalid_namespace_class",
            DiagnosticClass::Source,
            "An exact lookup class is outside the canonical namespace-class inventory.",
            "Select one query.namespace-class value from capabilities query.",
        ),
        diagnostic(
            "query_invalid_relation_kind",
            DiagnosticClass::Source,
            "A relation-kind filter is outside the canonical relation-kind inventory.",
            "Select one query.relation-kind value from capabilities query.",
        ),
        diagnostic(
            "query_invalid_direction",
            DiagnosticClass::Source,
            "A relation direction is not incoming or outgoing.",
            "Supply one exact query.direction value.",
        ),
        diagnostic(
            "query_invalid_context_direction",
            DiagnosticClass::Source,
            "A context direction is not incoming, outgoing, or both.",
            "Supply one exact query.context-direction value.",
        ),
        diagnostic(
            "query_missing_context_direction",
            DiagnosticClass::Source,
            "A context query omitted its required direction.",
            "Supply --direction incoming, outgoing, or both.",
        ),
        diagnostic(
            "query_missing_context_depth",
            DiagnosticClass::Source,
            "A context query omitted its required traversal depth.",
            "Supply --depth within the advertised context depth bounds.",
        ),
        diagnostic(
            "query_invalid_context_depth",
            DiagnosticClass::Source,
            "A context depth is noncanonical or outside its fixed public range.",
            "Supply a canonical integer within the reported context depth bounds.",
        ),
        diagnostic(
            "query_invalid_owner_identity",
            DiagnosticClass::Source,
            "A query owner or parent has a malformed or foreign typed identity domain.",
            "Use an exact owner identity returned by current query or change output.",
        ),
        diagnostic(
            "query_parent_required",
            DiagnosticClass::Source,
            "A child namespace class has no exact parent selector.",
            "Supply --parent with one live exact local owner.",
        ),
        diagnostic(
            "query_parent_forbidden",
            DiagnosticClass::Source,
            "A package-root namespace class was supplied a parent.",
            "Remove --parent for module or target lookup.",
        ),
        diagnostic(
            "query_invalid_parent_domain",
            DiagnosticClass::Source,
            "The typed parent domain cannot own the requested namespace class.",
            "Use a parent identity from the canonical class-parent inventory.",
        ),
        diagnostic(
            "query_invalid_limit",
            DiagnosticClass::Source,
            "The query item limit is noncanonical or outside its public bound.",
            "Supply a canonical integer within the reported query_items limit.",
        ),
        diagnostic(
            "query_invalid_byte_limit",
            DiagnosticClass::Source,
            "The query output-byte limit is noncanonical or outside its public bound.",
            "Supply a canonical integer within the reported query_bytes limit.",
        ),
        diagnostic(
            "query_owner_not_found",
            DiagnosticClass::Semantic,
            "The exact local relation endpoint or context root is not live at the observed revision.",
            "Refresh owner discovery at current HEAD and retry.",
        ),
        diagnostic(
            "query_parent_not_found",
            DiagnosticClass::Semantic,
            "The exact namespace parent is not live at the observed revision.",
            "Refresh the parent identity and retry exact lookup.",
        ),
        diagnostic(
            "query_continuation_oversized",
            DiagnosticClass::Source,
            "A query continuation exceeds its strict textual or decoded byte bound.",
            "Restart the query without the foreign token.",
        ),
        diagnostic(
            "query_continuation_malformed",
            DiagnosticClass::Source,
            "A query continuation is truncated or malformed.",
            "Restart the query and use the exact emitted token.",
        ),
        diagnostic(
            "query_continuation_noncanonical",
            DiagnosticClass::Source,
            "A query continuation is not canonical unpadded URL-safe base64.",
            "Use the exact continuation text emitted by the executable.",
        ),
        diagnostic(
            "query_continuation_integrity",
            DiagnosticClass::Source,
            "A query continuation integrity digest does not match its payload.",
            "Restart the query without modifying the emitted token.",
        ),
        diagnostic(
            "query_continuation_foreign",
            DiagnosticClass::Source,
            "A query continuation belongs to another repository or package.",
            "Restart pagination in the selected project.",
        ),
        diagnostic(
            "query_continuation_stale",
            DiagnosticClass::Source,
            "A query continuation is pinned to an accepted revision that is no longer HEAD.",
            "Refresh status and restart the query at current HEAD.",
        ),
        diagnostic(
            "query_continuation_mismatch",
            DiagnosticClass::Source,
            "A query continuation does not bind the normalized selector.",
            "Use the token only with the same operation, endpoint, direction, and filter.",
        ),
        diagnostic(
            "query_output_item_too_large",
            DiagnosticClass::Resource,
            "One compact owner, relation, or context record cannot fit the selected output bound.",
            "Increase --bytes within the reported maximum.",
        ),
        diagnostic(
            "query_admission_exhausted",
            DiagnosticClass::Resource,
            "One dimension of bounded normalized repository reading was exhausted.",
            "Request a smaller page or preserve the repository for locality inspection.",
        ),
        diagnostic(
            "query_context_owner_limit",
            DiagnosticClass::Resource,
            "A complete context traversal exceeds its unique-local-owner or expanded-owner admission.",
            "Reduce --depth or select a root with a smaller local neighborhood.",
        ),
        diagnostic(
            "query_context_relation_limit",
            DiagnosticClass::Resource,
            "A complete context traversal exceeds its unique selected-relation admission.",
            "Reduce --depth or select a root with a smaller relation neighborhood.",
        ),
        diagnostic(
            "query_context_witness_limit",
            DiagnosticClass::Resource,
            "A complete context traversal exceeds its relation-witness visit admission.",
            "Reduce --depth or select a root with less admitted fanout.",
        ),
        diagnostic(
            "query_context_depth_overflow",
            DiagnosticClass::Resource,
            "Internal context traversal depth accounting overflowed.",
            "Preserve the request and executable for resource-accounting inspection.",
        ),
        diagnostic(
            "query_namespace_owner_disagreement",
            DiagnosticClass::Corrupt,
            "Committed namespace witness and canonical owner facts disagree.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_context_owner_missing",
            DiagnosticClass::Corrupt,
            "A selected local relation endpoint is absent from canonical owner authority.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_context_owner_binding",
            DiagnosticClass::Corrupt,
            "A context owner key disagrees with its canonical owner record.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_relation_prefix_binding",
            DiagnosticClass::Corrupt,
            "A committed relation key disagrees with its selected endpoint or kind prefix.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_option_value",
            DiagnosticClass::Source,
            "A query option is missing its required value.",
            "Supply one value after the option using the focused query grammar.",
        ),
        diagnostic(
            "query_unexpected_argument",
            DiagnosticClass::Source,
            "A positional token appears outside the selected finite query grammar.",
            "Remove the token or place it in the advertised positional slot.",
        ),
        diagnostic(
            "query_invalid_name",
            DiagnosticClass::Source,
            "An exact namespace name violates the canonical Name contract.",
            "Supply one canonical case-sensitive semantic name.",
        ),
        diagnostic(
            "query_missing_direction",
            DiagnosticClass::Source,
            "A relation query omitted its required direction.",
            "Supply --direction incoming or --direction outgoing.",
        ),
        diagnostic(
            "query_continuation_contract",
            DiagnosticClass::Source,
            "A continuation has a foreign magic, envelope, continuation, or query version.",
            "Restart pagination using a token emitted by the current executable.",
        ),
        diagnostic(
            "query_continuation_operation",
            DiagnosticClass::Source,
            "A continuation contains an unknown normalized query operation tag.",
            "Restart pagination using the exact emitted token.",
        ),
        diagnostic(
            "query_continuation_reserved_identity",
            DiagnosticClass::Source,
            "A continuation contains a reserved all-zero authority identity.",
            "Discard the token and restart pagination.",
        ),
        diagnostic(
            "query_continuation_resume_key",
            DiagnosticClass::Source,
            "A continuation contains a malformed or selector-inconsistent logical resume key.",
            "Restart pagination using the exact token emitted for this selector.",
        ),
        diagnostic(
            "query_continuation_trailing",
            DiagnosticClass::Source,
            "A continuation payload contains noncanonical trailing bytes.",
            "Discard the token and restart pagination.",
        ),
        diagnostic(
            "query_continuation_length",
            DiagnosticClass::Resource,
            "A canonical continuation length cannot be represented within its fixed contract.",
            "Preserve the request and executable for contract inspection.",
        ),
        diagnostic(
            "query_output_envelope_too_large",
            DiagnosticClass::Resource,
            "The selected output bound cannot hold the fixed revision-pinned response envelope.",
            "Increase --bytes within the advertised query bounds.",
        ),
        diagnostic(
            "query_output_byte_overflow",
            DiagnosticClass::Resource,
            "Query result-byte accounting overflowed its numeric domain.",
            "Preserve the repository and request for resource-boundary inspection.",
        ),
        diagnostic(
            "query_admission_map_pages",
            DiagnosticClass::Resource,
            "Normalized query exhausted its persistent-map page-read admission.",
            "Request a smaller page or inspect unexpected map locality.",
        ),
        diagnostic(
            "query_admission_map_bytes",
            DiagnosticClass::Resource,
            "Normalized query exhausted its persistent-map byte-read admission.",
            "Request a smaller page or inspect unexpectedly large map pages.",
        ),
        diagnostic(
            "query_admission_map_entries",
            DiagnosticClass::Resource,
            "Normalized query exhausted or overflowed its map-entry visit admission.",
            "Request a smaller page or inspect unexpected logical scan work.",
        ),
        diagnostic(
            "query_admission_catalog_lookups",
            DiagnosticClass::Resource,
            "Normalized query exhausted its object-catalog lookup admission.",
            "Request a smaller page or inspect unexpected object locality.",
        ),
        diagnostic(
            "query_admission_store_objects",
            DiagnosticClass::Resource,
            "Normalized query exhausted its object-read admission.",
            "Request a smaller page or inspect unexpected canonical object reads.",
        ),
        diagnostic(
            "query_admission_store_bytes",
            DiagnosticClass::Resource,
            "Normalized query exhausted its immutable-store byte admission.",
            "Request a smaller page or inspect unexpectedly large canonical objects.",
        ),
        diagnostic(
            "query_admission_canonical_records",
            DiagnosticClass::Resource,
            "Normalized query exhausted its canonical-record decode admission.",
            "Request a smaller page or inspect unexpected canonical decoding.",
        ),
        diagnostic(
            "query_admission_witness_records",
            DiagnosticClass::Resource,
            "Normalized query exhausted its witness-record decode admission.",
            "Request a smaller page or inspect unexpected witness decoding.",
        ),
        diagnostic(
            "query_required_map_page_missing",
            DiagnosticClass::Corrupt,
            "A required canonical or witness map page is absent from immutable storage.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_required_object_missing",
            DiagnosticClass::Corrupt,
            "A required accepted object is absent from immutable storage.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_namespace_owner_missing",
            DiagnosticClass::Corrupt,
            "A namespace witness references an owner absent from canonical authority.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_owner_object_missing",
            DiagnosticClass::Corrupt,
            "A canonical owner binding references a missing immutable owner object.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_owner_range_count",
            DiagnosticClass::Corrupt,
            "Owner range traversal and logical query accounting disagree.",
            "Preserve the repository and executable for traversal inspection.",
        ),
        diagnostic(
            "query_parent_owner_disagreement",
            DiagnosticClass::Corrupt,
            "A live namespace parent record disagrees with its canonical owner key.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_relation_range_count",
            DiagnosticClass::Corrupt,
            "Relation range traversal and logical query accounting disagree.",
            "Preserve the repository and executable for traversal inspection.",
        ),
        diagnostic(
            "query_relation_value",
            DiagnosticClass::Corrupt,
            "A committed relation witness has a nonempty noncanonical value.",
            "Preserve the repository and run deep authority verification.",
        ),
        diagnostic(
            "query_revision_binding",
            DiagnosticClass::Corrupt,
            "A query read failed to retain its pinned accepted revision.",
            "Preserve the repository and executable for authority inspection.",
        ),
        diagnostic(
            "query_admission_logical_range",
            DiagnosticClass::Resource,
            "An internal logical range was given an invalid item admission.",
            "Preserve the request and executable for query-boundary inspection.",
        ),
        diagnostic(
            "query_descriptor_action",
            DiagnosticClass::Infrastructure,
            "The executable query descriptor inventory contains an unimplemented action.",
            "Use a matching executable and generated capability guide.",
        ),
        diagnostic(
            "query_output_record_configuration",
            DiagnosticClass::Infrastructure,
            "The fixed query envelope exceeds the global compact record capacity.",
            "Use a matching executable and capability projection.",
        ),
        diagnostic(
            "query_output_size_convergence",
            DiagnosticClass::Infrastructure,
            "Exact rendered-output byte reporting failed to reach its bounded fixed point.",
            "Preserve the request and executable for renderer inspection.",
        ),
        diagnostic(
            "query_record_limit",
            DiagnosticClass::Resource,
            "The compact response record capacity cannot hold a logical query item.",
            "Reduce the requested page or inspect the global compact response limit.",
        ),
        diagnostic(
            "query_response_field_inventory",
            DiagnosticClass::Infrastructure,
            "The query renderer attempted a field absent from its executable inventory.",
            "Use a matching executable and capability projection.",
        ),
        diagnostic(
            "query_scan_quantum",
            DiagnosticClass::Resource,
            "The derived owner scan quantum overflowed its numeric domain.",
            "Preserve the request and executable for admission inspection.",
        ),
        diagnostic(
            "query_selector_length",
            DiagnosticClass::Resource,
            "A normalized query selector exceeds its canonical length encoding.",
            "Use a canonical bounded semantic name and selector.",
        ),
        diagnostic(
            "query_visitor_diagnostic_missing",
            DiagnosticClass::Corrupt,
            "A bounded map visitor aborted without its owning typed diagnostic.",
            "Preserve the repository and executable for traversal inspection.",
        ),
        diagnostic(
            "query_work_overflow",
            DiagnosticClass::Resource,
            "One exact normalized query work counter overflowed.",
            "Preserve the repository and request for resource-accounting inspection.",
        ),
        diagnostic(
            "query_cancelled",
            DiagnosticClass::Cancelled,
            "A context query was cancelled by its owning execution scope.",
            "Retry the read-only query when the owning scope is active.",
        ),
        diagnostic(
            "http_client_contract",
            DiagnosticClass::Capability,
            "Outbound HTTP client limits use a predecessor or foreign adapter contract.",
            "Use the exact current deployment schema and regenerate the descriptor.",
        ),
        diagnostic(
            "http_client_endpoint",
            DiagnosticClass::Capability,
            "The deployment-bound HTTP endpoint is malformed, noncanonical, or incompatible with its address policy.",
            "Select one exact canonical http/https endpoint using the documented destination policy.",
        ),
        diagnostic(
            "http_client_limit",
            DiagnosticClass::Capability,
            "One outbound HTTP client deployment limit is zero, excessive, or internally inconsistent.",
            "Select positive values within the executable-generated independent maxima.",
        ),
        diagnostic(
            "http_client_trust",
            DiagnosticClass::Capability,
            "Outbound HTTPS trust material is absent, malformed, empty, excessive, or unsupported.",
            "Use pinned public WebPKI roots or bind one valid bounded named PEM root secret.",
        ),
        diagnostic(
            "http_client_argument",
            DiagnosticClass::Capability,
            "Graph-supplied HttpClient.get arguments do not have the exact ordered header-list shape.",
            "Pass the current built-in operation's exact structural header list.",
        ),
        diagnostic(
            "http_client_request_header_limit",
            DiagnosticClass::Resource,
            "Outbound request header count or bytes exceed the deployment limit.",
            "Reduce application-selected headers or deliberately raise only the relevant deployment limit.",
        ),
        diagnostic(
            "http_client_header",
            DiagnosticClass::Capability,
            "A graph-supplied outbound request header name or value is invalid.",
            "Use a canonical HTTP field name and bounded value without prohibited control bytes.",
        ),
        diagnostic(
            "http_client_header_forbidden",
            DiagnosticClass::Capability,
            "Graph meaning attempted to supply a transport-owned or credential-bearing outbound header.",
            "Remove the header; Host, framing, connection, proxy, authorization, and cookie policy belong to the adapter.",
        ),
        diagnostic(
            "http_client_dns",
            DiagnosticClass::Capability,
            "The exact outbound endpoint could not resolve to an admitted address set.",
            "Correct DNS or the deployment endpoint without broadening graph authority.",
        ),
        diagnostic(
            "http_client_dns_limit",
            DiagnosticClass::Resource,
            "Endpoint resolution returned more addresses than the deployment admits.",
            "Reduce the DNS result set or deliberately raise only the DNS-result limit.",
        ),
        diagnostic(
            "http_client_destination",
            DiagnosticClass::Capability,
            "At least one resolved endpoint address violates the exact public-only or loopback-only policy.",
            "Correct the endpoint or DNS answer; mixed and forbidden address sets fail closed.",
        ),
        diagnostic(
            "http_client_connect",
            DiagnosticClass::Capability,
            "No validated endpoint address accepted a connection within the common establishment bound.",
            "Correct endpoint reachability and retry only at graph-owned policy.",
        ),
        diagnostic(
            "http_client_tls",
            DiagnosticClass::Capability,
            "Outbound HTTPS certificate authentication or handshake failed.",
            "Correct the endpoint hostname, certificate chain, validity, or selected trust root.",
        ),
        diagnostic(
            "http_client_request_write",
            DiagnosticClass::Capability,
            "The outbound GET transport failed after remote visibility became possible.",
            "Treat visibility as possible and retry only under explicit application policy.",
        ),
        diagnostic(
            "http_client_protocol",
            DiagnosticClass::Capability,
            "The endpoint returned malformed, ambiguous, unsupported, or trailing HTTP/1.1 framing.",
            "Correct the endpoint protocol response; no partial response is returned.",
        ),
        diagnostic(
            "http_client_response_header_limit",
            DiagnosticClass::Resource,
            "Outbound response header count or bytes exceed the deployment limit.",
            "Reduce the remote response or deliberately raise only the response-header limit.",
        ),
        diagnostic(
            "http_client_response_body_limit",
            DiagnosticClass::Resource,
            "Outbound response body bytes exceed the deployment limit.",
            "Reduce the remote response or deliberately raise only the response-body limit.",
        ),
        diagnostic(
            "http_client_concurrency_limit",
            DiagnosticClass::Resource,
            "The deployment's outbound concurrent-request admission is exhausted.",
            "Wait for owned requests to finish or deliberately raise only the concurrency limit.",
        ),
        diagnostic(
            "http_client_deadline_overflow",
            DiagnosticClass::Resource,
            "The outbound request deadline cannot be represented by the runtime clock.",
            "Use a smaller valid total-time limit and preserve the exact diagnostic.",
        ),
        diagnostic(
            "http_client_timeout",
            DiagnosticClass::Cancelled,
            "The outbound request exceeded its connection, deployment-total, or owning task deadline.",
            "Correct endpoint latency or deliberately change the owning deployment deadline; no automatic retry occurs.",
        ),
        diagnostic(
            "http_client_cancelled",
            DiagnosticClass::Cancelled,
            "The owning task, inbound client, or resident shutdown cancelled the outbound request.",
            "Retry only when a new owning task and application policy authorize another observable GET.",
        ),
        diagnostic(
            "http_client_shutdown",
            DiagnosticClass::Cancelled,
            "The outbound HTTP client has stopped admitting new request resources.",
            "Prepare a new deployment after the previous resident runtime has shut down.",
        ),
        diagnostic(
            "http_client_cleanup",
            DiagnosticClass::Infrastructure,
            "Outbound request resources did not close within the configured cleanup bound.",
            "Preserve bounded logs and inspect cancellation or host-network resource behavior.",
        ),
        diagnostic(
            "normalized_http_client_operation_package",
            DiagnosticClass::Capability,
            "The prepared HttpClient operation does not share its interface package identity.",
            "Rebuild the artifact and descriptor against the exact current built-in package.",
        ),
        diagnostic(
            "normalized_http_client_policy",
            DiagnosticClass::Infrastructure,
            "Runtime HttpClient call policy disagrees with the exact prepared interface operation.",
            "Preserve the artifact and use a verified matching executable.",
        ),
        diagnostic(
            "normalized_http_client_shape",
            DiagnosticClass::Infrastructure,
            "Runtime HttpClient result construction disagrees with the exact structural result type.",
            "Preserve the artifact and use a verified matching executable.",
        ),
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

pub fn nonclaims() -> &'static [&'static str] {
    &[
        "no inbound TLS termination, client certificate, or insecure outbound trust mode",
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
    Query,
    Type,
    Expression,
    Owners,
    Relations,
    Limits,
    Diagnostics,
    Templates,
    Runners,
    Deployment,
    Security,
}

impl RegistrySection {
    pub const ALL: [Self; 14] = [
        Self::Contracts,
        Self::Operations,
        Self::Change,
        Self::Query,
        Self::Type,
        Self::Expression,
        Self::Owners,
        Self::Relations,
        Self::Limits,
        Self::Diagnostics,
        Self::Templates,
        Self::Runners,
        Self::Deployment,
        Self::Security,
    ];

    pub const PUBLIC: [Self; 13] = [
        Self::Operations,
        Self::Change,
        Self::Query,
        Self::Type,
        Self::Expression,
        Self::Owners,
        Self::Relations,
        Self::Limits,
        Self::Diagnostics,
        Self::Templates,
        Self::Runners,
        Self::Deployment,
        Self::Security,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Contracts => "contracts",
            Self::Operations => "operations",
            Self::Change => "change",
            Self::Query => "query",
            Self::Type => "type",
            Self::Expression => "expression",
            Self::Owners => "owners",
            Self::Relations => "relations",
            Self::Limits => "limits",
            Self::Diagnostics => "diagnostics",
            Self::Templates => "templates",
            Self::Runners => "runners",
            Self::Deployment => "deployment",
            Self::Security => "security",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|section| section.name() == value)
    }

    pub fn parse_public(value: &str) -> Option<Self> {
        Self::PUBLIC
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilitiesSnapshot {
    pub product_name: &'static str,
    pub product_version: &'static str,
    pub digest: String,
    pub bytes: Vec<u8>,
    pub sections: BTreeMap<RegistrySection, RegistrySectionSnapshot>,
}

impl CapabilitiesSnapshot {
    pub fn section(&self, section: RegistrySection) -> Option<&RegistrySectionSnapshot> {
        self.sections.get(&section)
    }
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

pub fn capabilities_snapshot() -> Result<CapabilitiesSnapshot, String> {
    let registry = registry_snapshot()?;
    let product_name = "lkjscript";
    let product_version = crate::PRODUCT_VERSION;
    let product = compact_record(
        "product",
        &[
            ("name", product_name.to_owned()),
            ("version", product_version.to_owned()),
        ],
    )?
    .into_bytes();
    let mut sections = BTreeMap::new();
    let mut body = Vec::new();
    for section in RegistrySection::PUBLIC {
        let snapshot = registry
            .section(section)
            .ok_or_else(|| format!("public capability section '{}' is missing", section.name()))?
            .clone();
        body.extend_from_slice(
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
        body.extend_from_slice(&snapshot.bytes);
        sections.insert(section, snapshot);
    }
    let mut canonical = product.clone();
    canonical.extend_from_slice(&body);
    let digest = digest(CAPABILITIES_DIGEST_DOMAIN, &canonical);
    let mut bytes = product;
    bytes.extend_from_slice(
        compact_record("capabilities", &[("digest", digest.clone())])?.as_bytes(),
    );
    bytes.extend_from_slice(&body);
    Ok(CapabilitiesSnapshot {
        product_name,
        product_version,
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
                    (
                        "request-model",
                        ControlModel::ChangeRequest.name().to_owned(),
                    ),
                    ("request-record", "request".to_owned()),
                    ("plan-prefix", "plan_".to_owned()),
                    ("plan-hex-characters", "128".to_owned()),
                    ("request-commitment", "opaque-digest".to_owned()),
                    ("prepared-plan", "opaque-commitment".to_owned()),
                    ("plan-output-action", "plan-only".to_owned()),
                ],
            )?);
            for descriptor in LOGICAL_PLAN_RECORD_DESCRIPTORS {
                records.push(compact_record(
                    "change.plan-record",
                    &[("name", descriptor.name.to_owned())],
                )?);
                for field in descriptor.fields {
                    records.push(compact_record(
                        "change.plan-record-field",
                        &[
                            ("record", descriptor.name.to_owned()),
                            ("name", (*field).to_owned()),
                        ],
                    )?);
                }
            }
            for descriptor in COMPACT_CHANGE_OPERATION_DESCRIPTORS {
                records.push(compact_record(
                    "change.operation",
                    &[("name", descriptor.name.to_owned())],
                )?);
                for field in descriptor.fields {
                    records.push(compact_record(
                        "change.operation-field",
                        &[
                            ("operation", descriptor.name.to_owned()),
                            ("name", field.name.to_owned()),
                            ("required", field.required.to_string()),
                            ("form", field.form.name().to_owned()),
                        ],
                    )?);
                }
                if let Some(direct) = descriptor.direct {
                    records.push(compact_record(
                        "change.direct-operation",
                        &[
                            ("name", descriptor.name.to_owned()),
                            ("plan-usage", direct.plan_usage.to_owned()),
                            ("apply-usage", direct.apply_usage.to_owned()),
                        ],
                    )?);
                }
            }
            for form in CompactChangeFieldForm::ALL {
                records.push(compact_record(
                    "change.field-form",
                    &[
                        ("name", form.name().to_owned()),
                        ("syntax", form.syntax().to_owned()),
                    ],
                )?);
            }
            for precondition in COMPACT_CHANGE_PRECONDITIONS {
                records.push(compact_record(
                    "change.precondition",
                    &[("name", (*precondition).to_owned())],
                )?);
            }
            for field in COMPACT_CHANGE_PRECONDITION_FIELDS {
                records.push(compact_record(
                    "change.precondition-field",
                    &[
                        ("precondition", field.record.to_owned()),
                        ("name", field.name.to_owned()),
                        ("required", field.required.to_string()),
                        ("form", field.form.name().to_owned()),
                    ],
                )?);
            }
            for policy in COMPACT_DELETE_POLICIES {
                records.push(compact_record(
                    "change.delete-policy",
                    &[("name", (*policy).to_owned())],
                )?);
            }
            for (name, _) in COMPACT_DECLARATION_VISIBILITIES {
                records.push(compact_record(
                    "change.declaration-visibility",
                    &[("name", (*name).to_owned())],
                )?);
            }
            for effect in COMPACT_FUNCTION_EFFECTS {
                records.push(compact_record(
                    "change.function-effect",
                    &[("name", (*effect).to_owned())],
                )?);
            }
            for (name, _) in COMPACT_NAMESPACE_CLASSES {
                records.push(compact_record(
                    "change.namespace-class",
                    &[("name", (*name).to_owned())],
                )?);
            }
            for (name, syntax) in [("package", "package"), ("exact_owner", "DOMAIN_HEX")] {
                records.push(compact_record(
                    "change.parent-form",
                    &[("name", name.to_owned()), ("syntax", syntax.to_owned())],
                )?);
            }
            for edge in COMPACT_CHANGE_EDGE_DESCRIPTORS {
                records.push(compact_record(
                    "change.edge",
                    &[
                        ("name", edge.name.to_owned()),
                        ("parent", edge.parent.to_owned()),
                        ("child", edge.child.to_owned()),
                        ("order", "zero-based-contiguous-index".to_owned()),
                    ],
                )?);
                for field in edge.fields {
                    records.push(compact_record(
                        "change.edge-field",
                        &[
                            ("edge", edge.name.to_owned()),
                            ("name", field.name.to_owned()),
                            ("required", field.required.to_string()),
                            ("syntax", field.syntax.to_owned()),
                        ],
                    )?);
                }
            }
            for (name, syntax) in [
                ("request_local_symbol", "$NAME"),
                ("request_local_type", "@NAME"),
                ("exact_owner", "DOMAIN_HEX"),
                ("exact_package", "pkg_HEX"),
                ("exact_revision", "rev_HEX"),
                ("exact_package_revision", "package_revision_HEX"),
                ("qualified_declaration", "MODULE/NAME"),
                ("exact_package_declaration", "pkg_HEX/decl_HEX"),
            ] {
                records.push(compact_record(
                    "change.reference",
                    &[("name", name.to_owned()), ("syntax", syntax.to_owned())],
                )?);
            }
        }
        RegistrySection::Query => {
            records.push(compact_record(
                "query",
                &[
                    ("authority", "normalized-current-revision".to_owned()),
                    ("ordering", "canonical-logical-key".to_owned()),
                    ("continuation", "stateless-exclusive-logical-key".to_owned()),
                ],
            )?);
            for descriptor in QUERY_OPERATION_DESCRIPTORS {
                records.push(compact_record(
                    "query.operation",
                    &[
                        ("name", descriptor.action.to_owned()),
                        ("command", descriptor.command.to_owned()),
                        ("usage", descriptor.usage.to_owned()),
                    ],
                )?);
                for (ordinal, positional) in descriptor.positionals.iter().enumerate() {
                    records.push(compact_record(
                        "query.positional",
                        &[
                            ("operation", descriptor.action.to_owned()),
                            ("ordinal", ordinal.saturating_add(1).to_string()),
                            ("name", (*positional).to_owned()),
                        ],
                    )?);
                }
                for option in descriptor.options {
                    records.push(compact_record(
                        "query.option",
                        &[
                            ("operation", descriptor.action.to_owned()),
                            ("name", (*option).to_owned()),
                            ("value", "required".to_owned()),
                        ],
                    )?);
                }
            }
            for kind in OwnerKind::ALL {
                records.push(compact_record(
                    "query.owner-kind",
                    &[("name", kind.name().to_owned())],
                )?);
            }
            for class in NamespaceClass::ALL {
                records.push(compact_record(
                    "query.namespace-class",
                    &[("name", class.name().to_owned())],
                )?);
            }
            for kind in RelationKind::ALL {
                records.push(compact_record(
                    "query.relation-kind",
                    &[("name", kind.name().to_owned())],
                )?);
            }
            for direction in QueryDirection::ALL {
                records.push(compact_record(
                    "query.direction",
                    &[("name", direction.name().to_owned())],
                )?);
            }
            for direction in ContextDirection::ALL {
                records.push(compact_record(
                    "query.context-direction",
                    &[("name", direction.name().to_owned())],
                )?);
            }
            for (name, value, unit) in [
                ("default-items", DEFAULT_QUERY_ITEMS.to_string(), "items"),
                ("maximum-items", MAXIMUM_QUERY_ITEMS.to_string(), "items"),
                (
                    "minimum-output-bytes",
                    MINIMUM_QUERY_OUTPUT_BYTES.to_string(),
                    "bytes",
                ),
                (
                    "default-output-bytes",
                    DEFAULT_QUERY_OUTPUT_BYTES.to_string(),
                    "bytes",
                ),
                (
                    "maximum-output-bytes",
                    MAXIMUM_QUERY_OUTPUT_BYTES.to_string(),
                    "bytes",
                ),
                (
                    "maximum-continuation-bytes",
                    MAXIMUM_QUERY_CONTINUATION_BYTES.to_string(),
                    "bytes",
                ),
                (
                    "minimum-context-depth",
                    MINIMUM_CONTEXT_DEPTH.to_string(),
                    "depth",
                ),
                (
                    "maximum-context-depth",
                    MAXIMUM_CONTEXT_DEPTH.to_string(),
                    "depth",
                ),
                (
                    "maximum-context-owners",
                    MAXIMUM_CONTEXT_OWNERS.to_string(),
                    "items",
                ),
                (
                    "maximum-context-relations",
                    MAXIMUM_CONTEXT_RELATIONS.to_string(),
                    "items",
                ),
                (
                    "maximum-context-relation-witnesses",
                    MAXIMUM_CONTEXT_RELATION_WITNESSES.to_string(),
                    "records",
                ),
                (
                    "maximum-context-map-pages",
                    MAXIMUM_CONTEXT_MAP_PAGES.to_string(),
                    "pages",
                ),
                (
                    "maximum-context-map-bytes",
                    MAXIMUM_CONTEXT_MAP_BYTES.to_string(),
                    "bytes",
                ),
                (
                    "maximum-context-map-entries",
                    MAXIMUM_CONTEXT_MAP_ENTRIES.to_string(),
                    "entries",
                ),
                (
                    "maximum-context-store-objects",
                    MAXIMUM_CONTEXT_STORE_OBJECTS.to_string(),
                    "objects",
                ),
                (
                    "maximum-context-store-bytes",
                    MAXIMUM_CONTEXT_STORE_BYTES.to_string(),
                    "bytes",
                ),
            ] {
                records.push(compact_record(
                    "query.limit",
                    &[
                        ("name", name.to_owned()),
                        ("value", value),
                        ("unit", unit.to_owned()),
                    ],
                )?);
            }
            for (record, field) in QUERY_RESPONSE_FIELDS {
                records.push(compact_record(
                    "query.response-field",
                    &[("record", record.to_owned()), ("name", field.to_owned())],
                )?);
            }
            for (_record, field) in QUERY_SELECTOR_FIELDS {
                records.push(compact_record(
                    "query.selector-field",
                    &[("name", field.to_owned())],
                )?);
            }
            for field in [
                "repository",
                "package",
                "revision",
                "operation",
                "selector-digest",
                "exclusive-resume-key",
                "integrity-digest",
            ] {
                records.push(compact_record(
                    "query.continuation-field",
                    &[("name", field.to_owned())],
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
            for field in COMPACT_TYPE_FORM_FIELDS {
                records.push(compact_record(
                    "type.field",
                    &[
                        ("form", field.form.to_owned()),
                        ("name", field.name.to_owned()),
                        ("required", field.required.to_string()),
                        ("syntax", field.syntax.to_owned()),
                    ],
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
            for field in COMPACT_EXPRESSION_FORM_FIELDS {
                records.push(compact_record(
                    "expression.field",
                    &[
                        ("form", field.form.to_owned()),
                        ("name", field.name.to_owned()),
                        ("required", field.required.to_string()),
                        ("syntax", field.syntax.to_owned()),
                    ],
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
            for role in RelationKind::ALL {
                records.push(compact_record(
                    "relation.kind",
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
            for template in ProjectTemplate::ALL {
                records.push(compact_record(
                    "template",
                    &[
                        ("name", template.name().to_owned()),
                        ("purpose", template.purpose().to_owned()),
                        ("runner", template.runner().to_owned()),
                        (
                            "starter-deployment",
                            template.emits_deployment().to_string(),
                        ),
                        (
                            "recommended-artifact-output",
                            template
                                .recommended_artifact_output()
                                .unwrap_or("none")
                                .to_owned(),
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
        RegistrySection::Deployment => {
            records.push(compact_record(
                "deployment.schema",
                &[
                    ("maximum-bytes", MAXIMUM_DEPLOYMENT_BYTES.to_string()),
                    ("maximum-grants", MAXIMUM_DEPLOYMENT_GRANTS.to_string()),
                    ("strict-json", "true".to_owned()),
                    ("unknown-fields", "reject".to_owned()),
                ],
            )?);
            for field in DEPLOYMENT_SCHEMA_FIELDS {
                let mut fields = vec![
                    ("path", field.path.to_owned()),
                    ("required", field.required.to_string()),
                    ("scalar", field.scalar.to_owned()),
                    ("secret-name", field.secret_name.to_string()),
                ];
                if let Some(minimum) = field.minimum {
                    fields.push(("minimum", minimum.to_string()));
                }
                if let Some(maximum) = field.maximum {
                    fields.push(("maximum", maximum.to_string()));
                }
                if let Some(nested) = field.nested {
                    fields.push(("nested", nested.to_owned()));
                }
                records.push(compact_record("deployment.field", &fields)?);
            }
            for adapter in DEPLOYMENT_ADAPTER_SCHEMAS {
                records.push(compact_record(
                    "deployment.adapter",
                    &[
                        ("kind", adapter.kind.to_owned()),
                        ("fields", adapter.fields.len().to_string()),
                    ],
                )?);
                for field in adapter.fields {
                    let mut fields = vec![
                        ("adapter", adapter.kind.to_owned()),
                        ("path", field.path.to_owned()),
                        ("required", field.required.to_string()),
                        ("scalar", field.scalar.to_owned()),
                        ("secret-name", field.secret_name.to_string()),
                    ];
                    if let Some(minimum) = field.minimum {
                        fields.push(("minimum", minimum.to_string()));
                    }
                    if let Some(maximum) = field.maximum {
                        fields.push(("maximum", maximum.to_string()));
                    }
                    if let Some(nested) = field.nested {
                        fields.push(("nested", nested.to_owned()));
                    }
                    records.push(compact_record("deployment.adapter-field", &fields)?);
                }
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
        ProjectTemplate::ALL.into_iter().map(ProjectTemplate::name),
        "template",
    )?;
    if operation_descriptors().len() != PublicOperation::ALL.len() {
        return Err("every public operation must have exactly one descriptor".to_owned());
    }
    for descriptor in COMPACT_CHANGE_OPERATION_DESCRIPTORS {
        if let Some(direct) = descriptor.direct
            && (direct.plan_usage.is_empty() || direct.apply_usage.is_empty())
        {
            return Err(format!(
                "direct change operation '{}' has empty usage",
                descriptor.name
            ));
        }
    }
    let (change_operations, change_fields, advertised_forms) =
        compact_change_validation_inventory();
    validate_compact_change_inventory(&change_operations, &change_fields, &advertised_forms)?;
    validate_compact_change_preconditions(&advertised_forms)?;
    validate_compact_form_grammar()?;
    validate_deployment_schema_inventory()?;
    Ok(())
}

fn validate_deployment_schema_inventory() -> Result<(), String> {
    unique(
        DEPLOYMENT_SCHEMA_FIELDS.iter().map(|field| field.path),
        "deployment schema field",
    )?;
    unique(
        DEPLOYMENT_ADAPTER_SCHEMAS
            .iter()
            .map(|adapter| adapter.kind),
        "deployment adapter kind",
    )?;
    let expected = [
        "configuration",
        "wall_clock",
        "secure_random",
        "identifier",
        "password_hash",
        "secret_verifier",
        "byte_stream",
        "http_client",
        "data",
        "object_memory",
        "object_local",
        "object_s3",
        "durable_queue_data",
    ];
    if DEPLOYMENT_ADAPTER_SCHEMAS
        .iter()
        .map(|adapter| adapter.kind)
        .collect::<Vec<_>>()
        != expected
    {
        return Err("deployment adapter schema does not match the closed decoder order".to_owned());
    }
    let mut paths = BTreeSet::new();
    for adapter in DEPLOYMENT_ADAPTER_SCHEMAS {
        if adapter.fields.is_empty()
            || !adapter.fields.iter().any(|field| {
                field.path == format!("adapter.{}.kind", adapter.kind) && field.required
            })
        {
            return Err(format!(
                "deployment adapter '{}' has no required discriminator",
                adapter.kind
            ));
        }
        for field in adapter.fields {
            if field.path.is_empty()
                || field.scalar.is_empty()
                || !paths.insert(field.path)
                || field
                    .minimum
                    .zip(field.maximum)
                    .is_some_and(|(minimum, maximum)| minimum > maximum)
            {
                return Err(format!(
                    "deployment adapter field '{}' is empty, duplicated, or has inverted bounds",
                    field.path
                ));
            }
        }
    }
    Ok(())
}

fn validate_compact_form_grammar() -> Result<(), String> {
    unique(COMPACT_TYPE_FORMS.iter().copied(), "compact type form")?;
    unique(
        COMPACT_EXPRESSION_FORMS.iter().copied(),
        "compact expression form",
    )?;
    unique(
        COMPACT_CHANGE_EDGE_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.name),
        "compact edge",
    )?;
    for (label, forms, fields) in [
        ("type", COMPACT_TYPE_FORMS, COMPACT_TYPE_FORM_FIELDS),
        (
            "expression",
            COMPACT_EXPRESSION_FORMS,
            COMPACT_EXPRESSION_FORM_FIELDS,
        ),
    ] {
        let mut names = BTreeSet::new();
        for field in fields {
            if !forms.contains(&field.form) {
                return Err(format!(
                    "compact {label} field '{}.{}' names an unknown form",
                    field.form, field.name
                ));
            }
            if field.name.is_empty()
                || field.syntax.is_empty()
                || !names.insert((field.form, field.name))
            {
                return Err(format!(
                    "compact {label} field '{}.{}' is empty or duplicated",
                    field.form, field.name
                ));
            }
        }
        for form in forms.iter().copied() {
            if !fields
                .iter()
                .any(|field| field.form == form && field.name == "as" && field.required)
            {
                return Err(format!(
                    "compact {label} form '{form}' has no required as field"
                ));
            }
        }
    }
    for edge in COMPACT_CHANGE_EDGE_DESCRIPTORS {
        if edge.parent.is_empty() || edge.child.is_empty() || edge.fields.is_empty() {
            return Err(format!("compact edge '{}' is incomplete", edge.name));
        }
        let mut fields = BTreeSet::new();
        for field in edge.fields {
            if field.form != edge.name
                || field.name.is_empty()
                || field.syntax.is_empty()
                || !fields.insert(field.name)
            {
                return Err(format!(
                    "compact edge field '{}.{}' is inconsistent or duplicated",
                    edge.name, field.name
                ));
            }
        }
        for required in ["parent", "index"] {
            if !edge
                .fields
                .iter()
                .any(|field| field.name == required && field.required)
            {
                return Err(format!(
                    "compact edge '{}' has no required {required} field",
                    edge.name
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct ChangeOperationValidation<'a> {
    operation: CompactChangeOperation,
    name: &'a str,
    direct: bool,
}

#[derive(Clone, Debug)]
struct ChangeFieldValidation<'a> {
    operation: &'a str,
    name: &'a str,
    required: bool,
    form: &'a str,
}

fn compact_change_validation_inventory() -> (
    Vec<ChangeOperationValidation<'static>>,
    Vec<ChangeFieldValidation<'static>>,
    Vec<&'static str>,
) {
    let operations = COMPACT_CHANGE_OPERATION_DESCRIPTORS
        .iter()
        .map(|descriptor| ChangeOperationValidation {
            operation: descriptor.operation,
            name: descriptor.name,
            direct: descriptor.direct.is_some(),
        })
        .collect();
    let fields = COMPACT_CHANGE_OPERATION_DESCRIPTORS
        .iter()
        .flat_map(|descriptor| {
            descriptor
                .fields
                .iter()
                .map(move |field| ChangeFieldValidation {
                    operation: descriptor.name,
                    name: field.name,
                    required: field.required,
                    form: field.form.name(),
                })
        })
        .collect();
    let forms = CompactChangeFieldForm::ALL
        .map(CompactChangeFieldForm::name)
        .to_vec();
    (operations, fields, forms)
}

fn validate_compact_change_inventory(
    operations: &[ChangeOperationValidation<'_>],
    fields: &[ChangeFieldValidation<'_>],
    advertised_forms: &[&str],
) -> Result<(), String> {
    unique(
        operations.iter().map(|value| value.name),
        "change operation",
    )?;
    unique(advertised_forms.iter().copied(), "change field form")?;

    let expected_operations = CompactChangeOperation::ALL
        .into_iter()
        .collect::<BTreeSet<_>>();
    let described_operations = operations
        .iter()
        .map(|value| value.operation)
        .collect::<BTreeSet<_>>();
    if operations.len() != described_operations.len() || described_operations != expected_operations
    {
        return Err(
            "compact change descriptor inventory and semantic decoder coverage differ".to_owned(),
        );
    }

    let operation_names = operations
        .iter()
        .map(|operation| operation.name)
        .collect::<BTreeSet<_>>();
    for operation in operations {
        if !fields.iter().any(|field| field.operation == operation.name) {
            return Err(format!(
                "change operation '{}' has no field descriptor",
                operation.name
            ));
        }
    }

    let mut field_names = BTreeSet::new();
    for field in fields {
        if !operation_names.contains(field.operation) {
            return Err(format!(
                "change field '{}.{}' refers to an unknown operation",
                field.operation, field.name
            ));
        }
        if field.name.is_empty() || !field_names.insert((field.operation, field.name)) {
            return Err(format!(
                "change field '{}.{}' is empty or duplicated",
                field.operation, field.name
            ));
        }
        if !advertised_forms.contains(&field.form) {
            return Err(format!(
                "change field '{}.{}' uses unadvertised form '{}'",
                field.operation, field.name, field.form
            ));
        }
        let optional = [
            ("add.case", "payload"),
            ("add.parameter", "function"),
            ("add.parameter", "operation"),
            ("add.parameter", "use"),
        ];
        if !field.required && !optional.contains(&(field.operation, field.name)) {
            return Err(format!(
                "change field '{}.{}' is unexpectedly optional",
                field.operation, field.name
            ));
        }
    }
    for (operation, name) in [
        ("add.case", "payload"),
        ("add.parameter", "function"),
        ("add.parameter", "operation"),
        ("add.parameter", "use"),
    ] {
        if !fields
            .iter()
            .any(|field| (field.operation, field.name, field.required) == (operation, name, false))
        {
            return Err(format!(
                "{operation}.{name} must be an optional change field"
            ));
        }
    }

    let direct_operations = operations
        .iter()
        .filter(|operation| operation.direct)
        .map(|operation| operation.name)
        .collect::<Vec<_>>();
    if direct_operations != ["rename.owner"] {
        return Err("rename.owner must be the sole direct compact change operation".to_owned());
    }
    Ok(())
}

fn validate_compact_change_preconditions(advertised_forms: &[&str]) -> Result<(), String> {
    unique(
        COMPACT_CHANGE_PRECONDITIONS.iter().copied(),
        "change precondition",
    )?;
    let preconditions = COMPACT_CHANGE_PRECONDITIONS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut fields = BTreeSet::new();
    for field in COMPACT_CHANGE_PRECONDITION_FIELDS {
        if !preconditions.contains(field.record) {
            return Err(format!(
                "change precondition field '{}.{}' refers to an unknown precondition",
                field.record, field.name
            ));
        }
        if field.name.is_empty() || !fields.insert((field.record, field.name)) {
            return Err(format!(
                "change precondition field '{}.{}' is empty or duplicated",
                field.record, field.name
            ));
        }
        if !advertised_forms.contains(&field.form.name()) {
            return Err(format!(
                "change precondition field '{}.{}' uses unadvertised form '{}'",
                field.record,
                field.name,
                field.form.name()
            ));
        }
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
    fn public_capabilities_are_canonical_and_hide_internal_compatibility() {
        let first = capabilities_snapshot().expect("valid public capabilities");
        let second = capabilities_snapshot().expect("repeat public capabilities");
        assert_eq!(first, second);
        assert_eq!(first.product_name, "lkjscript");
        assert_eq!(first.product_version, crate::PRODUCT_VERSION);
        assert_eq!(first.sections.len(), RegistrySection::PUBLIC.len());
        assert!(!first.sections.contains_key(&RegistrySection::Contracts));
        let text = std::str::from_utf8(&first.bytes).expect("capabilities UTF-8");
        assert!(text.starts_with(&format!(
            "product name=lkjscript version={}\ncapabilities digest=",
            crate::PRODUCT_VERSION
        )));
        for forbidden in [
            REGISTRY_CONTRACT_IDENTITY,
            "lkjscript-meaning-graph-",
            "lkjscript-cli-",
            "lkjscript-change-records-",
            "lkjscript-query-",
            "lkjscript-deployment-",
            "logical-plan.contracts",
            "contract-version",
        ] {
            assert!(
                !text.contains(forbidden),
                "public capabilities leaked {forbidden}"
            );
        }
        let mut canonical = compact_record(
            "product",
            &[
                ("name", "lkjscript".to_owned()),
                ("version", crate::PRODUCT_VERSION.to_owned()),
            ],
        )
        .expect("product record")
        .into_bytes();
        for section in RegistrySection::PUBLIC {
            let snapshot = first.section(section).expect("public section");
            canonical.extend_from_slice(
                compact_record(
                    "section",
                    &[
                        ("name", section.name().to_owned()),
                        ("digest", snapshot.digest.clone()),
                        ("records", snapshot.records.to_string()),
                        ("bytes", snapshot.bytes.len().to_string()),
                    ],
                )
                .expect("section record")
                .as_bytes(),
            );
            canonical.extend_from_slice(&snapshot.bytes);
        }
        assert_eq!(first.digest, digest(CAPABILITIES_DIGEST_DOMAIN, &canonical));
    }

    #[test]
    fn compact_change_registry_rejects_structural_descriptor_drift() {
        let (operations, fields, forms) = compact_change_validation_inventory();
        validate_compact_change_inventory(&operations, &fields, &forms)
            .expect("authoritative compact change descriptors");

        let mut duplicate_operations = operations.clone();
        duplicate_operations.push(operations[0].clone());
        assert!(
            validate_compact_change_inventory(&duplicate_operations, &fields, &forms)
                .expect_err("duplicate operation")
                .contains("duplicated")
        );

        let mut duplicate_fields = fields.clone();
        duplicate_fields.push(fields[0].clone());
        assert!(
            validate_compact_change_inventory(&operations, &duplicate_fields, &forms)
                .expect_err("duplicate field")
                .contains("duplicated")
        );

        let mut unknown_operation_fields = fields.clone();
        unknown_operation_fields[0].operation = "unknown.operation";
        assert!(
            validate_compact_change_inventory(&operations, &unknown_operation_fields, &forms)
                .expect_err("unknown field operation")
                .contains("unknown operation")
        );

        let mut missing_operation = operations.clone();
        missing_operation.pop();
        assert!(
            validate_compact_change_inventory(&missing_operation, &fields, &forms)
                .expect_err("missing operation descriptor")
                .contains("decoder coverage differ")
        );

        let first_operation = operations[0].name;
        let no_fields = fields
            .iter()
            .filter(|field| field.operation != first_operation)
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            validate_compact_change_inventory(&operations, &no_fields, &forms)
                .expect_err("operation without fields")
                .contains("no field descriptor")
        );

        let mut unadvertised = fields.clone();
        unadvertised[0].form = "unadvertised_form";
        assert!(
            validate_compact_change_inventory(&operations, &unadvertised, &forms)
                .expect_err("unadvertised form")
                .contains("unadvertised form")
        );
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

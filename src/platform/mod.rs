//! Canonical meaning-graph language and application platform.
//!
//! Accepted typed semantic graphs own program meaning. The public semantic protocol owns normal
//! inspection and mutation; artifacts, bytecode, runtime topology, and deployment bindings are
//! derived consumers.

pub mod artifact;
pub mod bootstrap;
#[allow(
    dead_code,
    reason = "Graph 5 generic changes remain private until repository publication cuts over"
)]
pub(crate) mod change;
pub mod cli;
pub mod configuration;
pub mod contract;
pub mod database;
pub mod deployment;
pub mod diagnostic;
pub mod execution;
pub mod graph;
pub mod http;
mod intrinsic_contract;
pub mod json;
#[allow(
    dead_code,
    reason = "Graph 5 remains private until its dependency-closed direct public cutover"
)]
pub(crate) mod kernel;
pub mod language;
pub mod meaning;
pub mod object;
pub mod package;
#[allow(
    dead_code,
    reason = "Graph 5 package objects remain private until repository publication cuts over"
)]
pub(crate) mod package_object;
pub(crate) mod packed;
pub mod persistent_map;
#[allow(
    dead_code,
    reason = "Graph 5 publication remains private until repository cutover"
)]
pub(crate) mod publication;
pub mod queue;
pub mod repository;
pub mod revision;
pub mod runtime;
pub mod secrets;
pub mod security;
pub mod semantic;
pub mod semantic_change;
pub mod semantic_diff;
pub mod semantic_digest;
pub mod semantic_draft;
pub mod semantic_fact;
pub mod semantic_id;
pub mod semantic_merge;
pub mod semantic_projection;
pub mod semantic_query;
pub mod semantic_summary;
pub mod semantic_transaction;
#[allow(
    dead_code,
    reason = "packed Graph 5 storage remains private until repository publication cuts over"
)]
pub(crate) mod storage;
pub mod stream;
mod syntax;
pub mod value;
#[allow(
    dead_code,
    reason = "Graph 5 validation witness remains private until repository publication cuts over"
)]
pub(crate) mod witness;
pub mod worker;
pub mod workspace;

#[cfg(test)]
pub(crate) use artifact::build_artifact;
pub use artifact::{ARTIFACT_CONTRACT_VERSION, ArtifactReceipt, LoadedArtifact, load_artifact};
pub use bootstrap::{
    BOOTSTRAP_CONTRACT_VERSION, BuiltinPackageInfo, ProjectCreationReceipt, ProjectTemplate,
    builtin_package_info, create_project, export_builtin_standard,
};
pub use cli::{CliSuccess, execute as execute_cli};
pub use configuration::{
    CONFIGURATION_ADAPTER_CONTRACT_VERSION, ConfigurationAdapter, ConfigurationObservation,
    ConfigurationValue,
};
pub use contract::{CLI_CONTRACT_VERSION, PublicOperation};
pub use database::{
    POSTGRES_ADAPTER_CONTRACT_VERSION, PostgresAdapter, PostgresPool, PostgresPoolConfig,
    PostgresPoolObservation, PostgresSecret,
};
pub use deployment::{
    AdapterDescriptor, DEPLOYMENT_CONTRACT_VERSION, DeploymentDescriptor, DeploymentGrant,
    DeploymentObservation, PreparedDeployment, decode_deployment,
};
pub use diagnostic::{Diagnostic, DiagnosticClass, SourceLocation};
pub use execution::{
    BoundCapabilities, CAPABILITY_GRANT_CONTRACT_VERSION, CallPolicy, CapabilityAdapter,
    CapabilityGrant, CapabilityGrantDescriptor, CapabilityTransaction, ExecutionError,
    ExecutionFailureClass, PreparedComponent, PreparedPort, PreparedProgram, PreparedRequirement,
    PreparedTarget, PreparedTest, ReferenceInterpreter, RunObservation, RunPolicy, ScriptedAdapter,
    ScriptedCall, Vm,
};
pub use graph::{
    DependencyBinding, GraphRoot, ModuleObjectRef, TargetBinding, Tombstone, TombstoneIdentity,
};
pub use http::{
    HTTP_ADAPTER_CONTRACT_VERSION, HttpApplication, HttpDispatchObservation, HttpHeader,
    HttpLimits, HttpRequest, HttpResponse, HttpServerReceipt, request_type as http_request_type,
    response_type as http_response_type,
};
pub use json::{
    JSON_CONTRACT_VERSION, JsonLimits, decode_strict as decode_strict_json,
    decode_typed as decode_typed_json, encode_typed as encode_typed_json,
};
#[cfg(test)]
pub(crate) use language::parse_module;
pub use language::{
    Component, Declaration, Effect, Expression, Function, Module, Requirement, Type, TypeParameter,
};
pub use meaning::{
    Annotation, AnnotationClass, DeclarationIdentity, DeclarationKind, Documentation,
    ExpressionIdentity, ExpressionKind, GRAPH_CONTRACT_IDENTITY, GRAPH_CONTRACT_VERSION,
    MeaningModule, MemberIdentity, MigrationIdentityAllocator, RelationRole, RelationSource,
    RelationTarget, RequestIdentityAllocator, SemanticRelation,
};
pub use object::{
    OBJECT_ADAPTER_CONTRACT_VERSION, ObjectAdapterIdentity, ObjectLimits, ObjectStorageAdapter,
    S3Config, S3Credentials,
};
#[cfg(test)]
pub(crate) use package::decode_package;
pub use package::{Dependency, ModuleLocator, PackageDescriptor, PackageId, RunnerKind, Target};
pub use queue::{
    DURABLE_QUEUE_CONTRACT_VERSION, DurableQueueAdapter, JobLease, JobSnapshot, JobState,
    QueueLimits,
};
pub use repository::{
    BACKUP_CONTRACT_VERSION, BACKUP_SEGMENT_ENTRY_LIMIT, BackupReceipt, CurrentBinding,
    CurrentRevision, DependencyArtifactObject, DoctorReport, InitialPublication,
    MAXIMUM_BACKUP_MANIFEST_BYTES, MAXIMUM_BACKUP_SEGMENT_BYTES, PublicationOutcome,
    RETENTION_CONTRACT_VERSION, ReconstructedRevision, RestoreReceipt, RetentionReport,
    RevisionSnapshot, SemanticRepository,
};
pub use revision::{
    AffectedOwner, ParentRevision, ReceiptStatus, RevisionCore, RevisionRecord, SemanticHead,
    TransactionReceipt, ValidationFacts,
};
pub use runtime::{
    InvocationReceipt, RESIDENT_RUNTIME_CONTRACT_VERSION, ResidentDeployment, ResidentLimits,
    ResidentObservation, ShutdownReceipt,
};
pub use secrets::{
    EnvironmentSecretBinding, SECRET_CATALOG_CONTRACT_VERSION, SECRET_VERIFIER_CONTRACT_VERSION,
    SecretCatalog, SecretValue, SecretVerifierAdapter,
};
pub use security::{
    DeterministicClockAdapter, DeterministicIdentifierAdapter, DeterministicRandomAdapter,
    IdentifierAdapter, PasswordHashAdapter, PasswordHashPolicy, SECURITY_ADAPTER_CONTRACT_VERSION,
    SecureRandomAdapter, WallClockAdapter, format_uuid, parse_uuid,
};
#[cfg(test)]
pub(crate) use semantic::validate_package_documents;
pub use semantic::{
    ExactDependency, ExactGraphDependency, OwnerId, ResolvedType, ValidatedModule,
    ValidatedPackage, validate_graph_package,
};
pub use semantic_change::{
    AllocatedIdentity, CHANGE_CONTRACT_VERSION, Change, ChangeRequest, ChangeResult,
    ExpressionForm, TypeForm, TypeParameterForm, execute_change,
};
pub use semantic_diff::{
    SEMANTIC_DIFF_CONTRACT_VERSION, SemanticChangeClass, SemanticDiffReport, SemanticDiffStatus,
    SemanticOwnerChange, SemanticOwnerState, diff_revisions,
};
pub use semantic_digest::{
    ArtifactDigest, BackupDigest, CleanupDigest, IndexDigest, ModuleObjectDigest, ReceiptDigest,
    RevisionRecordDigest, RootObjectDigest, SemanticDiffDigest, TransactionDigest,
};
pub use semantic_draft::{
    DRAFT_CONTRACT_VERSION, DraftConflict, DraftConflictKind, DraftHole, DraftMutationReceipt,
    DraftRebaseResult, DraftRecord, DraftSummary, SemanticDraftStore,
};
pub use semantic_id::{
    AnnotationId, BindingId, CaseId, ConflictId, DeclarationId, DocumentationId, DraftId,
    ExpressionId, FieldId, ModuleId, OperationId, ParameterId, PortId, RepositoryId, RequirementId,
    RevisionId, TargetId, TypeParameterId,
};
pub use semantic_merge::{
    MAXIMUM_MERGE_CONFLICTS, SEMANTIC_MERGE_CONTRACT_VERSION, SemanticMergeConflict,
    SemanticMergeConflictKind, SemanticMergeRequest, SemanticMergeResult, SemanticMergeStatus,
    merge_revisions,
};
pub use semantic_projection::{
    MAXIMUM_REVIEW_PROJECTION_BYTES, REVIEW_PROJECTION_CONTRACT_VERSION, ReviewProjectionReceipt,
    render_review_projection,
};
pub use semantic_query::{
    ContextItem, OwnerDetail, OwnerKind, OwnerSummary, QueryBudget, QueryPage, QueryStatus,
    RelationView, SemanticQueryIndex,
};
pub use semantic_transaction::{
    MAXIMUM_TRANSACTION_AFFECTED_OWNERS, MAXIMUM_TRANSACTION_OPERATIONS, MAXIMUM_TRANSACTION_WORK,
    SemanticOperation, SemanticPrecondition, TransactionBudget, TransactionMode,
    TransactionRequest, TransactionResult, TransactionStatus, execute_transaction,
};
pub use stream::{
    ByteStreamAdapter, ByteStreamProducer, STREAM_CONTRACT_VERSION, StreamLease, StreamLimits,
    StreamRegistry,
};
pub use syntax::SourceSpan;
#[cfg(test)]
pub(crate) use syntax::{SourceLimits, parse_source};
pub use value::{MapKey, ResourceId, ResourceKind, Value, ValueError, value_matches_type};
pub use worker::{WORKER_RUNNER_CONTRACT_VERSION, WorkerApplication, WorkerLimits, WorkerReceipt};
pub use workspace::{
    DependencyOrientation, ModuleOrientation, SemanticWorkspace, TargetOrientation,
    WORKSPACE_CONTRACT_VERSION, WorkspaceOrientation, WorkspaceStatus,
};

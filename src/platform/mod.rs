//! Canonical meaning-graph language and application platform.
//!
//! Accepted typed semantic graphs own program meaning. The public semantic protocol owns normal
//! inspection and mutation; artifacts, bytecode, runtime topology, and deployment bindings are
//! derived consumers.

pub(crate) mod builtin_discovery;
pub(crate) mod builtin_standard;
#[allow(
    dead_code,
    reason = "the typed engine includes tested forms not yet exposed by the closed CLI registry"
)]
pub(crate) mod change;
pub mod cli;
#[allow(
    dead_code,
    reason = "normalized compiler internals are process-private behind lifecycle preparation"
)]
pub(crate) mod compiler;
pub mod configuration;
pub mod contract;
pub mod control;
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
    reason = "the current semantic kernel is process-private behind typed public adapters"
)]
pub(crate) mod kernel;
pub mod language;
pub mod meaning;
pub(crate) mod normalized_lifecycle;
pub(crate) mod normalized_query;
pub mod object;
pub(crate) mod owned_output;
pub mod package;
#[allow(
    dead_code,
    reason = "package interfaces are internal exact dependency contracts"
)]
pub(crate) mod package_interface;
#[allow(
    dead_code,
    reason = "package transport is internal to exact built-in dependency handling"
)]
pub(crate) mod package_transport;
pub(crate) mod packed;
pub mod persistent_map;
pub(crate) mod project_creation;
pub(crate) mod project_discovery;
#[allow(
    dead_code,
    reason = "publication is reachable only through typed process-boundary operations"
)]
pub(crate) mod publication;
pub mod queue;
pub mod revision;
pub mod runtime;
pub mod secrets;
pub mod security;
pub mod semantic;
pub mod semantic_digest;
pub mod semantic_fact;
pub mod semantic_id;
pub mod semantic_summary;
#[allow(
    dead_code,
    reason = "storage implementations remain private behind GraphRepository"
)]
pub(crate) mod storage;
pub mod stream;
mod syntax;
#[allow(
    dead_code,
    reason = "validation witnesses are internal accepted evidence, not a public authoring API"
)]
pub(crate) mod witness;
pub mod worker;

pub use cli::{
    execute_build, execute_capabilities, execute_change, execute_check, execute_inspect,
    execute_inspect_owner, execute_new, execute_package_builtin, execute_query, execute_run,
    execute_status,
};
pub use configuration::{
    CONFIGURATION_ADAPTER_CONTRACT_VERSION, ConfigurationObservation, ConfigurationValue,
};
pub use contract::{CLI_CONTRACT_VERSION, PublicOperation};
pub use database::{
    POSTGRES_ADAPTER_CONTRACT_VERSION, PostgresPool, PostgresPoolConfig, PostgresPoolObservation,
    PostgresSecret,
};
pub use deployment::{
    AdapterDescriptor, DEPLOYMENT_CONTRACT_VERSION, DeploymentDescriptor, DeploymentGrant,
    DeploymentObservation, PreparedDeployment, decode_deployment,
};
pub use diagnostic::{Diagnostic, DiagnosticClass, SourceLocation};
pub use execution::{ExecutionControl, ExecutionError, ExecutionFailureClass, RunPolicy};
pub use graph::{
    DependencyBinding, GraphRoot, ModuleObjectRef, TargetBinding, Tombstone, TombstoneIdentity,
};
pub use http::{
    HTTP_ADAPTER_CONTRACT_VERSION, HttpDispatchObservation, HttpHeader, HttpLimits, HttpRequest,
    HttpResponse, HttpServerReceipt,
};
pub use json::{JSON_CONTRACT_VERSION, JsonLimits, decode_strict as decode_strict_json};
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
pub use object::{OBJECT_ADAPTER_CONTRACT_VERSION, ObjectLimits, S3Config, S3Credentials};
pub use package::{Dependency, ModuleLocator, PackageDescriptor, PackageId, RunnerKind, Target};
pub use queue::{DURABLE_QUEUE_CONTRACT_VERSION, JobLease, JobSnapshot, JobState, QueueLimits};
pub use revision::{
    AffectedOwner, ParentRevision, ReceiptStatus, RevisionCore, RevisionRecord, SemanticHead,
    TransactionReceipt, ValidationFacts,
};
pub use runtime::{
    RESIDENT_RUNTIME_CONTRACT_VERSION, ResidentLimits, ResidentObservation, ShutdownReceipt,
};
pub use secrets::{
    EnvironmentSecretBinding, SECRET_CATALOG_CONTRACT_VERSION, SECRET_VERIFIER_CONTRACT_VERSION,
    SecretCatalog, SecretValue,
};
pub use security::{
    PasswordHashPolicy, SECURITY_ADAPTER_CONTRACT_VERSION, format_uuid, parse_uuid,
};
pub use semantic::{
    ExactDependency, ExactGraphDependency, OwnerId, ResolvedType, ValidatedModule,
    ValidatedPackage, validate_graph_package,
};
pub use semantic_digest::{
    ArtifactDigest, BackupDigest, CleanupDigest, IndexDigest, ModuleObjectDigest, ReceiptDigest,
    RevisionRecordDigest, RootObjectDigest, SemanticDiffDigest, TransactionDigest,
};
pub use semantic_id::{
    AnnotationId, BindingId, CaseId, ConflictId, DeclarationId, DocumentationId, DraftId,
    ExpressionId, FieldId, ModuleId, OperationId, ParameterId, PortId, RepositoryId, RequirementId,
    RevisionId, TargetId, TypeParameterId,
};
pub use stream::{
    ByteStreamProducer, STREAM_CONTRACT_VERSION, StreamLease, StreamLimits, StreamRegistry,
};
pub use syntax::SourceSpan;
#[cfg(test)]
pub(crate) use syntax::{SourceLimits, parse_source};
pub use worker::{WORKER_RUNNER_CONTRACT_VERSION, WorkerLimits, WorkerReceipt};

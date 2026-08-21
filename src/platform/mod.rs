//! General-purpose authored source and application platform.
//!
//! This is the current direct-cutover implementation. Canonical modules own program edits; typed
//! semantic facts, artifacts, bytecode, runtime topology, and deployment bindings are derived.

pub mod artifact;
pub mod authority;
pub mod cli;
pub mod configuration;
pub mod database;
pub mod deployment;
pub mod diagnostic;
pub mod execution;
pub mod http;
mod intrinsic_contract;
pub mod json;
pub mod language;
pub mod object;
pub mod package;
pub mod queue;
pub mod runtime;
pub mod secrets;
pub mod security;
pub mod semantic;
pub mod stream;
pub mod syntax;
pub mod value;
pub mod worker;
pub mod workspace;

pub use artifact::{
    ARTIFACT_CONTRACT_VERSION, ArtifactReceipt, LoadedArtifact, build_artifact, load_artifact,
    package_artifact_digest,
};
pub use authority::{
    AcceptedRevision, ApplyReceipt, BackupReceipt, DependencyArtifact, DoctorReceipt, HistoryPage,
    ProjectAuthority, PublicationKind, RevisionSummary, WorkingStatus,
};
pub use cli::{CLI_CONTRACT_VERSION, CliSuccess, execute as execute_cli};
pub use configuration::{
    CONFIGURATION_ADAPTER_CONTRACT_VERSION, ConfigurationAdapter, ConfigurationObservation,
    ConfigurationValue,
};
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
pub use http::{
    HTTP_ADAPTER_CONTRACT_VERSION, HttpApplication, HttpDispatchObservation, HttpHeader,
    HttpLimits, HttpRequest, HttpResponse, HttpServerReceipt, request_type as http_request_type,
    response_type as http_response_type,
};
pub use json::{
    JSON_CONTRACT_VERSION, JsonLimits, decode_strict as decode_strict_json,
    decode_typed as decode_typed_json, encode_typed as encode_typed_json,
};
pub use language::{
    Component, Declaration, Effect, Expression, Function, Module, Requirement, Type, parse_module,
};
pub use object::{
    OBJECT_ADAPTER_CONTRACT_VERSION, ObjectAdapterIdentity, ObjectLimits, ObjectStorageAdapter,
    S3Config, S3Credentials,
};
pub use package::{
    Dependency, ModuleLocator, PackageDescriptor, PackageId, RunnerKind, Target, decode_package,
};
pub use queue::{
    DURABLE_QUEUE_CONTRACT_VERSION, DurableQueueAdapter, JobLease, JobSnapshot, JobState,
    QueueLimits,
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
pub use semantic::{
    ExactDependency, OwnerId, ResolvedType, ValidatedModule, ValidatedPackage,
    validate_package_documents,
};
pub use stream::{
    ByteStreamAdapter, ByteStreamProducer, STREAM_CONTRACT_VERSION, StreamLease, StreamLimits,
    StreamRegistry,
};
pub use syntax::{Atom, Form, FormKind, SourceDocument, SourceLimits, parse_source};
pub use value::{MapKey, ResourceId, ResourceKind, Value, ValueError, value_matches_type};
pub use worker::{WORKER_RUNNER_CONTRACT_VERSION, WorkerApplication, WorkerLimits, WorkerReceipt};
pub use workspace::{
    DependencyOrientation, ModuleOrientation, SourceWorkspace, TargetOrientation,
    WORKSPACE_CONTRACT_VERSION, WorkspaceOrientation,
};

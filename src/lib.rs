#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
// Public typed diagnostics intentionally remain rich values at every deterministic boundary.
#![allow(clippy::result_large_err)]

//! A programming system for coding agents that validates, saves, compiles, and runs typed program revisions.

pub mod application;
pub mod artifact;
mod artifact_io;
mod codec;
mod compile;
mod contract;
mod core_ir;
pub mod diff;
pub mod engine;
pub mod error;
pub mod graph;
pub mod history;
pub mod ids;
pub mod instance;
pub mod interpret;
pub mod machine;
mod machine_contract;
mod managed;
mod ownership;
mod persistence;
pub mod project;
pub mod protocol;
pub mod query;
pub mod release;
pub mod runtime;
pub mod runtime_protocol;
pub mod schema;
pub mod target;
pub mod transaction;
pub mod type_layout;
mod validate;
pub mod workbench;

#[cfg(test)]
mod generated_invariant_tests;

pub use application::{
    APPLICATION_CONTRACT_VERSION, ApplicationBuildReceipt, ApplicationBuildRequest,
    ApplicationDigest, ApplicationFieldValue, ApplicationGraphDigest, ApplicationImport,
    ApplicationInspection, ApplicationInvocation, ApplicationLimits, ApplicationLoadObservation,
    ApplicationReleaseInspection, ApplicationRunReceipt, ApplicationRunResult, ApplicationTarget,
    ApplicationTestCase, ApplicationTestDigest, ApplicationTestExpectation, ApplicationTestReport,
    ApplicationTestResult, ApplicationTestStatus, ApplicationTrap, ApplicationTrapCode,
    ApplicationValue, HostInterface, HostInterfaceId, HostOperation, HostOutcomeClass,
    HostOutcomeRoute, HostRequestRoute, InvocationProfile, StatefulApplicationProfile,
    StatefulCommand, StatefulTransition,
};
pub use error::{ErrorCode, LkError, Result};
pub use history::{
    HistoryPage, REVISION_RECORD_VERSION, RestorationReceipt, RevisionPublicationOutcome,
    RevisionRecord, RevisionRecordInspection, RevisionRecordSummary,
};
pub use ids::{
    ChangeDigest, DraftSymbol, IdempotencyKey, NodeId, NodeIdentityClass, QueryId, RequestId,
    Revision, RevisionRecordDigest, SnapshotHash, WorkspaceId,
};
pub use instance::{
    BlobDigest, CommandId, GrantBinding, HostAdapterInput, HostAdapterKind, HostExecutionReceipt,
    HostGrant, HostGrantDescriptor, HostGrantDigest, INSTANCE_CONTRACT_VERSION,
    INSTANCE_FORMAT_VERSION, InstanceCreateReceipt, InstanceCreateRequest, InstanceDeleteRequest,
    InstanceEventRequest, InstanceFakeHostRequest, InstanceHistoryItem, InstanceHistoryPage,
    InstanceHostRequest, InstanceId, InstanceInspection, InstanceMode,
    InstanceOperationObservation, InstancePolicy, InstanceResumeRequest, InstanceStore,
    InstanceTransitionReceipt, InstanceTransitionStatus, PendingCommand, StateDigest,
};
pub use interpret::{RunPolicy, RuntimeFieldValue, RuntimeValue};
pub use machine::{
    DescribeSchemaRequest, DescribeSchemaResult, MachineSchemaDigest, SchemaProjection, SchemaRoot,
};
pub use project::{
    PROJECT_CHANGE_VERSION, PROJECT_CONTRACT_VERSION, Project, ProjectBackupReceipt,
    ProjectChangeReceipt, ProjectChangeRequest, ProjectContextResult, ProjectDiffPage,
    ProjectHealth, ProjectInitReceipt, ProjectLocatorFacts, ProjectNodeInspection,
    ProjectOrientation, ProjectOrientationResult, ProjectStatus, ProjectTargetReceipt,
    TargetArtifactReceipt, TargetInspection,
};
pub use protocol::{Request, RequestCode, Response, ResponseCode};
pub use release::{
    PreparedRelease, RELEASE_CONTRACT_VERSION, ReleaseBuildReceipt, ReleaseBuildRequest,
    ReleaseContentDigest, ReleaseDependencyInspection, ReleaseDependencyRequest,
    ReleaseExportInspection, ReleaseExportKind, ReleaseExportRequest, ReleaseFieldInspection,
    ReleaseId, ReleaseImportRequest, ReleaseInspection, ReleaseItemId, ReleaseLimits,
    ReleaseSignatureInspection, ReleaseTestCase, ReleaseTestExpectation, ReleaseTestInspection,
    ReleaseTestReport, ReleaseTestResult, ReleaseTestStatus, ReleaseTrap, ReleaseTrapCode,
    ReleaseTypeRef, ReleaseVariantInspection,
};
pub use runtime::{
    RUNTIME_CONTRACT_VERSION, RuntimeCounters, RuntimeInspection, RuntimeInterfaceOrientation,
    RuntimeKernel, RuntimeOrientation, RuntimePolicy, RuntimeResourceState, RuntimeStage,
    RuntimeStageEntry, RuntimeStageObservation,
};
pub use runtime_protocol::{
    RuntimeErrorEnvelope, RuntimeRequest, RuntimeRequestEnvelope, RuntimeResponse,
    RuntimeResponseEnvelope,
};
pub use schema::{
    BlockArgumentDescriptor, BlockArgumentRole, ByteString, DirectReference, LiteralField,
    MatchArm, MatchArmOperationDraft, Node, NodeKind, OperandArity, OperandDescriptor, OperandUse,
    OperationCode, OperationDescriptor, OperationDraft, OperationKind, ProductFieldValue,
    ProductFieldValueDraft, RegionArity, RegionDescriptor, RegionRole, SemanticType, TypeDraft,
    TypeReferenceSlot, TypeRule, ValueDraft, ValueRef,
};
pub use target::{
    ApplicationTargetDefinition, BuildTargetDefinition, BuildTargetKind, ProductTargetDefinition,
    ReleaseTargetDefinition, TARGET_CONTRACT_VERSION, TargetApplicationImport,
    TargetApplicationTestCase, TargetFieldValue, TargetHostOutcomeRoute, TargetHostRequestRoute,
    TargetInvocationProfile, TargetItem, TargetReleaseDependency, TargetStatefulApplicationProfile,
    TargetSummary, TargetTestExpectation, TargetTrap, TargetValue,
};
pub use transaction::{
    ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
    FunctionParameterDraft, MAX_RETURNED_BINDINGS, MAX_STRUCTURED_DRAFT_DEPTH,
    MAX_STRUCTURED_DRAFT_ITEMS, MatchArmDraft, NodeTarget, ProductFieldDraft, SumVariantDraft,
    Transaction, TransactionMode, TransactionOp, TransactionOpCode, TransactionReceipt,
    TransactionResponseSpec, YieldingBodyDraft,
};

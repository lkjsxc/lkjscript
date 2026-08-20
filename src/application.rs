//! Independently transferable exact-release-graph applications.
//!
//! Application format 9 embeds one canonical exact reusable-release graph, exact exported
//! entries, an invocation profile, resource policy, and typed application cases. Source workspace
//! identity, mutable resolver state, proposal syntax, Core IR, and runtime handles are absent.

mod interactive;

pub use interactive::{
    INTERACTIVE_PROFILE_VERSION, InteractiveAction, InteractiveActionKind,
    InteractiveActionOutcome, InteractiveActionOutcomeClass, InteractiveActionRequest,
    InteractiveActionRoute, InteractiveActionRoutes, InteractiveApplicationProfile,
    InteractiveCursorShape, InteractiveEvent, InteractiveEventRoutes,
    InteractiveExecutionObservation, InteractiveFrame, InteractiveKeyCode, InteractiveKeyEvent,
    InteractiveKeyRoutes, InteractiveMouseButton, InteractiveMouseEvent, InteractiveMouseKind,
    InteractiveMouseRoutes, InteractiveOpenEvent, InteractiveOpenRoutes, InteractiveSession,
    InteractiveStep, MAXIMUM_INTERACTIVE_COLUMNS, MAXIMUM_INTERACTIVE_FRAME_SCALARS,
    MAXIMUM_INTERACTIVE_PASTE_SCALARS, MAXIMUM_INTERACTIVE_ROWS, MAXIMUM_INTERACTIVE_STATUS_BYTES,
    PreparedInteractiveApplication, prepare_interactive,
};

use crate::artifact;
use crate::artifact_io;
#[cfg(test)]
use crate::artifact_io::PublicationFault;
use crate::codec::{CodecError, CodecErrorKind, Reader, Writer};
use crate::compile;
use crate::error::{ErrorCode, LkError, Result};
use crate::interpret::{self, RunPolicy, RuntimeFieldValue, RuntimeValue};
use crate::release::graph::{FlattenedGraph, ReleaseGraph};
use crate::release::{self, ReleaseId, ReleaseItemId};
use crate::schema::{
    ByteString, MAXIMUM_BYTE_STRING_BYTES, MAXIMUM_SEQUENCE_ELEMENTS, Node, SemanticType,
    TextString,
};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::path::Path;
use std::time::Instant;

pub const APPLICATION_MAGIC: [u8; 8] = *b"LKJAPP\0\x08";
pub const APPLICATION_FORMAT_VERSION: u16 = 9;
pub const APPLICATION_CONTRACT_VERSION: u16 = 8;
pub const APPLICATION_INTERFACE_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_APPLICATION_ARTIFACT_BYTES: usize = 256 * 1024 * 1024;
pub const MAXIMUM_APPLICATION_TESTS: usize = 256;
pub const MAXIMUM_APPLICATION_TEST_NAME_BYTES: usize = 64;
pub const MAXIMUM_APPLICATION_SUITE_FUEL: u64 = 100_000_000;
pub const MAXIMUM_APPLICATION_PATH_BYTES: usize = artifact_io::MAXIMUM_ARTIFACT_PATH_BYTES;
const MAXIMUM_APPLICATION_VALUE_JSON_BYTES: usize = 64 * 1024 * 1024;
const APPLICATION_DIGEST_DOMAIN: &str = "lkjscript.application-artifact.v8";
const APPLICATION_GRAPH_DIGEST_DOMAIN: &str = "lkjscript.application-release-graph.v1";
const APPLICATION_TEST_DIGEST_DOMAIN: &str = "lkjscript.application-test-case.v2";
const TEMPORARY_PREFIX: &str = ".lkjscript-application-";

fn elapsed_nanoseconds(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationDigest([u8; 32]);

impl ApplicationDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ApplicationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&release::hex(&self.0))
    }
}

impl Serialize for ApplicationDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ApplicationDigest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ReleaseId::deserialize(deserializer).map(|digest| Self(digest.as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationGraphDigest([u8; 32]);

impl ApplicationGraphDigest {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ApplicationGraphDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&release::hex(&self.0))
    }
}

impl Serialize for ApplicationGraphDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationTestDigest([u8; 32]);

impl ApplicationTestDigest {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ApplicationTestDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&release::hex(&self.0))
    }
}

impl Serialize for ApplicationTestDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTarget {
    pub release: ReleaseId,
    pub item: ReleaseItemId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationFieldValue {
    pub field: ApplicationTarget,
    pub value: ApplicationValue,
}

/// A public application value whose nominal identities remain exact release/item pairs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationValue {
    Unit,
    Bool(bool),
    I64(i64),
    Bytes(ByteString),
    Text(TextString),
    Product {
        ty: ApplicationTarget,
        fields: Vec<ApplicationFieldValue>,
    },
    Sum {
        ty: ApplicationTarget,
        variant: ApplicationTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Box<ApplicationValue>>,
    },
    Sequence {
        ty: ApplicationTarget,
        elements: Vec<ApplicationValue>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostInterface {
    ImmutableBlob,
}

impl HostInterface {
    pub const fn contract_name(self) -> &'static str {
        match self {
            Self::ImmutableBlob => "immutable_blob_v1",
        }
    }

    pub fn identity(self) -> HostInterfaceId {
        let mut hasher = blake3::Hasher::new_derive_key("lkjscript.host-interface.identity.v1");
        hasher.update(self.contract_name().as_bytes());
        HostInterfaceId(*hasher.finalize().as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HostInterfaceId([u8; 32]);

impl HostInterfaceId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for HostInterfaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&release::hex(&self.0))
    }
}

impl Serialize for HostInterfaceId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HostInterfaceId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ReleaseId::deserialize(deserializer).map(|digest| Self(digest.as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOperation {
    PutBlob,
    InspectBlob,
}

impl HostOperation {
    pub const fn interface(self) -> HostInterface {
        match self {
            Self::PutBlob | Self::InspectBlob => HostInterface::ImmutableBlob,
        }
    }

    const fn stable_tag(self) -> u8 {
        match self {
            Self::PutBlob => 4,
            Self::InspectBlob => 5,
        }
    }

    const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            4 => Some(Self::PutBlob),
            5 => Some(Self::InspectBlob),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOutcomeClass {
    Succeeded,
    AlreadyPresent,
    KnownFailureBeforeVisibility,
    OutcomeUnknown,
    ReconciliationPresent,
    ReconciliationAbsent,
    ReconciliationIndeterminate,
    CancelledBeforeAction,
    TimeoutBeforeAction,
    TimeoutAfterPossibleVisibility,
    CleanupFailure,
}

impl HostOutcomeClass {
    const fn stable_tag(self) -> u8 {
        match self {
            Self::Succeeded => 1,
            Self::AlreadyPresent => 2,
            Self::KnownFailureBeforeVisibility => 3,
            Self::OutcomeUnknown => 4,
            Self::ReconciliationPresent => 5,
            Self::ReconciliationAbsent => 6,
            Self::ReconciliationIndeterminate => 7,
            Self::CancelledBeforeAction => 8,
            Self::TimeoutBeforeAction => 9,
            Self::TimeoutAfterPossibleVisibility => 10,
            Self::CleanupFailure => 11,
        }
    }

    const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Succeeded),
            2 => Some(Self::AlreadyPresent),
            3 => Some(Self::KnownFailureBeforeVisibility),
            4 => Some(Self::OutcomeUnknown),
            5 => Some(Self::ReconciliationPresent),
            6 => Some(Self::ReconciliationAbsent),
            7 => Some(Self::ReconciliationIndeterminate),
            8 => Some(Self::CancelledBeforeAction),
            9 => Some(Self::TimeoutBeforeAction),
            10 => Some(Self::TimeoutAfterPossibleVisibility),
            11 => Some(Self::CleanupFailure),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostRequestRoute {
    pub variant: ApplicationTarget,
    pub operation: HostOperation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostOutcomeRoute {
    pub operation: HostOperation,
    pub class: HostOutcomeClass,
    pub variant: ApplicationTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationImport {
    pub slot: String,
    pub interface: HostInterface,
    pub request: ApplicationTarget,
    pub outcome: ApplicationTarget,
    pub command_variant: ApplicationTarget,
    pub outcome_variant: ApplicationTarget,
    pub requests: Vec<HostRequestRoute>,
    pub outcomes: Vec<HostOutcomeRoute>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulApplicationProfile {
    pub resume: ApplicationTarget,
    pub query_entry: ApplicationTarget,
    pub state: ApplicationTarget,
    pub event: ApplicationTarget,
    pub response: ApplicationTarget,
    pub query: ApplicationTarget,
    pub query_result: ApplicationTarget,
    pub command: ApplicationTarget,
    pub outcome: ApplicationTarget,
    pub decision: ApplicationTarget,
    pub declined_variant: ApplicationTarget,
    pub declined_payload: ApplicationTarget,
    pub declined_response_field: ApplicationTarget,
    pub unchanged_variant: ApplicationTarget,
    pub unchanged_payload: ApplicationTarget,
    pub unchanged_response_field: ApplicationTarget,
    pub completed_variant: ApplicationTarget,
    pub completed_payload: ApplicationTarget,
    pub completed_state_field: ApplicationTarget,
    pub completed_response_field: ApplicationTarget,
    pub suspended_variant: ApplicationTarget,
    pub suspended_payload: ApplicationTarget,
    pub suspended_state_field: ApplicationTarget,
    pub suspended_response_field: ApplicationTarget,
    pub suspended_command_field: ApplicationTarget,
    pub imports: Vec<ApplicationImport>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[allow(clippy::large_enum_variant)] // Profiles are bounded manifests; indirection adds no consumer value.
pub enum InvocationProfile {
    Typed,
    BytesStream,
    Stateful(StatefulApplicationProfile),
    Interactive(Box<InteractiveApplicationProfile>),
}

impl InvocationProfile {
    const fn stable_tag(&self) -> u8 {
        match self {
            Self::Typed => 1,
            Self::BytesStream => 2,
            Self::Stateful(_) => 3,
            Self::Interactive(_) => 4,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulCommand {
    pub slot: String,
    pub interface: HostInterface,
    pub interface_id: HostInterfaceId,
    pub operation: HostOperation,
    pub request: ApplicationValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatefulDecisionStatus {
    Declined,
    Unchanged,
    Completed,
    Suspended,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulTransition {
    pub status: StatefulDecisionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ApplicationValue>,
    pub response: ApplicationValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<StatefulCommand>,
    pub compile_nanoseconds: u64,
    pub lowering_nanoseconds: u64,
    pub core_verification_nanoseconds: u64,
    pub execute_nanoseconds: u64,
    pub public_value_nanoseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulQueryResult {
    pub result: ApplicationValue,
    pub compile_nanoseconds: u64,
    pub lowering_nanoseconds: u64,
    pub core_verification_nanoseconds: u64,
    pub execute_nanoseconds: u64,
    pub public_value_nanoseconds: u64,
}

/// Operational application-load observation. It is derived, bounded, and never application
/// identity, semantic state, or durable authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationLoadObservation {
    pub application_bytes: u64,
    pub release_count: u64,
    pub flattened_semantic_items: u64,
    pub envelope_decode_nanoseconds: u64,
    pub release_graph_validation_nanoseconds: u64,
    pub closure_flattening_nanoseconds: u64,
    pub canonical_reencode_nanoseconds: u64,
    pub release_tests_nanoseconds: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationTrapCode {
    RuntimeTrap,
    ByteIndexOutOfBounds,
    ByteSliceOutOfBounds,
}

impl ApplicationTrapCode {
    const fn stable_tag(self) -> u8 {
        match self {
            Self::RuntimeTrap => 1,
            Self::ByteIndexOutOfBounds => 2,
            Self::ByteSliceOutOfBounds => 3,
        }
    }

    const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::RuntimeTrap),
            2 => Some(Self::ByteIndexOutOfBounds),
            3 => Some(Self::ByteSliceOutOfBounds),
            _ => None,
        }
    }

    const fn from_error(code: ErrorCode) -> Option<Self> {
        match code {
            ErrorCode::RuntimeTrap => Some(Self::RuntimeTrap),
            ErrorCode::ByteIndexOutOfBounds => Some(Self::ByteIndexOutOfBounds),
            ErrorCode::ByteSliceOutOfBounds => Some(Self::ByteSliceOutOfBounds),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTrap {
    pub code: ApplicationTrapCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ApplicationTarget>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationTestExpectation {
    Value(ApplicationValue),
    Trap(ApplicationTrap),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTestCase {
    pub name: String,
    pub target: ApplicationTarget,
    pub arguments: Vec<ApplicationValue>,
    pub expected: ApplicationTestExpectation,
    pub policy: RunPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationBuildRequest {
    pub version: u16,
    pub root_release: ReleaseId,
    pub entry: ApplicationTarget,
    pub profile: InvocationProfile,
    pub policy: RunPolicy,
    pub tests: Vec<ApplicationTestCase>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationInvocation {
    pub version: u16,
    pub arguments: Vec<ApplicationValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationTestStatus {
    Passed,
    UnexpectedValue,
    UnexpectedTrap,
    MissingTrap,
    WrongTrap,
    InvalidCase,
    Incomplete,
    ResourceFailure,
    EngineFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTestResult {
    pub name: String,
    pub status: ApplicationTestStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_trap: Option<ApplicationTrapCode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTestReport {
    pub total: u64,
    pub passed: u64,
    pub release_total: u64,
    pub application_total: u64,
    pub results: Vec<ApplicationTestResult>,
}

impl ApplicationTestReport {
    pub fn all_passed(&self) -> bool {
        self.total == self.passed
            && self
                .results
                .iter()
                .all(|result| result.status == ApplicationTestStatus::Passed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTestSummary {
    pub name: String,
    pub case_digest: ApplicationTestDigest,
    pub target: ApplicationTarget,
    pub expected: &'static str,
    pub policy: RunPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationReleaseInspection {
    pub release: ReleaseId,
    pub coordinate: String,
    pub user_version: String,
    pub dependencies: Vec<ReleaseId>,
    pub exports: u64,
    pub release_tests: u64,
    pub artifact_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationLimits {
    pub maximum_artifact_bytes: u64,
    pub maximum_release_artifact_bytes: u64,
    pub maximum_graph_nodes: u64,
    pub maximum_graph_edges: u64,
    pub maximum_graph_depth: u64,
    pub maximum_graph_bytes: u64,
    pub maximum_application_tests: u64,
    pub maximum_test_name_bytes: u64,
    pub maximum_suite_fuel: u64,
    pub maximum_value_json_bytes: u64,
    pub maximum_run_arguments: u64,
    pub maximum_run_fuel: u64,
    pub maximum_run_frames: u64,
    pub maximum_runtime_value_depth: u64,
    pub maximum_runtime_value_items: u64,
    pub maximum_runtime_value_bytes: u64,
    pub maximum_stream_input_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationInspection {
    pub contract_version: u16,
    pub format_version: u16,
    pub semantic_schema: &'static str,
    pub digest: ApplicationDigest,
    pub graph_digest: ApplicationGraphDigest,
    pub root_release: ReleaseId,
    pub entry: ApplicationTarget,
    pub profile: InvocationProfile,
    pub policy: RunPolicy,
    pub releases: Vec<ApplicationReleaseInspection>,
    pub graph_edges: u64,
    pub graph_depth: u64,
    pub graph_release_bytes: u64,
    pub flattened_semantic_items: u64,
    pub tests: Vec<ApplicationTestSummary>,
    pub artifact_bytes: u64,
    pub provenance: &'static str,
    pub signatures: &'static str,
    pub limits: ApplicationLimits,
}

/// Artifact-owned public interface facts for a native product boundary.
///
/// This descriptor is derived directly from the validated embedded root release. It is not a
/// binding source, an editable manifest, or a development-workspace identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationInterfaceInspection {
    pub contract_version: u16,
    pub application: ApplicationDigest,
    pub root_release: release::ReleaseInspection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationBuildReceipt {
    pub contract_version: u16,
    pub inspection: ApplicationInspection,
    pub tests: ApplicationTestReport,
    pub published: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRunResult {
    pub value: ApplicationValue,
    pub compile_nanoseconds: u64,
    pub lowering_nanoseconds: u64,
    pub core_verification_nanoseconds: u64,
    pub execute_nanoseconds: u64,
    pub public_value_nanoseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRunReceipt {
    pub contract_version: u16,
    pub digest: ApplicationDigest,
    pub result: ApplicationRunResult,
}

#[derive(Clone, Debug)]
pub struct PreparedApplication {
    bytes: Vec<u8>,
    inspection: ApplicationInspection,
    tests: ApplicationTestReport,
}

impl PreparedApplication {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn receipt(&self, published: bool) -> ApplicationBuildReceipt {
        ApplicationBuildReceipt {
            contract_version: APPLICATION_CONTRACT_VERSION,
            inspection: self.inspection.clone(),
            tests: self.tests.clone(),
            published,
        }
    }

    pub fn publish(&self, path: &Path) -> Result<()> {
        artifact_io::publish(
            path,
            &self.bytes,
            "application artifact",
            TEMPORARY_PREFIX,
            MAXIMUM_APPLICATION_ARTIFACT_BYTES,
        )
    }
}

#[derive(Clone, Debug)]
struct DecodedApplication {
    bytes: Vec<u8>,
    graph: ReleaseGraph,
    flattened: FlattenedGraph,
    entry: ApplicationTarget,
    profile: InvocationProfile,
    policy: RunPolicy,
    tests: Vec<ApplicationTestCase>,
    digest: ApplicationDigest,
}

pub(crate) struct StatefulReplayApplication {
    application: DecodedApplication,
    profile: StatefulApplicationProfile,
    event_entry: crate::ids::NodeId,
    resume_entry: crate::ids::NodeId,
    query_entry: crate::ids::NodeId,
    event_program: interpret::PreparedProgram,
    resume_program: interpret::PreparedProgram,
    query_program: interpret::PreparedProgram,
}

impl StatefulReplayApplication {
    pub(crate) fn transition_event(
        &self,
        state: &ApplicationValue,
        event: &ApplicationValue,
    ) -> Result<StatefulTransition> {
        self.run(
            self.event_entry,
            &self.event_program,
            &[state.clone(), event.clone()],
        )
    }

    pub(crate) fn transition_resume(
        &self,
        state: &ApplicationValue,
        outcome: &ApplicationValue,
    ) -> Result<StatefulTransition> {
        self.run(
            self.resume_entry,
            &self.resume_program,
            &[state.clone(), outcome.clone()],
        )
    }

    pub(crate) fn host_outcome_value(
        &self,
        command: &StatefulCommand,
        class: HostOutcomeClass,
        evidence: &ByteString,
    ) -> Result<ApplicationValue> {
        host_outcome_value_with_profile(&self.profile, command, class, evidence)
    }

    pub(crate) fn query_state(
        &self,
        state: &ApplicationValue,
        query: &ApplicationValue,
    ) -> Result<StatefulQueryResult> {
        let public_value_started = Instant::now();
        let arguments = [state, query]
            .into_iter()
            .map(|value| to_runtime(&self.application.flattened, value))
            .collect::<Result<Vec<_>>>()?;
        let argument_nanoseconds = elapsed_nanoseconds(public_value_started);
        let interpret::RunResult {
            value,
            compile_nanoseconds,
            lowering_nanoseconds,
            core_verification_nanoseconds,
            execute_nanoseconds,
        } = interpret::run_prepared(
            &self.application.flattened.snapshot,
            self.query_entry,
            &self.query_program,
            &arguments,
            self.application.policy,
        )?;
        let result_started = Instant::now();
        let result = from_runtime(&self.application.flattened, &value)?;
        Ok(StatefulQueryResult {
            result,
            compile_nanoseconds,
            lowering_nanoseconds,
            core_verification_nanoseconds,
            execute_nanoseconds,
            public_value_nanoseconds: argument_nanoseconds
                .saturating_add(elapsed_nanoseconds(result_started)),
        })
    }

    pub(crate) fn validate_state(&self, state: &ApplicationValue) -> Result<()> {
        let Node::Function { parameters, .. } =
            self.application.flattened.snapshot.node(self.event_entry)?
        else {
            return Err(LkError::new(
                ErrorCode::CoreIrInvalid,
                "stateful application entry is not a function after validation",
            ));
        };
        let expected = parameter_type(&self.application.flattened.snapshot, parameters[0])?;
        let value = to_runtime(&self.application.flattened, state)?;
        interpret::validate_runtime_value(
            &self.application.flattened.snapshot,
            &value,
            expected,
            self.event_entry,
        )
        .map(|_| ())
    }

    pub(crate) fn digest(&self) -> ApplicationDigest {
        self.application.digest
    }

    pub(crate) fn has_exact_bytes(&self, bytes: &[u8]) -> bool {
        self.application.bytes == bytes
    }

    fn run(
        &self,
        entry: crate::ids::NodeId,
        program: &interpret::PreparedProgram,
        arguments: &[ApplicationValue],
    ) -> Result<StatefulTransition> {
        let public_value_started = Instant::now();
        let arguments = arguments
            .iter()
            .map(|value| to_runtime(&self.application.flattened, value))
            .collect::<Result<Vec<_>>>()?;
        let public_value_nanoseconds = elapsed_nanoseconds(public_value_started);
        let result = interpret::run_prepared(
            &self.application.flattened.snapshot,
            entry,
            program,
            &arguments,
            self.application.policy,
        )?;
        decode_stateful_transition(
            &self.application.flattened,
            self.profile.clone(),
            result,
            public_value_nanoseconds,
        )
    }
}

pub(crate) fn prepare_stateful_replay(bytes: &[u8]) -> Result<StatefulReplayApplication> {
    prepare_stateful_replay_observed(bytes, &mut ApplicationLoadObservation::default())
}

pub(crate) fn prepare_stateful_replay_observed(
    bytes: &[u8],
    observation: &mut ApplicationLoadObservation,
) -> Result<StatefulReplayApplication> {
    let application = decode_application_observed(bytes, observation)?;
    let profile = stateful_profile(&application)?;
    let event_entry = application
        .flattened
        .item(application.entry.release, application.entry.item)?;
    let resume_entry = application
        .flattened
        .item(profile.resume.release, profile.resume.item)?;
    let query_entry = application
        .flattened
        .item(profile.query_entry.release, profile.query_entry.item)?;
    let event_program = interpret::prepare_entry(&application.flattened.snapshot, event_entry)?.0;
    let resume_program = interpret::prepare_entry(&application.flattened.snapshot, resume_entry)?.0;
    let query_program = interpret::prepare_entry(&application.flattened.snapshot, query_entry)?.0;
    Ok(StatefulReplayApplication {
        application,
        profile,
        event_entry,
        resume_entry,
        query_entry,
        event_program,
        resume_program,
        query_program,
    })
}

pub fn prepare(
    request: &ApplicationBuildRequest,
    supplied_release_bytes: &[Vec<u8>],
) -> Result<PreparedApplication> {
    validate_contract_version(request.version)?;
    let graph = decode_graph(request.root_release, supplied_release_bytes)?;
    let tests = normalize_tests(&request.tests)?;
    let profile = normalize_profile(&request.profile)?;
    let flattened = graph.flatten()?;
    validate_manifest(
        &graph,
        &flattened,
        request.entry,
        &profile,
        request.policy,
        &tests,
    )?;
    let entry = flattened.item(request.entry.release, request.entry.item)?;
    compile::compile(&flattened.snapshot, entry)?;
    let bytes = encode_application(&graph, request.entry, &profile, request.policy, &tests)?;
    let decoded = decode_application(&bytes)?;
    let report = run_all_tests(&decoded)?;
    if !report.all_passed() {
        return Err(LkError::new(
            ErrorCode::ApplicationTestFailed,
            "one or more embedded release or application tests did not pass",
        ));
    }
    let inspection = inspection(&decoded)?;
    Ok(PreparedApplication {
        bytes,
        inspection,
        tests: report,
    })
}

fn normalize_profile(profile: &InvocationProfile) -> Result<InvocationProfile> {
    let InvocationProfile::Stateful(stateful) = profile else {
        return Ok(profile.clone());
    };
    let mut stateful = stateful.clone();
    stateful
        .imports
        .sort_by(|left, right| left.slot.cmp(&right.slot));
    for import in &mut stateful.imports {
        import.requests.sort_by_key(|route| route.variant);
        import
            .outcomes
            .sort_by_key(|route| (route.operation, route.class, route.variant));
    }
    Ok(InvocationProfile::Stateful(stateful))
}

pub fn validate(bytes: &[u8]) -> Result<ApplicationInspection> {
    inspect_observed(bytes, &mut ApplicationLoadObservation::default())
}

pub fn inspect(bytes: &[u8]) -> Result<ApplicationInspection> {
    validate(bytes)
}

pub fn inspect_interface(bytes: &[u8]) -> Result<ApplicationInterfaceInspection> {
    let application =
        decode_application_observed(bytes, &mut ApplicationLoadObservation::default())?;
    let root = application.graph.release(application.graph.root())?;
    Ok(ApplicationInterfaceInspection {
        contract_version: APPLICATION_INTERFACE_CONTRACT_VERSION,
        application: application.digest,
        root_release: release::inspection(root)?,
    })
}

pub fn inspect_observed(
    bytes: &[u8],
    observation: &mut ApplicationLoadObservation,
) -> Result<ApplicationInspection> {
    inspection(&decode_application_observed(bytes, observation)?)
}

pub fn test(bytes: &[u8]) -> Result<ApplicationTestReport> {
    test_observed(bytes, &mut ApplicationLoadObservation::default())
}

pub fn test_observed(
    bytes: &[u8],
    observation: &mut ApplicationLoadObservation,
) -> Result<ApplicationTestReport> {
    let application = decode_application_observed(bytes, observation)?;
    let started = Instant::now();
    let result = run_all_tests(&application);
    observation.release_tests_nanoseconds = observation
        .release_tests_nanoseconds
        .saturating_add(elapsed_nanoseconds(started));
    result
}

pub fn run(bytes: &[u8], invocation: &ApplicationInvocation) -> Result<ApplicationRunReceipt> {
    run_observed(
        bytes,
        invocation,
        &mut ApplicationLoadObservation::default(),
    )
}

pub fn run_observed(
    bytes: &[u8],
    invocation: &ApplicationInvocation,
    observation: &mut ApplicationLoadObservation,
) -> Result<ApplicationRunReceipt> {
    validate_contract_version(invocation.version)?;
    let application = decode_application_observed(bytes, observation)?;
    let public_value_started = Instant::now();
    let arguments = invocation
        .arguments
        .iter()
        .map(|value| to_runtime(&application.flattened, value))
        .collect::<Result<Vec<_>>>()?;
    let mut public_value_nanoseconds = elapsed_nanoseconds(public_value_started);
    let entry = application
        .flattened
        .item(application.entry.release, application.entry.item)?;
    let result = interpret::compile_and_run(
        &application.flattened.snapshot,
        entry,
        &arguments,
        application.policy,
    )?;
    let public_value_started = Instant::now();
    let value = from_runtime(&application.flattened, &result.value)?;
    public_value_nanoseconds =
        public_value_nanoseconds.saturating_add(elapsed_nanoseconds(public_value_started));
    Ok(ApplicationRunReceipt {
        contract_version: APPLICATION_CONTRACT_VERSION,
        digest: application.digest,
        result: ApplicationRunResult {
            value,
            compile_nanoseconds: result.compile_nanoseconds,
            lowering_nanoseconds: result.lowering_nanoseconds,
            core_verification_nanoseconds: result.core_verification_nanoseconds,
            execute_nanoseconds: result.execute_nanoseconds,
            public_value_nanoseconds,
        },
    })
}

pub fn run_stream(bytes: &[u8], input: &[u8]) -> Result<Vec<u8>> {
    let application = decode_application(bytes)?;
    if !matches!(application.profile, InvocationProfile::BytesStream) {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "application does not declare the bytes_stream invocation profile",
        ));
    }
    let input = ByteString::from_slice(input).map_err(|_| {
        LkError::new(
            ErrorCode::RuntimeByteInputTooLarge,
            "application stream input exceeds byte policy",
        )
    })?;
    let entry = application
        .flattened
        .item(application.entry.release, application.entry.item)?;
    let result = interpret::compile_and_run(
        &application.flattened.snapshot,
        entry,
        &[RuntimeValue::Bytes(input)],
        application.policy,
    )?;
    let RuntimeValue::Bytes(output) = result.value else {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "bytes_stream application returned a non-bytes value",
        ));
    };
    Ok(output.into_vec())
}

/// Validates one durable state value against the exact stateful application interface.
pub fn validate_stateful_state(bytes: &[u8], state: &ApplicationValue) -> Result<()> {
    let application = decode_application(bytes)?;
    let _ = stateful_profile(&application)?;
    let entry = application
        .flattened
        .item(application.entry.release, application.entry.item)?;
    let Node::Function { parameters, .. } = application.flattened.snapshot.node(entry)? else {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "stateful application entry is not a function after validation",
        ));
    };
    let expected = parameter_type(&application.flattened.snapshot, parameters[0])?;
    let value = to_runtime(&application.flattened, state)?;
    interpret::validate_runtime_value(&application.flattened.snapshot, &value, expected, entry)
        .map(|_| ())
}

/// Evaluates one exact external event without performing host work or publishing state.
pub fn transition_event(
    bytes: &[u8],
    state: &ApplicationValue,
    event: &ApplicationValue,
) -> Result<StatefulTransition> {
    transition_event_observed(
        bytes,
        state,
        event,
        &mut ApplicationLoadObservation::default(),
    )
}

pub fn transition_event_observed(
    bytes: &[u8],
    state: &ApplicationValue,
    event: &ApplicationValue,
    observation: &mut ApplicationLoadObservation,
) -> Result<StatefulTransition> {
    let application = decode_application_observed(bytes, observation)?;
    let profile = stateful_profile(&application)?;
    run_stateful(
        &application,
        application.entry,
        profile,
        &[state.clone(), event.clone()],
    )
}

/// Resumes one suspended transition from a recorded typed host outcome.
pub fn transition_resume(
    bytes: &[u8],
    state: &ApplicationValue,
    outcome: &ApplicationValue,
) -> Result<StatefulTransition> {
    transition_resume_observed(
        bytes,
        state,
        outcome,
        &mut ApplicationLoadObservation::default(),
    )
}

pub fn transition_resume_observed(
    bytes: &[u8],
    state: &ApplicationValue,
    outcome: &ApplicationValue,
    observation: &mut ApplicationLoadObservation,
) -> Result<StatefulTransition> {
    let application = decode_application_observed(bytes, observation)?;
    let profile = stateful_profile(&application)?;
    run_stateful(
        &application,
        profile.resume,
        profile,
        &[state.clone(), outcome.clone()],
    )
}

/// Evaluates one exact pure application query against an already selected state value.
/// This function performs no instance, host, or application publication.
pub fn query_state(
    bytes: &[u8],
    state: &ApplicationValue,
    query: &ApplicationValue,
) -> Result<StatefulQueryResult> {
    query_state_observed(
        bytes,
        state,
        query,
        &mut ApplicationLoadObservation::default(),
    )
}

pub fn query_state_observed(
    bytes: &[u8],
    state: &ApplicationValue,
    query: &ApplicationValue,
    observation: &mut ApplicationLoadObservation,
) -> Result<StatefulQueryResult> {
    let application = decode_application_observed(bytes, observation)?;
    let profile = stateful_profile(&application)?;
    let target = application
        .flattened
        .item(profile.query_entry.release, profile.query_entry.item)?;
    let public_value_started = Instant::now();
    let arguments = [state, query]
        .into_iter()
        .map(|value| to_runtime(&application.flattened, value))
        .collect::<Result<Vec<_>>>()?;
    let argument_nanoseconds = elapsed_nanoseconds(public_value_started);
    let interpret::RunResult {
        value,
        compile_nanoseconds,
        lowering_nanoseconds,
        core_verification_nanoseconds,
        execute_nanoseconds,
    } = interpret::compile_and_run(
        &application.flattened.snapshot,
        target,
        &arguments,
        application.policy,
    )?;
    let result_started = Instant::now();
    let result = from_runtime(&application.flattened, &value)?;
    Ok(StatefulQueryResult {
        result,
        compile_nanoseconds,
        lowering_nanoseconds,
        core_verification_nanoseconds,
        execute_nanoseconds,
        public_value_nanoseconds: argument_nanoseconds
            .saturating_add(elapsed_nanoseconds(result_started)),
    })
}

fn stateful_profile(application: &DecodedApplication) -> Result<StatefulApplicationProfile> {
    match &application.profile {
        InvocationProfile::Stateful(profile) => Ok(profile.clone()),
        _ => Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "application does not declare the stateful invocation profile",
        )),
    }
}

fn run_stateful(
    application: &DecodedApplication,
    target: ApplicationTarget,
    profile: StatefulApplicationProfile,
    arguments: &[ApplicationValue],
) -> Result<StatefulTransition> {
    let target_node = application.flattened.item(target.release, target.item)?;
    let public_value_started = Instant::now();
    let arguments = arguments
        .iter()
        .map(|value| to_runtime(&application.flattened, value))
        .collect::<Result<Vec<_>>>()?;
    let public_value_nanoseconds = elapsed_nanoseconds(public_value_started);
    let result = interpret::compile_and_run(
        &application.flattened.snapshot,
        target_node,
        &arguments,
        application.policy,
    )?;
    decode_stateful_transition(
        &application.flattened,
        profile,
        result,
        public_value_nanoseconds,
    )
}

fn decode_stateful_transition(
    flattened: &FlattenedGraph,
    profile: StatefulApplicationProfile,
    result: interpret::RunResult,
    public_value_nanoseconds: u64,
) -> Result<StatefulTransition> {
    let interpret::RunResult {
        value,
        compile_nanoseconds,
        lowering_nanoseconds,
        core_verification_nanoseconds,
        execute_nanoseconds,
    } = result;
    let public_value_started = Instant::now();
    let value = from_runtime(flattened, &value)?;
    let ApplicationValue::Sum {
        ty,
        variant,
        payload,
    } = value
    else {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "stateful application returned a non-sum decision",
        ));
    };
    if ty != profile.decision {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "stateful application returned the wrong decision type",
        ));
    }
    let payload = payload.ok_or_else(|| corrupt("decision payload is absent"))?;
    let ApplicationValue::Product {
        ty: payload_type,
        fields,
    } = *payload
    else {
        return Err(corrupt("decision variant payload is not a product"));
    };
    let (status, expected_payload, expected_fields) = if variant == profile.declined_variant {
        (
            StatefulDecisionStatus::Declined,
            profile.declined_payload,
            vec![profile.declined_response_field],
        )
    } else if variant == profile.unchanged_variant {
        (
            StatefulDecisionStatus::Unchanged,
            profile.unchanged_payload,
            vec![profile.unchanged_response_field],
        )
    } else if variant == profile.completed_variant {
        (
            StatefulDecisionStatus::Completed,
            profile.completed_payload,
            vec![
                profile.completed_state_field,
                profile.completed_response_field,
            ],
        )
    } else if variant == profile.suspended_variant {
        (
            StatefulDecisionStatus::Suspended,
            profile.suspended_payload,
            vec![
                profile.suspended_state_field,
                profile.suspended_response_field,
                profile.suspended_command_field,
            ],
        )
    } else {
        return Err(corrupt(
            "decision uses an undeclared declined/unchanged/completed/suspended variant",
        ));
    };
    if payload_type != expected_payload || fields.len() != expected_fields.len() {
        return Err(corrupt("decision payload has the wrong exact product type"));
    }
    let mut values = Vec::with_capacity(fields.len());
    for (field, expected) in fields.into_iter().zip(expected_fields) {
        if field.field != expected {
            return Err(corrupt("decision payload fields are out of contract order"));
        }
        values.push(field.value);
    }
    let mut values = values.into_iter();
    let state = if matches!(
        status,
        StatefulDecisionStatus::Completed | StatefulDecisionStatus::Suspended
    ) {
        Some(
            values
                .next()
                .ok_or_else(|| corrupt("publishing decision state is absent"))?,
        )
    } else {
        None
    };
    let response = values
        .next()
        .ok_or_else(|| corrupt("decision response is absent"))?;
    let command = if status == StatefulDecisionStatus::Suspended {
        Some(decode_stateful_command(
            &profile,
            values
                .next()
                .ok_or_else(|| corrupt("suspended decision command is absent"))?,
        )?)
    } else {
        None
    };
    let public_value_nanoseconds =
        public_value_nanoseconds.saturating_add(elapsed_nanoseconds(public_value_started));
    Ok(StatefulTransition {
        status,
        state,
        response,
        command,
        compile_nanoseconds,
        lowering_nanoseconds,
        core_verification_nanoseconds,
        execute_nanoseconds,
        public_value_nanoseconds,
    })
}

fn decode_stateful_command(
    profile: &StatefulApplicationProfile,
    value: ApplicationValue,
) -> Result<StatefulCommand> {
    let ApplicationValue::Sum {
        ty,
        variant,
        payload,
    } = value
    else {
        return Err(corrupt("suspended decision command is not a nominal sum"));
    };
    if ty != profile.command {
        return Err(corrupt(
            "suspended decision command has the wrong nominal type",
        ));
    }
    let import = profile
        .imports
        .iter()
        .find(|import| import.command_variant == variant)
        .ok_or_else(|| corrupt("command variant does not route to a declared import slot"))?;
    let request = payload.ok_or_else(|| corrupt("command wrapper request is absent"))?;
    let ApplicationValue::Sum {
        ty: request_type,
        variant: request_variant,
        payload: request_payload,
    } = request.as_ref()
    else {
        return Err(corrupt("host-interface request is not a nominal sum"));
    };
    if *request_type != import.request {
        return Err(corrupt("host-interface request has the wrong nominal type"));
    }
    let route = import
        .requests
        .iter()
        .find(|route| route.variant == *request_variant)
        .ok_or_else(|| corrupt("host-interface request variant has no exact route"))?;
    validate_request_payload(route.operation, request_payload.as_deref())?;
    Ok(StatefulCommand {
        slot: import.slot.clone(),
        interface: import.interface,
        interface_id: import.interface.identity(),
        operation: route.operation,
        request: *request,
    })
}

/// Returns the exact bounded byte payload carried by one validated interface request.
pub fn host_request_bytes(command: &StatefulCommand) -> Result<&ByteString> {
    let ApplicationValue::Sum { payload, .. } = &command.request else {
        return Err(corrupt("validated host request lost its nominal sum shape"));
    };
    let Some(payload) = payload.as_deref() else {
        return Err(corrupt("validated host request lost its byte payload"));
    };
    let ApplicationValue::Bytes(bytes) = payload else {
        return Err(corrupt("validated host request payload is not bytes"));
    };
    Ok(bytes)
}

/// Constructs the exact application-visible nominal outcome for a retained host result.
pub fn host_outcome_value(
    bytes: &[u8],
    command: &StatefulCommand,
    class: HostOutcomeClass,
    evidence: &ByteString,
) -> Result<ApplicationValue> {
    let application = decode_application(bytes)?;
    let profile = stateful_profile(&application)?;
    host_outcome_value_with_profile(&profile, command, class, evidence)
}

fn host_outcome_value_with_profile(
    profile: &StatefulApplicationProfile,
    command: &StatefulCommand,
    class: HostOutcomeClass,
    evidence: &ByteString,
) -> Result<ApplicationValue> {
    let import = profile
        .imports
        .iter()
        .find(|import| {
            import.slot == command.slot
                && import.interface == command.interface
                && import.interface.identity() == command.interface_id
        })
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::CapabilityDenied,
                "pending command does not name an exact application import",
            )
        })?;
    let route = import
        .outcomes
        .iter()
        .find(|route| route.operation == command.operation && route.class == class)
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "host outcome class is incompatible with the pending interface operation",
            )
        })?;
    let payload = if outcome_requires_evidence(command.operation, class) {
        if evidence.is_empty() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "host outcome requires exact evidence bytes",
            ));
        }
        Some(Box::new(ApplicationValue::Bytes(evidence.clone())))
    } else {
        if !evidence.is_empty() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "host outcome class requires empty evidence",
            ));
        }
        None
    };
    let interface_outcome = ApplicationValue::Sum {
        ty: import.outcome,
        variant: route.variant,
        payload,
    };
    Ok(ApplicationValue::Sum {
        ty: profile.outcome,
        variant: import.outcome_variant,
        payload: Some(Box::new(interface_outcome)),
    })
}

pub const fn host_outcome_is_compatible(operation: HostOperation, class: HostOutcomeClass) -> bool {
    match operation {
        HostOperation::InspectBlob => matches!(
            class,
            HostOutcomeClass::ReconciliationPresent
                | HostOutcomeClass::ReconciliationAbsent
                | HostOutcomeClass::ReconciliationIndeterminate
        ),
        HostOperation::PutBlob => matches!(
            class,
            HostOutcomeClass::Succeeded
                | HostOutcomeClass::AlreadyPresent
                | HostOutcomeClass::KnownFailureBeforeVisibility
                | HostOutcomeClass::OutcomeUnknown
                | HostOutcomeClass::CancelledBeforeAction
                | HostOutcomeClass::TimeoutBeforeAction
                | HostOutcomeClass::TimeoutAfterPossibleVisibility
                | HostOutcomeClass::CleanupFailure
        ),
    }
}

const fn outcome_requires_evidence(operation: HostOperation, class: HostOutcomeClass) -> bool {
    host_outcome_is_compatible(operation, class)
}

fn validate_request_payload(
    operation: HostOperation,
    payload: Option<&ApplicationValue>,
) -> Result<()> {
    let Some(ApplicationValue::Bytes(bytes)) = payload else {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "host-interface request variant must carry exactly one bytes payload",
        ));
    };
    if operation == HostOperation::InspectBlob && bytes.len() != 32 {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "blob inspection requests require a 32-byte identity",
        ));
    }
    Ok(())
}

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    artifact_io::read_file(
        path,
        "application artifact",
        MAXIMUM_APPLICATION_ARTIFACT_BYTES,
    )
}

fn decode_graph(root: ReleaseId, supplied: &[Vec<u8>]) -> Result<ReleaseGraph> {
    if supplied.is_empty() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "application build requires an explicit exact release graph",
        ));
    }
    let mut decoded = supplied
        .iter()
        .map(|bytes| release::decode(bytes))
        .collect::<Result<Vec<_>>>()?;
    let position = decoded
        .iter()
        .position(|release| release.id == root)
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!("application root release {root} was not supplied"),
            )
        })?;
    let root = decoded.remove(position);
    ReleaseGraph::new(root, decoded)
}

fn normalize_tests(tests: &[ApplicationTestCase]) -> Result<Vec<ApplicationTestCase>> {
    if tests.is_empty() || tests.len() > MAXIMUM_APPLICATION_TESTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application test count is outside policy",
        ));
    }
    let mut tests = tests.to_vec();
    tests.sort_by(|left, right| left.name.cmp(&right.name));
    for test in &tests {
        validate_test_name(&test.name)?;
    }
    if tests.windows(2).any(|pair| pair[0].name == pair[1].name) {
        return Err(LkError::new(
            ErrorCode::DuplicateName,
            "application test names must be unique",
        ));
    }
    Ok(tests)
}

fn validate_test_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name.len() <= MAXIMUM_APPLICATION_TEST_NAME_BYTES
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "application test name must match [a-z][a-z0-9_]* within policy",
        ))
    }
}

fn validate_manifest(
    graph: &ReleaseGraph,
    flattened: &FlattenedGraph,
    entry: ApplicationTarget,
    profile: &InvocationProfile,
    policy: RunPolicy,
    tests: &[ApplicationTestCase],
) -> Result<()> {
    if entry.release != graph.root() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "application entry must be an export of the root release",
        ));
    }
    validate_exported_function(graph, entry)?;
    interpret::validate_policy(policy)?;
    let entry_node = flattened.item(entry.release, entry.item)?;
    validate_profile(graph, flattened, entry_node, profile)?;
    if !tests.iter().any(|test| test.target == entry) {
        return Err(LkError::new(
            ErrorCode::ApplicationTestFailed,
            "at least one application test must target the exact entry export",
        ));
    }
    if let InvocationProfile::Stateful(stateful) = profile {
        if !tests.iter().any(|test| test.target == stateful.resume) {
            return Err(LkError::new(
                ErrorCode::ApplicationTestFailed,
                "a stateful application requires at least one exact resume-entry test",
            ));
        }
        if !tests.iter().any(|test| test.target == stateful.query_entry) {
            return Err(LkError::new(
                ErrorCode::ApplicationTestFailed,
                "a stateful application requires at least one exact query-entry test",
            ));
        }
        let mut covered_decisions = [false; 4];
        for test in tests
            .iter()
            .filter(|test| test.target == entry || test.target == stateful.resume)
        {
            let ApplicationTestExpectation::Value(ApplicationValue::Sum { ty, variant, .. }) =
                &test.expected
            else {
                continue;
            };
            if *ty != stateful.decision {
                continue;
            }
            if *variant == stateful.declined_variant {
                covered_decisions[0] = true;
            } else if *variant == stateful.unchanged_variant {
                covered_decisions[1] = true;
            } else if *variant == stateful.completed_variant {
                covered_decisions[2] = true;
            } else if *variant == stateful.suspended_variant {
                covered_decisions[3] = true;
            }
        }
        if covered_decisions.iter().any(|covered| !covered) {
            return Err(LkError::new(
                ErrorCode::ApplicationTestFailed,
                "a stateful application requires exact declined, unchanged, completed, and suspended decision tests",
            ));
        }
    }
    let mut total_fuel = 0_u64;
    for test in tests {
        validate_exported_function(graph, test.target)?;
        interpret::validate_policy(test.policy)?;
        total_fuel = total_fuel.checked_add(test.policy.fuel).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "application suite fuel overflows",
            )
        })?;
        if total_fuel > MAXIMUM_APPLICATION_SUITE_FUEL {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "application suite fuel exceeds policy",
            ));
        }
        let target = flattened.item(test.target.release, test.target.item)?;
        let arguments = test
            .arguments
            .iter()
            .map(|value| to_runtime(flattened, value))
            .collect::<Result<Vec<_>>>()?;
        let result_type = interpret::validate_invocation(&flattened.snapshot, target, &arguments)?;
        match &test.expected {
            ApplicationTestExpectation::Value(value) => {
                let value = to_runtime(flattened, value)?;
                interpret::validate_runtime_value(
                    &flattened.snapshot,
                    &value,
                    result_type,
                    target,
                )?;
            }
            ApplicationTestExpectation::Trap(trap) => {
                if let Some(target) = trap.target {
                    let _ = flattened.item(target.release, target.item)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_exported_function(graph: &ReleaseGraph, target: ApplicationTarget) -> Result<()> {
    let release = graph.release(target.release)?;
    if !release.exports.iter().any(|export| {
        export.target == target.item && export.kind == release::ReleaseExportKind::Function
    }) {
        return Err(LkError::new(
            ErrorCode::NodeNotFound,
            format!(
                "application target {}:{} is not an exported function",
                target.release,
                target.item.get()
            ),
        ));
    }
    Ok(())
}

fn validate_profile(
    graph: &ReleaseGraph,
    flattened: &FlattenedGraph,
    entry: crate::ids::NodeId,
    profile: &InvocationProfile,
) -> Result<()> {
    let snapshot = &flattened.snapshot;
    let Node::Function {
        parameters, result, ..
    } = snapshot.node(entry)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "application entry must be a function",
        ));
    };
    match profile {
        InvocationProfile::Typed => Ok(()),
        InvocationProfile::BytesStream => {
            if parameters.len() != 1 || *result != SemanticType::Bytes {
                return Err(LkError::new(
                    ErrorCode::RunArgumentMismatch,
                    "bytes_stream entry must accept one bytes value and return bytes",
                ));
            }
            let Node::Parameter { ty, .. } = snapshot.node(parameters[0])? else {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "bytes_stream entry parameter is malformed",
                ));
            };
            if *ty != SemanticType::Bytes {
                return Err(LkError::new(
                    ErrorCode::RunArgumentMismatch,
                    "bytes_stream entry parameter must have bytes type",
                ));
            }
            Ok(())
        }
        InvocationProfile::Stateful(stateful) => {
            validate_stateful_profile(graph, flattened, parameters, *result, stateful)
        }
        InvocationProfile::Interactive(profile) => {
            interactive::validate_profile(graph, flattened, entry, profile)
        }
    }
}

fn validate_stateful_profile(
    graph: &ReleaseGraph,
    flattened: &FlattenedGraph,
    event_parameters: &[crate::ids::NodeId],
    event_result: SemanticType,
    profile: &StatefulApplicationProfile,
) -> Result<()> {
    let snapshot = &flattened.snapshot;
    if event_parameters.len() != 2 {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "stateful event entry must accept exact state and event values",
        ));
    }
    let state = flattened.item(profile.state.release, profile.state.item)?;
    let event = flattened.item(profile.event.release, profile.event.item)?;
    let response = flattened.item(profile.response.release, profile.response.item)?;
    let query = flattened.item(profile.query.release, profile.query.item)?;
    let query_result = flattened.item(profile.query_result.release, profile.query_result.item)?;
    let command = flattened.item(profile.command.release, profile.command.item)?;
    let outcome = flattened.item(profile.outcome.release, profile.outcome.item)?;
    let decision = flattened.item(profile.decision.release, profile.decision.item)?;
    let state_type = parameter_type(snapshot, event_parameters[0])?;
    let event_type = parameter_type(snapshot, event_parameters[1])?;
    if state_type != SemanticType::Nominal(state)
        || event_type != SemanticType::Nominal(event)
        || !matches!(
            snapshot.node(state)?,
            Node::ProductType { .. } | Node::SumType { .. }
        )
        || !matches!(
            snapshot.node(event)?,
            Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. }
        )
    {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful event entry must accept the exact declared nominal state and event types",
        ));
    }
    if event_result != SemanticType::Nominal(decision) {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful event entry must return the declared decision sum",
        ));
    }
    let Node::SumType { variants, .. } = snapshot.node(decision)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "stateful decision must be a nominal sum",
        ));
    };
    for target in [response, query, query_result] {
        if !matches!(
            snapshot.node(target)?,
            Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. }
        ) {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "stateful response, query, and query-result targets must be nominal declarations",
            ));
        }
    }
    let declined_variant = flattened.item(
        profile.declined_variant.release,
        profile.declined_variant.item,
    )?;
    let unchanged_variant = flattened.item(
        profile.unchanged_variant.release,
        profile.unchanged_variant.item,
    )?;
    let completed_variant = flattened.item(
        profile.completed_variant.release,
        profile.completed_variant.item,
    )?;
    let suspended_variant = flattened.item(
        profile.suspended_variant.release,
        profile.suspended_variant.item,
    )?;
    if variants.as_slice()
        != [
            declined_variant,
            unchanged_variant,
            completed_variant,
            suspended_variant,
        ]
    {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "stateful decision must declare declined, unchanged, completed, then suspended exactly once",
        ));
    }
    validate_decision_variant(
        flattened,
        declined_variant,
        profile.declined_payload,
        &[(
            profile.declined_response_field,
            SemanticType::Nominal(response),
        )],
    )?;
    validate_decision_variant(
        flattened,
        unchanged_variant,
        profile.unchanged_payload,
        &[(
            profile.unchanged_response_field,
            SemanticType::Nominal(response),
        )],
    )?;
    validate_decision_variant(
        flattened,
        completed_variant,
        profile.completed_payload,
        &[
            (profile.completed_state_field, SemanticType::Nominal(state)),
            (
                profile.completed_response_field,
                SemanticType::Nominal(response),
            ),
        ],
    )?;
    validate_decision_variant(
        flattened,
        suspended_variant,
        profile.suspended_payload,
        &[
            (profile.suspended_state_field, SemanticType::Nominal(state)),
            (
                profile.suspended_response_field,
                SemanticType::Nominal(response),
            ),
            (
                profile.suspended_command_field,
                SemanticType::Nominal(command),
            ),
        ],
    )?;

    let Node::SumType {
        variants: command_variants,
        ..
    } = snapshot.node(command)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "stateful command type must be a nominal sum",
        ));
    };
    let Node::SumType {
        variants: outcome_variants,
        ..
    } = snapshot.node(outcome)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "stateful outcome type must be a nominal sum",
        ));
    };
    validate_imports(
        flattened,
        profile,
        command,
        command_variants,
        outcome,
        outcome_variants,
    )?;

    validate_exported_function(graph, profile.resume)?;
    let resume = flattened.item(profile.resume.release, profile.resume.item)?;
    let Node::Function {
        parameters, result, ..
    } = snapshot.node(resume)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "stateful resume target must be a function",
        ));
    };
    if parameters.len() != 2
        || parameter_type(snapshot, parameters[0])? != SemanticType::Nominal(state)
        || parameter_type(snapshot, parameters[1])? != SemanticType::Nominal(outcome)
        || *result != SemanticType::Nominal(decision)
    {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful resume entry must accept exact state and outcome values and return the decision sum",
        ));
    }

    validate_exported_function(graph, profile.query_entry)?;
    let query_entry = flattened.item(profile.query_entry.release, profile.query_entry.item)?;
    let Node::Function {
        parameters, result, ..
    } = snapshot.node(query_entry)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "stateful query target must be a function",
        ));
    };
    if parameters.len() != 2
        || parameter_type(snapshot, parameters[0])? != SemanticType::Nominal(state)
        || parameter_type(snapshot, parameters[1])? != SemanticType::Nominal(query)
        || *result != SemanticType::Nominal(query_result)
    {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful query entry must accept exact state and query values and return the exact query-result type",
        ));
    }
    Ok(())
}

fn validate_decision_variant(
    flattened: &FlattenedGraph,
    variant: crate::ids::NodeId,
    payload_target: ApplicationTarget,
    expected_fields: &[(ApplicationTarget, SemanticType)],
) -> Result<()> {
    let snapshot = &flattened.snapshot;
    let payload = flattened.item(payload_target.release, payload_target.item)?;
    let Node::SumVariant {
        payload: Some(variant_payload),
        ..
    } = snapshot.node(variant)?
    else {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "completed and suspended decision variants require product payloads",
        ));
    };
    if *variant_payload != SemanticType::Nominal(payload) {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "decision variant payload does not match its declared exact product",
        ));
    }
    let Node::ProductType { fields, .. } = snapshot.node(payload)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "decision payload must be a nominal product",
        ));
    };
    let mapped = expected_fields
        .iter()
        .map(|(target, _)| flattened.item(target.release, target.item))
        .collect::<Result<Vec<_>>>()?;
    if fields != &mapped {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "decision payload fields are not in exact declared order",
        ));
    }
    for ((_, expected), field) in expected_fields.iter().zip(mapped) {
        let Node::ProductField { owner, ty, .. } = snapshot.node(field)? else {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "decision field mapping does not target a product field",
            ));
        };
        if *owner != payload || *ty != *expected {
            return Err(LkError::new(
                ErrorCode::TypeMismatch,
                "decision field has the wrong owner or exact type",
            ));
        }
    }
    Ok(())
}

fn validate_imports(
    flattened: &FlattenedGraph,
    profile: &StatefulApplicationProfile,
    command: crate::ids::NodeId,
    command_variants: &[crate::ids::NodeId],
    outcome: crate::ids::NodeId,
    outcome_variants: &[crate::ids::NodeId],
) -> Result<()> {
    let snapshot = &flattened.snapshot;
    if profile.imports.len() > 64 {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application import count exceeds policy",
        ));
    }
    let mut prior_slot: Option<&str> = None;
    let mut mapped_commands = Vec::new();
    let mut mapped_outcomes = Vec::new();
    for import in &profile.imports {
        validate_slot_name(&import.slot)?;
        if prior_slot.is_some_and(|prior| prior >= import.slot.as_str()) {
            return Err(LkError::new(
                ErrorCode::DuplicateName,
                "application import slots must be in strict canonical order",
            ));
        }
        prior_slot = Some(&import.slot);
        let request = flattened.item(import.request.release, import.request.item)?;
        let interface_outcome = flattened.item(import.outcome.release, import.outcome.item)?;
        let command_variant =
            flattened.item(import.command_variant.release, import.command_variant.item)?;
        let outcome_variant =
            flattened.item(import.outcome_variant.release, import.outcome_variant.item)?;
        validate_wrapper_variant(snapshot, command_variant, command, request)?;
        validate_wrapper_variant(snapshot, outcome_variant, outcome, interface_outcome)?;
        mapped_commands.push(command_variant);
        mapped_outcomes.push(outcome_variant);
        let Node::SumType {
            variants: request_variants,
            ..
        } = snapshot.node(request)?
        else {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "host-interface request type must be a nominal sum",
            ));
        };
        let Node::SumType {
            variants: interface_outcome_variants,
            ..
        } = snapshot.node(interface_outcome)?
        else {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "host-interface outcome type must be a nominal sum",
            ));
        };
        if import.requests.len() != request_variants.len() || import.requests.is_empty() {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "every interface request variant must have one route",
            ));
        }
        let mut routed_requests = Vec::new();
        for route in &import.requests {
            if route.operation.interface() != import.interface {
                return Err(LkError::new(
                    ErrorCode::TypeMismatch,
                    "request operation belongs to another host interface",
                ));
            }
            let variant = flattened.item(route.variant.release, route.variant.item)?;
            validate_bytes_variant(snapshot, variant, request)?;
            routed_requests.push(variant);
        }
        if routed_requests != *request_variants {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "interface request routes must cover variants in declaration order",
            ));
        }
        let mut routed_outcomes = std::collections::BTreeSet::new();
        let mut prior_outcome = None;
        for route in &import.outcomes {
            if route.operation.interface() != import.interface
                || !host_outcome_is_compatible(route.operation, route.class)
            {
                return Err(LkError::new(
                    ErrorCode::TypeMismatch,
                    "outcome mapping is incompatible with its host interface operation",
                ));
            }
            let order = (route.operation, route.class, route.variant);
            if prior_outcome.is_some_and(|prior| prior >= order) {
                return Err(LkError::new(
                    ErrorCode::DuplicateName,
                    "interface outcome routes must be in strict canonical order",
                ));
            }
            prior_outcome = Some(order);
            let variant = flattened.item(route.variant.release, route.variant.item)?;
            if outcome_requires_evidence(route.operation, route.class) {
                validate_bytes_variant(snapshot, variant, interface_outcome)?;
            } else {
                validate_empty_variant(snapshot, variant, interface_outcome)?;
            }
            routed_outcomes.insert(variant);
        }
        if routed_outcomes.len() != interface_outcome_variants.len()
            || interface_outcome_variants
                .iter()
                .any(|variant| !routed_outcomes.contains(variant))
        {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "every interface outcome variant must be used by one or more exact class routes",
            ));
        }
    }
    if mapped_commands != command_variants || mapped_outcomes != outcome_variants {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "command and outcome wrapper variants must match import-slot order exactly",
        ));
    }
    Ok(())
}

fn validate_slot_name(slot: &str) -> Result<()> {
    let valid = !slot.is_empty()
        && slot.len() <= 64
        && slot.as_bytes()[0].is_ascii_lowercase()
        && slot
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "application import slot must match [a-z][a-z0-9_]* within 64 bytes",
        ))
    }
}

fn validate_wrapper_variant(
    snapshot: &crate::graph::Snapshot,
    variant: crate::ids::NodeId,
    owner: crate::ids::NodeId,
    payload: crate::ids::NodeId,
) -> Result<()> {
    match snapshot.node(variant)? {
        Node::SumVariant {
            owner: actual_owner,
            payload: Some(SemanticType::Nominal(actual_payload)),
            ..
        } if *actual_owner == owner && *actual_payload == payload => Ok(()),
        _ => Err(LkError::new(
            ErrorCode::TypeMismatch,
            "interface wrapper variant has the wrong owner or nominal payload",
        )),
    }
}

fn validate_bytes_variant(
    snapshot: &crate::graph::Snapshot,
    variant: crate::ids::NodeId,
    owner: crate::ids::NodeId,
) -> Result<()> {
    match snapshot.node(variant)? {
        Node::SumVariant {
            owner: actual_owner,
            payload: Some(SemanticType::Bytes),
            ..
        } if *actual_owner == owner => Ok(()),
        _ => Err(LkError::new(
            ErrorCode::TypeMismatch,
            "interface variant must carry bytes evidence or request payload",
        )),
    }
}

fn validate_empty_variant(
    snapshot: &crate::graph::Snapshot,
    variant: crate::ids::NodeId,
    owner: crate::ids::NodeId,
) -> Result<()> {
    match snapshot.node(variant)? {
        Node::SumVariant {
            owner: actual_owner,
            payload: None,
            ..
        } if *actual_owner == owner => Ok(()),
        _ => Err(LkError::new(
            ErrorCode::TypeMismatch,
            "interface outcome variant must carry no payload for this result class",
        )),
    }
}

fn parameter_type(
    snapshot: &crate::graph::Snapshot,
    parameter: crate::ids::NodeId,
) -> Result<SemanticType> {
    let Node::Parameter { ty, .. } = snapshot.node(parameter)? else {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "application function parameter is malformed",
        ));
    };
    Ok(*ty)
}

fn encode_application(
    graph: &ReleaseGraph,
    entry: ApplicationTarget,
    profile: &InvocationProfile,
    policy: RunPolicy,
    tests: &[ApplicationTestCase],
) -> Result<Vec<u8>> {
    let mut payload = Writer::new();
    payload.fixed(&graph.root().as_bytes());
    put_target(&mut payload, entry);
    put_profile(&mut payload, profile)?;
    put_policy(&mut payload, policy);
    put_count(&mut payload, tests.len())?;
    for test in tests {
        put_test(&mut payload, test)?;
    }
    put_count(&mut payload, graph.releases().len())?;
    for release in graph.releases() {
        payload.bytes(&release.bytes).map_err(application_codec)?;
    }
    let payload = payload.finish();
    if payload.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application payload exceeds byte policy",
        ));
    }
    let digest = digest_payload(&payload);
    let mut writer = Writer::with_capacity(
        APPLICATION_MAGIC.len() + 2 + artifact::SCHEMA_ID.0.len() + 8 + payload.len() + 32,
    );
    writer.fixed(&APPLICATION_MAGIC);
    writer.u16(APPLICATION_FORMAT_VERSION);
    writer.fixed(&artifact::SCHEMA_ID.0);
    put_count(&mut writer, payload.len())?;
    writer.fixed(&payload);
    writer.fixed(&digest.as_bytes());
    let bytes = writer.finish();
    if bytes.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact exceeds byte policy",
        ));
    }
    Ok(bytes)
}

fn decode_application(bytes: &[u8]) -> Result<DecodedApplication> {
    decode_application_observed(bytes, &mut ApplicationLoadObservation::default())
}

fn decode_application_observed(
    bytes: &[u8],
    observation: &mut ApplicationLoadObservation,
) -> Result<DecodedApplication> {
    observation.application_bytes = observation
        .application_bytes
        .saturating_add(bytes.len() as u64);
    let envelope_started = Instant::now();
    if bytes.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact exceeds decoder byte policy",
        ));
    }
    let mut reader = Reader::new(bytes);
    if reader
        .fixed(APPLICATION_MAGIC.len())
        .map_err(application_codec)?
        != APPLICATION_MAGIC
    {
        return Err(corrupt("application artifact magic is invalid"));
    }
    if reader.u16().map_err(application_codec)? != APPLICATION_FORMAT_VERSION {
        return Err(corrupt(
            "application artifact format version is unsupported",
        ));
    }
    if reader
        .fixed(artifact::SCHEMA_ID.0.len())
        .map_err(application_codec)?
        != artifact::SCHEMA_ID.0
    {
        return Err(corrupt(
            "application artifact semantic schema identity is unsupported",
        ));
    }
    let payload_length = usize::try_from(reader.u64().map_err(application_codec)?)
        .map_err(|_| corrupt("application payload length overflows host indexes"))?;
    if payload_length > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application payload exceeds decoder byte policy",
        ));
    }
    let payload = reader.fixed(payload_length).map_err(application_codec)?;
    let encoded_digest = reader.fixed(32).map_err(application_codec)?;
    reader.finish().map_err(application_codec)?;
    let digest = digest_payload(payload);
    if encoded_digest != digest.as_bytes() {
        return Err(corrupt("application artifact content digest is invalid"));
    }

    let mut payload_reader = Reader::new(payload);
    let root_release = ReleaseId::from_bytes(read_digest(&mut payload_reader)?);
    let entry = read_target(&mut payload_reader)?;
    let profile = read_profile(&mut payload_reader)?;
    if normalize_profile(&profile).map_err(decoded_error)? != profile {
        return Err(corrupt(
            "application imports or routes are not in canonical order",
        ));
    }
    let policy = read_policy(&mut payload_reader)?;
    let test_count = payload_reader
        .count(MAXIMUM_APPLICATION_TESTS)
        .map_err(application_codec)?;
    let mut tests = Vec::with_capacity(test_count);
    let mut prior_test: Option<String> = None;
    for _ in 0..test_count {
        let test = read_test(&mut payload_reader)?;
        if prior_test
            .as_deref()
            .is_some_and(|prior| prior >= test.name.as_str())
        {
            return Err(corrupt(
                "application tests are not in strict canonical name order",
            ));
        }
        prior_test = Some(test.name.clone());
        tests.push(test);
    }
    let release_count = payload_reader
        .count(release::MAXIMUM_RELEASE_GRAPH_NODES)
        .map_err(application_codec)?;
    if release_count == 0 {
        return Err(corrupt("application exact release graph is empty"));
    }
    let mut decoded = Vec::with_capacity(release_count);
    let mut prior_release = None;
    for _ in 0..release_count {
        let item = payload_reader
            .bytes(release::MAXIMUM_RELEASE_ARTIFACT_BYTES)
            .map_err(application_codec)?
            .to_vec();
        let release = release::decode(&item)?;
        if prior_release.is_some_and(|prior| prior >= release.id) {
            return Err(corrupt(
                "application releases are not in strict exact-identity order",
            ));
        }
        prior_release = Some(release.id);
        decoded.push(release);
    }
    payload_reader.finish().map_err(application_codec)?;
    observation.envelope_decode_nanoseconds = observation
        .envelope_decode_nanoseconds
        .saturating_add(elapsed_nanoseconds(envelope_started));
    observation.release_count = observation
        .release_count
        .saturating_add(decoded.len() as u64);
    let graph_validation_started = Instant::now();
    let root_position = decoded
        .iter()
        .position(|release| release.id == root_release)
        .ok_or_else(|| corrupt("application root release is absent"))?;
    let root = decoded.remove(root_position);
    let graph = ReleaseGraph::new(root, decoded)?;
    observation.release_graph_validation_nanoseconds = observation
        .release_graph_validation_nanoseconds
        .saturating_add(elapsed_nanoseconds(graph_validation_started));
    let flattening_started = Instant::now();
    let flattened = graph.flatten()?;
    observation.closure_flattening_nanoseconds = observation
        .closure_flattening_nanoseconds
        .saturating_add(elapsed_nanoseconds(flattening_started));
    observation.flattened_semantic_items = observation
        .flattened_semantic_items
        .saturating_add(u64::try_from(flattened.snapshot.node_count()).unwrap_or(u64::MAX));
    let manifest_validation_started = Instant::now();
    validate_manifest(&graph, &flattened, entry, &profile, policy, &tests)
        .map_err(decoded_error)?;
    observation.release_graph_validation_nanoseconds = observation
        .release_graph_validation_nanoseconds
        .saturating_add(elapsed_nanoseconds(manifest_validation_started));
    let application = DecodedApplication {
        bytes: bytes.to_vec(),
        graph,
        flattened,
        entry,
        profile,
        policy,
        tests,
        digest,
    };
    let canonical_reencode_started = Instant::now();
    if encode_application(
        &application.graph,
        application.entry,
        &application.profile,
        application.policy,
        &application.tests,
    )? != bytes
    {
        return Err(corrupt("application artifact encoding is not canonical"));
    }
    observation.canonical_reencode_nanoseconds = observation
        .canonical_reencode_nanoseconds
        .saturating_add(elapsed_nanoseconds(canonical_reencode_started));
    Ok(application)
}

fn run_all_tests(application: &DecodedApplication) -> Result<ApplicationTestReport> {
    let mut results = Vec::new();
    let mut release_total = 0_u64;
    for release in application.graph.releases() {
        let report = application.graph.run_release_tests(release.id)?;
        release_total = release_total.saturating_add(report.total);
        for result in report.results {
            results.push(ApplicationTestResult {
                name: format!("release:{}:{}", release.id, result.name),
                status: match result.status {
                    release::ReleaseTestStatus::Passed => ApplicationTestStatus::Passed,
                    release::ReleaseTestStatus::UnexpectedValue => {
                        ApplicationTestStatus::UnexpectedValue
                    }
                    release::ReleaseTestStatus::UnexpectedTrap => {
                        ApplicationTestStatus::UnexpectedTrap
                    }
                    release::ReleaseTestStatus::MissingTrap => ApplicationTestStatus::MissingTrap,
                    release::ReleaseTestStatus::WrongTrap => ApplicationTestStatus::WrongTrap,
                    release::ReleaseTestStatus::InvalidCase => ApplicationTestStatus::InvalidCase,
                    release::ReleaseTestStatus::Incomplete => ApplicationTestStatus::Incomplete,
                    release::ReleaseTestStatus::ResourceFailure => {
                        ApplicationTestStatus::ResourceFailure
                    }
                    release::ReleaseTestStatus::EngineFailure => {
                        ApplicationTestStatus::EngineFailure
                    }
                },
                observed_trap: result.observed_trap.map(|trap| match trap {
                    release::ReleaseTrapCode::RuntimeTrap => ApplicationTrapCode::RuntimeTrap,
                    release::ReleaseTrapCode::ByteIndexOutOfBounds => {
                        ApplicationTrapCode::ByteIndexOutOfBounds
                    }
                    release::ReleaseTrapCode::ByteSliceOutOfBounds => {
                        ApplicationTrapCode::ByteSliceOutOfBounds
                    }
                }),
            });
        }
    }
    let application_total = u64::try_from(application.tests.len()).unwrap_or(u64::MAX);
    for test in &application.tests {
        results.push(run_case(application, test)?);
    }
    let passed = u64::try_from(
        results
            .iter()
            .filter(|result| result.status == ApplicationTestStatus::Passed)
            .count(),
    )
    .unwrap_or(u64::MAX);
    Ok(ApplicationTestReport {
        total: u64::try_from(results.len()).unwrap_or(u64::MAX),
        passed,
        release_total,
        application_total,
        results,
    })
}

fn run_case(
    application: &DecodedApplication,
    test: &ApplicationTestCase,
) -> Result<ApplicationTestResult> {
    let target = application
        .flattened
        .item(test.target.release, test.target.item)?;
    let arguments = test
        .arguments
        .iter()
        .map(|value| to_runtime(&application.flattened, value))
        .collect::<Result<Vec<_>>>()?;
    Ok(
        match interpret::compile_and_run(
            &application.flattened.snapshot,
            target,
            &arguments,
            test.policy,
        ) {
            Ok(run) => match &test.expected {
                ApplicationTestExpectation::Value(expected)
                    if from_runtime(&application.flattened, &run.value)? == *expected =>
                {
                    ApplicationTestResult {
                        name: test.name.clone(),
                        status: ApplicationTestStatus::Passed,
                        observed_trap: None,
                    }
                }
                ApplicationTestExpectation::Value(_) => ApplicationTestResult {
                    name: test.name.clone(),
                    status: ApplicationTestStatus::UnexpectedValue,
                    observed_trap: None,
                },
                ApplicationTestExpectation::Trap(_) => ApplicationTestResult {
                    name: test.name.clone(),
                    status: ApplicationTestStatus::MissingTrap,
                    observed_trap: None,
                },
            },
            Err(error) => classify_test_error(application, test, &error)?,
        },
    )
}

fn classify_test_error(
    application: &DecodedApplication,
    test: &ApplicationTestCase,
    error: &LkError,
) -> Result<ApplicationTestResult> {
    let observed = ApplicationTrapCode::from_error(error.code);
    let status = if is_resource_error(error.code) {
        ApplicationTestStatus::ResourceFailure
    } else if error.code == ErrorCode::CompileIncomplete {
        ApplicationTestStatus::Incomplete
    } else if matches!(
        error.code,
        ErrorCode::RunArgumentMismatch
            | ErrorCode::TypeMismatch
            | ErrorCode::WrongKind
            | ErrorCode::WrongWorkspace
            | ErrorCode::NodeNotFound
    ) {
        ApplicationTestStatus::InvalidCase
    } else if let Some(actual) = observed {
        match test.expected {
            ApplicationTestExpectation::Value(_) => ApplicationTestStatus::UnexpectedTrap,
            ApplicationTestExpectation::Trap(expected)
                if expected.code == actual
                    && expected
                        .target
                        .map(|target| application.flattened.item(target.release, target.item))
                        .transpose()?
                        .is_none_or(|target| Some(target) == error.target) =>
            {
                ApplicationTestStatus::Passed
            }
            ApplicationTestExpectation::Trap(_) => ApplicationTestStatus::WrongTrap,
        }
    } else {
        ApplicationTestStatus::EngineFailure
    };
    Ok(ApplicationTestResult {
        name: test.name.clone(),
        status,
        observed_trap: observed,
    })
}

const fn is_resource_error(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::ExecutionFuelExhausted
            | ErrorCode::ExecutionFrameExhausted
            | ErrorCode::RuntimeByteInputTooLarge
            | ErrorCode::ManagedObjectPolicyExceeded
            | ErrorCode::ManagedVisibleBytePolicyExceeded
            | ErrorCode::RetainedBytePolicyExceeded
            | ErrorCode::ResultBytePolicyExceeded
            | ErrorCode::ByteValueTooLarge
            | ErrorCode::ExecutionMemoryExhausted
    )
}

fn inspection(application: &DecodedApplication) -> Result<ApplicationInspection> {
    let releases = application
        .graph
        .releases()
        .map(|release| ApplicationReleaseInspection {
            release: release.id,
            coordinate: release.coordinate.clone(),
            user_version: release.user_version.clone(),
            dependencies: release
                .dependencies
                .iter()
                .map(|dependency| dependency.release)
                .collect(),
            exports: u64::try_from(release.exports.len()).unwrap_or(u64::MAX),
            release_tests: u64::try_from(release.tests.len()).unwrap_or(u64::MAX),
            artifact_bytes: u64::try_from(release.bytes.len()).unwrap_or(u64::MAX),
        })
        .collect();
    let tests = application
        .tests
        .iter()
        .map(|test| {
            Ok(ApplicationTestSummary {
                name: test.name.clone(),
                case_digest: digest_test(test)?,
                target: test.target,
                expected: match test.expected {
                    ApplicationTestExpectation::Value(_) => "value",
                    ApplicationTestExpectation::Trap(_) => "trap",
                },
                policy: test.policy,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ApplicationInspection {
        contract_version: APPLICATION_CONTRACT_VERSION,
        format_version: APPLICATION_FORMAT_VERSION,
        semantic_schema: crate::artifact::SCHEMA_NAME,
        digest: application.digest,
        graph_digest: graph_digest(&application.graph),
        root_release: application.graph.root(),
        entry: application.entry,
        profile: application.profile.clone(),
        policy: application.policy,
        releases,
        graph_edges: u64::try_from(application.graph.edge_count()).unwrap_or(u64::MAX),
        graph_depth: u64::try_from(application.graph.depth()).unwrap_or(u64::MAX),
        graph_release_bytes: u64::try_from(application.graph.aggregate_bytes()).unwrap_or(u64::MAX),
        flattened_semantic_items: u64::try_from(application.flattened.snapshot.node_count())
            .unwrap_or(u64::MAX),
        tests,
        artifact_bytes: u64::try_from(application.bytes.len()).unwrap_or(u64::MAX),
        provenance: "absent",
        signatures: "absent",
        limits: application_limits(),
    })
}

fn to_runtime(flattened: &FlattenedGraph, value: &ApplicationValue) -> Result<RuntimeValue> {
    let mut items = 0_usize;
    to_runtime_bounded(flattened, value, 1, &mut items)
}

fn to_runtime_bounded(
    flattened: &FlattenedGraph,
    value: &ApplicationValue,
    depth: usize,
    items: &mut usize,
) -> Result<RuntimeValue> {
    if depth > interpret::MAX_RUNTIME_VALUE_DEPTH {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value nesting exceeds policy",
        ));
    }
    *items = items.saturating_add(1);
    if *items > interpret::MAX_RUNTIME_VALUE_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value item count exceeds policy",
        ));
    }
    Ok(match value {
        ApplicationValue::Unit => RuntimeValue::Unit,
        ApplicationValue::Bool(value) => RuntimeValue::Bool(*value),
        ApplicationValue::I64(value) => RuntimeValue::I64(*value),
        ApplicationValue::Bytes(value) => RuntimeValue::Bytes(value.clone()),
        ApplicationValue::Text(value) => {
            RuntimeValue::Text(crate::runtime_text::RuntimeText::from_text(value))
        }
        ApplicationValue::Product { ty, fields } => RuntimeValue::Product {
            ty: flattened.item(ty.release, ty.item)?,
            fields: fields
                .iter()
                .map(|field| {
                    Ok(RuntimeFieldValue {
                        field: flattened.item(field.field.release, field.field.item)?,
                        value: to_runtime_bounded(
                            flattened,
                            &field.value,
                            depth.saturating_add(1),
                            items,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        },
        ApplicationValue::Sum {
            ty,
            variant,
            payload,
        } => RuntimeValue::Sum {
            ty: flattened.item(ty.release, ty.item)?,
            variant: flattened.item(variant.release, variant.item)?,
            payload: payload
                .as_deref()
                .map(|value| {
                    to_runtime_bounded(flattened, value, depth.saturating_add(1), items)
                        .map(Box::new)
                })
                .transpose()?,
        },
        ApplicationValue::Sequence { ty, elements } => RuntimeValue::Sequence {
            ty: flattened.item(ty.release, ty.item)?,
            elements: elements
                .iter()
                .map(|value| to_runtime_bounded(flattened, value, depth.saturating_add(1), items))
                .collect::<Result<Vec<_>>>()?,
        },
    })
}

fn from_runtime(flattened: &FlattenedGraph, value: &RuntimeValue) -> Result<ApplicationValue> {
    let mut items = 0_usize;
    from_runtime_bounded(flattened, value, 1, &mut items)
}

fn from_runtime_bounded(
    flattened: &FlattenedGraph,
    value: &RuntimeValue,
    depth: usize,
    items: &mut usize,
) -> Result<ApplicationValue> {
    if depth > interpret::MAX_RUNTIME_VALUE_DEPTH {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "runtime produced an application value beyond depth policy",
        ));
    }
    *items = items.saturating_add(1);
    if *items > interpret::MAX_RUNTIME_VALUE_ITEMS {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "runtime produced an application value beyond item policy",
        ));
    }
    let target = |node| {
        flattened
            .global_item(node)
            .map(|item| ApplicationTarget {
                release: item.release,
                item: item.item,
            })
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::CoreIrInvalid,
                    "runtime nominal identity has no exact release origin",
                )
            })
    };
    Ok(match value {
        RuntimeValue::Unit => ApplicationValue::Unit,
        RuntimeValue::Bool(value) => ApplicationValue::Bool(*value),
        RuntimeValue::I64(value) => ApplicationValue::I64(*value),
        RuntimeValue::Bytes(value) => ApplicationValue::Bytes(value.clone()),
        RuntimeValue::Text(value) => {
            ApplicationValue::Text(value.to_text_string().map_err(|_| {
                LkError::new(
                    ErrorCode::ResultBytePolicyExceeded,
                    "runtime text exceeds application value policy",
                )
            })?)
        }
        RuntimeValue::Product { ty, fields } => ApplicationValue::Product {
            ty: target(*ty)?,
            fields: fields
                .iter()
                .map(|field| {
                    Ok(ApplicationFieldValue {
                        field: target(field.field)?,
                        value: from_runtime_bounded(
                            flattened,
                            &field.value,
                            depth.saturating_add(1),
                            items,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        },
        RuntimeValue::Sum {
            ty,
            variant,
            payload,
        } => ApplicationValue::Sum {
            ty: target(*ty)?,
            variant: target(*variant)?,
            payload: payload
                .as_deref()
                .map(|value| {
                    from_runtime_bounded(flattened, value, depth.saturating_add(1), items)
                        .map(Box::new)
                })
                .transpose()?,
        },
        RuntimeValue::Sequence { ty, elements } => ApplicationValue::Sequence {
            ty: target(*ty)?,
            elements: elements
                .iter()
                .map(|value| from_runtime_bounded(flattened, value, depth.saturating_add(1), items))
                .collect::<Result<Vec<_>>>()?,
        },
    })
}

fn put_test(writer: &mut Writer, test: &ApplicationTestCase) -> Result<()> {
    writer.string(&test.name).map_err(application_codec)?;
    put_target(writer, test.target);
    put_policy(writer, test.policy);
    put_count(writer, test.arguments.len())?;
    for argument in &test.arguments {
        put_value(writer, argument)?;
    }
    match &test.expected {
        ApplicationTestExpectation::Value(value) => {
            writer.u8(1);
            put_value(writer, value)?;
        }
        ApplicationTestExpectation::Trap(trap) => {
            writer.u8(2);
            writer.u8(trap.code.stable_tag());
            writer.bool(trap.target.is_some());
            if let Some(target) = trap.target {
                put_target(writer, target);
            }
        }
    }
    Ok(())
}

fn read_test(reader: &mut Reader<'_>) -> Result<ApplicationTestCase> {
    let name = reader
        .string(MAXIMUM_APPLICATION_TEST_NAME_BYTES)
        .map_err(application_codec)?;
    let target = read_target(reader)?;
    let policy = read_policy(reader)?;
    let argument_count = reader
        .count(interpret::MAX_RUN_ARGUMENTS)
        .map_err(application_codec)?;
    let mut arguments = Vec::with_capacity(argument_count);
    for _ in 0..argument_count {
        arguments.push(read_value(reader)?);
    }
    let expected = match reader.u8().map_err(application_codec)? {
        1 => ApplicationTestExpectation::Value(read_value(reader)?),
        2 => {
            let code =
                ApplicationTrapCode::from_stable_tag(reader.u8().map_err(application_codec)?)
                    .ok_or_else(|| corrupt("application trap tag is unknown"))?;
            let target = if reader.bool().map_err(application_codec)? {
                Some(read_target(reader)?)
            } else {
                None
            };
            ApplicationTestExpectation::Trap(ApplicationTrap { code, target })
        }
        _ => return Err(corrupt("application test expectation tag is unknown")),
    };
    Ok(ApplicationTestCase {
        name,
        target,
        arguments,
        expected,
        policy,
    })
}

fn put_value(writer: &mut Writer, value: &ApplicationValue) -> Result<()> {
    let encoded = encode_application_value_binary(value, MAXIMUM_APPLICATION_VALUE_JSON_BYTES)?;
    writer.bytes(&encoded).map_err(application_codec)
}

fn read_value(reader: &mut Reader<'_>) -> Result<ApplicationValue> {
    let encoded = reader
        .bytes(MAXIMUM_APPLICATION_VALUE_JSON_BYTES)
        .map_err(application_codec)?;
    decode_application_value_binary(encoded, MAXIMUM_APPLICATION_VALUE_JSON_BYTES)
}

pub(crate) fn encode_application_value_binary(
    value: &ApplicationValue,
    maximum_bytes: usize,
) -> Result<Vec<u8>> {
    let mut items = 0_usize;
    let size = application_value_binary_size(value, 1, &mut items)?;
    if size > maximum_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary encoding exceeds byte policy",
        ));
    }
    let mut writer = Writer::with_capacity(size);
    put_application_value_binary(&mut writer, value)?;
    let encoded = writer.finish();
    debug_assert_eq!(encoded.len(), size);
    Ok(encoded)
}

pub(crate) fn decode_application_value_binary(
    bytes: &[u8],
    maximum_bytes: usize,
) -> Result<ApplicationValue> {
    if bytes.len() > maximum_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary input exceeds byte policy",
        ));
    }
    let mut reader = Reader::new(bytes);
    let mut items = 0_usize;
    let value = read_application_value_binary(&mut reader, 1, &mut items)?;
    reader.finish().map_err(application_codec)?;
    Ok(value)
}

fn application_value_binary_size(
    value: &ApplicationValue,
    depth: usize,
    items: &mut usize,
) -> Result<usize> {
    if depth > interpret::MAX_RUNTIME_VALUE_DEPTH {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary nesting exceeds policy",
        ));
    }
    *items = items.checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary item count overflows",
        )
    })?;
    if *items > interpret::MAX_RUNTIME_VALUE_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary item count exceeds policy",
        ));
    }
    let add = |left: usize, right: usize| {
        left.checked_add(right).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "application value binary size overflows",
            )
        })
    };
    match value {
        ApplicationValue::Unit => Ok(1),
        ApplicationValue::Bool(_) => Ok(2),
        ApplicationValue::I64(_) => Ok(9),
        ApplicationValue::Bytes(value) => add(9, value.len()),
        ApplicationValue::Text(value) => add(9, value.len_bytes()),
        ApplicationValue::Product { fields, .. } => {
            let mut size = 49_usize;
            for field in fields {
                size = add(size, 40)?;
                size = add(
                    size,
                    application_value_binary_size(&field.value, depth.saturating_add(1), items)?,
                )?;
            }
            Ok(size)
        }
        ApplicationValue::Sum { payload, .. } => {
            let mut size = 82_usize;
            if let Some(payload) = payload {
                size = add(
                    size,
                    application_value_binary_size(payload, depth.saturating_add(1), items)?,
                )?;
            }
            Ok(size)
        }
        ApplicationValue::Sequence { elements, .. } => {
            if elements.len() > MAXIMUM_SEQUENCE_ELEMENTS {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "application value binary sequence length exceeds policy",
                ));
            }
            let mut size = 49_usize;
            for element in elements {
                size = add(
                    size,
                    application_value_binary_size(element, depth.saturating_add(1), items)?,
                )?;
            }
            Ok(size)
        }
    }
}

fn put_application_value_binary(writer: &mut Writer, value: &ApplicationValue) -> Result<()> {
    match value {
        ApplicationValue::Unit => writer.u8(1),
        ApplicationValue::Bool(value) => {
            writer.u8(2);
            writer.bool(*value);
        }
        ApplicationValue::I64(value) => {
            writer.u8(3);
            writer.i64(*value);
        }
        ApplicationValue::Bytes(value) => {
            writer.u8(4);
            writer.bytes(value.as_slice()).map_err(application_codec)?;
        }
        ApplicationValue::Text(value) => {
            writer.u8(5);
            writer.string(value.as_str()).map_err(application_codec)?;
        }
        ApplicationValue::Product { ty, fields } => {
            writer.u8(6);
            put_target(writer, *ty);
            put_count(writer, fields.len())?;
            for field in fields {
                put_target(writer, field.field);
                put_application_value_binary(writer, &field.value)?;
            }
        }
        ApplicationValue::Sum {
            ty,
            variant,
            payload,
        } => {
            writer.u8(7);
            put_target(writer, *ty);
            put_target(writer, *variant);
            writer.bool(payload.is_some());
            if let Some(payload) = payload {
                put_application_value_binary(writer, payload)?;
            }
        }
        ApplicationValue::Sequence { ty, elements } => {
            writer.u8(8);
            put_target(writer, *ty);
            put_count(writer, elements.len())?;
            for element in elements {
                put_application_value_binary(writer, element)?;
            }
        }
    }
    Ok(())
}

fn read_application_value_binary(
    reader: &mut Reader<'_>,
    depth: usize,
    items: &mut usize,
) -> Result<ApplicationValue> {
    if depth > interpret::MAX_RUNTIME_VALUE_DEPTH {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary nesting exceeds policy",
        ));
    }
    *items = items.checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary item count overflows",
        )
    })?;
    if *items > interpret::MAX_RUNTIME_VALUE_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value binary item count exceeds policy",
        ));
    }
    let tag = reader.u8().map_err(application_codec)?;
    Ok(match tag {
        1 => ApplicationValue::Unit,
        2 => ApplicationValue::Bool(reader.bool().map_err(application_codec)?),
        3 => ApplicationValue::I64(reader.i64().map_err(application_codec)?),
        4 => ApplicationValue::Bytes(
            ByteString::from_slice(
                reader
                    .bytes(MAXIMUM_BYTE_STRING_BYTES)
                    .map_err(application_codec)?,
            )
            .map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "application binary byte value exceeds policy",
                )
            })?,
        ),
        5 => ApplicationValue::Text(
            TextString::try_from_str(
                &reader
                    .string(crate::schema::MAXIMUM_TEXT_BYTES)
                    .map_err(application_codec)?,
            )
            .map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "application binary text value exceeds policy",
                )
            })?,
        ),
        6 => {
            let ty = read_target(reader)?;
            let count = reader
                .count(interpret::MAX_RUNTIME_VALUE_ITEMS.saturating_sub(*items))
                .map_err(application_codec)?;
            let mut fields = Vec::with_capacity(count);
            for _ in 0..count {
                fields.push(ApplicationFieldValue {
                    field: read_target(reader)?,
                    value: read_application_value_binary(reader, depth.saturating_add(1), items)?,
                });
            }
            ApplicationValue::Product { ty, fields }
        }
        7 => {
            let ty = read_target(reader)?;
            let variant = read_target(reader)?;
            let payload = if reader.bool().map_err(application_codec)? {
                Some(Box::new(read_application_value_binary(
                    reader,
                    depth.saturating_add(1),
                    items,
                )?))
            } else {
                None
            };
            ApplicationValue::Sum {
                ty,
                variant,
                payload,
            }
        }
        8 => {
            let ty = read_target(reader)?;
            let count = reader
                .count(
                    MAXIMUM_SEQUENCE_ELEMENTS
                        .min(interpret::MAX_RUNTIME_VALUE_ITEMS.saturating_sub(*items)),
                )
                .map_err(application_codec)?;
            let mut elements = Vec::with_capacity(count);
            for _ in 0..count {
                elements.push(read_application_value_binary(
                    reader,
                    depth.saturating_add(1),
                    items,
                )?);
            }
            ApplicationValue::Sequence { ty, elements }
        }
        _ => {
            return Err(application_codec(
                reader.unknown_tag(crate::codec::TagDomain::Value, tag),
            ));
        }
    })
}

fn put_target(writer: &mut Writer, target: ApplicationTarget) {
    writer.fixed(&target.release.as_bytes());
    writer.u64(target.item.get());
}

fn put_profile(writer: &mut Writer, profile: &InvocationProfile) -> Result<()> {
    writer.u8(profile.stable_tag());
    match profile {
        InvocationProfile::Typed | InvocationProfile::BytesStream => {}
        InvocationProfile::Stateful(stateful) => {
            put_target(writer, stateful.resume);
            put_target(writer, stateful.query_entry);
            put_target(writer, stateful.state);
            put_target(writer, stateful.event);
            put_target(writer, stateful.response);
            put_target(writer, stateful.query);
            put_target(writer, stateful.query_result);
            put_target(writer, stateful.command);
            put_target(writer, stateful.outcome);
            put_target(writer, stateful.decision);
            put_target(writer, stateful.declined_variant);
            put_target(writer, stateful.declined_payload);
            put_target(writer, stateful.declined_response_field);
            put_target(writer, stateful.unchanged_variant);
            put_target(writer, stateful.unchanged_payload);
            put_target(writer, stateful.unchanged_response_field);
            put_target(writer, stateful.completed_variant);
            put_target(writer, stateful.completed_payload);
            put_target(writer, stateful.completed_state_field);
            put_target(writer, stateful.completed_response_field);
            put_target(writer, stateful.suspended_variant);
            put_target(writer, stateful.suspended_payload);
            put_target(writer, stateful.suspended_state_field);
            put_target(writer, stateful.suspended_response_field);
            put_target(writer, stateful.suspended_command_field);
            put_count(writer, stateful.imports.len())?;
            for import in &stateful.imports {
                writer.string(&import.slot).map_err(application_codec)?;
                writer.u8(match import.interface {
                    HostInterface::ImmutableBlob => 2,
                });
                put_target(writer, import.request);
                put_target(writer, import.outcome);
                put_target(writer, import.command_variant);
                put_target(writer, import.outcome_variant);
                put_count(writer, import.requests.len())?;
                for route in &import.requests {
                    put_target(writer, route.variant);
                    writer.u8(route.operation.stable_tag());
                }
                put_count(writer, import.outcomes.len())?;
                for route in &import.outcomes {
                    writer.u8(route.operation.stable_tag());
                    writer.u8(route.class.stable_tag());
                    put_target(writer, route.variant);
                }
            }
        }
        InvocationProfile::Interactive(profile) => interactive::put_profile(writer, profile),
    }
    Ok(())
}

fn read_profile(reader: &mut Reader<'_>) -> Result<InvocationProfile> {
    Ok(match reader.u8().map_err(application_codec)? {
        1 => InvocationProfile::Typed,
        2 => InvocationProfile::BytesStream,
        3 => {
            let resume = read_target(reader)?;
            let query_entry = read_target(reader)?;
            let state = read_target(reader)?;
            let event = read_target(reader)?;
            let response = read_target(reader)?;
            let query = read_target(reader)?;
            let query_result = read_target(reader)?;
            let command = read_target(reader)?;
            let outcome = read_target(reader)?;
            let decision = read_target(reader)?;
            let declined_variant = read_target(reader)?;
            let declined_payload = read_target(reader)?;
            let declined_response_field = read_target(reader)?;
            let unchanged_variant = read_target(reader)?;
            let unchanged_payload = read_target(reader)?;
            let unchanged_response_field = read_target(reader)?;
            let completed_variant = read_target(reader)?;
            let completed_payload = read_target(reader)?;
            let completed_state_field = read_target(reader)?;
            let completed_response_field = read_target(reader)?;
            let suspended_variant = read_target(reader)?;
            let suspended_payload = read_target(reader)?;
            let suspended_state_field = read_target(reader)?;
            let suspended_response_field = read_target(reader)?;
            let suspended_command_field = read_target(reader)?;
            let import_count = reader.count(64).map_err(application_codec)?;
            let mut imports = Vec::with_capacity(import_count);
            for _ in 0..import_count {
                let slot = reader.string(64).map_err(application_codec)?;
                let interface = match reader.u8().map_err(application_codec)? {
                    2 => HostInterface::ImmutableBlob,
                    _ => return Err(corrupt("host-interface identity tag is unknown")),
                };
                let request = read_target(reader)?;
                let interface_outcome = read_target(reader)?;
                let command_variant = read_target(reader)?;
                let outcome_variant = read_target(reader)?;
                let request_count = reader.count(64).map_err(application_codec)?;
                let mut requests = Vec::with_capacity(request_count);
                for _ in 0..request_count {
                    let variant = read_target(reader)?;
                    let operation =
                        HostOperation::from_stable_tag(reader.u8().map_err(application_codec)?)
                            .ok_or_else(|| corrupt("host operation tag is unknown"))?;
                    requests.push(HostRequestRoute { variant, operation });
                }
                let outcome_count = reader.count(128).map_err(application_codec)?;
                let mut outcomes = Vec::with_capacity(outcome_count);
                for _ in 0..outcome_count {
                    let operation =
                        HostOperation::from_stable_tag(reader.u8().map_err(application_codec)?)
                            .ok_or_else(|| corrupt("host outcome operation tag is unknown"))?;
                    let class =
                        HostOutcomeClass::from_stable_tag(reader.u8().map_err(application_codec)?)
                            .ok_or_else(|| corrupt("host outcome class tag is unknown"))?;
                    outcomes.push(HostOutcomeRoute {
                        operation,
                        class,
                        variant: read_target(reader)?,
                    });
                }
                imports.push(ApplicationImport {
                    slot,
                    interface,
                    request,
                    outcome: interface_outcome,
                    command_variant,
                    outcome_variant,
                    requests,
                    outcomes,
                });
            }
            InvocationProfile::Stateful(StatefulApplicationProfile {
                resume,
                query_entry,
                state,
                event,
                response,
                query,
                query_result,
                command,
                outcome,
                decision,
                declined_variant,
                declined_payload,
                declined_response_field,
                unchanged_variant,
                unchanged_payload,
                unchanged_response_field,
                completed_variant,
                completed_payload,
                completed_state_field,
                completed_response_field,
                suspended_variant,
                suspended_payload,
                suspended_state_field,
                suspended_response_field,
                suspended_command_field,
                imports,
            })
        }
        4 => InvocationProfile::Interactive(Box::new(interactive::read_profile(reader)?)),
        _ => return Err(corrupt("application invocation profile tag is unknown")),
    })
}

fn read_target(reader: &mut Reader<'_>) -> Result<ApplicationTarget> {
    Ok(ApplicationTarget {
        release: ReleaseId::from_bytes(read_digest(reader)?),
        item: ReleaseItemId::new(reader.u64().map_err(application_codec)?)
            .map_err(decoded_error)?,
    })
}

fn put_policy(writer: &mut Writer, policy: RunPolicy) {
    writer.u64(policy.fuel);
    writer.u32(policy.maximum_frames);
}

fn read_policy(reader: &mut Reader<'_>) -> Result<RunPolicy> {
    let policy = RunPolicy {
        fuel: reader.u64().map_err(application_codec)?,
        maximum_frames: reader.u32().map_err(application_codec)?,
    };
    interpret::validate_policy(policy).map_err(decoded_error)?;
    Ok(policy)
}

fn put_count(writer: &mut Writer, count: usize) -> Result<()> {
    writer.u64(u64::try_from(count).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "application collection length does not fit canonical u64",
        )
    })?);
    Ok(())
}

fn read_digest(reader: &mut Reader<'_>) -> Result<[u8; 32]> {
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(reader.fixed(32).map_err(application_codec)?);
    Ok(digest)
}

fn digest_payload(payload: &[u8]) -> ApplicationDigest {
    let mut hasher = blake3::Hasher::new_derive_key(APPLICATION_DIGEST_DOMAIN);
    hasher.update(payload);
    ApplicationDigest(*hasher.finalize().as_bytes())
}

fn graph_digest(graph: &ReleaseGraph) -> ApplicationGraphDigest {
    let mut hasher = blake3::Hasher::new_derive_key(APPLICATION_GRAPH_DIGEST_DOMAIN);
    hasher.update(&graph.root().as_bytes());
    for release in graph.releases() {
        hasher.update(&release.id.as_bytes());
        for dependency in &release.dependencies {
            hasher.update(&(dependency.slot.len() as u64).to_le_bytes());
            hasher.update(dependency.slot.as_bytes());
            hasher.update(&dependency.release.as_bytes());
        }
    }
    ApplicationGraphDigest(*hasher.finalize().as_bytes())
}

fn digest_test(test: &ApplicationTestCase) -> Result<ApplicationTestDigest> {
    let mut writer = Writer::new();
    put_test(&mut writer, test)?;
    let mut hasher = blake3::Hasher::new_derive_key(APPLICATION_TEST_DIGEST_DOMAIN);
    hasher.update(&writer.finish());
    Ok(ApplicationTestDigest(*hasher.finalize().as_bytes()))
}

fn validate_contract_version(version: u16) -> Result<()> {
    if version == APPLICATION_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolVersion,
            format!(
                "application command contract version {version} is unsupported; expected {APPLICATION_CONTRACT_VERSION}"
            ),
        ))
    }
}

fn application_limits() -> ApplicationLimits {
    ApplicationLimits {
        maximum_artifact_bytes: MAXIMUM_APPLICATION_ARTIFACT_BYTES as u64,
        maximum_release_artifact_bytes: release::MAXIMUM_RELEASE_ARTIFACT_BYTES as u64,
        maximum_graph_nodes: release::MAXIMUM_RELEASE_GRAPH_NODES as u64,
        maximum_graph_edges: release::MAXIMUM_RELEASE_GRAPH_EDGES as u64,
        maximum_graph_depth: release::MAXIMUM_RELEASE_GRAPH_DEPTH as u64,
        maximum_graph_bytes: release::MAXIMUM_RELEASE_GRAPH_BYTES as u64,
        maximum_application_tests: MAXIMUM_APPLICATION_TESTS as u64,
        maximum_test_name_bytes: MAXIMUM_APPLICATION_TEST_NAME_BYTES as u64,
        maximum_suite_fuel: MAXIMUM_APPLICATION_SUITE_FUEL,
        maximum_value_json_bytes: MAXIMUM_APPLICATION_VALUE_JSON_BYTES as u64,
        maximum_run_arguments: interpret::MAX_RUN_ARGUMENTS as u64,
        maximum_run_fuel: interpret::MAX_RUN_FUEL,
        maximum_run_frames: u64::from(interpret::MAX_RUN_FRAMES),
        maximum_runtime_value_depth: interpret::MAX_RUNTIME_VALUE_DEPTH as u64,
        maximum_runtime_value_items: interpret::MAX_RUNTIME_VALUE_ITEMS as u64,
        maximum_runtime_value_bytes: interpret::MAX_RUNTIME_VALUE_BYTES as u64,
        maximum_stream_input_bytes: MAXIMUM_BYTE_STRING_BYTES as u64,
    }
}

fn application_codec(error: CodecError) -> LkError {
    LkError::new(
        if error.kind == CodecErrorKind::PolicyExceeded {
            ErrorCode::PolicyExceeded
        } else {
            ErrorCode::ArtifactCorrupt
        },
        format!("canonical application decoding failed: {error}"),
    )
}

fn decoded_error(mut error: LkError) -> LkError {
    if error.code != ErrorCode::PolicyExceeded {
        error.code = ErrorCode::ArtifactCorrupt;
    }
    error
}

fn corrupt(message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
}

#[cfg(test)]
fn publish_with_fault(path: &Path, bytes: &[u8], fault: PublicationFault) -> Result<()> {
    artifact_io::publish_with_fault(
        path,
        bytes,
        "application artifact",
        TEMPORARY_PREFIX,
        MAXIMUM_APPLICATION_ARTIFACT_BYTES,
        fault,
    )
}

#[cfg(test)]
mod tests;

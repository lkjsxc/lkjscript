//! Independently transferable exact-release-graph applications.
//!
//! Application format 3 embeds one canonical exact reusable-release graph, exact exported
//! entries, an invocation profile, resource policy, and typed application cases. Source workspace
//! identity, mutable resolver state, proposal syntax, Core IR, and runtime handles are absent.

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
use crate::schema::{ByteString, MAXIMUM_BYTE_STRING_BYTES, Node, SemanticType};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::path::Path;

pub const APPLICATION_MAGIC: [u8; 8] = *b"LKJAPP\0\x03";
pub const APPLICATION_FORMAT_VERSION: u16 = 3;
pub const APPLICATION_CONTRACT_VERSION: u16 = 3;
pub const MAXIMUM_APPLICATION_ARTIFACT_BYTES: usize = 256 * 1024 * 1024;
pub const MAXIMUM_APPLICATION_TESTS: usize = 256;
pub const MAXIMUM_APPLICATION_TEST_NAME_BYTES: usize = 64;
pub const MAXIMUM_APPLICATION_SUITE_FUEL: u64 = 100_000_000;
pub const MAXIMUM_APPLICATION_PATH_BYTES: usize = artifact_io::MAXIMUM_ARTIFACT_PATH_BYTES;
const MAXIMUM_APPLICATION_VALUE_JSON_BYTES: usize = 1024 * 1024;
const APPLICATION_DIGEST_DOMAIN: &str = "lkjscript.application-artifact.v3";
const APPLICATION_GRAPH_DIGEST_DOMAIN: &str = "lkjscript.application-release-graph.v1";
const APPLICATION_TEST_DIGEST_DOMAIN: &str = "lkjscript.application-test-case.v2";
const TEMPORARY_PREFIX: &str = ".lkjscript-application-";

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
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulApplicationProfile {
    pub resume: ApplicationTarget,
    pub decision: ApplicationTarget,
    pub state_field: ApplicationTarget,
    pub response_field: ApplicationTarget,
    pub command_field: ApplicationTarget,
    pub target_field: ApplicationTarget,
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
}

impl InvocationProfile {
    const fn stable_tag(&self) -> u8 {
        match self {
            Self::Typed => 1,
            Self::BytesStream => 2,
            Self::Stateful(_) => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatefulCommandKind {
    ValidateApplication,
    ActivateApplication,
    ReconcileActivation,
}

impl StatefulCommandKind {
    const fn from_semantic_tag(tag: i64) -> Option<Option<Self>> {
        match tag {
            0 => Some(None),
            1 => Some(Some(Self::ValidateApplication)),
            2 => Some(Some(Self::ActivateApplication)),
            3 => Some(Some(Self::ReconcileActivation)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOutcomeKind {
    KnownSuccess,
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

impl HostOutcomeKind {
    pub const fn semantic_tag(self) -> i64 {
        match self {
            Self::KnownSuccess => 1,
            Self::KnownFailureBeforeVisibility => 2,
            Self::OutcomeUnknown => 3,
            Self::ReconciliationPresent => 4,
            Self::ReconciliationAbsent => 5,
            Self::ReconciliationIndeterminate => 6,
            Self::CancelledBeforeAction => 7,
            Self::TimeoutBeforeAction => 8,
            Self::TimeoutAfterPossibleVisibility => 9,
            Self::CleanupFailure => 10,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulCommand {
    pub kind: StatefulCommandKind,
    pub application: ApplicationDigest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulTransition {
    pub state: ApplicationValue,
    pub response: ByteString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<StatefulCommand>,
    pub compile_nanoseconds: u64,
    pub execute_nanoseconds: u64,
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
    pub execute_nanoseconds: u64,
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

pub fn prepare(
    request: &ApplicationBuildRequest,
    supplied_release_bytes: &[Vec<u8>],
) -> Result<PreparedApplication> {
    validate_contract_version(request.version)?;
    let graph = decode_graph(request.root_release, supplied_release_bytes)?;
    let tests = normalize_tests(&request.tests)?;
    let flattened = graph.flatten()?;
    validate_manifest(
        &graph,
        &flattened,
        request.entry,
        &request.profile,
        request.policy,
        &tests,
    )?;
    let entry = flattened.item(request.entry.release, request.entry.item)?;
    compile::compile(&flattened.snapshot, entry)?;
    let bytes = encode_application(
        &graph,
        request.entry,
        &request.profile,
        request.policy,
        &tests,
    )?;
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

pub fn validate(bytes: &[u8]) -> Result<ApplicationInspection> {
    inspection(&decode_application(bytes)?)
}

pub fn inspect(bytes: &[u8]) -> Result<ApplicationInspection> {
    validate(bytes)
}

pub fn test(bytes: &[u8]) -> Result<ApplicationTestReport> {
    run_all_tests(&decode_application(bytes)?)
}

pub fn run(bytes: &[u8], invocation: &ApplicationInvocation) -> Result<ApplicationRunReceipt> {
    validate_contract_version(invocation.version)?;
    let application = decode_application(bytes)?;
    let arguments = invocation
        .arguments
        .iter()
        .map(|value| to_runtime(&application.flattened, value))
        .collect::<Result<Vec<_>>>()?;
    let entry = application
        .flattened
        .item(application.entry.release, application.entry.item)?;
    let result = interpret::compile_and_run(
        &application.flattened.snapshot,
        entry,
        &arguments,
        application.policy,
    )?;
    Ok(ApplicationRunReceipt {
        contract_version: APPLICATION_CONTRACT_VERSION,
        digest: application.digest,
        result: ApplicationRunResult {
            value: from_runtime(&application.flattened, &result.value)?,
            compile_nanoseconds: result.compile_nanoseconds,
            execute_nanoseconds: result.execute_nanoseconds,
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
    let application = decode_application(bytes)?;
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
    outcome: HostOutcomeKind,
    evidence: &ByteString,
) -> Result<StatefulTransition> {
    let application = decode_application(bytes)?;
    let profile = stateful_profile(&application)?;
    run_stateful(
        &application,
        profile.resume,
        profile,
        &[
            state.clone(),
            ApplicationValue::I64(outcome.semantic_tag()),
            ApplicationValue::Bytes(evidence.clone()),
        ],
    )
}

fn stateful_profile(application: &DecodedApplication) -> Result<StatefulApplicationProfile> {
    match &application.profile {
        InvocationProfile::Stateful(profile) => Ok(*profile),
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
    let arguments = arguments
        .iter()
        .map(|value| to_runtime(&application.flattened, value))
        .collect::<Result<Vec<_>>>()?;
    let result = interpret::compile_and_run(
        &application.flattened.snapshot,
        target_node,
        &arguments,
        application.policy,
    )?;
    decode_stateful_transition(
        &application.flattened,
        profile,
        result.value,
        result.compile_nanoseconds,
        result.execute_nanoseconds,
    )
}

fn decode_stateful_transition(
    flattened: &FlattenedGraph,
    profile: StatefulApplicationProfile,
    value: RuntimeValue,
    compile_nanoseconds: u64,
    execute_nanoseconds: u64,
) -> Result<StatefulTransition> {
    let value = from_runtime(flattened, &value)?;
    let ApplicationValue::Product { ty, fields } = value else {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "stateful application returned a non-product transition",
        ));
    };
    if ty != profile.decision || fields.len() != 4 {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "stateful application returned the wrong transition product",
        ));
    }
    let expected = [
        profile.state_field,
        profile.response_field,
        profile.command_field,
        profile.target_field,
    ];
    let mut values = Vec::with_capacity(4);
    for (field, expected) in fields.into_iter().zip(expected) {
        if field.field != expected {
            return Err(LkError::new(
                ErrorCode::CoreIrInvalid,
                "stateful application returned transition fields out of contract order",
            ));
        }
        values.push(field.value);
    }
    let mut values = values.into_iter();
    let state = values
        .next()
        .ok_or_else(|| corrupt("transition state is absent"))?;
    let response = match values.next() {
        Some(ApplicationValue::Bytes(value)) => value,
        _ => return Err(corrupt("transition response is not bytes")),
    };
    let command_tag = match values.next() {
        Some(ApplicationValue::I64(value)) => value,
        _ => return Err(corrupt("transition command tag is not i64")),
    };
    let target = match values.next() {
        Some(ApplicationValue::Bytes(value)) => value,
        _ => return Err(corrupt("transition command target is not bytes")),
    };
    let command_kind = StatefulCommandKind::from_semantic_tag(command_tag).ok_or_else(|| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            "stateful application returned an unknown host-command tag",
        )
    })?;
    let command = match command_kind {
        None if target.is_empty() => None,
        None => {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "completed transition must carry an empty command target",
            ));
        }
        Some(kind) => {
            let application = <[u8; 32]>::try_from(target.as_slice()).map_err(|_| {
                LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "activation command target must be one exact 32-byte application digest",
                )
            })?;
            Some(StatefulCommand {
                kind,
                application: ApplicationDigest::from_bytes(application),
            })
        }
    };
    Ok(StatefulTransition {
        state,
        response,
        command,
        compile_nanoseconds,
        execute_nanoseconds,
    })
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
    if let InvocationProfile::Stateful(stateful) = profile
        && !tests.iter().any(|test| test.target == stateful.resume)
    {
        return Err(LkError::new(
            ErrorCode::ApplicationTestFailed,
            "a stateful application requires at least one exact resume-entry test",
        ));
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
            validate_stateful_profile(graph, flattened, parameters, *result, *stateful)
        }
    }
}

fn validate_stateful_profile(
    graph: &ReleaseGraph,
    flattened: &FlattenedGraph,
    event_parameters: &[crate::ids::NodeId],
    event_result: SemanticType,
    profile: StatefulApplicationProfile,
) -> Result<()> {
    let snapshot = &flattened.snapshot;
    if event_parameters.len() != 2 {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "stateful event entry must accept exact state and event values",
        ));
    }
    let state_type = parameter_type(snapshot, event_parameters[0])?;
    let event_type = parameter_type(snapshot, event_parameters[1])?;
    if !matches!(state_type, SemanticType::Nominal(_))
        || !matches!(event_type, SemanticType::Nominal(_))
    {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful state and event types must be nominal",
        ));
    }

    let decision = flattened.item(profile.decision.release, profile.decision.item)?;
    if event_result != SemanticType::Nominal(decision) {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful event entry must return the declared transition product",
        ));
    }
    let Node::ProductType { fields, .. } = snapshot.node(decision)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "stateful transition result must be a nominal product",
        ));
    };
    let declared_fields = [
        profile.state_field,
        profile.response_field,
        profile.command_field,
        profile.target_field,
    ];
    let mapped_fields = declared_fields
        .iter()
        .map(|target| flattened.item(target.release, target.item))
        .collect::<Result<Vec<_>>>()?;
    if fields.as_slice() != mapped_fields.as_slice() {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "stateful transition fields must be declared once in state, response, command, target order",
        ));
    }
    let expected_types = [
        state_type,
        SemanticType::Bytes,
        SemanticType::I64,
        SemanticType::Bytes,
    ];
    for (field, expected) in mapped_fields.iter().zip(expected_types) {
        let Node::ProductField { owner, ty, .. } = snapshot.node(*field)? else {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "stateful transition field mapping must target product fields",
            ));
        };
        if *owner != decision || *ty != expected {
            return Err(LkError::new(
                ErrorCode::TypeMismatch,
                "stateful transition field has the wrong owner or exact type",
            ));
        }
    }

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
    if parameters.len() != 3
        || parameter_type(snapshot, parameters[0])? != state_type
        || parameter_type(snapshot, parameters[1])? != SemanticType::I64
        || parameter_type(snapshot, parameters[2])? != SemanticType::Bytes
        || *result != SemanticType::Nominal(decision)
    {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            "stateful resume entry must accept state, outcome i64, evidence bytes and return the transition product",
        ));
    }
    Ok(())
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
    put_profile(&mut payload, profile);
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
    let root_position = decoded
        .iter()
        .position(|release| release.id == root_release)
        .ok_or_else(|| corrupt("application root release is absent"))?;
    let root = decoded.remove(root_position);
    let graph = ReleaseGraph::new(root, decoded)?;
    let flattened = graph.flatten()?;
    validate_manifest(&graph, &flattened, entry, &profile, policy, &tests)
        .map_err(decoded_error)?;
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
        semantic_schema: "lkjscript-tsm006",
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
    let encoded = serde_json::to_vec(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode application value: {error}"),
        )
    })?;
    if encoded.len() > MAXIMUM_APPLICATION_VALUE_JSON_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application value encoding exceeds byte policy",
        ));
    }
    writer.bytes(&encoded).map_err(application_codec)
}

fn read_value(reader: &mut Reader<'_>) -> Result<ApplicationValue> {
    let encoded = reader
        .bytes(MAXIMUM_APPLICATION_VALUE_JSON_BYTES)
        .map_err(application_codec)?;
    let value = serde_json::from_slice::<ApplicationValue>(encoded)
        .map_err(|error| corrupt(&format!("application value JSON is malformed: {error}")))?;
    if serde_json::to_vec(&value)
        .map_err(|error| corrupt(&format!("application value cannot be encoded: {error}")))?
        != encoded
    {
        return Err(corrupt("application value JSON is not canonical"));
    }
    Ok(value)
}

fn put_target(writer: &mut Writer, target: ApplicationTarget) {
    writer.fixed(&target.release.as_bytes());
    writer.u64(target.item.get());
}

fn put_profile(writer: &mut Writer, profile: &InvocationProfile) {
    writer.u8(profile.stable_tag());
    if let InvocationProfile::Stateful(stateful) = profile {
        put_target(writer, stateful.resume);
        put_target(writer, stateful.decision);
        put_target(writer, stateful.state_field);
        put_target(writer, stateful.response_field);
        put_target(writer, stateful.command_field);
        put_target(writer, stateful.target_field);
    }
}

fn read_profile(reader: &mut Reader<'_>) -> Result<InvocationProfile> {
    Ok(match reader.u8().map_err(application_codec)? {
        1 => InvocationProfile::Typed,
        2 => InvocationProfile::BytesStream,
        3 => InvocationProfile::Stateful(StatefulApplicationProfile {
            resume: read_target(reader)?,
            decision: read_target(reader)?,
            state_field: read_target(reader)?,
            response_field: read_target(reader)?,
            command_field: read_target(reader)?,
            target_field: read_target(reader)?,
        }),
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

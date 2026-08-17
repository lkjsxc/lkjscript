//! Canonical, run-only application artifacts.
//!
//! An application artifact contains one exact semantic dependency closure, one entry and
//! invocation profile, and immutable invocation cases. It retains source workspace-qualified
//! semantic identities so nominal equality remains exact after transfer. It does not contain
//! workspace history, HEAD, idempotency state, aliases, proposal text, or executable IR.

use crate::artifact;
use crate::codec::{CodecError, CodecErrorKind, Reader, Writer};
use crate::compile;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision, SnapshotHash, WorkspaceId};
use crate::interpret::{self, RunPolicy, RunResult, RuntimeValue};
use crate::query;
use crate::schema::{ByteString, DirectReference, Node, SemanticType};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

pub const APPLICATION_MAGIC: [u8; 8] = *b"LKJAPP\0\x01";
pub const APPLICATION_FORMAT_VERSION: u16 = 1;
pub const APPLICATION_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_APPLICATION_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_APPLICATION_TESTS: usize = 256;
pub const MAXIMUM_APPLICATION_TEST_NAME_BYTES: usize = 64;
pub const MAXIMUM_APPLICATION_NODES: usize = 100_000;
pub const MAXIMUM_APPLICATION_DURABLE_SERIAL: u64 = 262_144;
pub const MAXIMUM_APPLICATION_SUITE_FUEL: u64 = 100_000_000;
pub const MAXIMUM_APPLICATION_PATH_BYTES: usize = 4_096;
const MAXIMUM_RUNTIME_VALUE_JSON_BYTES: usize = 1024 * 1024;
const APPLICATION_DIGEST_DOMAIN: &str = "lkjscript.application-artifact.v1";
const APPLICATION_TEST_DIGEST_DOMAIN: &str = "lkjscript.application-test-case.v1";
const APPLICATION_DIGEST_BYTES: usize = 32;
const TEMPORARY_PREFIX: &str = ".lkjscript-application-";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationDigest([u8; APPLICATION_DIGEST_BYTES]);

impl ApplicationDigest {
    pub const fn as_bytes(self) -> [u8; APPLICATION_DIGEST_BYTES] {
        self.0
    }
}

impl fmt::Display for ApplicationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationTestDigest([u8; APPLICATION_DIGEST_BYTES]);

impl ApplicationTestDigest {
    pub const fn as_bytes(self) -> [u8; APPLICATION_DIGEST_BYTES] {
        self.0
    }
}

impl fmt::Display for ApplicationTestDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationProfile {
    Typed,
    BytesStream,
}

impl InvocationProfile {
    const fn stable_tag(self) -> u8 {
        match self {
            Self::Typed => 1,
            Self::BytesStream => 2,
        }
    }

    fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Typed),
            2 => Some(Self::BytesStream),
            _ => None,
        }
    }
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

    fn from_stable_tag(tag: u8) -> Option<Self> {
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

    const fn error_code(self) -> ErrorCode {
        match self {
            Self::RuntimeTrap => ErrorCode::RuntimeTrap,
            Self::ByteIndexOutOfBounds => ErrorCode::ByteIndexOutOfBounds,
            Self::ByteSliceOutOfBounds => ErrorCode::ByteSliceOutOfBounds,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTrap {
    pub code: ApplicationTrapCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<NodeId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationTestExpectation {
    Value(RuntimeValue),
    Trap(ApplicationTrap),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTestCase {
    pub name: String,
    pub target: NodeId,
    pub arguments: Vec<RuntimeValue>,
    pub expected: ApplicationTestExpectation,
    pub policy: RunPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationBuildRequest {
    pub version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub entry: NodeId,
    pub profile: InvocationProfile,
    pub policy: RunPolicy,
    pub tests: Vec<ApplicationTestCase>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationInvocation {
    pub version: u16,
    pub arguments: Vec<RuntimeValue>,
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
    pub observed_trap: Option<ApplicationTrap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTestReport {
    pub total: u64,
    pub passed: u64,
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
    pub target: NodeId,
    pub argument_types: Vec<SemanticType>,
    pub result_type: SemanticType,
    pub expected: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_trap: Option<ApplicationTrap>,
    pub policy: RunPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationLimits {
    pub maximum_artifact_bytes: u64,
    pub maximum_path_bytes: u64,
    pub maximum_semantic_nodes: u64,
    pub maximum_release_tests: u64,
    pub maximum_test_name_bytes: u64,
    pub maximum_durable_serial: u64,
    pub maximum_suite_fuel: u64,
    pub maximum_runtime_value_json_bytes: u64,
    pub maximum_run_arguments: u64,
    pub maximum_run_fuel: u64,
    pub maximum_run_frames: u64,
    pub maximum_run_live_cells: u64,
    pub maximum_runtime_value_depth: u64,
    pub maximum_runtime_value_items: u64,
    pub maximum_runtime_value_bytes: u64,
    pub maximum_run_argument_byte_bytes: u64,
    pub maximum_managed_visible_bytes: u64,
    pub maximum_retained_backing_bytes: u64,
    pub maximum_managed_objects: u64,
    pub maximum_stream_input_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationInspection {
    pub contract_version: u16,
    pub format_version: u16,
    pub semantic_schema: &'static str,
    pub digest: ApplicationDigest,
    pub semantic_digest: SnapshotHash,
    pub source_workspace: WorkspaceId,
    pub source_revision: Revision,
    pub entry: NodeId,
    pub profile: InvocationProfile,
    pub policy: RunPolicy,
    pub limits: ApplicationLimits,
    pub tests: Vec<ApplicationTestSummary>,
    pub node_count: u64,
    pub durable_identity_count: u64,
    pub function_local_reference_count: u64,
    pub artifact_bytes: u64,
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
pub struct ApplicationRunReceipt {
    pub contract_version: u16,
    pub digest: ApplicationDigest,
    pub result: RunResult,
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

    /// Atomically publishes these already decoded, compiled, and release-tested bytes.
    pub fn publish(&self, path: &Path) -> Result<()> {
        publish_with_fault(path, &self.bytes, PublicationFault::None)
    }
}

#[derive(Clone, Debug)]
struct DecodedApplication {
    snapshot: Snapshot,
    entry: NodeId,
    profile: InvocationProfile,
    policy: RunPolicy,
    tests: Vec<ApplicationTestCase>,
    digest: ApplicationDigest,
    artifact_bytes: usize,
}

pub(crate) fn prepare(
    source: &Snapshot,
    request: &ApplicationBuildRequest,
) -> Result<PreparedApplication> {
    validate_contract_version(request.version)?;
    if request.workspace != source.workspace() {
        return Err(LkError::new(
            ErrorCode::WrongWorkspace,
            "application build workspace does not match the selected authority",
        )
        .for_workspace(request.workspace)
        .at_revision(request.revision)
        .for_node(request.entry));
    }
    if request.revision != source.revision() {
        return Err(LkError::new(
            ErrorCode::RevisionConflict,
            "application build revision does not match the selected immutable snapshot",
        )
        .for_workspace(request.workspace)
        .at_revision(request.revision)
        .for_node(request.entry));
    }
    let tests = normalize_tests(&request.tests)?;
    validate_manifest(
        source,
        request.entry,
        request.profile,
        request.policy,
        &tests,
        false,
    )?;
    let mut roots = Vec::with_capacity(tests.len().saturating_add(1));
    roots.push(request.entry);
    roots.extend(tests.iter().map(|test| test.target));
    let snapshot = project_snapshot(source, &roots)?;
    validate_manifest(
        &snapshot,
        request.entry,
        request.profile,
        request.policy,
        &tests,
        true,
    )?;
    compile::compile(&snapshot, request.entry)?;
    let bytes = encode_application(
        &snapshot,
        request.entry,
        request.profile,
        request.policy,
        &tests,
    )?;
    let decoded = decode_application(&bytes)?;
    let report = run_declared_tests(&decoded);
    if !report.all_passed() {
        let failed = report
            .results
            .iter()
            .filter(|result| result.status != ApplicationTestStatus::Passed)
            .take(8)
            .map(|result| result.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(LkError::new(
            ErrorCode::ApplicationTestFailed,
            format!("application release tests did not pass: {failed}"),
        )
        .for_workspace(source.workspace())
        .at_revision(source.revision())
        .for_node(request.entry));
    }
    let inspection = inspection(&decoded)?;
    Ok(PreparedApplication {
        bytes,
        inspection,
        tests: report,
    })
}

pub fn validate(bytes: &[u8]) -> Result<ApplicationInspection> {
    let application = decode_application(bytes)?;
    inspection(&application)
}

pub fn inspect(bytes: &[u8]) -> Result<ApplicationInspection> {
    validate(bytes)
}

pub fn test(bytes: &[u8]) -> Result<ApplicationTestReport> {
    let application = decode_application(bytes)?;
    Ok(run_declared_tests(&application))
}

pub fn run(bytes: &[u8], invocation: &ApplicationInvocation) -> Result<ApplicationRunReceipt> {
    validate_contract_version(invocation.version)?;
    let application = decode_application(bytes)?;
    let result = interpret::compile_and_run(
        &application.snapshot,
        application.entry,
        &invocation.arguments,
        application.policy,
    )?;
    Ok(ApplicationRunReceipt {
        contract_version: APPLICATION_CONTRACT_VERSION,
        digest: application.digest,
        result,
    })
}

pub fn run_stream(bytes: &[u8], input: &[u8]) -> Result<Vec<u8>> {
    let application = decode_application(bytes)?;
    if application.profile != InvocationProfile::BytesStream {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "application artifact does not declare the bytes_stream invocation profile",
        )
        .for_workspace(application.snapshot.workspace())
        .at_revision(application.snapshot.revision())
        .for_node(application.entry));
    }
    let input = ByteString::from_slice(input).map_err(|_| {
        LkError::new(
            ErrorCode::RuntimeByteInputTooLarge,
            "stream input exceeds the bytes invocation policy",
        )
        .for_node(application.entry)
    })?;
    let result = interpret::compile_and_run(
        &application.snapshot,
        application.entry,
        &[RuntimeValue::Bytes(input)],
        application.policy,
    )?;
    let RuntimeValue::Bytes(output) = result.value else {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "bytes_stream entry returned a non-bytes value after profile validation",
        )
        .for_node(application.entry));
    };
    Ok(output.into_vec())
}

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    validate_existing_regular_path(path, "application artifact")?;
    let file = File::open(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!(
                "cannot open application artifact {}: {error}",
                path.display()
            ),
        )
    })?;
    let limit = u64::try_from(MAXIMUM_APPLICATION_ARTIFACT_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!(
                "cannot read application artifact {}: {error}",
                path.display()
            ),
        )
    })?;
    if bytes.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact exceeds the input byte policy",
        ));
    }
    Ok(bytes)
}

fn normalize_tests(tests: &[ApplicationTestCase]) -> Result<Vec<ApplicationTestCase>> {
    if tests.is_empty() {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifacts require at least one release test",
        ));
    }
    if tests.len() > MAXIMUM_APPLICATION_TESTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application release test count exceeds policy",
        ));
    }
    let mut normalized = tests.to_vec();
    normalized.sort_by(|left, right| left.name.cmp(&right.name));
    for test in &normalized {
        validate_test_name(&test.name)?;
    }
    if normalized
        .windows(2)
        .any(|tests| tests[0].name == tests[1].name)
    {
        return Err(LkError::new(
            ErrorCode::DuplicateName,
            "application release test names must be unique",
        ));
    }
    Ok(normalized)
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
            "application test name must match [a-z][a-z0-9_]* within its byte policy",
        ))
    }
}

fn validate_manifest(
    snapshot: &Snapshot,
    entry: NodeId,
    profile: InvocationProfile,
    policy: RunPolicy,
    tests: &[ApplicationTestCase],
    require_exact_closure: bool,
) -> Result<()> {
    interpret::validate_policy(policy)?;
    validate_profile(snapshot, entry, profile)?;
    if tests.is_empty() || tests.len() > MAXIMUM_APPLICATION_TESTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application release test count is outside policy",
        )
        .for_node(entry));
    }
    if !tests.iter().any(|test| test.target == entry) {
        return Err(LkError::new(
            ErrorCode::ApplicationTestFailed,
            "at least one release test must target the application entry",
        )
        .for_node(entry));
    }
    let mut suite_fuel = 0_u64;
    let mut prior_name: Option<&str> = None;
    for test in tests {
        validate_test_name(&test.name)?;
        if prior_name.is_some_and(|prior| prior >= test.name.as_str()) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "application release tests are not in strict canonical name order",
            ));
        }
        prior_name = Some(&test.name);
        suite_fuel = suite_fuel.checked_add(test.policy.fuel).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "application release test suite fuel overflows",
            )
        })?;
        if suite_fuel > MAXIMUM_APPLICATION_SUITE_FUEL {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "application release test suite fuel exceeds policy",
            ));
        }
        validate_test_case(snapshot, test)?;
    }
    for root in std::iter::once(entry).chain(tests.iter().map(|test| test.target)) {
        let blockers = query::entry_blockers(snapshot, root)?;
        if !blockers.is_empty() {
            return Err(LkError::new(
                ErrorCode::CompileIncomplete,
                "application entry or test dependency closure is incomplete",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision())
            .for_node(root)
            .with_related(
                blockers
                    .iter()
                    .map(|blocker| blocker.target.unwrap_or(blocker.owner)),
            ));
        }
    }
    if require_exact_closure {
        if snapshot
            .nodes()
            .any(|(_, node)| matches!(node, Node::Package { entry: Some(_), .. }))
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "application semantic closure must not carry a workspace package entry",
            ));
        }
        let mut roots = Vec::with_capacity(tests.len().saturating_add(1));
        roots.push(entry);
        roots.extend(tests.iter().map(|test| test.target));
        let expected = semantic_closure_ids(snapshot, &roots)?;
        let actual = snapshot.nodes().map(|(id, _)| id).collect::<BTreeSet<_>>();
        if expected != actual {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "application artifact contains missing or unrelated semantic content",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision())
            .for_node(entry));
        }
    }
    Ok(())
}

fn validate_profile(snapshot: &Snapshot, entry: NodeId, profile: InvocationProfile) -> Result<()> {
    let Node::Function {
        parameters, result, ..
    } = snapshot.node(entry)?
    else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "application entry must be a function")
                .for_node(entry),
        );
    };
    if profile == InvocationProfile::Typed {
        return Ok(());
    }
    if parameters.len() != 1 || *result != SemanticType::Bytes {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "bytes_stream entry must accept exactly one bytes value and return bytes",
        )
        .for_node(entry));
    }
    let Node::Parameter { ty, .. } = snapshot.node(parameters[0])? else {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "bytes_stream entry parameter slot is not a parameter",
        )
        .for_node(parameters[0]));
    };
    if *ty != SemanticType::Bytes {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "bytes_stream entry parameter must have bytes type",
        )
        .for_node(parameters[0]));
    }
    Ok(())
}

fn validate_test_case(snapshot: &Snapshot, test: &ApplicationTestCase) -> Result<()> {
    interpret::validate_policy(test.policy)?;
    let result_type = interpret::validate_invocation(snapshot, test.target, &test.arguments)?;
    match &test.expected {
        ApplicationTestExpectation::Value(value) => {
            interpret::validate_runtime_value(snapshot, value, result_type, test.target)?;
        }
        ApplicationTestExpectation::Trap(trap) => {
            let _ = trap.code.error_code();
            if let Some(target) = trap.target {
                snapshot.node(target)?;
            }
        }
    }
    Ok(())
}

fn semantic_closure_ids(snapshot: &Snapshot, roots: &[NodeId]) -> Result<BTreeSet<NodeId>> {
    let mut selected = BTreeSet::new();
    let mut pending = roots.iter().copied().collect::<BTreeSet<_>>();
    while let Some(target) = pending.pop_first() {
        let definition = closure_definition(snapshot, target)?;
        let mut stack = vec![definition];
        let mut added = Vec::new();
        while let Some(id) = stack.pop() {
            if !selected.insert(id) {
                continue;
            }
            let node = snapshot.node(id)?;
            added.push(id);
            for index in (0..node.owned_child_count()).rev() {
                if let Some(child) = node.owned_child(index) {
                    stack.push(child);
                }
            }
        }
        for id in added {
            let node = snapshot.node(id)?;
            for index in 0..node.direct_reference_count() {
                let Some(reference) = node.direct_reference(index) else {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "semantic direct-reference count is inconsistent",
                    )
                    .for_node(id));
                };
                match reference {
                    DirectReference::Definition { target }
                    | DirectReference::Type { target, .. } => {
                        let dependency = closure_definition(snapshot, target)?;
                        if !selected.contains(&dependency) {
                            pending.insert(dependency);
                        }
                    }
                    DirectReference::ValueOperand { value, .. } => {
                        let value = value.referenced_node();
                        if !selected.contains(&value) {
                            return Err(LkError::new(
                                ErrorCode::InvalidContainment,
                                "application function closure contains a foreign value reference",
                            )
                            .for_node(value)
                            .with_related([id]));
                        }
                    }
                }
            }
        }
    }
    let semantic = selected.iter().copied().collect::<Vec<_>>();
    for id in semantic {
        let mut current = snapshot.node(id)?.owner();
        while let Some(owner) = current {
            selected.insert(owner);
            current = snapshot.node(owner)?.owner();
        }
    }
    selected.insert(snapshot.root());
    Ok(selected)
}

fn closure_definition(snapshot: &Snapshot, target: NodeId) -> Result<NodeId> {
    Ok(match snapshot.node(target)? {
        Node::Function { .. } | Node::ProductType { .. } | Node::SumType { .. } => target,
        Node::ProductField { owner, .. } | Node::SumVariant { owner, .. } => *owner,
        node => {
            let mut current = node.owner();
            let mut function = None;
            while let Some(owner) = current {
                match snapshot.node(owner)? {
                    Node::Function { .. } => {
                        function = Some(owner);
                        break;
                    }
                    parent => current = parent.owner(),
                }
            }
            function.ok_or_else(|| {
                LkError::new(
                    ErrorCode::WrongKind,
                    "application closure root is not a function or nominal declaration",
                )
                .for_node(target)
            })?
        }
    })
}

fn project_snapshot(source: &Snapshot, roots: &[NodeId]) -> Result<Snapshot> {
    let selected = semantic_closure_ids(source, roots)?;
    let mut nodes = BTreeMap::new();
    for id in &selected {
        let node = normalize_application_node(source.node(*id)?.clone(), &selected);
        nodes.insert(*id, node);
    }
    snapshot_from_application_nodes(source.workspace(), source.revision(), nodes, false)
}

fn normalize_application_node(mut node: Node, selected: &BTreeSet<NodeId>) -> Node {
    match &mut node {
        Node::WorkspaceRoot { packages } => packages.retain(|id| selected.contains(id)),
        Node::Package { modules, entry, .. } => {
            modules.retain(|id| selected.contains(id));
            *entry = None;
        }
        Node::Module {
            types, functions, ..
        } => {
            types.retain(|id| selected.contains(id));
            functions.retain(|id| selected.contains(id));
        }
        Node::ProductType { fields, .. } => fields.retain(|id| selected.contains(id)),
        Node::SumType { variants, .. } => variants.retain(|id| selected.contains(id)),
        Node::Function {
            parameters, body, ..
        } => {
            parameters.retain(|id| selected.contains(id));
            if body.is_some_and(|id| !selected.contains(&id)) {
                *body = None;
            }
        }
        Node::Region { blocks, .. } => blocks.retain(|id| selected.contains(id)),
        Node::Block {
            arguments,
            operations,
            terminator,
            ..
        } => {
            arguments.retain(|id| selected.contains(id));
            operations.retain(|id| selected.contains(id));
            if terminator.is_some_and(|id| !selected.contains(&id)) {
                *terminator = None;
            }
        }
        Node::ProductField { .. }
        | Node::SumVariant { .. }
        | Node::Parameter { .. }
        | Node::BlockArgument { .. }
        | Node::Operation { .. } => {}
    }
    node
}

fn snapshot_from_application_nodes(
    workspace: WorkspaceId,
    revision: Revision,
    nodes: BTreeMap<NodeId, Node>,
    decoded: bool,
) -> Result<Snapshot> {
    let maximum = nodes
        .keys()
        .filter(|id| id.is_durable())
        .map(|id| id.serial())
        .max()
        .unwrap_or(0);
    if maximum == 0 || maximum > MAXIMUM_APPLICATION_DURABLE_SERIAL {
        return Err(LkError::new(
            if decoded {
                ErrorCode::ArtifactCorrupt
            } else {
                ErrorCode::PolicyExceeded
            },
            "application durable identity range is outside policy",
        ));
    }
    let next_serial = maximum.checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "application durable identity frontier overflows",
        )
    })?;
    let root = NodeId::new(workspace, 1).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("application root identity is invalid: {error}"),
        )
    })?;
    let tombstones = (1..next_serial)
        .filter(|serial| {
            NodeId::new(workspace, *serial)
                .ok()
                .is_none_or(|id| !nodes.contains_key(&id))
        })
        .collect::<BTreeSet<_>>();
    Snapshot::from_parts(workspace, revision, root, next_serial, tombstones, nodes).map_err(
        |mut error| {
            if decoded && error.code != ErrorCode::WrongWorkspace {
                error.code = ErrorCode::ArtifactCorrupt;
            }
            error
        },
    )
}

fn encode_application(
    snapshot: &Snapshot,
    entry: NodeId,
    profile: InvocationProfile,
    policy: RunPolicy,
    tests: &[ApplicationTestCase],
) -> Result<Vec<u8>> {
    let mut payload = Writer::new();
    payload.fixed(&snapshot.workspace().as_bytes());
    payload.u64(snapshot.revision().get());
    put_node_id(&mut payload, entry);
    payload.u8(profile.stable_tag());
    put_policy(&mut payload, policy);
    put_count(&mut payload, tests.len())?;
    for test in tests {
        put_test_case(&mut payload, test)?;
    }
    put_count(&mut payload, snapshot.node_count())?;
    for (id, node) in snapshot.nodes() {
        put_node_id(&mut payload, id);
        artifact::put_node(&mut payload, node)?;
    }
    let payload = payload.finish();
    if payload.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact payload exceeds byte policy",
        ));
    }
    let digest = digest_payload(&payload);
    let mut writer = Writer::with_capacity(
        APPLICATION_MAGIC.len()
            + 2
            + artifact::SCHEMA_ID.0.len()
            + 8
            + payload.len()
            + APPLICATION_DIGEST_BYTES,
    );
    writer.fixed(&APPLICATION_MAGIC);
    writer.u16(APPLICATION_FORMAT_VERSION);
    writer.fixed(&artifact::SCHEMA_ID.0);
    writer.u64(u64::try_from(payload.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "application payload length does not fit canonical u64",
        )
    })?);
    writer.fixed(&payload);
    writer.fixed(&digest.0);
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
    let length_offset = reader.position();
    let payload_length =
        usize::try_from(reader.u64().map_err(application_codec)?).map_err(|_| {
            corrupt(&format!(
                "application payload length overflows host indexes at byte {length_offset}"
            ))
        })?;
    if payload_length > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application payload exceeds decoder byte policy",
        ));
    }
    let payload = reader.fixed(payload_length).map_err(application_codec)?;
    let encoded_digest = reader
        .fixed(APPLICATION_DIGEST_BYTES)
        .map_err(application_codec)?;
    reader.finish().map_err(application_codec)?;
    let digest = digest_payload(payload);
    if encoded_digest != digest.0 {
        return Err(corrupt("application artifact content digest is invalid"));
    }

    let mut payload_reader = Reader::new(payload);
    let mut workspace = [0_u8; WorkspaceId::BYTE_LEN];
    workspace.copy_from_slice(
        payload_reader
            .fixed(WorkspaceId::BYTE_LEN)
            .map_err(application_codec)?,
    );
    let workspace = WorkspaceId::from_bytes(workspace);
    let revision = Revision::new(payload_reader.u64().map_err(application_codec)?);
    let entry = read_node_id(&mut payload_reader, workspace)?;
    let profile_tag = payload_reader.u8().map_err(application_codec)?;
    let profile = InvocationProfile::from_stable_tag(profile_tag)
        .ok_or_else(|| corrupt("application artifact invocation profile tag is unknown"))?;
    let policy = read_policy(&mut payload_reader)?;
    let test_count = payload_reader
        .count(MAXIMUM_APPLICATION_TESTS)
        .map_err(application_codec)?;
    if test_count == 0 {
        return Err(corrupt("application artifact contains no release tests"));
    }
    let mut tests = Vec::with_capacity(test_count);
    for _ in 0..test_count {
        let name = payload_reader
            .string(MAXIMUM_APPLICATION_TEST_NAME_BYTES)
            .map_err(application_codec)?;
        let target = read_node_id(&mut payload_reader, workspace)?;
        let test_policy = read_policy(&mut payload_reader)?;
        let argument_count = payload_reader
            .count(interpret::MAX_RUN_ARGUMENTS)
            .map_err(application_codec)?;
        let mut arguments = Vec::with_capacity(argument_count);
        for _ in 0..argument_count {
            arguments.push(read_runtime_value(&mut payload_reader)?);
        }
        let expected = match payload_reader.u8().map_err(application_codec)? {
            1 => ApplicationTestExpectation::Value(read_runtime_value(&mut payload_reader)?),
            2 => {
                let tag = payload_reader.u8().map_err(application_codec)?;
                let code = ApplicationTrapCode::from_stable_tag(tag)
                    .ok_or_else(|| corrupt("application expected-trap tag is unknown"))?;
                let target = read_optional_node_id(&mut payload_reader, workspace)?;
                ApplicationTestExpectation::Trap(ApplicationTrap { code, target })
            }
            _ => return Err(corrupt("application test expectation tag is unknown")),
        };
        tests.push(ApplicationTestCase {
            name,
            target,
            arguments,
            expected,
            policy: test_policy,
        });
    }
    let node_count = payload_reader
        .count(MAXIMUM_APPLICATION_NODES)
        .map_err(application_codec)?;
    if node_count == 0 {
        return Err(corrupt("application semantic closure is empty"));
    }
    let mut nodes = BTreeMap::new();
    let mut prior = None;
    for _ in 0..node_count {
        let id = read_node_id(&mut payload_reader, workspace)?;
        if prior.is_some_and(|prior| prior >= id) {
            return Err(corrupt(
                "application semantic nodes are not in strict canonical identity order",
            ));
        }
        prior = Some(id);
        let node = artifact::read_node(
            &mut payload_reader,
            workspace,
            artifact::DecodePolicy::default(),
        )?;
        nodes.insert(id, node);
    }
    payload_reader.finish().map_err(application_codec)?;
    let snapshot = snapshot_from_application_nodes(workspace, revision, nodes, true)?;
    validate_manifest(&snapshot, entry, profile, policy, &tests, true).map_err(|mut error| {
        if error.code != ErrorCode::PolicyExceeded && error.code != ErrorCode::WrongWorkspace {
            error.code = ErrorCode::ArtifactCorrupt;
        }
        error
    })?;
    let application = DecodedApplication {
        snapshot,
        entry,
        profile,
        policy,
        tests,
        digest,
        artifact_bytes: bytes.len(),
    };
    let canonical = encode_application(
        &application.snapshot,
        application.entry,
        application.profile,
        application.policy,
        &application.tests,
    )?;
    if canonical != bytes {
        return Err(corrupt("application artifact encoding is not canonical"));
    }
    Ok(application)
}

fn run_declared_tests(application: &DecodedApplication) -> ApplicationTestReport {
    let mut results = Vec::with_capacity(application.tests.len());
    let mut passed = 0_u64;
    for test in &application.tests {
        let result = match interpret::compile_and_run(
            &application.snapshot,
            test.target,
            &test.arguments,
            test.policy,
        ) {
            Ok(run) => match &test.expected {
                ApplicationTestExpectation::Value(expected) if run.value == *expected => {
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
            Err(error) => classify_test_error(test, &error),
        };
        if result.status == ApplicationTestStatus::Passed {
            passed = passed.saturating_add(1);
        }
        results.push(result);
    }
    ApplicationTestReport {
        total: u64::try_from(results.len()).unwrap_or(u64::MAX),
        passed,
        results,
    }
}

fn classify_test_error(test: &ApplicationTestCase, error: &LkError) -> ApplicationTestResult {
    let observed_trap = ApplicationTrapCode::from_error(error.code).map(|code| ApplicationTrap {
        code,
        target: error.target,
    });
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
    } else if let Some(actual) = observed_trap {
        match &test.expected {
            ApplicationTestExpectation::Value(_) => ApplicationTestStatus::UnexpectedTrap,
            ApplicationTestExpectation::Trap(expected)
                if expected.code == actual.code
                    && expected
                        .target
                        .is_none_or(|target| Some(target) == actual.target) =>
            {
                ApplicationTestStatus::Passed
            }
            ApplicationTestExpectation::Trap(_) => ApplicationTestStatus::WrongTrap,
        }
    } else {
        ApplicationTestStatus::EngineFailure
    };
    ApplicationTestResult {
        name: test.name.clone(),
        status,
        observed_trap,
    }
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
    let tests = application
        .tests
        .iter()
        .map(|test| {
            let Node::Function {
                parameters, result, ..
            } = application.snapshot.node(test.target)?
            else {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "application test target is not a function during inspection",
                )
                .for_node(test.target));
            };
            let argument_types = parameters
                .iter()
                .map(|parameter| match application.snapshot.node(*parameter)? {
                    Node::Parameter { ty, .. } => Ok(*ty),
                    _ => Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "application function parameter slot is invalid during inspection",
                    )
                    .for_node(*parameter)),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(ApplicationTestSummary {
                name: test.name.clone(),
                case_digest: digest_test_case(test)?,
                target: test.target,
                argument_types,
                result_type: *result,
                expected: match test.expected {
                    ApplicationTestExpectation::Value(_) => "value",
                    ApplicationTestExpectation::Trap(_) => "trap",
                },
                expected_trap: match test.expected {
                    ApplicationTestExpectation::Value(_) => None,
                    ApplicationTestExpectation::Trap(trap) => Some(trap),
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
        semantic_digest: application.snapshot.hash(),
        source_workspace: application.snapshot.workspace(),
        source_revision: application.snapshot.revision(),
        entry: application.entry,
        profile: application.profile,
        policy: application.policy,
        limits: application_limits(),
        tests,
        node_count: u64::try_from(application.snapshot.node_count()).unwrap_or(u64::MAX),
        durable_identity_count: u64::try_from(application.snapshot.durable_identity_count())
            .unwrap_or(u64::MAX),
        function_local_reference_count: u64::try_from(
            application.snapshot.function_local_reference_count(),
        )
        .unwrap_or(u64::MAX),
        artifact_bytes: u64::try_from(application.artifact_bytes).unwrap_or(u64::MAX),
    })
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

fn put_runtime_value(writer: &mut Writer, value: &RuntimeValue) -> Result<()> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode application runtime value: {error}"),
        )
    })?;
    if encoded.len() > MAXIMUM_RUNTIME_VALUE_JSON_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application runtime value encoding exceeds byte policy",
        ));
    }
    writer.bytes(&encoded).map_err(application_codec)
}

fn put_test_case(writer: &mut Writer, test: &ApplicationTestCase) -> Result<()> {
    writer.string(&test.name).map_err(application_codec)?;
    put_node_id(writer, test.target);
    put_policy(writer, test.policy);
    put_count(writer, test.arguments.len())?;
    for argument in &test.arguments {
        put_runtime_value(writer, argument)?;
    }
    match &test.expected {
        ApplicationTestExpectation::Value(value) => {
            writer.u8(1);
            put_runtime_value(writer, value)?;
        }
        ApplicationTestExpectation::Trap(trap) => {
            writer.u8(2);
            writer.u8(trap.code.stable_tag());
            put_optional_node_id(writer, trap.target);
        }
    }
    Ok(())
}

fn read_runtime_value(reader: &mut Reader<'_>) -> Result<RuntimeValue> {
    let encoded = reader
        .bytes(MAXIMUM_RUNTIME_VALUE_JSON_BYTES)
        .map_err(application_codec)?;
    let value = serde_json::from_slice::<RuntimeValue>(encoded).map_err(|error| {
        corrupt(&format!(
            "application runtime value JSON is malformed: {error}"
        ))
    })?;
    let canonical = serde_json::to_vec(&value).map_err(|error| {
        corrupt(&format!(
            "application runtime value JSON cannot be canonicalized: {error}"
        ))
    })?;
    if canonical != encoded {
        return Err(corrupt(
            "application runtime value JSON is not in canonical encoding",
        ));
    }
    Ok(value)
}

fn put_policy(writer: &mut Writer, policy: RunPolicy) {
    writer.u64(policy.fuel);
    writer.u32(policy.maximum_frames);
}

fn read_policy(reader: &mut Reader<'_>) -> Result<RunPolicy> {
    Ok(RunPolicy {
        fuel: reader.u64().map_err(application_codec)?,
        maximum_frames: reader.u32().map_err(application_codec)?,
    })
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

fn put_node_id(writer: &mut Writer, id: NodeId) {
    writer.u64(id.serial());
}

fn read_node_id(reader: &mut Reader<'_>, workspace: WorkspaceId) -> Result<NodeId> {
    let serial = reader.u64().map_err(application_codec)?;
    NodeId::from_encoded(workspace, serial).map_err(|error| {
        corrupt(&format!(
            "application artifact contains an invalid node identity: {error}"
        ))
    })
}

fn put_optional_node_id(writer: &mut Writer, id: Option<NodeId>) {
    writer.bool(id.is_some());
    if let Some(id) = id {
        put_node_id(writer, id);
    }
}

fn read_optional_node_id(
    reader: &mut Reader<'_>,
    workspace: WorkspaceId,
) -> Result<Option<NodeId>> {
    if reader.bool().map_err(application_codec)? {
        read_node_id(reader, workspace).map(Some)
    } else {
        Ok(None)
    }
}

fn digest_payload(payload: &[u8]) -> ApplicationDigest {
    let mut hasher = blake3::Hasher::new_derive_key(APPLICATION_DIGEST_DOMAIN);
    hasher.update(payload);
    ApplicationDigest(*hasher.finalize().as_bytes())
}

fn digest_test_case(test: &ApplicationTestCase) -> Result<ApplicationTestDigest> {
    let mut writer = Writer::new();
    writer.fixed(&test.target.workspace().as_bytes());
    put_test_case(&mut writer, test)?;
    let mut hasher = blake3::Hasher::new_derive_key(APPLICATION_TEST_DIGEST_DOMAIN);
    hasher.update(&writer.finish());
    Ok(ApplicationTestDigest(*hasher.finalize().as_bytes()))
}

fn application_limits() -> ApplicationLimits {
    ApplicationLimits {
        maximum_artifact_bytes: u64::try_from(MAXIMUM_APPLICATION_ARTIFACT_BYTES)
            .unwrap_or(u64::MAX),
        maximum_path_bytes: u64::try_from(MAXIMUM_APPLICATION_PATH_BYTES).unwrap_or(u64::MAX),
        maximum_semantic_nodes: u64::try_from(MAXIMUM_APPLICATION_NODES).unwrap_or(u64::MAX),
        maximum_release_tests: u64::try_from(MAXIMUM_APPLICATION_TESTS).unwrap_or(u64::MAX),
        maximum_test_name_bytes: u64::try_from(MAXIMUM_APPLICATION_TEST_NAME_BYTES)
            .unwrap_or(u64::MAX),
        maximum_durable_serial: MAXIMUM_APPLICATION_DURABLE_SERIAL,
        maximum_suite_fuel: MAXIMUM_APPLICATION_SUITE_FUEL,
        maximum_runtime_value_json_bytes: u64::try_from(MAXIMUM_RUNTIME_VALUE_JSON_BYTES)
            .unwrap_or(u64::MAX),
        maximum_run_arguments: u64::try_from(interpret::MAX_RUN_ARGUMENTS).unwrap_or(u64::MAX),
        maximum_run_fuel: interpret::MAX_RUN_FUEL,
        maximum_run_frames: u64::from(interpret::MAX_RUN_FRAMES),
        maximum_run_live_cells: u64::try_from(interpret::MAX_RUN_LIVE_CELLS).unwrap_or(u64::MAX),
        maximum_runtime_value_depth: u64::try_from(interpret::MAX_RUNTIME_VALUE_DEPTH)
            .unwrap_or(u64::MAX),
        maximum_runtime_value_items: u64::try_from(interpret::MAX_RUNTIME_VALUE_ITEMS)
            .unwrap_or(u64::MAX),
        maximum_runtime_value_bytes: u64::try_from(interpret::MAX_RUNTIME_VALUE_BYTES)
            .unwrap_or(u64::MAX),
        maximum_run_argument_byte_bytes: u64::try_from(interpret::MAX_RUN_ARGUMENT_BYTE_BYTES)
            .unwrap_or(u64::MAX),
        maximum_managed_visible_bytes: u64::try_from(interpret::MAX_RUN_MANAGED_VISIBLE_BYTES)
            .unwrap_or(u64::MAX),
        maximum_retained_backing_bytes: u64::try_from(interpret::MAX_RUN_RETAINED_BACKING_BYTES)
            .unwrap_or(u64::MAX),
        maximum_managed_objects: u64::try_from(interpret::MAX_RUN_MANAGED_OBJECTS)
            .unwrap_or(u64::MAX),
        maximum_stream_input_bytes: u64::try_from(crate::schema::MAXIMUM_BYTE_STRING_BYTES)
            .unwrap_or(u64::MAX),
    }
}

fn application_codec(error: CodecError) -> LkError {
    let code = if error.kind == CodecErrorKind::PolicyExceeded {
        ErrorCode::PolicyExceeded
    } else {
        ErrorCode::ArtifactCorrupt
    };
    LkError::new(
        code,
        format!("canonical application artifact decoding failed: {error}"),
    )
}

fn corrupt(message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

fn validate_canonical_absolute_path(path: &Path) -> Result<()> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() > MAXIMUM_APPLICATION_PATH_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact path exceeds its byte policy",
        ));
    }
    if !path.is_absolute() || bytes.first() != Some(&b'/') {
        return Err(LkError::new(
            ErrorCode::Io,
            "application artifact paths must be absolute",
        ));
    }
    if bytes[1..]
        .split(|byte| *byte == b'/')
        .any(|component| component.is_empty() || component == b"." || component == b"..")
    {
        return Err(LkError::new(
            ErrorCode::Io,
            "application artifact paths must use one separator and no empty or dot components",
        ));
    }
    Ok(())
}

fn validate_existing_regular_path(path: &Path, label: &str) -> Result<()> {
    validate_canonical_absolute_path(path)?;
    validate_parent_chain(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot inspect {label} {}: {error}", path.display()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} must be a regular non-symlink file"),
        ));
    }
    if metadata.len() > u64::try_from(MAXIMUM_APPLICATION_ARTIFACT_BYTES).unwrap_or(u64::MAX) {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} exceeds the artifact byte policy"),
        ));
    }
    Ok(())
}

fn validate_parent_chain(path: &Path) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        LkError::new(
            ErrorCode::Io,
            "application artifact path has no parent directory",
        )
    })?;
    let mut current = PathBuf::from("/");
    for component in parent.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(part) => current.push(part),
            _ => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "application artifact parent path is not canonical",
                ));
            }
        }
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!(
                    "cannot inspect application artifact parent {}: {error}",
                    current.display()
                ),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LkError::new(
                ErrorCode::Io,
                "application artifact parent components must be non-symlink directories",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicationFault {
    None,
    BeforeWrite,
    AfterWrite,
    AfterFileSync,
    AfterLink,
    AfterTemporaryRemoval,
    AfterDirectorySync,
}

fn publish_with_fault(path: &Path, bytes: &[u8], fault: PublicationFault) -> Result<()> {
    if bytes.len() > MAXIMUM_APPLICATION_ARTIFACT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "application artifact publication bytes exceed policy",
        ));
    }
    validate_canonical_absolute_path(path)?;
    validate_parent_chain(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(LkError::new(
                ErrorCode::Io,
                "application artifact destination already exists",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = path.parent().ok_or_else(|| {
        LkError::new(
            ErrorCode::Io,
            "application artifact destination has no parent",
        )
    })?;
    let (temporary_path, mut temporary) = create_temporary_file(parent)?;
    let before_publication = (|| -> Result<()> {
        if fault == PublicationFault::BeforeWrite {
            return Err(LkError::new(
                ErrorCode::Io,
                "injected application publication failure before write",
            ));
        }
        temporary.write_all(bytes)?;
        if fault == PublicationFault::AfterWrite {
            return Err(LkError::new(
                ErrorCode::Io,
                "injected application publication failure after write",
            ));
        }
        temporary.sync_all()?;
        if fault == PublicationFault::AfterFileSync {
            return Err(LkError::new(
                ErrorCode::Io,
                "injected application publication failure after file sync",
            ));
        }
        fs::hard_link(&temporary_path, path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                LkError::new(
                    ErrorCode::Io,
                    "application artifact destination appeared during publication",
                )
            } else {
                LkError::new(
                    ErrorCode::Io,
                    format!("cannot publish application artifact link: {error}"),
                )
            }
        })?;
        Ok(())
    })();
    if let Err(error) = before_publication {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    if fault == PublicationFault::AfterLink {
        let cleanup = fs::remove_file(&temporary_path);
        return Err(LkError::new(
            ErrorCode::ArtifactPublicationOutcomeUnknown,
            match cleanup {
                Ok(()) => "injected failure after application artifact link publication".into(),
                Err(error) => format!(
                    "injected failure after application artifact link publication; temporary cleanup also failed: {error}"
                ),
            },
        ));
    }
    if let Err(error) = fs::remove_file(&temporary_path) {
        return Err(LkError::new(
            ErrorCode::ArtifactPublicationOutcomeUnknown,
            format!("application artifact was linked but temporary cleanup failed: {error}"),
        ));
    }
    if fault == PublicationFault::AfterTemporaryRemoval {
        return Err(LkError::new(
            ErrorCode::ArtifactPublicationOutcomeUnknown,
            "injected failure before application artifact directory sync",
        ));
    }
    let directory = File::open(parent).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactPublicationOutcomeUnknown,
            format!("application artifact was linked but parent open failed: {error}"),
        )
    })?;
    directory.sync_all().map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactPublicationOutcomeUnknown,
            format!("application artifact was linked but parent sync failed: {error}"),
        )
    })?;
    if fault == PublicationFault::AfterDirectorySync {
        return Err(LkError::new(
            ErrorCode::ArtifactPublicationOutcomeUnknown,
            "injected failure after application artifact directory sync",
        ));
    }
    Ok(())
}

fn create_temporary_file(parent: &Path) -> Result<(PathBuf, File)> {
    for _ in 0..16 {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|error| {
            LkError::new(
                ErrorCode::Io,
                format!("cannot generate application temporary file name: {error}"),
            )
        })?;
        let path = parent.join(format!("{TEMPORARY_PREFIX}{}.tmp", hex(&random)));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
                return Ok((path, file));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(LkError::new(
        ErrorCode::Io,
        "cannot allocate a unique application temporary file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Node, OperationKind, ValueRef};

    fn fixture() -> (Snapshot, NodeId) {
        let workspace = WorkspaceId::from_bytes([0x41; 16]);
        let durable = |serial| NodeId::new(workspace, serial).expect("durable ID");
        let local =
            |ordinal| NodeId::new_function_local(workspace, durable(4), ordinal).expect("local ID");
        let nodes = BTreeMap::from([
            (
                durable(1),
                Node::WorkspaceRoot {
                    packages: vec![durable(2)],
                },
            ),
            (
                durable(2),
                Node::Package {
                    owner: durable(1),
                    name: "fixture".into(),
                    modules: vec![durable(3)],
                    entry: Some(durable(4)),
                },
            ),
            (
                durable(3),
                Node::Module {
                    owner: durable(2),
                    name: "main".into(),
                    types: Vec::new(),
                    functions: vec![durable(4), durable(6)],
                },
            ),
            (
                durable(4),
                Node::Function {
                    owner: durable(3),
                    name: "identity".into(),
                    parameters: vec![durable(5)],
                    result: SemanticType::Bytes,
                    body: Some(local(1)),
                },
            ),
            (
                durable(5),
                Node::Parameter {
                    owner: durable(4),
                    ordinal: 0,
                    name: "input".into(),
                    ty: SemanticType::Bytes,
                },
            ),
            (
                durable(6),
                Node::Function {
                    owner: durable(3),
                    name: "unrelated".into(),
                    parameters: Vec::new(),
                    result: SemanticType::Unit,
                    body: None,
                },
            ),
            (
                local(1),
                Node::Region {
                    owner: durable(4),
                    blocks: vec![local(2)],
                },
            ),
            (
                local(2),
                Node::Block {
                    owner: local(1),
                    arguments: Vec::new(),
                    operations: Vec::new(),
                    terminator: Some(local(3)),
                },
            ),
            (
                local(3),
                Node::Operation {
                    owner: local(2),
                    operation: OperationKind::Return {
                        value: ValueRef::FunctionParameter(durable(5)),
                    },
                },
            ),
        ]);
        let snapshot = Snapshot::from_parts(
            workspace,
            Revision::new(7),
            durable(1),
            7,
            BTreeSet::new(),
            nodes,
        )
        .expect("fixture snapshot");
        (snapshot, durable(4))
    }

    fn request(snapshot: &Snapshot, entry: NodeId) -> ApplicationBuildRequest {
        ApplicationBuildRequest {
            version: APPLICATION_CONTRACT_VERSION,
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            entry,
            profile: InvocationProfile::BytesStream,
            policy: RunPolicy {
                fuel: 100,
                maximum_frames: 16,
            },
            tests: vec![ApplicationTestCase {
                name: "identity".into(),
                target: entry,
                arguments: vec![RuntimeValue::Bytes(
                    ByteString::from_slice(b"abc").expect("bytes"),
                )],
                expected: ApplicationTestExpectation::Value(RuntimeValue::Bytes(
                    ByteString::from_slice(b"abc").expect("bytes"),
                )),
                policy: RunPolicy {
                    fuel: 100,
                    maximum_frames: 16,
                },
            }],
        }
    }

    #[test]
    fn application_round_trip_excludes_unrelated_semantics_and_runs_without_workspace() {
        let (snapshot, entry) = fixture();
        let prepared = prepare(&snapshot, &request(&snapshot, entry)).expect("prepare");
        let first = prepared.bytes().to_vec();
        let decoded = decode_application(&first).expect("decode");
        assert!(decoded.snapshot.nodes().all(|(id, _)| id.serial() != 6));
        assert_eq!(
            encode_application(
                &decoded.snapshot,
                decoded.entry,
                decoded.profile,
                decoded.policy,
                &decoded.tests,
            )
            .expect("encode"),
            first
        );
        assert_eq!(
            run_stream(&first, b"transferred").expect("stream"),
            b"transferred"
        );
        assert!(test(&first).expect("tests").all_passed());
        let inspection = inspect(&first).expect("inspect");
        assert_eq!(inspection.tests.len(), 1);
        assert_eq!(
            inspection.tests[0].case_digest,
            digest_test_case(&request(&snapshot, entry).tests[0]).expect("test digest")
        );
        assert_eq!(
            inspection.limits.maximum_artifact_bytes,
            u64::try_from(MAXIMUM_APPLICATION_ARTIFACT_BYTES).expect("artifact limit")
        );
    }

    #[test]
    fn application_rejects_corruption_old_version_trailing_and_failing_release_test() {
        let (snapshot, entry) = fixture();
        let prepared = prepare(&snapshot, &request(&snapshot, entry)).expect("prepare");
        let bytes = prepared.bytes();

        let mut old_request = request(&snapshot, entry);
        old_request.version = 0;
        assert_eq!(
            prepare(&snapshot, &old_request)
                .expect_err("old command contract")
                .code,
            ErrorCode::ProtocolVersion
        );
        assert_eq!(
            run(
                bytes,
                &ApplicationInvocation {
                    version: 0,
                    arguments: vec![RuntimeValue::Bytes(
                        ByteString::from_slice(b"abc").expect("bytes"),
                    )],
                },
            )
            .expect_err("old invocation contract")
            .code,
            ErrorCode::ProtocolVersion
        );

        let mut corrupt_payload = bytes.to_vec();
        corrupt_payload[48] ^= 1;
        assert_eq!(
            validate(&corrupt_payload)
                .expect_err("corrupt payload")
                .code,
            ErrorCode::ArtifactCorrupt
        );
        let mut old = bytes.to_vec();
        old[8..10].copy_from_slice(&0_u16.to_le_bytes());
        assert_eq!(
            validate(&old).expect_err("old version").code,
            ErrorCode::ArtifactCorrupt
        );
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert_eq!(
            validate(&trailing).expect_err("trailing").code,
            ErrorCode::ArtifactCorrupt
        );

        let mut failing = request(&snapshot, entry);
        failing.tests[0].expected = ApplicationTestExpectation::Value(RuntimeValue::Bytes(
            ByteString::from_slice(b"wrong").expect("bytes"),
        ));
        assert_eq!(
            prepare(&snapshot, &failing)
                .expect_err("failing release test")
                .code,
            ErrorCode::ApplicationTestFailed
        );
    }

    #[test]
    fn application_artifact_rejects_ten_thousand_deterministic_mutations() {
        const SEED: u64 = 0x1a11_cafe_2026_0818;
        const CASES: usize = 10_000;

        let (snapshot, entry) = fixture();
        let prepared = prepare(&snapshot, &request(&snapshot, entry)).expect("prepare");
        let original = prepared.bytes();
        let mut state = SEED;
        for case in 0..CASES {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let offset = usize::try_from(state).unwrap_or(0) % original.len();
            let bit = 1_u8 << u32::try_from((state >> 61) & 7).unwrap_or(0);
            let mut mutated = original.to_vec();
            mutated[offset] ^= bit;
            assert!(
                validate(&mutated).is_err(),
                "mutation case {case} at byte {offset} unexpectedly validated"
            );
        }
    }

    #[test]
    fn publication_is_no_overwrite_and_faults_before_or_after_one_atomic_link() {
        let (snapshot, entry) = fixture();
        let prepared = prepare(&snapshot, &request(&snapshot, entry)).expect("prepare");
        let directory = tempfile::tempdir().expect("directory");
        let destination = directory.path().join("application.lkja");
        prepared.publish(&destination).expect("publish");
        assert_eq!(read_file(&destination).expect("read"), prepared.bytes());
        assert_eq!(
            prepared
                .publish(&destination)
                .expect_err("no overwrite")
                .code,
            ErrorCode::Io
        );
        assert_eq!(
            read_file(&destination).expect("unchanged"),
            prepared.bytes()
        );

        for (name, fault) in [
            ("before-write", PublicationFault::BeforeWrite),
            ("after-write", PublicationFault::AfterWrite),
            ("after-file-sync", PublicationFault::AfterFileSync),
        ] {
            let before = directory.path().join(format!("{name}.lkja"));
            assert_eq!(
                publish_with_fault(&before, prepared.bytes(), fault)
                    .expect_err("failure before link")
                    .code,
                ErrorCode::Io
            );
            assert!(!before.exists());
        }

        for (name, fault) in [
            ("after-link", PublicationFault::AfterLink),
            (
                "after-temporary-removal",
                PublicationFault::AfterTemporaryRemoval,
            ),
            ("after-directory-sync", PublicationFault::AfterDirectorySync),
        ] {
            let after = directory.path().join(format!("{name}.lkja"));
            assert_eq!(
                publish_with_fault(&after, prepared.bytes(), fault)
                    .expect_err("failure after link")
                    .code,
                ErrorCode::ArtifactPublicationOutcomeUnknown
            );
            assert_eq!(
                read_file(&after).expect("published exact bytes"),
                prepared.bytes()
            );
        }

        assert!(
            fs::read_dir(directory.path())
                .expect("published directory")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(TEMPORARY_PREFIX))
        );

        for invalid in [
            PathBuf::from("relative.lkja"),
            PathBuf::from(format!("{}/./dot.lkja", directory.path().display())),
            PathBuf::from(format!("{}/../parent.lkja", directory.path().display())),
            PathBuf::from(format!("{}//double.lkja", directory.path().display())),
        ] {
            assert!(prepared.publish(&invalid).is_err());
        }

        let source = directory.path().join("source.lkja");
        prepared.publish(&source).expect("symlink source");
        let linked_input = directory.path().join("linked-input.lkja");
        std::os::unix::fs::symlink(&source, &linked_input).expect("input symlink");
        assert_eq!(
            read_file(&linked_input).expect_err("symlink input").code,
            ErrorCode::Io
        );

        let real_parent = directory.path().join("real-parent");
        fs::create_dir(&real_parent).expect("real parent");
        let linked_parent = directory.path().join("linked-parent");
        std::os::unix::fs::symlink(&real_parent, &linked_parent).expect("parent symlink");
        assert_eq!(
            prepared
                .publish(&linked_parent.join("application.lkja"))
                .expect_err("symlink parent")
                .code,
            ErrorCode::Io
        );
    }
}

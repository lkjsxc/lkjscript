//! Workspace-independent immutable reusable semantic releases.

mod canonical;
mod codec;
pub(crate) mod graph;

use crate::artifact_io;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision, WorkspaceId};
use crate::interpret::{RunPolicy, RuntimeValue};
use crate::schema::Node;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

pub const RELEASE_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_RELEASE_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_RELEASE_ITEMS: usize = 100_000;
pub const MAXIMUM_RELEASE_EXPORTS: usize = 256;
pub const MAXIMUM_RELEASE_DEPENDENCIES: usize = 256;
pub const MAXIMUM_RELEASE_IMPORTS: usize = 4_096;
pub const MAXIMUM_RELEASE_TESTS: usize = 256;
pub const MAXIMUM_RELEASE_SUITE_FUEL: u64 = 100_000_000;
pub const MAXIMUM_RELEASE_NAME_BYTES: usize = 256;
pub const MAXIMUM_RELEASE_COORDINATE_BYTES: usize = 128;
pub const MAXIMUM_RELEASE_VERSION_BYTES: usize = 64;
pub const MAXIMUM_RELEASE_SLOT_BYTES: usize = 64;
pub const MAXIMUM_RELEASE_GRAPH_NODES: usize = 256;
pub const MAXIMUM_RELEASE_GRAPH_EDGES: usize = 4_096;
pub const MAXIMUM_RELEASE_GRAPH_DEPTH: usize = 64;
pub const MAXIMUM_RELEASE_GRAPH_BYTES: usize = 256 * 1024 * 1024;
const RELEASE_TEMPORARY_PREFIX: &str = ".lkjscript-release-";
const DIGEST_BYTES: usize = 32;

/// Exact immutable reusable-release identity.
///
/// This is a domain-separated digest of the complete canonical release payload. It is not a
/// coordinate, user version, publisher identity, signature, authorization, or provenance claim.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReleaseId([u8; DIGEST_BYTES]);

impl ReleaseId {
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl fmt::Display for ReleaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
    }
}

impl FromStr for ReleaseId {
    type Err = &'static str;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        decode_digest(value).map(Self)
    }
}

impl Serialize for ReleaseId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ReleaseId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_digest(deserializer, "release ID").map(Self)
    }
}

/// Independent integrity digest over the same canonical payload as `ReleaseId`, under a distinct
/// domain. Equality has no provenance, authorization, or nominal meaning by itself.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReleaseContentDigest([u8; DIGEST_BYTES]);

impl ReleaseContentDigest {
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl fmt::Display for ReleaseContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
    }
}

impl Serialize for ReleaseContentDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Canonical durable item identity inside one exact release.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ReleaseItemId(u64);

impl ReleaseItemId {
    pub fn new(value: u64) -> Result<Self> {
        if value == 0 || value >= (1_u64 << 63) {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "release-local item ID must be a nonzero durable u63 integer",
            ));
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn from_local_node(id: NodeId) -> Result<Self> {
        if id.workspace() != canonical::RELEASE_LOCAL_WORKSPACE || !id.is_durable() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "release-local item has the wrong identity domain",
            ));
        }
        Self::new(id.serial()).map_err(|mut error| {
            error.code = ErrorCode::ArtifactCorrupt;
            error
        })
    }

    pub(crate) fn to_local_node(self) -> Result<NodeId> {
        NodeId::new(canonical::RELEASE_LOCAL_WORKSPACE, self.0).map_err(|error| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!("release-local item ID is invalid: {error}"),
            )
        })
    }
}

impl<'de> Deserialize<'de> for ReleaseItemId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseExportKind {
    Function,
    ProductType,
    SumType,
}

impl ReleaseExportKind {
    pub(crate) const fn stable_tag(self) -> u8 {
        match self {
            Self::Function => 1,
            Self::ProductType => 2,
            Self::SumType => 3,
        }
    }

    pub(crate) const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Function),
            2 => Some(Self::ProductType),
            3 => Some(Self::SumType),
            _ => None,
        }
    }

    pub(crate) fn for_node(node: &Node) -> Option<Self> {
        match node {
            Node::Function { .. } => Some(Self::Function),
            Node::ProductType { .. } => Some(Self::ProductType),
            Node::SumType { .. } => Some(Self::SumType),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseExportRequest {
    pub name: String,
    pub target: NodeId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseDependencyRequest {
    pub slot: String,
    pub release: ReleaseId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseImportRequest {
    pub local: NodeId,
    pub dependency_slot: String,
    pub export: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseTrapCode {
    RuntimeTrap,
    ByteIndexOutOfBounds,
    ByteSliceOutOfBounds,
}

impl ReleaseTrapCode {
    pub(crate) const fn stable_tag(self) -> u8 {
        match self {
            Self::RuntimeTrap => 1,
            Self::ByteIndexOutOfBounds => 2,
            Self::ByteSliceOutOfBounds => 3,
        }
    }

    pub(crate) const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::RuntimeTrap),
            2 => Some(Self::ByteIndexOutOfBounds),
            3 => Some(Self::ByteSliceOutOfBounds),
            _ => None,
        }
    }

    pub(crate) const fn from_error(code: ErrorCode) -> Option<Self> {
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
pub struct ReleaseTrap {
    pub code: ReleaseTrapCode,
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
pub enum ReleaseTestExpectation {
    Value(RuntimeValue),
    Trap(ReleaseTrap),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTestCase {
    pub name: String,
    pub target: NodeId,
    pub arguments: Vec<RuntimeValue>,
    pub expected: ReleaseTestExpectation,
    pub policy: RunPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseBuildRequest {
    pub version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub root: NodeId,
    pub coordinate: String,
    pub user_version: String,
    pub exports: Vec<ReleaseExportRequest>,
    #[serde(default)]
    pub dependencies: Vec<ReleaseDependencyRequest>,
    #[serde(default)]
    pub imports: Vec<ReleaseImportRequest>,
    pub tests: Vec<ReleaseTestCase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDependency {
    pub slot: String,
    pub release: ReleaseId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseImport {
    pub local: ReleaseItemId,
    pub dependency_slot: String,
    pub target: ReleaseItemId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseExport {
    pub name: String,
    pub kind: ReleaseExportKind,
    pub target: ReleaseItemId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalReleaseTest {
    pub name: String,
    pub target: ReleaseItemId,
    pub arguments: Vec<RuntimeValue>,
    pub expected: ReleaseTestExpectation,
    pub policy: RunPolicy,
}

#[derive(Clone, Debug)]
pub(crate) struct DecodedRelease {
    pub bytes: Vec<u8>,
    pub id: ReleaseId,
    pub content_digest: ReleaseContentDigest,
    pub coordinate: String,
    pub user_version: String,
    pub unit_root: ReleaseItemId,
    pub dependencies: Vec<ReleaseDependency>,
    pub imports: Vec<ReleaseImport>,
    pub exports: Vec<ReleaseExport>,
    pub tests: Vec<CanonicalReleaseTest>,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ReleaseTypeRef {
    Unit,
    Bool,
    I64,
    Bytes,
    Local(ReleaseItemId),
    Dependency { slot: String, target: ReleaseItemId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseFieldInspection {
    pub target: ReleaseItemId,
    pub name: String,
    pub ty: ReleaseTypeRef,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseVariantInspection {
    pub target: ReleaseItemId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<ReleaseTypeRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ReleaseSignatureInspection {
    Function {
        parameters: Vec<ReleaseTypeRef>,
        result: ReleaseTypeRef,
    },
    ProductType {
        fields: Vec<ReleaseFieldInspection>,
    },
    SumType {
        variants: Vec<ReleaseVariantInspection>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseExportInspection {
    pub name: String,
    pub target: ReleaseItemId,
    pub signature: ReleaseSignatureInspection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseDependencyInspection {
    pub slot: String,
    pub release: ReleaseId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTestInspection {
    pub name: String,
    pub target: ReleaseItemId,
    pub policy: RunPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseLimits {
    pub maximum_artifact_bytes: u64,
    pub maximum_items: u64,
    pub maximum_exports: u64,
    pub maximum_dependencies: u64,
    pub maximum_imports: u64,
    pub maximum_tests: u64,
    pub maximum_graph_nodes: u64,
    pub maximum_graph_edges: u64,
    pub maximum_graph_depth: u64,
    pub maximum_graph_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseInspection {
    pub contract_version: u16,
    pub format_version: u16,
    pub semantic_schema: &'static str,
    pub release: ReleaseId,
    pub content_digest: ReleaseContentDigest,
    pub coordinate: String,
    pub user_version: String,
    pub unit_root: ReleaseItemId,
    pub exports: Vec<ReleaseExportInspection>,
    pub dependencies: Vec<ReleaseDependencyInspection>,
    pub tests: Vec<ReleaseTestInspection>,
    pub semantic_items: u64,
    pub private_durable_items: u64,
    pub artifact_bytes: u64,
    pub provenance: &'static str,
    pub signatures: &'static str,
    pub limits: ReleaseLimits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseTestStatus {
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
pub struct ReleaseTestResult {
    pub name: String,
    pub status: ReleaseTestStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_trap: Option<ReleaseTrapCode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTestReport {
    pub total: u64,
    pub passed: u64,
    pub results: Vec<ReleaseTestResult>,
}

impl ReleaseTestReport {
    pub fn all_passed(&self) -> bool {
        self.total == self.passed
            && self
                .results
                .iter()
                .all(|result| result.status == ReleaseTestStatus::Passed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseBuildReceipt {
    pub contract_version: u16,
    pub inspection: ReleaseInspection,
    pub tests: ReleaseTestReport,
    pub published: bool,
}

#[derive(Clone, Debug)]
pub struct PreparedRelease {
    decoded: DecodedRelease,
    inspection: ReleaseInspection,
    tests: ReleaseTestReport,
}

impl PreparedRelease {
    pub fn bytes(&self) -> &[u8] {
        &self.decoded.bytes
    }

    pub const fn release_id(&self) -> ReleaseId {
        self.decoded.id
    }

    pub fn receipt(&self, published: bool) -> ReleaseBuildReceipt {
        ReleaseBuildReceipt {
            contract_version: RELEASE_CONTRACT_VERSION,
            inspection: self.inspection.clone(),
            tests: self.tests.clone(),
            published,
        }
    }

    pub fn publish(&self, path: &Path) -> Result<()> {
        artifact_io::publish(
            path,
            &self.decoded.bytes,
            "reusable release artifact",
            RELEASE_TEMPORARY_PREFIX,
            MAXIMUM_RELEASE_ARTIFACT_BYTES,
        )
    }
}

pub(crate) fn prepare(
    source: &Snapshot,
    request: &ReleaseBuildRequest,
    supplied_dependency_bytes: &[Vec<u8>],
) -> Result<PreparedRelease> {
    validate_contract_version(request.version)?;
    if request.workspace != source.workspace() {
        return Err(LkError::new(
            ErrorCode::WrongWorkspace,
            "release build workspace does not match the selected authority",
        )
        .for_workspace(request.workspace)
        .at_revision(request.revision));
    }
    if request.revision != source.revision() {
        return Err(LkError::new(
            ErrorCode::RevisionConflict,
            "release build revision does not match the selected immutable snapshot",
        )
        .for_workspace(request.workspace)
        .at_revision(request.revision));
    }
    let dependencies = decode_supplied(supplied_dependency_bytes)?;
    let projected = canonical::project(source, request, &dependencies)?;
    let bytes = codec::encode(&projected)?;
    let decoded = codec::decode(&bytes)?;
    let graph = graph::ReleaseGraph::new(decoded.clone(), dependencies)?;
    let mut root_tests = None;
    for release in graph.releases() {
        let report = graph.run_release_tests(release.id)?;
        if !report.all_passed() {
            return Err(LkError::new(
                ErrorCode::ApplicationTestFailed,
                format!(
                    "one or more tests did not pass for exact release {}",
                    release.id
                ),
            ));
        }
        if release.id == decoded.id {
            root_tests = Some(report);
        }
    }
    let tests = root_tests.ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "prepared reusable release disappeared from its exact graph",
        )
    })?;
    let inspection = inspection(&decoded)?;
    Ok(PreparedRelease {
        decoded,
        inspection,
        tests,
    })
}

pub fn validate(bytes: &[u8]) -> Result<ReleaseInspection> {
    inspection(&codec::decode(bytes)?)
}

pub fn inspect(bytes: &[u8]) -> Result<ReleaseInspection> {
    validate(bytes)
}

pub fn test(bytes: &[u8], supplied_dependency_bytes: &[Vec<u8>]) -> Result<ReleaseTestReport> {
    let root = codec::decode(bytes)?;
    let root_id = root.id;
    let graph = graph::ReleaseGraph::new(root, decode_supplied(supplied_dependency_bytes)?)?;
    graph.run_release_tests(root_id)
}

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    artifact_io::read_file(
        path,
        "reusable release artifact",
        MAXIMUM_RELEASE_ARTIFACT_BYTES,
    )
}

pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedRelease> {
    codec::decode(bytes)
}

fn decode_supplied(bytes: &[Vec<u8>]) -> Result<Vec<DecodedRelease>> {
    bytes.iter().map(|bytes| codec::decode(bytes)).collect()
}

fn inspection(release: &DecodedRelease) -> Result<ReleaseInspection> {
    let exports = release
        .exports
        .iter()
        .map(|export| export_inspection(release, export))
        .collect::<Result<Vec<_>>>()?;
    let exported = release
        .exports
        .iter()
        .map(|export| export.target)
        .collect::<BTreeSet<_>>();
    let durable = release
        .snapshot
        .nodes()
        .filter(|(id, _)| id.is_durable())
        .count();
    Ok(ReleaseInspection {
        contract_version: RELEASE_CONTRACT_VERSION,
        format_version: codec::RELEASE_FORMAT_VERSION,
        semantic_schema: "lkjscript-tsm006",
        release: release.id,
        content_digest: release.content_digest,
        coordinate: release.coordinate.clone(),
        user_version: release.user_version.clone(),
        unit_root: release.unit_root,
        exports,
        dependencies: release
            .dependencies
            .iter()
            .map(|dependency| ReleaseDependencyInspection {
                slot: dependency.slot.clone(),
                release: dependency.release,
            })
            .collect(),
        tests: release
            .tests
            .iter()
            .map(|test| ReleaseTestInspection {
                name: test.name.clone(),
                target: test.target,
                policy: test.policy,
            })
            .collect(),
        semantic_items: u64::try_from(release.snapshot.node_count()).unwrap_or(u64::MAX),
        private_durable_items: u64::try_from(durable.saturating_sub(exported.len()))
            .unwrap_or(u64::MAX),
        artifact_bytes: u64::try_from(release.bytes.len()).unwrap_or(u64::MAX),
        provenance: "absent",
        signatures: "absent",
        limits: release_limits(),
    })
}

fn export_inspection(
    release: &DecodedRelease,
    export: &ReleaseExport,
) -> Result<ReleaseExportInspection> {
    let target = export.target.to_local_node()?;
    let signature = match release.snapshot.node(target)? {
        Node::Function {
            parameters, result, ..
        } => ReleaseSignatureInspection::Function {
            parameters: parameters
                .iter()
                .map(|parameter| match release.snapshot.node(*parameter)? {
                    Node::Parameter { ty, .. } => release_type(release, *ty),
                    _ => Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "release function parameter is not a parameter",
                    )),
                })
                .collect::<Result<Vec<_>>>()?,
            result: release_type(release, *result)?,
        },
        Node::ProductType { fields, .. } => ReleaseSignatureInspection::ProductType {
            fields: fields
                .iter()
                .map(|field| match release.snapshot.node(*field)? {
                    Node::ProductField { name, ty, .. } => Ok(ReleaseFieldInspection {
                        target: ReleaseItemId::from_local_node(*field)?,
                        name: name.clone(),
                        ty: release_type(release, *ty)?,
                    }),
                    _ => Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "release product member is not a field",
                    )),
                })
                .collect::<Result<Vec<_>>>()?,
        },
        Node::SumType { variants, .. } => ReleaseSignatureInspection::SumType {
            variants: variants
                .iter()
                .map(|variant| match release.snapshot.node(*variant)? {
                    Node::SumVariant { name, payload, .. } => Ok(ReleaseVariantInspection {
                        target: ReleaseItemId::from_local_node(*variant)?,
                        name: name.clone(),
                        payload: payload.map(|ty| release_type(release, ty)).transpose()?,
                    }),
                    _ => Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "release sum member is not a variant",
                    )),
                })
                .collect::<Result<Vec<_>>>()?,
        },
        _ => {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "release export has an unsupported kind",
            ));
        }
    };
    Ok(ReleaseExportInspection {
        name: export.name.clone(),
        target: export.target,
        signature,
    })
}

fn release_type(
    release: &DecodedRelease,
    ty: crate::schema::SemanticType,
) -> Result<ReleaseTypeRef> {
    use crate::schema::SemanticType;
    Ok(match ty {
        SemanticType::Unit => ReleaseTypeRef::Unit,
        SemanticType::Bool => ReleaseTypeRef::Bool,
        SemanticType::I64 => ReleaseTypeRef::I64,
        SemanticType::Bytes => ReleaseTypeRef::Bytes,
        SemanticType::Nominal(target) => {
            let local = ReleaseItemId::from_local_node(target)?;
            if let Some(import) = release.imports.iter().find(|import| import.local == local) {
                ReleaseTypeRef::Dependency {
                    slot: import.dependency_slot.clone(),
                    target: import.target,
                }
            } else {
                ReleaseTypeRef::Local(local)
            }
        }
    })
}

pub(crate) fn validate_contract_version(version: u16) -> Result<()> {
    if version == RELEASE_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "reusable release command contract version is unsupported",
        ))
    }
}

pub(crate) fn validate_symbol(value: &str, maximum: usize, label: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= maximum
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().skip(1).all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        });
    if valid {
        Ok(())
    } else {
        Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} must be bounded lowercase ASCII with digits, '_' or '-'"),
        ))
    }
}

pub(crate) fn validate_coordinate(value: &str) -> Result<()> {
    if value.len() > MAXIMUM_RELEASE_COORDINATE_BYTES
        || value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.split('/').any(|part| {
            part.is_empty()
                || validate_symbol(
                    part,
                    MAXIMUM_RELEASE_COORDINATE_BYTES,
                    "coordinate component",
                )
                .is_err()
        })
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "release coordinate must be a bounded slash-separated lowercase ASCII name",
        ));
    }
    Ok(())
}

pub(crate) fn validate_user_version(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAXIMUM_RELEASE_VERSION_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'/' && byte != b'\\')
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "release user version must be bounded printable ASCII without path separators",
        ));
    }
    Ok(())
}

pub(crate) fn validate_release_test_name(value: &str) -> Result<()> {
    validate_symbol(value, MAXIMUM_RELEASE_NAME_BYTES, "release test name")
}

pub(crate) fn primitive_runtime_value(value: &RuntimeValue) -> bool {
    matches!(
        value,
        RuntimeValue::Unit | RuntimeValue::Bool(_) | RuntimeValue::I64(_) | RuntimeValue::Bytes(_)
    )
}

fn release_limits() -> ReleaseLimits {
    ReleaseLimits {
        maximum_artifact_bytes: u64::try_from(MAXIMUM_RELEASE_ARTIFACT_BYTES).unwrap_or(u64::MAX),
        maximum_items: u64::try_from(MAXIMUM_RELEASE_ITEMS).unwrap_or(u64::MAX),
        maximum_exports: u64::try_from(MAXIMUM_RELEASE_EXPORTS).unwrap_or(u64::MAX),
        maximum_dependencies: u64::try_from(MAXIMUM_RELEASE_DEPENDENCIES).unwrap_or(u64::MAX),
        maximum_imports: u64::try_from(MAXIMUM_RELEASE_IMPORTS).unwrap_or(u64::MAX),
        maximum_tests: u64::try_from(MAXIMUM_RELEASE_TESTS).unwrap_or(u64::MAX),
        maximum_graph_nodes: u64::try_from(MAXIMUM_RELEASE_GRAPH_NODES).unwrap_or(u64::MAX),
        maximum_graph_edges: u64::try_from(MAXIMUM_RELEASE_GRAPH_EDGES).unwrap_or(u64::MAX),
        maximum_graph_depth: u64::try_from(MAXIMUM_RELEASE_GRAPH_DEPTH).unwrap_or(u64::MAX),
        maximum_graph_bytes: u64::try_from(MAXIMUM_RELEASE_GRAPH_BYTES).unwrap_or(u64::MAX),
    }
}

#[cfg(test)]
fn publish_with_fault(
    path: &Path,
    bytes: &[u8],
    fault: artifact_io::PublicationFault,
) -> Result<()> {
    artifact_io::publish_with_fault(
        path,
        bytes,
        "reusable release artifact",
        RELEASE_TEMPORARY_PREFIX,
        MAXIMUM_RELEASE_ARTIFACT_BYTES,
        fault,
    )
}

fn deserialize_digest<'de, D>(
    deserializer: D,
    expected: &'static str,
) -> std::result::Result<[u8; DIGEST_BYTES], D::Error>
where
    D: Deserializer<'de>,
{
    struct DigestVisitor(&'static str);
    impl Visitor<'_> for DigestVisitor {
        type Value = [u8; DIGEST_BYTES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a canonical lowercase hexadecimal {}", self.0)
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            decode_digest(value).map_err(E::custom)
        }
    }
    deserializer.deserialize_str(DigestVisitor(expected))
}

fn decode_digest(value: &str) -> std::result::Result<[u8; DIGEST_BYTES], &'static str> {
    if value.len() != DIGEST_BYTES * 2 {
        return Err("digest has the wrong length");
    }
    let mut output = [0_u8; DIGEST_BYTES];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (decode_nibble(pair[0])? << 4) | decode_nibble(pair[1])?;
    }
    Ok(output)
}

fn decode_nibble(value: u8) -> std::result::Result<u8, &'static str> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("digest is not canonical lowercase hexadecimal"),
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
pub(crate) mod tests;

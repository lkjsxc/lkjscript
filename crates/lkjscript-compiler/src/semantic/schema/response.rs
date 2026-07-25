use serde::{Deserialize, Serialize};

use super::{
    ChangedSource, Charges, DeclarationRecord, DiagnosticRecord, EntityRecord, IdentityRelation,
    NodeQueryRecord, NodeRecord, ResourceProfile, SourceUnitRecord,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProtocolErrorCode {
    InvalidJson,
    InvalidSchema,
    UnsupportedVersion,
    ResourceLimit,
    SourceLoad,
    StaleRevision,
    PreconditionFailed,
    UnknownDeclaration,
    UnknownNode,
    InvalidOperation,
    ValidationFailed,
    PublicationFailed,
    OutputLimit,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProtocolError {
    pub code: ProtocolErrorCode,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotResult {
    pub repository_identity: String,
    pub source_units: Vec<SourceUnitRecord>,
    pub declarations: Vec<DeclarationRecord>,
    pub nodes: Vec<NodeRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiagnosticsResult {
    pub complete: bool,
    pub diagnostics: Vec<DiagnosticRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransactionResult {
    pub mode: String,
    pub base_revision: String,
    pub new_revision: String,
    pub changed_sources: Vec<ChangedSource>,
    pub semantic_diff: Vec<IdentityRelation>,
    pub identities: Vec<IdentityRelation>,
    pub diagnostics: Vec<DiagnosticRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ResponseResult {
    Snapshot {
        snapshot: Box<SnapshotResult>,
    },
    ReadEntity {
        entity: Box<EntityRecord>,
    },
    QueryNode {
        query: Box<NodeQueryRecord>,
    },
    Diagnostics {
        result: Box<DiagnosticsResult>,
    },
    ApplyTransaction {
        transaction: Box<TransactionResult>,
    },
    Error {
        error: Box<ProtocolError>,
        diagnostic: Option<Box<DiagnosticRecord>>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Response {
    pub schema: String,
    pub version: u32,
    pub compiler_build: String,
    pub profile: ResourceProfile,
    pub revision: Option<String>,
    pub charges: Charges,
    pub result: ResponseResult,
}

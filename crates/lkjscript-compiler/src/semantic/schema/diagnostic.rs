use serde::{Deserialize, Serialize};

use super::SpanRecord;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub(crate) enum DiagnosticCode {
    #[serde(rename = "LKJ-SRC-UNMATCHED-MARKER")]
    UnmatchedMarker,
    #[serde(rename = "LKJ-DECL-DUPLICATE")]
    DuplicateDeclaration,
    #[serde(rename = "LKJ-NAME-UNKNOWN")]
    UnknownName,
    #[serde(rename = "LKJ-CALL-ARITY")]
    CallArity,
    #[serde(rename = "LKJ-TYPE-MISMATCH")]
    TypeMismatch,
    #[serde(rename = "LKJ-EDIT-STALE")]
    StaleEdit,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Severity {
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DiagnosticCategory {
    SourceSyntax,
    Declaration,
    NameResolution,
    Call,
    Type,
    Edit,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Certainty {
    Guaranteed,
    Conditional,
    Informational,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelatedRecord {
    pub label: String,
    pub node: Option<u32>,
    pub source: String,
    pub span: SpanRecord,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum RepairRecord {
    RefreshSnapshot,
    ReplaceName { node: u32, name: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiagnosticRecord {
    pub schema: String,
    pub contract: String,
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub category: DiagnosticCategory,
    pub certainty: Certainty,
    pub primary_node: Option<u32>,
    pub primary_source: String,
    pub primary_span: SpanRecord,
    pub related: Vec<RelatedRecord>,
    pub declaration: Option<String>,
    pub binding: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub effect_mismatch: Option<String>,
    pub ownership_path: Option<String>,
    pub agent_rendering: String,
    pub human_rendering: String,
    pub repairs: Vec<RepairRecord>,
}

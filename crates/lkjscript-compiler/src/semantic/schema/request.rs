use serde::{Deserialize, Serialize};

use super::Expression;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ResourceProfile {
    Sandbox,
    Default,
    Build,
    TrustedLocal,
    Deterministic,
}

impl ResourceProfile {
    pub(crate) const fn core(self) -> lkjscript_core::ResourceProfile {
        use lkjscript_core::ResourceProfileName;
        let name = match self {
            Self::Sandbox => ResourceProfileName::Sandbox,
            Self::Default => ResourceProfileName::Default,
            Self::Build => ResourceProfileName::Build,
            Self::TrustedLocal => ResourceProfileName::TrustedLocal,
            Self::Deterministic => ResourceProfileName::Deterministic,
        };
        lkjscript_core::ResourceProfile::new(name)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub schema: String,
    pub version: u32,
    pub profile: ResourceProfile,
    pub root: String,
    pub operation: OperationRequest,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum OperationRequest {
    Snapshot {
        expected_repository_identity: Option<String>,
    },
    ReadEntity {
        revision: String,
        declaration_key: String,
        entity_fingerprint: Option<String>,
    },
    QueryNode {
        revision: String,
        node: u32,
    },
    Diagnostics {
        revision: String,
        analysis: AnalysisLevel,
    },
    ApplyTransaction {
        mode: ApplyMode,
        base_revision: String,
        file_preconditions: Vec<FilePrecondition>,
        operations: Vec<TransactionOperation>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnalysisLevel {
    Source,
    Hir,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyMode {
    Preview,
    Publish,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FilePrecondition {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TransactionOperation {
    RenameDeclaration {
        declaration_key: String,
        entity_fingerprint: String,
        new_name: String,
    },
    ReplaceExpression {
        declaration_key: String,
        entity_fingerprint: String,
        node: u32,
        node_fingerprint: String,
        expression: Expression,
    },
}

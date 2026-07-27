use serde::{Deserialize, Serialize};

use super::{BlockerCode, BoundedReason, ExpectedTypeFact, ExplorationRecord, HoleCandidate};
use super::{HoleConstraints, ScopeEntity, TypedHoleIdentity};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HoleContextResult {
    pub hole: TypedHoleIdentity,
    pub goal: Option<String>,
    pub containing_return_type: String,
    pub expected_type: ExpectedTypeFact,
    pub scope_entities: Vec<ScopeEntity>,
    pub constraints: HoleConstraints,
    pub candidates: Vec<HoleCandidate>,
    pub exploration: ExplorationRecord,
    pub blockers: Vec<ActionBlocker>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LegalActionsResult {
    pub hole: TypedHoleIdentity,
    pub legal_child_kinds: Vec<LegalChildKind>,
    pub constructors: Vec<ConstructorAction>,
    pub required_fields: Vec<RequiredField>,
    pub expected_type: ExpectedTypeFact,
    pub applicable_bindings: Vec<String>,
    pub transaction_kinds: Vec<HoleTransactionKind>,
    pub coverage: ExplorationRecord,
    pub blockers: Vec<ActionBlocker>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LegalChildKind {
    Literal,
    NameReference,
    ProductValue,
    OptionConstructor,
    ResultConstructor,
    BuiltinCall,
    UserCall,
    OwnershipOperation,
    Loop,
    Return,
    Break,
    Continue,
    Trap,
    Exit,
    TypedHole,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConstructorAction {
    pub kind: LegalChildKind,
    pub name: String,
    pub result_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequiredField {
    pub name: String,
    pub expected_type: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(
    clippy::enum_variant_names,
    reason = "protocol uses the exact closed operation names"
)]
pub(crate) enum HoleTransactionKind {
    InsertHole,
    FillHole,
    RefineHole,
    DeleteHole,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActionBlocker {
    pub code: BlockerCode,
    pub subject: String,
    pub reason: String,
}

pub(crate) fn unsupported_exploration(reason: BoundedReason) -> ExplorationRecord {
    ExplorationRecord {
        supported: false,
        truncated: false,
        charged_category: "hole_candidates".into(),
        charged_count: 0,
        search_work: 0,
        omitted: Vec::new(),
        reason: Some(reason),
    }
}

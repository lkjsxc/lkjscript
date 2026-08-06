use serde::{Deserialize, Serialize};

use super::super::{Expression, SemanticEffect};
use super::OwnershipAccess;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CandidateCategory {
    ExactLiteral,
    VisibleBinding,
    ProductConstructor,
    OptionConstructor,
    ResultConstructor,
    DirectFunction,
    DirectBuiltin,
    ExactConversion,
    MatchSkeleton,
    ControlForm,
    NeverForm,
}

impl CandidateCategory {
    pub(crate) const fn rank(self) -> u16 {
        match self {
            Self::ExactLiteral => 0,
            Self::VisibleBinding => 1,
            Self::ProductConstructor => 2,
            Self::OptionConstructor => 3,
            Self::ResultConstructor => 4,
            Self::DirectFunction => 5,
            Self::DirectBuiltin => 6,
            Self::ExactConversion => 7,
            Self::MatchSkeleton => 8,
            Self::ControlForm => 9,
            Self::NeverForm => 10,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HoleCandidate {
    pub identity: String,
    pub category: CandidateCategory,
    pub rank: CandidateRank,
    pub result_type: String,
    pub effects: Vec<SemanticEffect>,
    pub ownership: OwnershipAccess,
    pub capabilities: Vec<String>,
    pub construction_cost: u64,
    pub expression: Expression,
    pub snippets: Vec<ConcreteSnippet>,
    pub edits: Vec<ExactSemanticEdit>,
    pub inclusion_reason: InclusionReason,
    pub validating_checker: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CandidateRank {
    pub category: u16,
    pub effect_cost: u16,
    pub ownership_cost: u16,
    pub construction_cost: u64,
    pub canonical_source: String,
    pub identity: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConcreteSnippet {
    pub source: String,
    pub complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ExactSemanticEdit {
    ReplaceHole {
        declaration_key: String,
        node: u64,
        node_fingerprint: String,
        expression: Expression,
    },
    InsertImport {
        source: String,
        path: String,
        before_declaration: String,
    },
    QualifyReference {
        node: u64,
        qualification: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum InclusionReason {
    ExactTypeAndConstraints,
    ExactTypeRequiresMove,
    ExactConstructor,
    CheckerValidatedCall,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExplorationRecord {
    pub supported: bool,
    pub truncated: bool,
    pub charged_category: String,
    pub charged_count: u64,
    pub search_work: u64,
    pub omitted: Vec<OmittedCategory>,
    pub reason: Option<BoundedReason>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OmittedCategory {
    pub category: CandidateCategory,
    pub known_count: Option<u64>,
    pub blocker: BlockerCode,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BlockerCode {
    ExpectedTypeUnavailable,
    CandidateBudgetExhausted,
    SearchBudgetExhausted,
    CapabilityUnavailable,
    QualificationUnsupported,
    OwnershipRejected,
    EffectRejected,
    CheckerRejected,
    StructurallyIllegal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundedReason {
    pub code: BlockerCode,
    pub message: String,
}

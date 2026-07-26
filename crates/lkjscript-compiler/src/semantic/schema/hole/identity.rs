use serde::{Deserialize, Serialize};

use super::super::{SemanticEffect, SpanRecord};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TypedHoleIdentity {
    pub schema: String,
    pub version: u32,
    pub source_revision: String,
    pub identity: String,
    pub declaration_key: String,
    pub local_identity: String,
    pub node: u32,
    pub node_fingerprint: String,
    pub source: String,
    pub span: SpanRecord,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "availability", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ExpectedTypeFact {
    Available {
        canonical: String,
        instantiated: bool,
    },
    Unavailable {
        reason: TypeUnavailableReason,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TypeUnavailableReason {
    UnconstrainedLetInitializer,
    UnsupportedBuiltinInstantiation,
    UnsupportedStructuralPosition,
    AmbiguousInstantiation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScopeEntity {
    pub schema: String,
    pub version: u32,
    pub source_revision: String,
    pub identity: String,
    pub kind: ScopeEntityKind,
    pub name: String,
    pub instantiated_type: String,
    pub ownership: OwnershipAccess,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScopeEntityKind {
    Parameter,
    ImmutableLocal,
    MutableLocal,
    Function,
    Product,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OwnershipAccess {
    Copy,
    Move,
    SharedBorrow,
    MutableBorrow,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HoleConstraints {
    pub generic_variables: Vec<String>,
    pub trait_obligations: Vec<TraitObligation>,
    pub allowed_effects: Vec<SemanticEffect>,
    pub already_required_effects: Vec<SemanticEffect>,
    pub capabilities: ConstraintAvailability,
    pub ownership: OwnershipConstraint,
    pub control: ControlConstraint,
    pub never_admissible: bool,
    pub material_incomplete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TraitObligation {
    pub variable: String,
    pub trait_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnershipConstraint {
    pub expected_access: OwnershipAccess,
    pub checker_validated: bool,
    pub place_and_loan_facts: ConstraintAvailability,
    pub region: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "availability", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ConstraintAvailability {
    Available { values: Vec<String> },
    Unavailable { reason: ConstraintUnavailableReason },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConstraintUnavailableReason {
    NoCapabilityModel,
    NoExactSourceCorrelation,
    ExpectedTypeUnavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ControlConstraint {
    pub target: String,
    pub required_result: Option<String>,
    pub loop_depth: u32,
}

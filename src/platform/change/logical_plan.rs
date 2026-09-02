//! Authored-only logical review evidence retained beside one prepared publication.

use super::{ChangeBudget, ImpactReason, RelationDelta};
use crate::platform::kernel::{
    DependencyRecord, FunctionEffect, IdentityKind, LocalValueReference, Name, OwnerKey,
    ParameterUse, RequirementReference, RetirementRecord, TypeObjectDigest,
};
use crate::platform::semantic_id::{DeclarationId, ExpressionId, ParameterId};
use std::collections::{BTreeMap, BTreeSet};

/// One request-local identity without its presentation-only source label.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthoredAllocation {
    pub domain: IdentityKind,
    pub ordinal: u64,
    pub owner: OwnerKey,
}

/// Logical dependency values that supplement the digest-only semantic-diff entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalDependencyValues {
    pub before: Option<DependencyRecord>,
    pub after: Option<DependencyRecord>,
}

/// Logical retirement values that supplement the digest-only semantic-diff entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalRetirementValues {
    pub before: Option<RetirementRecord>,
    pub after: Option<RetirementRecord>,
}

/// One inferred free-local parameter in an identity-preserving function extraction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionExtractionCapture {
    pub source: LocalValueReference,
    pub parameter: ParameterId,
    pub name: Name,
    pub ty: TypeObjectDigest,
    pub use_mode: ParameterUse,
    pub resource_requirement: Option<RequirementReference>,
    pub rewritten_uses: Vec<ExpressionId>,
}

/// Derived review evidence for the sole graph-native function extraction in a request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionExtractionEvidence {
    /// The exact public definition projection digest is attached from the same pinned repository
    /// view at the public control boundary.
    pub base_definition: Option<[u8; 32]>,
    pub function: DeclarationId,
    pub selected_root: ExpressionId,
    pub moved_digest: [u8; 32],
    pub moved_owners: Vec<OwnerKey>,
    pub captures: Vec<FunctionExtractionCapture>,
    pub helper: DeclarationId,
    pub helper_name: Name,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
    pub preserved_owners: Vec<OwnerKey>,
    pub changed_owners: Vec<OwnerKey>,
    pub generated_owners: Vec<OwnerKey>,
    pub caller_body_records: u64,
    pub helper_body_records: u64,
}

/// Exact logical facts selected from the same analysis that produced a publication candidate.
///
/// Owner/type/dependency/retirement digest edits remain owned by the prepared semantic diff. This
/// value retains only facts that the generic publication did not already keep, plus the logical
/// values needed to interpret dependency and retirement digest edits.
#[derive(Clone, Debug)]
pub struct LogicalChangePlanEvidence {
    pub budget: ChangeBudget,
    pub allocations: Vec<AuthoredAllocation>,
    pub dependencies: BTreeMap<crate::platform::kernel::PackageId, LogicalDependencyValues>,
    pub retirements: BTreeMap<OwnerKey, LogicalRetirementValues>,
    pub relations: RelationDelta,
    pub structurally_checked: BTreeSet<OwnerKey>,
    pub semantically_checked: BTreeSet<OwnerKey>,
    pub tests: BTreeSet<OwnerKey>,
    pub reasons: Vec<ImpactReason>,
    pub extraction: Option<FunctionExtractionEvidence>,
}

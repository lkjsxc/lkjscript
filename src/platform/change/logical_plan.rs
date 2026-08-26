//! Authored-only logical review evidence retained beside one prepared publication.

use super::{ChangeBudget, ImpactReason, RelationDelta};
use crate::platform::kernel::{DependencyRecord, IdentityKind, OwnerKey, RetirementRecord};
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
}

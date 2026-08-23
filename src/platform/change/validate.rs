//! Owner-frontier structural and semantic validation over an isolated candidate overlay.

use super::{CanonicalDelta, DerivedDelta, ImpactPlan, KernelOverlay};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExpressionRead, OwnerKey, OwnerRecord, PackageId, TypeObject, TypeObjectDigest,
    validate_expression_roots,
};
use crate::platform::witness::{
    FullWitness, OwnershipEntry, OwnershipParent, aggregation_children,
};
use std::collections::{BTreeMap, BTreeSet};

pub const INCREMENTAL_VALIDATION_PROFILE: &str = "incremental_owner_frontier";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IncrementalValidationWork {
    pub owner_records_checked: u64,
    pub ownership_entries_checked: u64,
    pub type_objects_checked: u64,
    pub expression_work: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncrementalValidationReport {
    pub profile: &'static str,
    pub canonical_owners_changed: u64,
    pub structurally_checked: BTreeSet<OwnerKey>,
    pub semantically_checked: BTreeSet<OwnerKey>,
    pub summaries_reused: u64,
    pub tests_selected: u64,
    pub work: IncrementalValidationWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralValidationReport {
    pub structurally_checked: BTreeSet<OwnerKey>,
    pub work: IncrementalValidationWork,
}

pub fn validate_structural_frontier(
    overlay: &KernelOverlay<'_>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &FullWitness,
) -> Result<StructuralValidationReport, Vec<Diagnostic>> {
    let mut validator = IncrementalValidator {
        overlay,
        canonical,
        derived,
        base_witness,
        diagnostics: Vec::new(),
        work: IncrementalValidationWork::default(),
        ownership_edits: derived
            .ownership
            .iter()
            .map(|edit| (edit.key, edit.after))
            .collect(),
    };
    validator.validate_structural();
    if validator.diagnostics.is_empty() {
        Ok(StructuralValidationReport {
            structurally_checked: canonical.owners.keys().copied().collect(),
            work: validator.work,
        })
    } else {
        Err(validator.diagnostics)
    }
}

pub fn validate_incremental_frontier(
    overlay: &KernelOverlay<'_>,
    canonical: &CanonicalDelta,
    impact: &ImpactPlan,
    base_witness: &FullWitness,
    mut structural: StructuralValidationReport,
) -> Result<IncrementalValidationReport, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut work = 0_usize;
    validate_expression_roots(
        overlay,
        impact.semantically_checked.iter().copied(),
        &mut diagnostics,
        &mut work,
    );
    structural.work.expression_work = work as u64;
    if diagnostics.is_empty() {
        Ok(IncrementalValidationReport {
            profile: INCREMENTAL_VALIDATION_PROFILE,
            canonical_owners_changed: canonical.owners.len() as u64,
            structurally_checked: structural.structurally_checked,
            semantically_checked: impact.semantically_checked.clone(),
            summaries_reused: base_witness
                .summaries
                .keys()
                .filter(|owner| !impact.summary_owners.contains(owner))
                .count() as u64,
            tests_selected: impact.tests.len() as u64,
            work: structural.work,
        })
    } else {
        Err(diagnostics)
    }
}

struct IncrementalValidator<'a> {
    overlay: &'a KernelOverlay<'a>,
    canonical: &'a CanonicalDelta,
    derived: &'a DerivedDelta,
    base_witness: &'a FullWitness,
    diagnostics: Vec<Diagnostic>,
    work: IncrementalValidationWork,
    ownership_edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
}

impl IncrementalValidator<'_> {
    fn validate_structural(&mut self) {
        self.validate_changed_records();
        self.validate_ownership_frontier();
        self.validate_type_frontier();
    }

    fn validate_changed_records(&mut self) {
        for (owner, edit) in &self.canonical.owners {
            self.work.owner_records_checked = self.work.owner_records_checked.saturating_add(1);
            if let Some((_, record)) = &edit.after {
                self.capture(record.validate_local());
                if record.owner() != *owner {
                    self.error(
                        "change_validate_owner_key",
                        format!("candidate owner key {owner:?} disagrees with its record"),
                    );
                }
                self.capture(crate::platform::kernel::encode_owner(record).map(|_| ()));
            }
        }
        for (package, edit) in &self.canonical.dependencies {
            if let Some((_, dependency)) = &edit.after {
                self.capture(dependency.validate_local());
                if dependency.package != *package
                    || dependency.package == self.overlay.base().root.package_id
                {
                    self.error(
                        "change_validate_dependency_key",
                        "candidate dependency key is foreign or self-referential",
                    );
                }
            }
        }
        for (owner, edit) in &self.canonical.retirements {
            if let Some((_, retirement)) = &edit.after {
                self.capture(retirement.validate_local());
                if retirement.owner != *owner || self.overlay.owner(*owner).is_some() {
                    self.error(
                        "change_validate_retirement_key",
                        "candidate retirement key is foreign or remains live",
                    );
                }
            }
        }
    }

    fn validate_ownership_frontier(&mut self) {
        let mut parents = BTreeSet::new();
        for edit in &self.derived.ownership {
            for entry in [edit.before, edit.after].into_iter().flatten() {
                if let OwnershipParent::Owner(parent) = entry.parent {
                    parents.insert(parent);
                }
            }
        }
        for owner in self.canonical.owners.keys() {
            self.work.ownership_entries_checked =
                self.work.ownership_entries_checked.saturating_add(1);
            match self.overlay.owner(*owner) {
                Some(_) if self.ownership(*owner).is_none() => self.error(
                    "change_validate_ownership_missing",
                    format!("live candidate owner {owner:?} has no ownership witness"),
                ),
                None if self.ownership(*owner).is_some() => self.error(
                    "change_validate_ownership_stale",
                    format!("deleted candidate owner {owner:?} retains ownership witness"),
                ),
                Some(_) | None => {}
            }
        }
        for parent in parents {
            let Some(record) = self.overlay.owner(parent) else {
                continue;
            };
            match aggregation_children(record) {
                Ok(children) => {
                    for (role, child) in children {
                        if !role.aggregates_into_parent() {
                            continue;
                        }
                        self.work.ownership_entries_checked =
                            self.work.ownership_entries_checked.saturating_add(1);
                        let expected = OwnershipEntry::new(OwnershipParent::Owner(parent), role);
                        if self.ownership(child) != Some(expected) {
                            self.error(
                                "change_validate_ownership_child",
                                format!("candidate parent {parent:?} and child {child:?} disagree"),
                            );
                        }
                    }
                }
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        let parent_entries = self
            .canonical
            .owners
            .keys()
            .filter_map(|owner| self.ownership(*owner).map(|entry| (*owner, entry)))
            .collect::<Vec<_>>();
        for (owner, entry) in parent_entries {
            if let OwnershipParent::Owner(parent) = entry.parent
                && self.overlay.owner(parent).is_none()
            {
                self.error(
                    "change_validate_parent_missing",
                    format!("candidate owner {owner:?} has missing parent {parent:?}"),
                );
            }
        }
    }

    fn validate_type_frontier(&mut self) {
        let mut pending = self
            .canonical
            .owners
            .values()
            .filter_map(|edit| edit.after.as_ref())
            .flat_map(|(_, record)| record.type_roots())
            .map(|digest| (digest, 0_usize))
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        while let Some((digest, depth)) = pending.pop() {
            if !visited.insert(digest) {
                continue;
            }
            self.work.type_objects_checked = self.work.type_objects_checked.saturating_add(1);
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                self.error(
                    "change_validate_type_depth",
                    "candidate type closure exceeds the hostile-input depth bound",
                );
                continue;
            }
            let Some(object) = self.overlay.type_object(digest) else {
                self.error(
                    "change_validate_type_missing",
                    format!("candidate type object {digest} is missing"),
                );
                continue;
            };
            self.capture(object.validate_local());
            match crate::platform::kernel::encode_type_object(object) {
                Ok((actual, _)) if actual == digest => {}
                Ok(_) => self.error(
                    "change_validate_type_digest",
                    "candidate type object is bound under a foreign digest",
                ),
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
            pending.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|child| (child, depth.saturating_add(1))),
            );
        }
        for digest in self.canonical.type_additions.keys() {
            if !visited.contains(digest) {
                self.error(
                    "change_validate_type_unreachable",
                    format!("new type object {digest} is unreachable from changed meaning"),
                );
            }
        }
    }

    fn ownership(&self, owner: OwnerKey) -> Option<OwnershipEntry> {
        match self.ownership_edits.get(&owner) {
            Some(entry) => *entry,
            None => self.base_witness.entries.ownership.get(&owner).copied(),
        }
    }

    fn capture(&mut self, result: Result<(), Diagnostic>) {
        if let Err(diagnostic) = result {
            self.diagnostics.push(diagnostic);
        }
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticClass::Semantic, code, message));
    }
}

impl ExpressionRead for KernelOverlay<'_> {
    fn package_id(&self) -> PackageId {
        self.base().root.package_id
    }

    fn owner(&self, owner: OwnerKey) -> Option<&OwnerRecord> {
        KernelOverlay::owner(self, owner)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Option<&TypeObject> {
        KernelOverlay::type_object(self, digest)
    }

    fn has_dependency(&self, package: PackageId) -> bool {
        self.dependency(package).is_some()
    }
}

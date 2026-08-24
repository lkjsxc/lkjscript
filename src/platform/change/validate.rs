//! Owner-frontier structural and semantic validation over an isolated candidate overlay.

use super::{
    CanonicalBaseRead, CanonicalDelta, DerivedDelta, ImpactPlan, KernelOverlay, SummaryDelta,
    WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExpressionRead, OwnerKey, OwnerRecord, PackageId, TypeObject, TypeObjectDigest,
    validate_expression_roots,
};
use crate::platform::witness::{OwnershipEntry, OwnershipParent, aggregation_children};
use std::collections::{BTreeMap, BTreeSet};

pub const INCREMENTAL_VALIDATION_PROFILE: &str = "incremental_owner_frontier";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IncrementalValidationWork {
    pub owner_records_checked: u64,
    pub ownership_entries_checked: u64,
    pub type_objects_checked: u64,
    pub expression_work: u64,
    pub witness_reads: WitnessReadWork,
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

pub fn validate_structural_frontier<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
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
        ownership_cache: BTreeMap::new(),
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

pub fn validate_incremental_frontier<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    impact: &ImpactPlan,
    summaries: &SummaryDelta,
    base_witness: &W,
    mut structural: StructuralValidationReport,
) -> Result<IncrementalValidationReport, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut work = 0_usize;
    let mut live_semantic_roots = Vec::new();
    for owner in &impact.semantically_checked {
        match overlay.owner(*owner) {
            Ok(Some(_)) => live_semantic_roots.push(*owner),
            Ok(None) => {}
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    validate_expression_roots(overlay, live_semantic_roots, &mut diagnostics, &mut work);
    structural.work.expression_work = work as u64;
    if summaries.selected != impact.summary_owners {
        diagnostics.push(validation_error(
            DiagnosticClass::Corrupt,
            "change_validate_summary_selection",
            "validation summary selection disagrees with the exact impact plan",
        ));
    }
    let summaries_reused = base_witness
        .owner_summary_count()
        .checked_sub(summaries.base_summaries_selected)
        .ok_or_else(|| {
            vec![validation_error(
                DiagnosticClass::Corrupt,
                "change_validate_summary_count",
                "selected base summaries exceed the committed witness summary count",
            )]
        })?;
    if diagnostics.is_empty() {
        Ok(IncrementalValidationReport {
            profile: INCREMENTAL_VALIDATION_PROFILE,
            canonical_owners_changed: canonical.owners.len() as u64,
            structurally_checked: structural.structurally_checked,
            semantically_checked: impact.semantically_checked.clone(),
            summaries_reused,
            tests_selected: impact.tests.len() as u64,
            work: structural.work,
        })
    } else {
        Err(diagnostics)
    }
}

struct IncrementalValidator<'a, B: ?Sized, W: ?Sized> {
    overlay: &'a KernelOverlay<'a, B>,
    canonical: &'a CanonicalDelta,
    derived: &'a DerivedDelta,
    base_witness: &'a W,
    diagnostics: Vec<Diagnostic>,
    work: IncrementalValidationWork,
    ownership_edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    ownership_cache: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> IncrementalValidator<'_, B, W> {
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
                if dependency.package != *package || dependency.package == self.overlay.package_id()
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
                let remains_live = match self.overlay.owner(*owner) {
                    Ok(record) => record.is_some(),
                    Err(diagnostic) => {
                        self.diagnostics.push(diagnostic);
                        continue;
                    }
                };
                if retirement.owner != *owner || remains_live {
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
        let changed_owners = self.canonical.owners.keys().copied().collect::<Vec<_>>();
        for owner in &changed_owners {
            self.work.ownership_entries_checked =
                self.work.ownership_entries_checked.saturating_add(1);
            let candidate_ownership = match self.ownership(*owner) {
                Ok(entry) => entry,
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    continue;
                }
            };
            let candidate_record = match self.overlay.owner(*owner) {
                Ok(record) => record,
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    continue;
                }
            };
            match candidate_record {
                Some(_) if candidate_ownership.is_none() => self.error(
                    "change_validate_ownership_missing",
                    format!("live candidate owner {owner:?} has no ownership witness"),
                ),
                None if candidate_ownership.is_some() => self.error(
                    "change_validate_ownership_stale",
                    format!("deleted candidate owner {owner:?} retains ownership witness"),
                ),
                Some(_) | None => {}
            }
        }
        for parent in parents {
            let record = match self.overlay.owner(parent) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    continue;
                }
            };
            let children = match aggregation_children(&record) {
                Ok(children) => children,
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    continue;
                }
            };
            for (role, child) in children {
                if !role.aggregates_into_parent() {
                    continue;
                }
                self.work.ownership_entries_checked =
                    self.work.ownership_entries_checked.saturating_add(1);
                let expected = OwnershipEntry::new(OwnershipParent::Owner(parent), role);
                match self.ownership(child) {
                    Ok(Some(actual)) if actual == expected => {}
                    Ok(_) => self.error(
                        "change_validate_ownership_child",
                        format!("candidate parent {parent:?} and child {child:?} disagree"),
                    ),
                    Err(diagnostic) => self.diagnostics.push(diagnostic),
                }
            }
        }
        let mut parent_entries = Vec::new();
        for owner in changed_owners {
            match self.ownership(owner) {
                Ok(Some(entry)) => parent_entries.push((owner, entry)),
                Ok(None) => {}
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        for (owner, entry) in parent_entries {
            if let OwnershipParent::Owner(parent) = entry.parent {
                match self.overlay.owner(parent) {
                    Ok(Some(_)) => {}
                    Ok(None) => self.error(
                        "change_validate_parent_missing",
                        format!("candidate owner {owner:?} has missing parent {parent:?}"),
                    ),
                    Err(diagnostic) => self.diagnostics.push(diagnostic),
                }
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
            let object = match self.overlay.type_object(digest) {
                Ok(Some(object)) => object,
                Ok(None) => {
                    self.error(
                        "change_validate_type_missing",
                        format!("candidate type object {digest} is missing"),
                    );
                    continue;
                }
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    continue;
                }
            };
            self.capture(object.validate_local());
            match crate::platform::kernel::encode_type_object(&object) {
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

    fn ownership(&mut self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        match self.ownership_edits.get(&owner) {
            Some(entry) => Ok(*entry),
            None => {
                if let Some(cached) = self.ownership_cache.get(&owner) {
                    return Ok(*cached);
                }
                let read = self.base_witness.read_ownership(owner)?;
                self.work.witness_reads.add(read.work);
                self.ownership_cache.insert(owner, read.value);
                Ok(read.value)
            }
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

fn validation_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

impl<B: CanonicalBaseRead + ?Sized> ExpressionRead for KernelOverlay<'_, B> {
    fn package_id(&self) -> PackageId {
        KernelOverlay::package_id(self)
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        KernelOverlay::owner(self, owner)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        KernelOverlay::type_object(self, digest)
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.dependency(package)?.is_some())
    }
}

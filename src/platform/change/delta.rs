//! Exact normalized primitive edits over Graph 8 canonical maps.

use super::{CanonicalBaseRead, CanonicalReadWork};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::OwnerKey;
use crate::platform::kernel::{
    DependencyObjectDigest, DependencyRecord, KernelSnapshot, OwnerObjectDigest, OwnerRecord,
    PackageId, RetirementObjectDigest, RetirementRecord, TypeObject, TypeObjectDigest,
    encode_dependency, encode_owner, encode_retirement, encode_type_object,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct CanonicalNormalization {
    pub canonical: CanonicalDelta,
    pub base_revision: Option<crate::platform::semantic_id::RevisionId>,
    pub work: CanonicalReadWork,
}

#[derive(Clone, Debug)]
pub enum PrimitiveEdit {
    InsertOwner {
        record: OwnerRecord,
    },
    ReplaceOwner {
        expected: OwnerObjectDigest,
        record: OwnerRecord,
    },
    DeleteOwner {
        owner: OwnerKey,
        expected: OwnerObjectDigest,
    },
    AddTypeObject {
        digest: TypeObjectDigest,
        object: TypeObject,
    },
    InsertDependency {
        record: DependencyRecord,
    },
    ReplaceDependency {
        expected: DependencyObjectDigest,
        record: DependencyRecord,
    },
    DeleteDependency {
        package: PackageId,
        expected: DependencyObjectDigest,
    },
    InsertRetirement {
        record: RetirementRecord,
    },
    ReplaceRetirement {
        expected: RetirementObjectDigest,
        record: RetirementRecord,
    },
    DeleteRetirement {
        owner: OwnerKey,
        expected: RetirementObjectDigest,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactEdit<D, V> {
    pub before: Option<D>,
    pub after: Option<(D, V)>,
}

#[derive(Clone, Debug, Default)]
pub struct CanonicalDelta {
    pub owners: BTreeMap<OwnerKey, ExactEdit<OwnerObjectDigest, OwnerRecord>>,
    pub type_additions: BTreeMap<TypeObjectDigest, TypeObject>,
    pub dependencies: BTreeMap<PackageId, ExactEdit<DependencyObjectDigest, DependencyRecord>>,
    pub retirements: BTreeMap<OwnerKey, ExactEdit<RetirementObjectDigest, RetirementRecord>>,
}

impl CanonicalDelta {
    pub fn normalize(base: &KernelSnapshot, edits: Vec<PrimitiveEdit>) -> Result<Self, Diagnostic> {
        Self::normalize_from(base, edits).map(|normalization| normalization.canonical)
    }

    pub fn normalize_from<R: CanonicalBaseRead + ?Sized>(
        base: &R,
        edits: Vec<PrimitiveEdit>,
    ) -> Result<CanonicalNormalization, Diagnostic> {
        let base_revision = base.exact_revision();
        let mut base = NormalizationBase::new(base);
        let mut delta = Self::default();
        let mut seen_owners = BTreeSet::new();
        let mut seen_types = BTreeSet::new();
        let mut seen_dependencies = BTreeSet::new();
        let mut seen_retirements = BTreeSet::new();
        for edit in edits {
            match edit {
                PrimitiveEdit::InsertOwner { record } => {
                    let owner = record.owner();
                    reject_duplicate(&mut seen_owners, owner, "owner")?;
                    if base.owner(owner)?.is_some() {
                        return Err(change_error(
                            "change_owner_present",
                            format!("owner {owner:?} already exists at the exact base"),
                        ));
                    }
                    let (digest, _) = encode_owner(&record)?;
                    delta.owners.insert(
                        owner,
                        ExactEdit {
                            before: None,
                            after: Some((digest, record)),
                        },
                    );
                }
                PrimitiveEdit::ReplaceOwner { expected, record } => {
                    let owner = record.owner();
                    reject_duplicate(&mut seen_owners, owner, "owner")?;
                    let before = exact_owner_digest(&mut base, owner)?;
                    require_expected("owner", before, expected)?;
                    let (after, _) = encode_owner(&record)?;
                    if after != before {
                        delta.owners.insert(
                            owner,
                            ExactEdit {
                                before: Some(before),
                                after: Some((after, record)),
                            },
                        );
                    }
                }
                PrimitiveEdit::DeleteOwner { owner, expected } => {
                    reject_duplicate(&mut seen_owners, owner, "owner")?;
                    let before = exact_owner_digest(&mut base, owner)?;
                    require_expected("owner", before, expected)?;
                    delta.owners.insert(
                        owner,
                        ExactEdit {
                            before: Some(before),
                            after: None,
                        },
                    );
                }
                PrimitiveEdit::AddTypeObject { digest, object } => {
                    reject_duplicate(&mut seen_types, digest, "type object")?;
                    let (actual, _) = encode_type_object(&object)?;
                    require_expected("type object", actual, digest)?;
                    match base.type_object(digest)? {
                        Some(existing) if existing == &object => {}
                        Some(_) => {
                            return Err(change_corrupt(
                                "change_type_collision",
                                "one type digest is bound to different canonical values",
                            ));
                        }
                        None => {
                            delta.type_additions.insert(digest, object);
                        }
                    }
                }
                PrimitiveEdit::InsertDependency { record } => {
                    let package = record.package;
                    reject_duplicate(&mut seen_dependencies, package, "dependency")?;
                    if base.dependency(package)?.is_some() {
                        return Err(change_error(
                            "change_dependency_present",
                            format!("dependency {package} already exists at the exact base"),
                        ));
                    }
                    let (digest, _) = encode_dependency(&record)?;
                    delta.dependencies.insert(
                        package,
                        ExactEdit {
                            before: None,
                            after: Some((digest, record)),
                        },
                    );
                }
                PrimitiveEdit::ReplaceDependency { expected, record } => {
                    let package = record.package;
                    reject_duplicate(&mut seen_dependencies, package, "dependency")?;
                    let before = exact_dependency_digest(&mut base, package)?;
                    require_expected("dependency", before, expected)?;
                    let (after, _) = encode_dependency(&record)?;
                    if after != before {
                        delta.dependencies.insert(
                            package,
                            ExactEdit {
                                before: Some(before),
                                after: Some((after, record)),
                            },
                        );
                    }
                }
                PrimitiveEdit::DeleteDependency { package, expected } => {
                    reject_duplicate(&mut seen_dependencies, package, "dependency")?;
                    let before = exact_dependency_digest(&mut base, package)?;
                    require_expected("dependency", before, expected)?;
                    delta.dependencies.insert(
                        package,
                        ExactEdit {
                            before: Some(before),
                            after: None,
                        },
                    );
                }
                PrimitiveEdit::InsertRetirement { record } => {
                    let owner = record.owner;
                    reject_duplicate(&mut seen_retirements, owner, "retirement")?;
                    if base.retirement(owner)?.is_some() {
                        return Err(change_error(
                            "change_retirement_present",
                            format!("retirement {owner:?} already exists at the exact base"),
                        ));
                    }
                    let (digest, _) = encode_retirement(&record)?;
                    delta.retirements.insert(
                        owner,
                        ExactEdit {
                            before: None,
                            after: Some((digest, record)),
                        },
                    );
                }
                PrimitiveEdit::ReplaceRetirement { expected, record } => {
                    let owner = record.owner;
                    reject_duplicate(&mut seen_retirements, owner, "retirement")?;
                    let before = exact_retirement_digest(&mut base, owner)?;
                    require_expected("retirement", before, expected)?;
                    let (after, _) = encode_retirement(&record)?;
                    if after != before {
                        delta.retirements.insert(
                            owner,
                            ExactEdit {
                                before: Some(before),
                                after: Some((after, record)),
                            },
                        );
                    }
                }
                PrimitiveEdit::DeleteRetirement { owner, expected } => {
                    reject_duplicate(&mut seen_retirements, owner, "retirement")?;
                    let before = exact_retirement_digest(&mut base, owner)?;
                    require_expected("retirement", before, expected)?;
                    delta.retirements.insert(
                        owner,
                        ExactEdit {
                            before: Some(before),
                            after: None,
                        },
                    );
                }
            }
        }
        validate_live_retired_exclusion(&mut base, &delta)?;
        Ok(CanonicalNormalization {
            canonical: delta,
            base_revision,
            work: base.work,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.owners.is_empty()
            && self.type_additions.is_empty()
            && self.dependencies.is_empty()
            && self.retirements.is_empty()
    }

    pub fn changed_owner_count(&self) -> usize {
        self.owners.len()
    }
}

fn exact_owner_digest<R: CanonicalBaseRead + ?Sized>(
    base: &mut NormalizationBase<'_, R>,
    owner: OwnerKey,
) -> Result<OwnerObjectDigest, Diagnostic> {
    base.owner(owner)?
        .ok_or_else(|| {
            change_error(
                "change_owner_missing",
                format!("owner {owner:?} is absent at the exact base"),
            )
        })
        .and_then(|record| encode_owner(record).map(|(digest, _)| digest))
}

fn exact_dependency_digest<R: CanonicalBaseRead + ?Sized>(
    base: &mut NormalizationBase<'_, R>,
    package: PackageId,
) -> Result<DependencyObjectDigest, Diagnostic> {
    base.dependency(package)?
        .ok_or_else(|| {
            change_error(
                "change_dependency_missing",
                format!("dependency {package} is absent at the exact base"),
            )
        })
        .and_then(|record| encode_dependency(record).map(|(digest, _)| digest))
}

fn exact_retirement_digest<R: CanonicalBaseRead + ?Sized>(
    base: &mut NormalizationBase<'_, R>,
    owner: OwnerKey,
) -> Result<RetirementObjectDigest, Diagnostic> {
    base.retirement(owner)?
        .ok_or_else(|| {
            change_error(
                "change_retirement_missing",
                format!("retirement {owner:?} is absent at the exact base"),
            )
        })
        .and_then(|record| encode_retirement(record).map(|(digest, _)| digest))
}

fn require_expected<D: Eq + std::fmt::Debug>(
    label: &str,
    actual: D,
    expected: D,
) -> Result<(), Diagnostic> {
    if actual != expected {
        return Err(change_error(
            "change_exact_precondition",
            format!(
                "{label} digest precondition failed: expected {expected:?}, observed {actual:?}"
            ),
        ));
    }
    Ok(())
}

fn reject_duplicate<K: Copy + Ord>(
    seen: &mut BTreeSet<K>,
    key: K,
    label: &str,
) -> Result<(), Diagnostic> {
    if !seen.insert(key) {
        return Err(duplicate_edit(label));
    }
    Ok(())
}

fn duplicate_edit(label: &str) -> Diagnostic {
    change_error(
        "change_duplicate_primitive",
        format!("normalized request contains more than one primitive edit for one {label} key"),
    )
}

fn validate_live_retired_exclusion<R: CanonicalBaseRead + ?Sized>(
    base: &mut NormalizationBase<'_, R>,
    delta: &CanonicalDelta,
) -> Result<(), Diagnostic> {
    let touched = delta
        .owners
        .keys()
        .chain(delta.retirements.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for owner in touched {
        let live = match delta.owners.get(&owner) {
            Some(edit) => edit.after.is_some(),
            None => base.owner(owner)?.is_some(),
        };
        let retired = match delta.retirements.get(&owner) {
            Some(edit) => edit.after.is_some(),
            None => base.retirement(owner)?.is_some(),
        };
        if live && retired {
            return Err(change_error(
                "change_live_retired_overlap",
                format!("owner {owner:?} would be both live and retired"),
            ));
        }
    }
    Ok(())
}

struct NormalizationBase<'a, R: ?Sized> {
    base: &'a R,
    owners: BTreeMap<OwnerKey, Option<OwnerRecord>>,
    types: BTreeMap<TypeObjectDigest, Option<TypeObject>>,
    dependencies: BTreeMap<PackageId, Option<DependencyRecord>>,
    retirements: BTreeMap<OwnerKey, Option<RetirementRecord>>,
    work: CanonicalReadWork,
}

impl<'a, R: CanonicalBaseRead + ?Sized> NormalizationBase<'a, R> {
    fn new(base: &'a R) -> Self {
        Self {
            base,
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            retirements: BTreeMap::new(),
            work: CanonicalReadWork::default(),
        }
    }

    fn owner(&mut self, owner: OwnerKey) -> Result<Option<&OwnerRecord>, Diagnostic> {
        if !self.owners.contains_key(&owner) {
            let read = self.base.read_owner(owner)?;
            self.work.add(read.work);
            self.owners.insert(owner, read.value);
        }
        Ok(self.owners.get(&owner).and_then(Option::as_ref))
    }

    fn type_object(&mut self, digest: TypeObjectDigest) -> Result<Option<&TypeObject>, Diagnostic> {
        if !self.types.contains_key(&digest) {
            let read = self.base.read_type_object(digest)?;
            self.work.add(read.work);
            self.types.insert(digest, read.value);
        }
        Ok(self.types.get(&digest).and_then(Option::as_ref))
    }

    fn dependency(&mut self, package: PackageId) -> Result<Option<&DependencyRecord>, Diagnostic> {
        if !self.dependencies.contains_key(&package) {
            let read = self.base.read_dependency(package)?;
            self.work.add(read.work);
            self.dependencies.insert(package, read.value);
        }
        Ok(self.dependencies.get(&package).and_then(Option::as_ref))
    }

    fn retirement(&mut self, owner: OwnerKey) -> Result<Option<&RetirementRecord>, Diagnostic> {
        if !self.retirements.contains_key(&owner) {
            let read = self.base.read_retirement(owner)?;
            self.work.add(read.work);
            self.retirements.insert(owner, read.value);
        }
        Ok(self.retirements.get(&owner).and_then(Option::as_ref))
    }
}

fn change_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

fn change_corrupt(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

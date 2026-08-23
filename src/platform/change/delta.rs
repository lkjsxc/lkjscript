//! Exact normalized primitive edits over Graph 5 canonical maps.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::OwnerKey;
use crate::platform::kernel::{
    DependencyObjectDigest, DependencyRecord, KernelSnapshot, OwnerObjectDigest, OwnerRecord,
    PackageId, RetirementObjectDigest, RetirementRecord, TypeObject, TypeObjectDigest,
    encode_dependency, encode_owner, encode_retirement, encode_type_object,
};
use std::collections::{BTreeMap, BTreeSet};

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
                    if base.owners.contains_key(&owner) {
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
                    let before = exact_owner_digest(base, owner)?;
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
                    let before = exact_owner_digest(base, owner)?;
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
                    match base.types.get(&digest) {
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
                    if base.dependencies.contains_key(&package) {
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
                    let before = exact_dependency_digest(base, package)?;
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
                    let before = exact_dependency_digest(base, package)?;
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
                    if base.retirements.contains_key(&owner) {
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
                    let before = exact_retirement_digest(base, owner)?;
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
                    let before = exact_retirement_digest(base, owner)?;
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
        validate_live_retired_exclusion(base, &delta)?;
        Ok(delta)
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

fn exact_owner_digest(
    base: &KernelSnapshot,
    owner: OwnerKey,
) -> Result<OwnerObjectDigest, Diagnostic> {
    base.owners
        .get(&owner)
        .ok_or_else(|| {
            change_error(
                "change_owner_missing",
                format!("owner {owner:?} is absent at the exact base"),
            )
        })
        .and_then(|record| encode_owner(record).map(|(digest, _)| digest))
}

fn exact_dependency_digest(
    base: &KernelSnapshot,
    package: PackageId,
) -> Result<DependencyObjectDigest, Diagnostic> {
    base.dependencies
        .get(&package)
        .ok_or_else(|| {
            change_error(
                "change_dependency_missing",
                format!("dependency {package} is absent at the exact base"),
            )
        })
        .and_then(|record| encode_dependency(record).map(|(digest, _)| digest))
}

fn exact_retirement_digest(
    base: &KernelSnapshot,
    owner: OwnerKey,
) -> Result<RetirementObjectDigest, Diagnostic> {
    base.retirements
        .get(&owner)
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

fn validate_live_retired_exclusion(
    base: &KernelSnapshot,
    delta: &CanonicalDelta,
) -> Result<(), Diagnostic> {
    let touched = delta
        .owners
        .keys()
        .chain(delta.retirements.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for owner in touched {
        let live = delta.owners.get(&owner).map_or_else(
            || base.owners.contains_key(&owner),
            |edit| edit.after.is_some(),
        );
        let retired = delta.retirements.get(&owner).map_or_else(
            || base.retirements.contains_key(&owner),
            |edit| edit.after.is_some(),
        );
        if live && retired {
            return Err(change_error(
                "change_live_retired_overlap",
                format!("owner {owner:?} would be both live and retired"),
            ));
        }
    }
    Ok(())
}

fn change_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

fn change_corrupt(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

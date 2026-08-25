//! Exact semantic caller intent evaluated against one accepted base revision.

use super::{AuthoredLowerer, WorkingOwner};
use crate::platform::change::{CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, Name, NamespaceClass, OwnerKey, OwnerRecord, PackageId,
    PackageRevisionDigest, encode_owner, owner_namespace,
};
use crate::platform::semantic_id::RevisionId;
use crate::platform::witness::{NamespaceKey, OwnershipParent, ownership_contributions};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthoredOwnerParent {
    Package,
    Owner(OwnerKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredPrecondition {
    OwnerExists {
        owner: OwnerKey,
    },
    OwnerAbsent {
        owner: OwnerKey,
    },
    OwnerName {
        owner: OwnerKey,
        equals: Name,
    },
    OwnerParent {
        owner: OwnerKey,
        equals: AuthoredOwnerParent,
    },
    NamespaceAbsent {
        parent: Option<OwnerKey>,
        class: NamespaceClass,
        name: Name,
    },
    NamespacePointsTo {
        parent: Option<OwnerKey>,
        class: NamespaceClass,
        name: Name,
        owner: OwnerKey,
    },
    DependencyBinding {
        package: PackageId,
        semantic_revision: RevisionId,
        package_revision: PackageRevisionDigest,
    },
}

pub(super) fn evaluate<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    preconditions: &[AuthoredPrecondition],
) -> Result<(), Diagnostic> {
    let mut dependencies = BTreeMap::<PackageId, Option<DependencyRecord>>::new();

    for precondition in preconditions {
        lowerer.work.preconditions_checked = lowerer.work.preconditions_checked.saturating_add(1);
        match precondition {
            AuthoredPrecondition::OwnerExists { owner } => {
                require(
                    read_owner(lowerer, *owner)?.is_some(),
                    "change_precondition_owner_missing",
                    format!("required owner {owner:?} is absent at the exact base"),
                )?;
            }
            AuthoredPrecondition::OwnerAbsent { owner } => {
                require(
                    read_owner(lowerer, *owner)?.is_none(),
                    "change_precondition_owner_present",
                    format!("owner {owner:?} expected to be absent is live at the exact base"),
                )?;
            }
            AuthoredPrecondition::OwnerName { owner, equals } => {
                let observed = read_owner(lowerer, *owner)?
                    .as_ref()
                    .and_then(OwnerRecord::name)
                    .cloned();
                require(
                    observed.as_ref() == Some(equals),
                    "change_precondition_owner_name",
                    format!(
                        "owner {owner:?} name precondition failed: expected {equals:?}, observed {observed:?}"
                    ),
                )?;
            }
            AuthoredPrecondition::OwnerParent { owner, equals } => {
                let observed = canonical_owner_parent(lowerer, *owner, *equals)?;
                require(
                    observed == Some(*equals),
                    "change_precondition_owner_parent",
                    format!(
                        "owner {owner:?} parent precondition failed: expected {equals:?}, observed {observed:?}"
                    ),
                )?;
            }
            AuthoredPrecondition::NamespaceAbsent {
                parent,
                class,
                name,
            } => {
                let key = NamespaceKey {
                    parent: *parent,
                    class: *class,
                    name: name.clone(),
                };
                let observed = read_namespace(lowerer, &key)?;
                if let Some(observed) = observed {
                    verify_namespace_target(lowerer, &key, observed)?;
                }
                require(
                    observed.is_none(),
                    "change_precondition_namespace_present",
                    format!(
                        "namespace {key:?} expected to be absent points to {observed:?} at the exact base"
                    ),
                )?;
            }
            AuthoredPrecondition::NamespacePointsTo {
                parent,
                class,
                name,
                owner,
            } => {
                let key = NamespaceKey {
                    parent: *parent,
                    class: *class,
                    name: name.clone(),
                };
                let observed = read_namespace(lowerer, &key)?;
                if let Some(observed) = observed {
                    verify_namespace_target(lowerer, &key, observed)?;
                }
                require(
                    observed == Some(*owner),
                    "change_precondition_namespace_owner",
                    format!(
                        "namespace {key:?} precondition failed: expected {owner:?}, observed {observed:?}"
                    ),
                )?;
            }
            AuthoredPrecondition::DependencyBinding {
                package,
                semantic_revision,
                package_revision,
            } => {
                let observed = read_dependency(lowerer, &mut dependencies, *package)?;
                require(
                    observed.as_ref().is_some_and(|record| {
                        record.package == *package
                            && record.semantic_revision == *semantic_revision
                            && record.package_revision == *package_revision
                    }),
                    "change_precondition_dependency_binding",
                    format!(
                        "dependency {package} binding precondition failed: expected semantic revision {semantic_revision} and package revision {package_revision}, observed {observed:?}"
                    ),
                )?;
            }
        }
        lowerer.check_budget("authored precondition evaluation")?;
    }
    Ok(())
}

fn read_owner<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<Option<OwnerRecord>, Diagnostic> {
    if let Some(working) = lowerer.owners.get(&owner) {
        return Ok(working.before.map(|_| working.record.clone()));
    }
    let read = lowerer.base.read_owner(owner)?;
    lowerer.work.canonical.add(read.work);
    if let Some(record) = read.value {
        let (before, _) = encode_owner(&record)?;
        lowerer.owners.insert(
            owner,
            WorkingOwner {
                before: Some(before),
                original: Some(record.clone()),
                record: record.clone(),
                deleted: false,
            },
        );
        Ok(Some(record))
    } else {
        Ok(None)
    }
}

fn read_namespace<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    key: &NamespaceKey,
) -> Result<Option<OwnerKey>, Diagnostic> {
    if !lowerer.namespace.contains_key(key) {
        let read = lowerer.witness.read_namespace(key)?;
        lowerer.work.witness.add(read.work);
        lowerer.namespace.insert(key.clone(), read.value);
    }
    Ok(lowerer.namespace.get(key).copied().flatten())
}

fn verify_namespace_target<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    key: &NamespaceKey,
    owner: OwnerKey,
) -> Result<(), Diagnostic> {
    let observed = read_owner(lowerer, owner)?;
    let canonical = observed
        .as_ref()
        .and_then(owner_namespace)
        .map(|entry| NamespaceKey {
            parent: entry.parent,
            class: entry.class,
            name: entry.name.clone(),
        });
    if canonical.as_ref() == Some(key) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticClass::Corrupt,
            "change_precondition_namespace_witness",
            format!(
                "namespace witness maps {key:?} to owner {owner:?}, whose canonical namespace is {canonical:?}"
            ),
        ))
    }
}

fn canonical_owner_parent<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
    expected: AuthoredOwnerParent,
) -> Result<Option<AuthoredOwnerParent>, Diagnostic> {
    let owner_record = read_owner(lowerer, owner)?;
    let mut canonical = owner_record
        .as_ref()
        .map(ownership_contributions)
        .transpose()?
        .and_then(|entries| entries.get(&owner).copied());
    if canonical.is_none()
        && let AuthoredOwnerParent::Owner(parent) = expected
    {
        canonical = read_owner(lowerer, parent)?
            .as_ref()
            .map(ownership_contributions)
            .transpose()?
            .and_then(|entries| entries.get(&owner).copied());
    }
    Ok(canonical.map(|entry| match entry.parent {
        OwnershipParent::Package => AuthoredOwnerParent::Package,
        OwnershipParent::Owner(parent) => AuthoredOwnerParent::Owner(parent),
    }))
}

fn read_dependency<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    cache: &mut BTreeMap<PackageId, Option<DependencyRecord>>,
    package: PackageId,
) -> Result<Option<DependencyRecord>, Diagnostic> {
    if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(package) {
        let read = lowerer.base.read_dependency(package)?;
        lowerer.work.canonical.add(read.work);
        entry.insert(read.value);
    }
    Ok(cache.get(&package).cloned().flatten())
}

fn require(
    condition: bool,
    code: &'static str,
    message: impl Into<String>,
) -> Result<(), Diagnostic> {
    if condition {
        Ok(())
    } else {
        Err(Diagnostic::new(DiagnosticClass::Semantic, code, message))
    }
}

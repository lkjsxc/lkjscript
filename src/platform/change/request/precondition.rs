//! Exact, base-only preconditions for authored Graph 5 changes.

use super::{AuthoredLowerer, WorkingOwner};
use crate::platform::change::{CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyObjectDigest, DependencyRecord, Name, NamespaceClass, OwnerKey, OwnerObjectDigest,
    OwnerRecord, PackageId, RetirementObjectDigest, RetirementRecord, SemanticRootDigest,
    encode_dependency, encode_owner, encode_retirement, encode_root,
};
use crate::platform::witness::{NamespaceKey, OwnerSummaryDigest, OwnershipEntry, OwnershipParent};
use bincode::{Decode, Encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredPreconditionV1")]
#[serde(tag = "precondition", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoredPrecondition {
    SemanticRoot {
        equals: SemanticRootDigest,
    },
    OwnerExists {
        owner: OwnerKey,
    },
    OwnerAbsent {
        owner: OwnerKey,
    },
    OwnerDigest {
        owner: OwnerKey,
        equals: OwnerObjectDigest,
    },
    OwnerName {
        owner: OwnerKey,
        equals: Name,
    },
    OwnerParent {
        owner: OwnerKey,
        equals: OwnershipParent,
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
    OwnerSummaryDigest {
        owner: OwnerKey,
        equals: OwnerSummaryDigest,
    },
    DependencyDigest {
        package: PackageId,
        equals: DependencyObjectDigest,
    },
    RetirementDigest {
        owner: OwnerKey,
        equals: RetirementObjectDigest,
    },
}

pub(super) fn evaluate<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    preconditions: &[AuthoredPrecondition],
) -> Result<(), Diagnostic> {
    let mut ownership = BTreeMap::<OwnerKey, Option<OwnershipEntry>>::new();
    let mut summaries = BTreeMap::<OwnerKey, Option<OwnerSummaryDigest>>::new();
    let mut dependencies = BTreeMap::<PackageId, Option<DependencyRecord>>::new();
    let mut retirements = BTreeMap::<OwnerKey, Option<RetirementRecord>>::new();

    for precondition in preconditions {
        lowerer.work.preconditions_checked = lowerer.work.preconditions_checked.saturating_add(1);
        match precondition {
            AuthoredPrecondition::SemanticRoot { equals } => {
                let observed = encode_root(lowerer.base.semantic_root())?.0;
                require(
                    observed == *equals,
                    "change_precondition_semantic_root",
                    format!(
                        "semantic-root precondition failed: expected {equals}, observed {observed}"
                    ),
                )?;
            }
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
            AuthoredPrecondition::OwnerDigest { owner, equals } => {
                let observed = read_owner(lowerer, *owner)?
                    .as_ref()
                    .map(encode_owner)
                    .transpose()?
                    .map(|(digest, _)| digest);
                require(
                    observed == Some(*equals),
                    "change_precondition_owner_digest",
                    format!(
                        "owner {owner:?} digest precondition failed: expected {equals}, observed {observed:?}"
                    ),
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
                let observed =
                    read_ownership(lowerer, &mut ownership, *owner)?.map(|entry| entry.parent);
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
                require(
                    observed == Some(*owner),
                    "change_precondition_namespace_owner",
                    format!(
                        "namespace {key:?} precondition failed: expected {owner:?}, observed {observed:?}"
                    ),
                )?;
            }
            AuthoredPrecondition::OwnerSummaryDigest { owner, equals } => {
                let observed = read_summary(lowerer, &mut summaries, *owner)?;
                require(
                    observed == Some(*equals),
                    "change_precondition_owner_summary",
                    format!(
                        "owner {owner:?} summary precondition failed: expected {equals}, observed {observed:?}"
                    ),
                )?;
            }
            AuthoredPrecondition::DependencyDigest { package, equals } => {
                let observed = read_dependency(lowerer, &mut dependencies, *package)?
                    .as_ref()
                    .map(encode_dependency)
                    .transpose()?
                    .map(|(digest, _)| digest);
                require(
                    observed == Some(*equals),
                    "change_precondition_dependency_digest",
                    format!(
                        "dependency {package} digest precondition failed: expected {equals}, observed {observed:?}"
                    ),
                )?;
            }
            AuthoredPrecondition::RetirementDigest { owner, equals } => {
                let observed = read_retirement(lowerer, &mut retirements, *owner)?
                    .as_ref()
                    .map(encode_retirement)
                    .transpose()?
                    .map(|(digest, _)| digest);
                require(
                    observed == Some(*equals),
                    "change_precondition_retirement_digest",
                    format!(
                        "retirement {owner:?} digest precondition failed: expected {equals}, observed {observed:?}"
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

fn read_ownership<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    cache: &mut BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    owner: OwnerKey,
) -> Result<Option<OwnershipEntry>, Diagnostic> {
    if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(owner) {
        let read = lowerer.witness.read_ownership(owner)?;
        lowerer.work.witness.add(read.work);
        entry.insert(read.value);
    }
    Ok(cache.get(&owner).copied().flatten())
}

fn read_summary<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    cache: &mut BTreeMap<OwnerKey, Option<OwnerSummaryDigest>>,
    owner: OwnerKey,
) -> Result<Option<OwnerSummaryDigest>, Diagnostic> {
    if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(owner) {
        let read = lowerer.witness.read_owner_summary(owner)?;
        lowerer.work.witness.add(read.work);
        entry.insert(read.value.map(|bound| bound.digest));
    }
    Ok(cache.get(&owner).copied().flatten())
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

fn read_retirement<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    cache: &mut BTreeMap<OwnerKey, Option<RetirementRecord>>,
    owner: OwnerKey,
) -> Result<Option<RetirementRecord>, Diagnostic> {
    if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(owner) {
        let read = lowerer.base.read_retirement(owner)?;
        lowerer.work.canonical.add(read.work);
        entry.insert(read.value);
    }
    Ok(cache.get(&owner).cloned().flatten())
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

//! Locally derived witness deltas from changed canonical owner records.

use super::{CanonicalBaseRead, CanonicalDelta, KernelOverlay, WitnessBaseRead, WitnessReadWork};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExactOwnerKey, OwnerKey, OwnerRecord, RelationEdge, RelationEndpoint, RelationKind,
    extract_owner_relations, owner_namespace,
};
use crate::platform::witness::{
    NamespaceKey, OwnershipEntry, OwnershipParent, ownership_contributions,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedValueEdit<K, V> {
    pub key: K,
    pub before: Option<V>,
    pub after: Option<V>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelationDelta {
    pub removed: BTreeSet<RelationEdge>,
    pub added: BTreeSet<RelationEdge>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedDelta {
    pub namespaces: Vec<DerivedValueEdit<NamespaceKey, OwnerKey>>,
    pub ownership: Vec<DerivedValueEdit<OwnerKey, OwnershipEntry>>,
    pub relations: RelationDelta,
    pub summary_candidates: BTreeSet<OwnerKey>,
    pub read_work: WitnessReadWork,
}

pub fn derive_local_delta<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    delta: &CanonicalDelta,
    base_witness: &W,
) -> Result<DerivedDelta, Diagnostic> {
    if !base_witness.witness_contract_is_current()
        || base_witness.witness_repository_id() != overlay.repository_id()
        || base_witness.witness_package_id() != overlay.package_id()
    {
        return Err(derived_error(
            DiagnosticClass::Corrupt,
            "change_witness_authority",
            "base witness belongs to a different repository or package",
        ));
    }

    let mut before_namespaces = BTreeMap::new();
    let mut after_namespaces = BTreeMap::new();
    let mut before_ownership = BTreeMap::new();
    let mut after_ownership = BTreeMap::new();
    let mut before_relations = BTreeSet::new();
    let mut after_relations = BTreeSet::new();
    let mut witness = CachedWitness::new(base_witness);

    for (owner, edit) in &delta.owners {
        if edit.before.is_some() {
            let record = overlay.base_owner(*owner)?.ok_or_else(|| {
                derived_error(
                    DiagnosticClass::Corrupt,
                    "change_delta_before_owner",
                    "canonical owner delta names a missing before record",
                )
            })?;
            insert_namespace_contribution(&mut before_namespaces, &record)?;
            insert_ownership_contributions(&mut before_ownership, &record)?;
            before_relations.extend(extract_owner_relations(
                overlay.package_id(),
                *owner,
                &record,
                |digest| overlay.base_type_object(digest),
                |package, case| {
                    if package != overlay.package_id() {
                        return Ok(None);
                    }
                    Ok(match overlay.base_owner(OwnerKey::Case(case))? {
                        Some(OwnerRecord::Case(record)) => Some(record.declaration),
                        _ => None,
                    })
                },
            )?);
        }
        if let Some((_, record)) = &edit.after {
            insert_namespace_contribution(&mut after_namespaces, record)?;
            insert_ownership_contributions(&mut after_ownership, record)?;
            after_relations.extend(extract_owner_relations(
                overlay.package_id(),
                *owner,
                record,
                |digest| overlay.type_object(digest),
                |package, case| {
                    if package != overlay.package_id() {
                        return Ok(None);
                    }
                    Ok(match overlay.owner(OwnerKey::Case(case))? {
                        Some(OwnerRecord::Case(record)) => Some(record.declaration),
                        _ => None,
                    })
                },
            )?);
        }
    }

    for (package, edit) in &delta.dependencies {
        if edit.before.is_some() {
            before_relations.insert(package_dependency(overlay.package_id(), *package));
        }
        if edit.after.is_some() {
            after_relations.insert(package_dependency(overlay.package_id(), *package));
        }
    }

    let namespaces =
        contribution_edits(&before_namespaces, &after_namespaces, "namespace", |key| {
            witness.namespace(key)
        })?;
    let ownership =
        contribution_edits(&before_ownership, &after_ownership, "ownership", |owner| {
            witness.ownership(*owner)
        })?;
    let removed = before_relations
        .difference(&after_relations)
        .copied()
        .collect::<BTreeSet<_>>();
    let added = after_relations
        .difference(&before_relations)
        .copied()
        .collect::<BTreeSet<_>>();
    for edge in &removed {
        if !witness.contains_relation(*edge)? {
            return Err(derived_error(
                DiagnosticClass::Corrupt,
                "change_relation_before",
                "locally removed relation is absent from the base witness",
            ));
        }
    }

    let summary_candidates = summary_candidates(
        overlay.package_id(),
        delta,
        &ownership,
        &removed,
        &added,
        &mut witness,
    )?;
    Ok(DerivedDelta {
        namespaces,
        ownership,
        relations: RelationDelta { removed, added },
        summary_candidates,
        read_work: witness.work,
    })
}

fn insert_namespace_contribution(
    entries: &mut BTreeMap<NamespaceKey, OwnerKey>,
    record: &OwnerRecord,
) -> Result<(), Diagnostic> {
    let Some(namespace) = owner_namespace(record) else {
        return Ok(());
    };
    let key = NamespaceKey {
        parent: namespace.parent,
        class: namespace.class,
        name: namespace.name.clone(),
    };
    if let Some(previous) = entries.insert(key, record.owner()) {
        return Err(derived_error(
            DiagnosticClass::Semantic,
            "change_namespace_candidate_duplicate",
            format!(
                "changed owners {previous:?} and {:?} select the same namespace",
                record.owner()
            ),
        ));
    }
    Ok(())
}

fn insert_ownership_contributions(
    entries: &mut BTreeMap<OwnerKey, OwnershipEntry>,
    record: &OwnerRecord,
) -> Result<(), Diagnostic> {
    for (owner, entry) in ownership_contributions(record)? {
        if let Some(previous) = entries.insert(owner, entry) {
            return Err(derived_error(
                DiagnosticClass::Semantic,
                "change_ownership_candidate_duplicate",
                format!("owner {owner:?} receives candidate parents {previous:?} and {entry:?}"),
            ));
        }
    }
    Ok(())
}

fn contribution_edits<K, V>(
    before: &BTreeMap<K, V>,
    after: &BTreeMap<K, V>,
    label: &str,
    mut read: impl FnMut(&K) -> Result<Option<V>, Diagnostic>,
) -> Result<Vec<DerivedValueEdit<K, V>>, Diagnostic>
where
    K: Clone + Debug + Ord,
    V: Clone + Debug + Eq,
{
    let keys = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut edits = Vec::new();
    for key in keys {
        let observed = read(&key)?;
        let mut candidate = observed.clone();
        if let Some(expected) = before.get(&key) {
            if candidate.as_ref() != Some(expected) {
                return Err(derived_error(
                    DiagnosticClass::Corrupt,
                    "change_derived_before",
                    format!("base {label} witness disagrees at key {key:?}"),
                ));
            }
            candidate = None;
        }
        if let Some(value) = after.get(&key) {
            if candidate.as_ref().is_some_and(|current| current != value) {
                return Err(derived_error(
                    DiagnosticClass::Semantic,
                    "change_derived_collision",
                    format!("candidate {label} collides at key {key:?}"),
                ));
            }
            candidate = Some(value.clone());
        }
        if observed != candidate {
            edits.push(DerivedValueEdit {
                key,
                before: observed,
                after: candidate,
            });
        }
    }
    Ok(edits)
}

fn package_dependency(
    source: crate::platform::kernel::PackageId,
    target: crate::platform::kernel::PackageId,
) -> RelationEdge {
    RelationEdge {
        source: RelationEndpoint::Package(source),
        kind: RelationKind::PackageDependency,
        target: RelationEndpoint::Package(target),
    }
}

fn summary_candidates<W: WitnessBaseRead + ?Sized>(
    package: crate::platform::kernel::PackageId,
    delta: &CanonicalDelta,
    ownership: &[DerivedValueEdit<OwnerKey, OwnershipEntry>],
    removed_relations: &BTreeSet<RelationEdge>,
    added_relations: &BTreeSet<RelationEdge>,
    witness: &mut CachedWitness<'_, W>,
) -> Result<BTreeSet<OwnerKey>, Diagnostic> {
    let candidate_edits = ownership
        .iter()
        .map(|edit| (edit.key, edit.after))
        .collect::<BTreeMap<_, _>>();
    let mut seeds = delta
        .owners
        .keys()
        .chain(ownership.iter().map(|edit| &edit.key))
        .copied()
        .collect::<BTreeSet<_>>();
    seeds.extend(removed_relations.iter().chain(added_relations).filter_map(
        |edge| match edge.source {
            RelationEndpoint::Owner(ExactOwnerKey {
                package: source_package,
                owner,
            }) if source_package == package => Some(owner),
            RelationEndpoint::Owner(_) | RelationEndpoint::Package(_) => None,
        },
    ));
    let mut affected = BTreeSet::new();
    for seed in seeds {
        walk_summary_ancestors(seed, |owner| witness.ownership(owner), &mut affected)?;
        walk_summary_ancestors(
            seed,
            |owner| match candidate_edits.get(&owner) {
                Some(candidate) => Ok(*candidate),
                None => witness.ownership(owner),
            },
            &mut affected,
        )?;
    }
    Ok(affected)
}

fn walk_summary_ancestors(
    seed: OwnerKey,
    mut ownership: impl FnMut(OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic>,
    affected: &mut BTreeSet<OwnerKey>,
) -> Result<(), Diagnostic> {
    let mut current = seed;
    loop {
        affected.insert(current);
        let Some(entry) = ownership(current)? else {
            break;
        };
        if !entry.role.aggregates_into_parent() {
            break;
        }
        let OwnershipParent::Owner(parent) = entry.parent else {
            break;
        };
        current = parent;
    }
    Ok(())
}

struct CachedWitness<'a, W: ?Sized> {
    base: &'a W,
    namespaces: BTreeMap<NamespaceKey, Option<OwnerKey>>,
    ownership: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    relations: BTreeMap<RelationEdge, bool>,
    work: WitnessReadWork,
}

impl<'a, W: WitnessBaseRead + ?Sized> CachedWitness<'a, W> {
    fn new(base: &'a W) -> Self {
        Self {
            base,
            namespaces: BTreeMap::new(),
            ownership: BTreeMap::new(),
            relations: BTreeMap::new(),
            work: WitnessReadWork::default(),
        }
    }

    fn namespace(&mut self, key: &NamespaceKey) -> Result<Option<OwnerKey>, Diagnostic> {
        if !self.namespaces.contains_key(key) {
            let read = self.base.read_namespace(key)?;
            self.work.add(read.work);
            self.namespaces.insert(key.clone(), read.value);
        }
        Ok(self.namespaces.get(key).copied().flatten())
    }

    fn ownership(&mut self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        if !self.ownership.contains_key(&owner) {
            let read = self.base.read_ownership(owner)?;
            self.work.add(read.work);
            self.ownership.insert(owner, read.value);
        }
        Ok(self.ownership.get(&owner).copied().flatten())
    }

    fn contains_relation(&mut self, edge: RelationEdge) -> Result<bool, Diagnostic> {
        if !self.relations.contains_key(&edge) {
            let read = self.base.contains_forward_relation(edge)?;
            self.work.add(read.work);
            self.relations.insert(edge, read.value);
        }
        Ok(self.relations.get(&edge).copied().unwrap_or(false))
    }
}

fn derived_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

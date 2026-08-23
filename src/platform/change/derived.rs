//! Locally derived witness deltas from changed canonical owner records.

use super::{CanonicalDelta, KernelOverlay};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    OwnerKey, OwnerRecord, RelationEdge, RelationEndpoint, RelationKind, extract_owner_relations,
    owner_namespace,
};
use crate::platform::witness::{
    FullWitness, NamespaceKey, OwnershipEntry, OwnershipParent, ownership_contributions,
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
}

pub fn derive_local_delta(
    base: &crate::platform::kernel::KernelSnapshot,
    overlay: &KernelOverlay<'_>,
    delta: &CanonicalDelta,
    base_witness: &FullWitness,
) -> Result<DerivedDelta, Diagnostic> {
    if base_witness.manifest.repository_id != base.root.repository_id
        || base_witness.manifest.package_id != base.root.package_id
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

    for (owner, edit) in &delta.owners {
        if edit.before.is_some() {
            let record = base.owners.get(owner).ok_or_else(|| {
                derived_error(
                    DiagnosticClass::Corrupt,
                    "change_delta_before_owner",
                    "canonical owner delta names a missing before record",
                )
            })?;
            insert_namespace_contribution(&mut before_namespaces, record)?;
            insert_ownership_contributions(&mut before_ownership, record)?;
            before_relations.extend(extract_owner_relations(
                base.root.package_id,
                *owner,
                record,
                |digest| base.types.get(&digest).cloned(),
            )?);
        }
        if let Some((_, record)) = &edit.after {
            insert_namespace_contribution(&mut after_namespaces, record)?;
            insert_ownership_contributions(&mut after_ownership, record)?;
            after_relations.extend(extract_owner_relations(
                base.root.package_id,
                *owner,
                record,
                |digest| overlay.type_object(digest).cloned(),
            )?);
        }
    }

    for (package, edit) in &delta.dependencies {
        if edit.before.is_some() {
            before_relations.insert(package_dependency(base.root.package_id, *package));
        }
        if edit.after.is_some() {
            after_relations.insert(package_dependency(base.root.package_id, *package));
        }
    }

    let namespaces = contribution_edits(
        &base_witness.entries.namespaces,
        &before_namespaces,
        &after_namespaces,
        "namespace",
    )?;
    let ownership = contribution_edits(
        &base_witness.entries.ownership,
        &before_ownership,
        &after_ownership,
        "ownership",
    )?;
    let removed = before_relations
        .difference(&after_relations)
        .copied()
        .collect::<BTreeSet<_>>();
    let added = after_relations
        .difference(&before_relations)
        .copied()
        .collect::<BTreeSet<_>>();
    for edge in &removed {
        if base_witness.entries.relations.binary_search(edge).is_err() {
            return Err(derived_error(
                DiagnosticClass::Corrupt,
                "change_relation_before",
                "locally removed relation is absent from the base witness",
            ));
        }
    }

    let summary_candidates = summary_candidates(delta, &ownership, base_witness);
    Ok(DerivedDelta {
        namespaces,
        ownership,
        relations: RelationDelta { removed, added },
        summary_candidates,
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
    base: &BTreeMap<K, V>,
    before: &BTreeMap<K, V>,
    after: &BTreeMap<K, V>,
    label: &str,
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
        let observed = base.get(&key).cloned();
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

fn summary_candidates(
    delta: &CanonicalDelta,
    ownership: &[DerivedValueEdit<OwnerKey, OwnershipEntry>],
    witness: &FullWitness,
) -> BTreeSet<OwnerKey> {
    let candidate_edits = ownership
        .iter()
        .map(|edit| (edit.key, edit.after))
        .collect::<BTreeMap<_, _>>();
    let seeds = delta
        .owners
        .keys()
        .chain(ownership.iter().map(|edit| &edit.key))
        .copied()
        .collect::<BTreeSet<_>>();
    let mut affected = BTreeSet::new();
    for seed in seeds {
        walk_summary_ancestors(
            seed,
            |owner| witness.entries.ownership.get(&owner).copied(),
            &mut affected,
        );
        walk_summary_ancestors(
            seed,
            |owner| match candidate_edits.get(&owner) {
                Some(candidate) => *candidate,
                None => witness.entries.ownership.get(&owner).copied(),
            },
            &mut affected,
        );
    }
    affected
}

fn walk_summary_ancestors(
    seed: OwnerKey,
    mut ownership: impl FnMut(OwnerKey) -> Option<OwnershipEntry>,
    affected: &mut BTreeSet<OwnerKey>,
) {
    let mut current = seed;
    loop {
        affected.insert(current);
        let Some(entry) = ownership(current) else {
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
}

fn derived_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

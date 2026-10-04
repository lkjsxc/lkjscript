//! Bounded semantic observations that determine one authored proposal's intent.
//!
//! Validation reads are deliberately renewed against the publication base. Only reads made
//! while decoding and lowering the original proposal belong to this conflict footprint.

use super::{
    BoundOwnerSummary, BudgetedCanonicalBase, BudgetedWitnessBase, CanonicalBaseRead,
    CanonicalReadWork, ChangeBudget, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, EncodedOwnerKey, OwnerKey, OwnerRecord, PackageId, PackageInterfaceRecord,
    PackageRevisionDigest, RelationEdge, RelationKind, RetirementRecord, encode_dependency,
    encode_owner, encode_retirement,
};
use crate::platform::semantic_id::RevisionId;
use crate::platform::witness::{
    NamespaceKey, OwnershipEntry, TestDependency, encode_ownership, forward_relation_key,
    reverse_relation_key, test_dependency_keys,
};
use std::collections::BTreeMap;

pub const MAXIMUM_REFRESH_GUARDS: usize = 1_000_000;
pub const MAXIMUM_REFRESH_GUARD_BYTES: usize = 64 * 1_048_576;
const FOOTPRINT_DOMAIN: &str = "lkjscript.authored-intent-footprint.v1";
const VALUE_DOMAIN: &str = "lkjscript.authored-intent-observation.v1";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ReadKey {
    Owner(OwnerKey),
    Dependency(PackageId),
    Retirement(OwnerKey),
    Namespace(NamespaceKey),
    Ownership(OwnerKey),
    Summary(OwnerKey),
    Relation(RelationEdge),
    Relations {
        owner: OwnerKey,
        kind: Option<RelationKind>,
        incoming: bool,
        maximum: usize,
    },
    PackageRelations {
        package: PackageId,
        maximum: usize,
    },
    TestDependencies {
        test: OwnerKey,
        maximum: usize,
    },
    Interface {
        package: PackageId,
        revision: PackageRevisionDigest,
        semantic: RevisionId,
        graph: u16,
    },
    InterfaceOwner {
        package: PackageId,
        revision: PackageRevisionDigest,
        semantic: RevisionId,
        graph: u16,
        owner: OwnerKey,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Observation {
    present: bool,
    digest: [u8; 32],
    items: u64,
}

/// Exact positive, negative and complete-range observations from one admitted proposal.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthoredReadFootprint {
    guards: BTreeMap<ReadKey, Observation>,
    bytes: usize,
}

impl AuthoredReadFootprint {
    pub fn len(&self) -> usize {
        self.guards.len()
    }
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    pub fn digest(&self) -> [u8; 32] {
        let mut hash = blake3::Hasher::new_derive_key(FOOTPRINT_DOMAIN);
        hash.update(&(self.guards.len() as u64).to_be_bytes());
        for (key, value) in &self.guards {
            let key = key.bytes();
            hash.update(&(key.len() as u64).to_be_bytes());
            hash.update(&key);
            hash.update(&[u8::from(value.present)]);
            hash.update(&value.digest);
            hash.update(&value.items.to_be_bytes());
        }
        *hash.finalize().as_bytes()
    }

    pub fn merge(&mut self, other: &Self) -> Result<(), Diagnostic> {
        // Admit the complete union before any mutation, including all duplicate observations.
        let mut records = self.len();
        let mut bytes = self.bytes;
        for (key, observed) in &other.guards {
            if let Some(previous) = self.guards.get(key) {
                if previous != observed {
                    return Err(inconsistent(key));
                }
            } else {
                records = records.checked_add(1).ok_or_else(capacity)?;
                bytes = bytes.checked_add(key.size()).ok_or_else(capacity)?;
                admit(records, bytes)?;
            }
        }
        self.guards.extend(
            other
                .guards
                .iter()
                .map(|(key, value)| (key.clone(), *value)),
        );
        self.bytes = bytes;
        Ok(())
    }

    fn insert(&mut self, key: ReadKey, observed: Observation) -> Result<(), Diagnostic> {
        if let Some(previous) = self.guards.get(&key) {
            return if *previous == observed {
                Ok(())
            } else {
                Err(inconsistent(&key))
            };
        }
        let bytes = self.bytes.checked_add(key.size()).ok_or_else(capacity)?;
        admit(self.len().checked_add(1).ok_or_else(capacity)?, bytes)?;
        self.guards.insert(key, observed);
        self.bytes = bytes;
        Ok(())
    }

    pub fn record_owner(
        &mut self,
        owner: OwnerKey,
        value: Option<&OwnerRecord>,
    ) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Owner(owner),
            optional(
                value
                    .map(|record| encode_owner(record).map(|(digest, _)| digest.bytes()))
                    .transpose()?,
            ),
        )
    }
    pub(crate) fn record_owner_digest(
        &mut self,
        owner: OwnerKey,
        value: Option<[u8; 32]>,
    ) -> Result<(), Diagnostic> {
        self.insert(ReadKey::Owner(owner), optional(value))
    }
    pub(crate) fn record_dependency_digest(
        &mut self,
        package: PackageId,
        value: Option<[u8; 32]>,
    ) -> Result<(), Diagnostic> {
        self.insert(ReadKey::Dependency(package), optional(value))
    }
    pub(crate) fn record_retirement_digest(
        &mut self,
        owner: OwnerKey,
        value: Option<[u8; 32]>,
    ) -> Result<(), Diagnostic> {
        self.insert(ReadKey::Retirement(owner), optional(value))
    }
    pub fn record_dependency(
        &mut self,
        package: PackageId,
        value: Option<&DependencyRecord>,
    ) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Dependency(package),
            optional(
                value
                    .map(|record| encode_dependency(record).map(|(digest, _)| digest.bytes()))
                    .transpose()?,
            ),
        )
    }
    pub fn record_retirement(
        &mut self,
        owner: OwnerKey,
        value: Option<&RetirementRecord>,
    ) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Retirement(owner),
            optional(
                value
                    .map(|record| encode_retirement(record).map(|(digest, _)| digest.bytes()))
                    .transpose()?,
            ),
        )
    }
    pub fn record_namespace(
        &mut self,
        key: &NamespaceKey,
        value: Option<OwnerKey>,
    ) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Namespace(key.clone()),
            optional(value.map(|owner| hash_value(&EncodedOwnerKey::new(owner).bytes()))),
        )
    }
    pub fn record_ownership(
        &mut self,
        owner: OwnerKey,
        value: Option<&OwnershipEntry>,
    ) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Ownership(owner),
            optional(
                value
                    .map(|entry| encode_ownership(entry).map(|bytes| hash_value(&bytes)))
                    .transpose()?,
            ),
        )
    }
    pub fn record_summary(
        &mut self,
        owner: OwnerKey,
        value: Option<&BoundOwnerSummary>,
    ) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Summary(owner),
            optional(value.map(|value| value.digest.bytes())),
        )
    }
    pub fn record_relation(&mut self, edge: RelationEdge, value: bool) -> Result<(), Diagnostic> {
        self.insert(
            ReadKey::Relation(edge),
            optional(value.then(|| hash_value(&forward_relation_key(edge)))),
        )
    }
    pub fn record_relations(
        &mut self,
        owner: OwnerKey,
        kind: Option<RelationKind>,
        incoming: bool,
        maximum_items: usize,
        edges: &[RelationEdge],
        truncated: bool,
    ) -> Result<(), Diagnostic> {
        complete(truncated)?;
        self.insert(
            ReadKey::Relations {
                owner,
                kind,
                incoming,
                maximum: maximum_items,
            },
            range_observation(edges.iter().map(|edge| {
                if incoming {
                    reverse_relation_key(*edge)
                } else {
                    forward_relation_key(*edge)
                }
            }))?,
        )
    }
    pub fn record_package_relations(
        &mut self,
        package: PackageId,
        maximum_items: usize,
        edges: &[RelationEdge],
        truncated: bool,
    ) -> Result<(), Diagnostic> {
        complete(truncated)?;
        self.insert(
            ReadKey::PackageRelations {
                package,
                maximum: maximum_items,
            },
            range_observation(edges.iter().map(|edge| reverse_relation_key(*edge)))?,
        )
    }
    pub fn record_test_dependencies(
        &mut self,
        test: OwnerKey,
        maximum_items: usize,
        values: &[TestDependency],
        truncated: bool,
    ) -> Result<(), Diagnostic> {
        complete(truncated)?;
        self.insert(
            ReadKey::TestDependencies {
                test,
                maximum: maximum_items,
            },
            range_observation(
                values
                    .iter()
                    .map(|value| test_dependency_keys(*value)[0].clone()),
            )?,
        )
    }
    pub fn record_interface(
        &mut self,
        dependency: &DependencyRecord,
        digest: [u8; 32],
    ) -> Result<(), Diagnostic> {
        self.insert(ReadKey::interface(dependency, None), optional(Some(digest)))
    }
    pub fn record_interface_owner(
        &mut self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
        value: Option<&PackageInterfaceRecord>,
    ) -> Result<(), Diagnostic> {
        let value = value
            .map(|record| {
                bincode::encode_to_vec(
                    record,
                    bincode::config::standard()
                        .with_little_endian()
                        .with_variable_int_encoding(),
                )
                .map(|bytes| hash_value(&bytes))
                .map_err(|error| {
                    Diagnostic::new(
                        DiagnosticClass::Infrastructure,
                        "change_refresh_interface_encoding",
                        format!("cannot encode admitted interface observation: {error}"),
                    )
                })
            })
            .transpose()?;
        self.insert(ReadKey::interface(dependency, Some(owner)), optional(value))
    }

    /// Recheck every original intent observation against an independently admitted target view.
    pub fn check_against<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
        &self,
        canonical: &B,
        witness: &W,
        budget: ChangeBudget,
    ) -> Result<(), Diagnostic> {
        self.check_against_with_prior(
            canonical,
            witness,
            budget,
            super::ChangeBudgetWork::default(),
        )
        .map(|_| ())
    }

    pub(crate) fn check_against_with_prior<
        B: CanonicalBaseRead + ?Sized,
        W: WitnessBaseRead + ?Sized,
    >(
        &self,
        canonical: &B,
        witness: &W,
        budget: ChangeBudget,
        mut prior: super::ChangeBudgetWork,
    ) -> Result<super::ChangeBudgetWork, Diagnostic> {
        budget.check_observed(prior, "reviewed refresh original preparation")?;
        let canonical =
            BudgetedCanonicalBase::new(canonical, budget.canonical_reads, prior.canonical_reads)?;
        let witness = BudgetedWitnessBase::new(witness, budget.witness_reads, prior.witness_reads)?;
        let mut observed = Self::default();
        for (key, expected) in &self.guards {
            canonical.validation_checkpoint()?;
            match key {
                ReadKey::Owner(owner) => {
                    observed.record_owner(*owner, canonical.read_owner(*owner)?.value.as_ref())?
                }
                ReadKey::Dependency(package) => observed.record_dependency(
                    *package,
                    canonical.read_dependency(*package)?.value.as_ref(),
                )?,
                ReadKey::Retirement(owner) => observed
                    .record_retirement(*owner, canonical.read_retirement(*owner)?.value.as_ref())?,
                ReadKey::Namespace(key) => {
                    observed.record_namespace(key, witness.read_namespace(key)?.value)?
                }
                ReadKey::Ownership(owner) => observed
                    .record_ownership(*owner, witness.read_ownership(*owner)?.value.as_ref())?,
                ReadKey::Summary(owner) => observed
                    .record_summary(*owner, witness.read_owner_summary(*owner)?.value.as_ref())?,
                ReadKey::Relation(edge) => observed
                    .record_relation(*edge, witness.contains_forward_relation(*edge)?.value)?,
                ReadKey::Relations {
                    owner,
                    kind,
                    incoming,
                    maximum,
                } => {
                    let admitted =
                        range_admission(*maximum, expected.items, prior.relation_edges, budget)?;
                    let read = match (*incoming, *kind) {
                        (false, None) => witness.read_outgoing_relations(*owner, admitted)?,
                        (true, None) => witness.read_incoming_relations(*owner, admitted)?,
                        (true, Some(kind)) => {
                            witness.read_incoming_relations_of_kind(*owner, kind, admitted)?
                        }
                        (false, Some(_)) => return Err(inconsistent(key)),
                    };
                    charge_range(&mut prior, read.value.edges.len(), budget)?;
                    if read.value.truncated {
                        return Err(conflict(key));
                    }
                    observed.record_relations(
                        *owner,
                        *kind,
                        *incoming,
                        *maximum,
                        &read.value.edges,
                        false,
                    )?;
                }
                ReadKey::PackageRelations { package, maximum } => {
                    let admitted =
                        range_admission(*maximum, expected.items, prior.relation_edges, budget)?;
                    let read = witness.read_incoming_package_relations(*package, admitted)?;
                    charge_range(&mut prior, read.value.edges.len(), budget)?;
                    if read.value.truncated {
                        return Err(conflict(key));
                    }
                    observed.record_package_relations(
                        *package,
                        *maximum,
                        &read.value.edges,
                        false,
                    )?;
                }
                ReadKey::TestDependencies { test, maximum } => {
                    let admitted =
                        range_admission(*maximum, expected.items, prior.relation_edges, budget)?;
                    let read = witness.read_test_dependencies(*test, admitted)?;
                    charge_range(&mut prior, read.value.dependencies.len(), budget)?;
                    if read.value.truncated {
                        return Err(conflict(key));
                    }
                    observed.record_test_dependencies(
                        *test,
                        *maximum,
                        &read.value.dependencies,
                        false,
                    )?;
                }
                ReadKey::Interface { .. } => {
                    let dependency = key.dependency().ok_or_else(|| inconsistent(key))?;
                    let read = canonical.read_reference_interface(&dependency)?;
                    observed
                        .record_interface(&dependency, read.value.revision.interface.bytes())?;
                }
                ReadKey::InterfaceOwner { owner, .. } => {
                    let dependency = key.dependency().ok_or_else(|| inconsistent(key))?;
                    let read = canonical.read_package_interface_owner(&dependency, *owner)?;
                    observed.record_interface_owner(&dependency, *owner, read.value.as_ref())?;
                }
            }
            if observed.guards.get(key) != Some(expected) {
                return Err(conflict(key));
            }
        }
        prior.canonical_reads = canonical.work();
        prior.witness_reads = witness.work();
        budget.check_observed(prior, "reviewed refresh intent guards")?;
        Ok(prior)
    }
}

impl ReadKey {
    fn interface(dependency: &DependencyRecord, owner: Option<OwnerKey>) -> Self {
        let DependencyRecord {
            graph_contract_version: graph,
            package,
            semantic_revision: semantic,
            package_revision: revision,
        } = *dependency;
        match owner {
            None => Self::Interface {
                package,
                revision,
                semantic,
                graph,
            },
            Some(owner) => Self::InterfaceOwner {
                package,
                revision,
                semantic,
                graph,
                owner,
            },
        }
    }
    fn dependency(&self) -> Option<DependencyRecord> {
        match self {
            Self::Interface {
                package,
                revision,
                semantic,
                graph,
            }
            | Self::InterfaceOwner {
                package,
                revision,
                semantic,
                graph,
                ..
            } => Some(DependencyRecord {
                graph_contract_version: *graph,
                package: *package,
                semantic_revision: *semantic,
                package_revision: *revision,
            }),
            _ => None,
        }
    }
    fn size(&self) -> usize {
        self.bytes().len() + std::mem::size_of::<Self>() + std::mem::size_of::<Observation>()
    }
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            Self::Owner(owner)
            | Self::Retirement(owner)
            | Self::Ownership(owner)
            | Self::Summary(owner) => {
                bytes.push(match self {
                    Self::Owner(_) => 1,
                    Self::Retirement(_) => 3,
                    Self::Ownership(_) => 5,
                    _ => 6,
                });
                bytes.extend_from_slice(&EncodedOwnerKey::new(*owner).bytes());
            }
            Self::Dependency(package) => {
                bytes.push(2);
                bytes.extend_from_slice(&package.bytes());
            }
            Self::Namespace(key) => {
                bytes.push(4);
                bytes.extend_from_slice(&key.encode());
            }
            Self::Relation(edge) => {
                bytes.push(7);
                bytes.extend_from_slice(&forward_relation_key(*edge));
            }
            Self::Relations {
                owner,
                kind,
                incoming,
                maximum,
            } => {
                bytes.push(8);
                bytes.extend_from_slice(&EncodedOwnerKey::new(*owner).bytes());
                bytes.push(kind.map_or(0, |kind| kind.tag()));
                bytes.push(u8::from(*incoming));
                bytes.extend_from_slice(&(*maximum as u64).to_be_bytes());
            }
            Self::PackageRelations { package, maximum } => {
                bytes.push(9);
                bytes.extend_from_slice(&package.bytes());
                bytes.extend_from_slice(&(*maximum as u64).to_be_bytes());
            }
            Self::TestDependencies { test, maximum } => {
                bytes.push(10);
                bytes.extend_from_slice(&EncodedOwnerKey::new(*test).bytes());
                bytes.extend_from_slice(&(*maximum as u64).to_be_bytes());
            }
            Self::Interface {
                package,
                revision,
                semantic,
                graph,
            }
            | Self::InterfaceOwner {
                package,
                revision,
                semantic,
                graph,
                ..
            } => {
                bytes.push(if matches!(self, Self::Interface { .. }) {
                    11
                } else {
                    12
                });
                bytes.extend_from_slice(&package.bytes());
                bytes.extend_from_slice(&revision.bytes());
                bytes.extend_from_slice(&semantic.bytes());
                bytes.extend_from_slice(&graph.to_be_bytes());
                if let Self::InterfaceOwner { owner, .. } = self {
                    bytes.extend_from_slice(&EncodedOwnerKey::new(*owner).bytes());
                }
            }
        }
        bytes
    }
}

fn hash_value(bytes: &[u8]) -> [u8; 32] {
    *blake3::Hasher::new_derive_key(VALUE_DOMAIN)
        .update(bytes)
        .finalize()
        .as_bytes()
}
fn optional(value: Option<[u8; 32]>) -> Observation {
    Observation {
        present: value.is_some(),
        digest: value.unwrap_or([0; 32]),
        items: 0,
    }
}
fn range_observation(values: impl Iterator<Item = Vec<u8>>) -> Result<Observation, Diagnostic> {
    let (minimum, maximum) = values.size_hint();
    let maximum = maximum
        .filter(|maximum| *maximum == minimum)
        .ok_or_else(capacity)?;
    if maximum > MAXIMUM_REFRESH_GUARDS {
        return Err(capacity());
    }
    let mut bytes = maximum
        .checked_mul(std::mem::size_of::<Vec<u8>>())
        .ok_or_else(capacity)?;
    if bytes > MAXIMUM_REFRESH_GUARD_BYTES {
        return Err(capacity());
    }
    let mut keys = Vec::new();
    keys.try_reserve_exact(maximum).map_err(|_| capacity())?;
    for value in values {
        bytes = bytes.checked_add(value.len()).ok_or_else(capacity)?;
        if bytes > MAXIMUM_REFRESH_GUARD_BYTES {
            return Err(capacity());
        }
        keys.push(value);
    }
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Diagnostic::new(
            DiagnosticClass::Corrupt,
            "change_refresh_footprint_inconsistent",
            "complete intent range repeats a semantic key",
        ));
    }
    let mut hasher = blake3::Hasher::new_derive_key(VALUE_DOMAIN);
    let count = keys.len() as u64;
    for value in keys {
        hasher.update(&(value.len() as u64).to_be_bytes());
        hasher.update(&value);
    }
    hasher.update(&count.to_be_bytes());
    Ok(Observation {
        present: true,
        digest: *hasher.finalize().as_bytes(),
        items: count,
    })
}
fn range_admission(
    maximum: usize,
    expected: u64,
    previous: u64,
    budget: ChangeBudget,
) -> Result<usize, Diagnostic> {
    let remaining = budget
        .impact
        .maximum_relation_edges
        .checked_sub(previous)
        .ok_or_else(range_capacity)?;
    if expected > remaining {
        return Err(range_capacity());
    }
    // A zero-result existence probe retains at most one sentinel under the independent
    // witness/object-read admissions. Empty prefixes traverse zero semantic edges; any
    // observed sentinel is charged and rejected before footprint encoding or acceptance.
    if remaining == 0 {
        return Ok(1);
    }
    Ok(maximum.min(usize::try_from(remaining).unwrap_or(usize::MAX)))
}
fn charge_range(
    prior: &mut super::ChangeBudgetWork,
    items: usize,
    budget: ChangeBudget,
) -> Result<(), Diagnostic> {
    prior.relation_edges = prior
        .relation_edges
        .checked_add(items as u64)
        .ok_or_else(range_capacity)?;
    budget.check_observed(*prior, "reviewed refresh range traversal")
}
fn range_capacity() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "change_budget_relation_edges",
        "reviewed refresh range guards exceed the aggregate relation traversal allowance",
    )
}
fn complete(truncated: bool) -> Result<(), Diagnostic> {
    if truncated {
        Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "change_refresh_incomplete_footprint",
            "a truncated intent range cannot certify a complete conflict footprint",
        ))
    } else {
        Ok(())
    }
}
fn admit(records: usize, bytes: usize) -> Result<(), Diagnostic> {
    if records > MAXIMUM_REFRESH_GUARDS || bytes > MAXIMUM_REFRESH_GUARD_BYTES {
        Err(capacity())
    } else {
        Ok(())
    }
}
fn capacity() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "change_refresh_footprint_capacity",
        "authored intent observations exceed complete refresh footprint admission",
    )
}
fn inconsistent(key: &ReadKey) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "change_refresh_footprint_inconsistent",
        format!("one pinned authored intent observed inconsistent values for {key:?}"),
    )
}
fn conflict(key: &ReadKey) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "change_refresh_conflict",
        format!("authored intent observation changed at {key:?}; prepare and review a new request"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::RelationEndpoint;
    use crate::platform::semantic_id::ModuleId;
    use crate::platform::witness::rebuild_full_witness;

    #[test]
    fn complete_range_fingerprint_is_independent_of_returned_order() {
        let base = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&base).unwrap();
        let edges = &witness.entries.relations[..2];
        let owner = OwnerKey::Module(ModuleId::migrate(b"intent-range-order", 0));
        let mut first = AuthoredReadFootprint::default();
        let mut second = AuthoredReadFootprint::default();
        first
            .record_relations(owner, None, true, 2, edges, false)
            .unwrap();
        second
            .record_relations(owner, None, true, 2, &[edges[1], edges[0]], false)
            .unwrap();
        assert_eq!(first.digest(), second.digest());
    }

    #[test]
    fn empty_range_guard_fits_zero_edges_and_new_edge_exhausts_before_hashing() {
        let base = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&base).unwrap();
        let mut budget = ChangeBudget::default();
        budget.impact.maximum_relation_edges = 0;
        let absent = OwnerKey::Module(ModuleId::migrate(b"intent-range-empty", 0));
        let mut empty = AuthoredReadFootprint::default();
        empty
            .record_relations(absent, None, true, 1, &[], false)
            .unwrap();
        empty.check_against(&base, &witness, budget).unwrap();

        let existing = witness
            .entries
            .relations
            .iter()
            .find_map(|edge| match edge.target {
                RelationEndpoint::Owner(owner) if owner.package == base.root.package_id => {
                    Some(owner.owner)
                }
                _ => None,
            })
            .unwrap();
        let mut changed = AuthoredReadFootprint::default();
        changed
            .record_relations(existing, None, true, 1, &[], false)
            .unwrap();
        let error = changed.check_against(&base, &witness, budget).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "change_budget_relation_edges");
    }
}

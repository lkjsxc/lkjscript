//! Read-through candidate authority over one exact base and one canonical delta.

use super::delta::CanonicalDelta;
use crate::platform::kernel::{
    DependencyRecord, KernelSnapshot, OwnerKey, OwnerRecord, PackageId, RetirementRecord,
    TypeObject, TypeObjectDigest,
};
use crate::platform::persistent_map::MapRoot;

pub struct KernelOverlay<'a> {
    base: &'a KernelSnapshot,
    delta: &'a CanonicalDelta,
}

impl<'a> KernelOverlay<'a> {
    pub const fn new(base: &'a KernelSnapshot, delta: &'a CanonicalDelta) -> Self {
        Self { base, delta }
    }

    pub const fn base(&self) -> &'a KernelSnapshot {
        self.base
    }

    pub const fn delta(&self) -> &'a CanonicalDelta {
        self.delta
    }

    pub fn owner(&self, owner: OwnerKey) -> Option<&OwnerRecord> {
        self.delta.owners.get(&owner).map_or_else(
            || self.base.owners.get(&owner),
            |edit| edit.after.as_ref().map(|(_, record)| record),
        )
    }

    pub fn type_object(&self, digest: TypeObjectDigest) -> Option<&TypeObject> {
        self.delta
            .type_additions
            .get(&digest)
            .or_else(|| self.base.types.get(&digest))
    }

    pub fn dependency(&self, package: PackageId) -> Option<&DependencyRecord> {
        self.delta.dependencies.get(&package).map_or_else(
            || self.base.dependencies.get(&package),
            |edit| edit.after.as_ref().map(|(_, record)| record),
        )
    }

    pub fn retirement(&self, owner: OwnerKey) -> Option<&RetirementRecord> {
        self.delta.retirements.get(&owner).map_or_else(
            || self.base.retirements.get(&owner),
            |edit| edit.after.as_ref().map(|(_, record)| record),
        )
    }

    pub fn owner_count(&self) -> usize {
        adjusted_count(self.base.owners.len(), self.delta.owners.values())
    }

    pub fn dependency_count(&self) -> usize {
        adjusted_count(
            self.base.dependencies.len(),
            self.delta.dependencies.values(),
        )
    }

    pub fn retirement_count(&self) -> usize {
        adjusted_count(self.base.retirements.len(), self.delta.retirements.values())
    }

    /// Iterates the candidate owner map in canonical key order without materializing or cloning
    /// unchanged owners.
    pub fn for_each_owner(&self, mut visitor: impl FnMut(OwnerKey, &OwnerRecord)) {
        let mut base = self.base.owners.iter();
        let mut edits = self.delta.owners.iter();
        let mut next_base = base.next();
        let mut next_edit = edits.next();
        loop {
            match (next_base, next_edit) {
                (Some((key, record)), Some((edit_key, _))) if key < edit_key => {
                    visitor(*key, record);
                    next_base = base.next();
                }
                (Some((base_key, _)), Some((key, edit))) if base_key > key => {
                    if let Some((_, record)) = &edit.after {
                        visitor(*key, record);
                    }
                    next_edit = edits.next();
                }
                (Some(_), Some((key, edit))) => {
                    if let Some((_, record)) = &edit.after {
                        visitor(*key, record);
                    }
                    next_base = base.next();
                    next_edit = edits.next();
                }
                (Some((key, record)), None) => {
                    visitor(*key, record);
                    next_base = base.next();
                }
                (None, Some((key, edit))) => {
                    if let Some((_, record)) = &edit.after {
                        visitor(*key, record);
                    }
                    next_edit = edits.next();
                }
                (None, None) => break,
            }
        }
    }

    /// Explicit broad materialization retained only for full-oracle comparison. Map page digests
    /// remain the base placeholders while entry counts are corrected; callers must not publish or
    /// identify this logical test view as canonical repository authority.
    pub fn materialize_logical_oracle(&self) -> KernelSnapshot {
        let mut owners = std::collections::BTreeMap::new();
        self.for_each_owner(|owner, record| {
            owners.insert(owner, record.clone());
        });
        let mut types = self.base.types.clone();
        types.extend(self.delta.type_additions.clone());
        let mut dependencies = self.base.dependencies.clone();
        apply_exact_edits(&mut dependencies, &self.delta.dependencies);
        let mut retirements = self.base.retirements.clone();
        apply_exact_edits(&mut retirements, &self.delta.retirements);
        let mut root = self.base.root.clone();
        root.owners = count_root(root.owners, owners.len());
        root.dependencies = count_root(root.dependencies, dependencies.len());
        root.retirements = count_root(root.retirements, retirements.len());
        KernelSnapshot {
            root,
            owners,
            types,
            blobs: self.base.blobs.clone(),
            dependencies,
            retirements,
        }
    }
}

fn adjusted_count<'a, D: 'a, V: 'a>(
    base: usize,
    edits: impl Iterator<Item = &'a super::delta::ExactEdit<D, V>>,
) -> usize {
    edits.fold(base, |count, edit| match (&edit.before, &edit.after) {
        (None, Some(_)) => count.saturating_add(1),
        (Some(_), None) => count.saturating_sub(1),
        _ => count,
    })
}

fn apply_exact_edits<K: Copy + Ord, D, V: Clone>(
    values: &mut std::collections::BTreeMap<K, V>,
    edits: &std::collections::BTreeMap<K, super::delta::ExactEdit<D, V>>,
) {
    for (key, edit) in edits {
        match &edit.after {
            Some((_, value)) => {
                values.insert(*key, value.clone());
            }
            None => {
                values.remove(key);
            }
        }
    }
}

fn count_root(root: MapRoot, entries: usize) -> MapRoot {
    MapRoot::from_parts(root.page(), entries as u64)
}

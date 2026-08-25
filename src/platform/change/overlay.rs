//! Read-through candidate authority over one exact base and one canonical delta.

use super::{CanonicalBaseRead, CanonicalDelta, CanonicalReadWork};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::{
    DependencyRecord, KernelSnapshot, OwnerKey, OwnerRecord, PackageId, PackageInterfaceRecord,
    PackageRevisionDigest, RetirementRecord, TypeObject, TypeObjectDigest,
};
use crate::platform::persistent_map::MapRoot;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// One candidate semantic view. Unchanged records are loaded by exact key and cached for the
/// lifetime of the view; canonical delta records always take precedence.
pub struct KernelOverlay<'a, B: ?Sized = KernelSnapshot> {
    base: &'a B,
    delta: &'a CanonicalDelta,
    owners: RefCell<BTreeMap<OwnerKey, Option<OwnerRecord>>>,
    types: RefCell<BTreeMap<TypeObjectDigest, Option<TypeObject>>>,
    package_interfaces:
        RefCell<BTreeMap<(PackageRevisionDigest, OwnerKey), Option<PackageInterfaceRecord>>>,
    dependencies: RefCell<BTreeMap<PackageId, Option<DependencyRecord>>>,
    retirements: RefCell<BTreeMap<OwnerKey, Option<RetirementRecord>>>,
    work: RefCell<CanonicalReadWork>,
}

impl<'a, B: CanonicalBaseRead + ?Sized> KernelOverlay<'a, B> {
    pub fn new(base: &'a B, delta: &'a CanonicalDelta) -> Self {
        Self {
            base,
            delta,
            owners: RefCell::new(BTreeMap::new()),
            types: RefCell::new(BTreeMap::new()),
            package_interfaces: RefCell::new(BTreeMap::new()),
            dependencies: RefCell::new(BTreeMap::new()),
            retirements: RefCell::new(BTreeMap::new()),
            work: RefCell::new(CanonicalReadWork::default()),
        }
    }

    pub const fn delta(&self) -> &'a CanonicalDelta {
        self.delta
    }

    pub fn repository_id(&self) -> RepositoryId {
        self.base.repository_id()
    }

    pub fn package_id(&self) -> PackageId {
        self.base.package_id()
    }

    pub fn exact_revision(&self) -> Option<RevisionId> {
        self.base.exact_revision()
    }

    pub fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        match self.delta.owners.get(&owner) {
            Some(edit) => Ok(edit.after.as_ref().map(|(_, record)| record.clone())),
            None => self.base_owner(owner),
        }
    }

    pub fn base_owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        if !self.owners.borrow().contains_key(&owner) {
            let read = self.base.read_owner(owner)?;
            self.work.borrow_mut().add(read.work);
            self.owners.borrow_mut().insert(owner, read.value);
        }
        Ok(self.owners.borrow().get(&owner).cloned().flatten())
    }

    pub fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        if let Some(object) = self.delta.type_additions.get(&digest) {
            return Ok(Some(object.clone()));
        }
        self.base_type_object(digest)
    }

    pub fn base_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        if !self.types.borrow().contains_key(&digest) {
            let read = self.base.read_type_object(digest)?;
            self.work.borrow_mut().add(read.work);
            self.types.borrow_mut().insert(digest, read.value);
        }
        Ok(self.types.borrow().get(&digest).cloned().flatten())
    }

    pub fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        let Some(dependency) = self.dependency(package)? else {
            return Ok(None);
        };
        let key = (dependency.package_revision, owner);
        if !self.package_interfaces.borrow().contains_key(&key) {
            let read = self.base.read_package_interface_owner(&dependency, owner)?;
            self.work.borrow_mut().add(read.work);
            self.package_interfaces.borrow_mut().insert(key, read.value);
        }
        Ok(self
            .package_interfaces
            .borrow()
            .get(&key)
            .cloned()
            .flatten())
    }

    pub fn dependency(&self, package: PackageId) -> Result<Option<DependencyRecord>, Diagnostic> {
        if let Some(edit) = self.delta.dependencies.get(&package) {
            return Ok(edit.after.as_ref().map(|(_, record)| record.clone()));
        }
        if !self.dependencies.borrow().contains_key(&package) {
            let read = self.base.read_dependency(package)?;
            self.work.borrow_mut().add(read.work);
            self.dependencies.borrow_mut().insert(package, read.value);
        }
        Ok(self.dependencies.borrow().get(&package).cloned().flatten())
    }

    pub fn retirement(&self, owner: OwnerKey) -> Result<Option<RetirementRecord>, Diagnostic> {
        if let Some(edit) = self.delta.retirements.get(&owner) {
            return Ok(edit.after.as_ref().map(|(_, record)| record.clone()));
        }
        if !self.retirements.borrow().contains_key(&owner) {
            let read = self.base.read_retirement(owner)?;
            self.work.borrow_mut().add(read.work);
            self.retirements.borrow_mut().insert(owner, read.value);
        }
        Ok(self.retirements.borrow().get(&owner).cloned().flatten())
    }

    pub fn owner_count(&self) -> u64 {
        adjusted_count(self.base.owner_count(), self.delta.owners.values())
    }

    pub fn dependency_count(&self) -> u64 {
        adjusted_count(
            self.base.dependency_count(),
            self.delta.dependencies.values(),
        )
    }

    pub fn retirement_count(&self) -> u64 {
        adjusted_count(
            self.base.retirement_count(),
            self.delta.retirements.values(),
        )
    }

    pub fn work(&self) -> CanonicalReadWork {
        *self.work.borrow()
    }
}

impl<'a> KernelOverlay<'a, KernelSnapshot> {
    /// The full in-memory base remains available only to the independent oracle adapter.
    pub const fn base(&self) -> &'a KernelSnapshot {
        self.base
    }

    /// Iterates the candidate owner map in canonical key order without materializing or cloning
    /// unchanged owners. This broad operation exists only on the in-memory oracle adapter.
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
        let mut owners = BTreeMap::new();
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
            dependency_interfaces: self.base.dependency_interfaces.clone(),
            dependency_types: self.base.dependency_types.clone(),
            blobs: self.base.blobs.clone(),
            dependencies,
            retirements,
        }
    }
}

fn adjusted_count<'a, D: 'a, V: 'a>(
    base: u64,
    edits: impl Iterator<Item = &'a super::delta::ExactEdit<D, V>>,
) -> u64 {
    edits.fold(base, |count, edit| match (&edit.before, &edit.after) {
        (None, Some(_)) => count.saturating_add(1),
        (Some(_), None) => count.saturating_sub(1),
        _ => count,
    })
}

fn apply_exact_edits<K: Copy + Ord, D, V: Clone>(
    values: &mut BTreeMap<K, V>,
    edits: &BTreeMap<K, super::delta::ExactEdit<D, V>>,
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
    MapRoot::from_parts(root.page(), entries as u64, root.content())
}

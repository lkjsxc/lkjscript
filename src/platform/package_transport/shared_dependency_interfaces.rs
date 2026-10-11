//! Admission-local sharing of exact immutable interface records, not validation results.
use super::*;
use std::sync::Arc;

/// The immutable input borrow binds this pool to one exact admitted interface inventory.
/// It cannot be retargeted or carried into another load or publication attempt.
pub(super) struct SharedDependencyInterfaces<'a, 'store, S: ?Sized> {
    interfaces: &'a BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    store: &'a CollectingStore<'store, S>,
    owners: BTreeMap<PackageRevisionDigest, DependencyInterface>,
}

impl<'a, 'store, S: ?Sized> SharedDependencyInterfaces<'a, 'store, S> {
    pub(super) fn new(
        interfaces: &'a BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
        store: &'a CollectingStore<'store, S>,
    ) -> Self {
        Self {
            interfaces,
            store,
            owners: BTreeMap::new(),
        }
    }

    pub(super) fn attach(
        &mut self,
        snapshot: &mut KernelSnapshot,
        revisions: &BTreeMap<PackageRevisionDigest, PackageRevision>,
    ) -> Result<(), Diagnostic> {
        let mut pending = Vec::new();
        for dependency in snapshot.dependencies.values() {
            self.store.charge_visits(1)?;
            pending.push(dependency);
        }
        while let Some(dependency) = pending.pop() {
            let package = dependency.package;
            let binding = DependencyRecord {
                graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                ..dependency.clone()
            };
            if let Some(previous) = snapshot.representation_dependencies.get(&package) {
                if previous != &binding {
                    return Err(corrupt(
                        "package_source_representation_conflict",
                        "one opaque representation closure selects conflicting exact package revisions",
                    ));
                }
                continue;
            }
            // Every queued binding reserved its lookup and handle insertion before growth.
            let revision = revisions.get(&dependency.package_revision).ok_or_else(|| {
                corrupt(
                    "package_source_dependency",
                    "exact dependency revision is unavailable",
                )
            })?;
            revision.matches_dependency(dependency.package_revision, dependency)?;
            for child in &revision.dependencies {
                self.store.charge_visits(1)?;
                if let Some(previous) = snapshot.representation_dependencies.get(&child.package) {
                    let binding = DependencyRecord {
                        graph_contract_version:
                            crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                        ..child.clone()
                    };
                    if previous != &binding {
                        return Err(corrupt(
                            "package_source_representation_conflict",
                            "one opaque representation closure selects conflicting exact package revisions",
                        ));
                    }
                } else {
                    pending.push(child);
                }
            }
            let interface = self
                .interfaces
                .get(&dependency.package_revision)
                .ok_or_else(|| {
                    corrupt(
                        "package_source_dependency",
                        "exact dependency representation interface is unavailable",
                    )
                })?;
            let owners = if let Some(owners) = self.owners.get(&dependency.package_revision) {
                Arc::clone(owners)
            } else {
                // Account the new shared map/catalogue entry even for an empty interface.
                self.store.charge_visits(1)?;
                let mut owners = BTreeMap::new();
                for (key, owner) in &interface.owners {
                    self.store
                        .charge_visits(interface_owner_validation_visits(owner))?;
                    #[cfg(test)]
                    DEPENDENCY_COPY_COUNTS.with(|counts| {
                        let (owners, types) = counts.get();
                        counts.set((owners + 1, types));
                    });
                    owners.insert(*key, owner.record.clone());
                }
                let owners = Arc::new(owners);
                // An incomplete record projection never enters the reuse catalogue.
                self.owners
                    .insert(dependency.package_revision, Arc::clone(&owners));
                owners
            };
            snapshot
                .dependency_interfaces
                .insert(dependency.package_revision, owners);
            // The flattened type map remains snapshot-private. Do not claim that
            // sharing owner records also eliminates these separate type copies.
            for (digest, object) in &interface.type_objects {
                let children = object.child_type_count();
                let effect_atoms = match &object.form {
                    TypeForm::TaskFunction { effect, .. } => effect
                        .requirements
                        .len()
                        .checked_add(effect.parameters.len())
                        .ok_or_else(|| limit("validation visits"))?,
                    _ => 0,
                };
                let copy_visits = children
                    .checked_add(effect_atoms)
                    .and_then(|count| count.checked_add(1))
                    .and_then(|count| u64::try_from(count).ok())
                    .ok_or_else(|| limit("validation visits"))?;
                self.store.charge_visits(copy_visits)?;
                #[cfg(test)]
                DEPENDENCY_COPY_COUNTS.with(|counts| {
                    let (owners, types) = counts.get();
                    counts.set((owners, types + 1));
                });
                snapshot.dependency_types.insert(*digest, object.clone());
            }
            snapshot
                .representation_dependencies
                .insert(package, binding);
        }
        Ok(())
    }
}

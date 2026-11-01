//! Invocation-local reuse of complete transport admission during one exact artifact link.
//! This retains no readiness or semantic proof beyond the borrowing RepositoryView scope.
use super::*;
use std::sync::Arc;

const MAXIMUM_RETAINED_BYTES: u64 = 32 * 1_048_576;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LinkReadReuseObservation {
    pub(crate) transport_misses: u64,
    pub(crate) transport_hits: u64,
    pub(crate) retained_closures: u64,
    pub(crate) capacity_skips: u64,
    pub(crate) estimate_overflows: u64,
    pub(crate) peak_modeled_retained_bytes: u64,
    pub(crate) maximum_miss_physical_bytes: u64,
    pub(crate) maximum_entry_modeled_bytes: u64,
    pub(crate) copy_work: u64,
    pub(crate) rechecked_objects: u64,
    pub(crate) work_steps: u64,
    pub(crate) saturated: bool,
}

#[derive(Debug)]
struct RetainedTransport {
    closure: PackageTransportClosureValidation,
    owner_copy_work: BTreeMap<OwnerKey, u64>,
}

#[derive(Default, Debug)]
pub(super) struct LinkTransportReuse {
    entries: BTreeMap<PackageRevisionDigest, Arc<RetainedTransport>>,
    retained_bytes: u64,
    remaining_work: Option<usize>,
    observation: LinkReadReuseObservation,
}

fn link_work_error() -> Diagnostic {
    read_error(
        DiagnosticClass::Resource,
        "publication_link_read_work",
        "link read reuse exceeds its cumulative validation work allowance",
    )
}

impl LinkTransportReuse {
    fn charge_work(&mut self, count: usize) -> Result<(), Diagnostic> {
        if let Some(remaining) = self.remaining_work {
            self.remaining_work = Some(remaining.checked_sub(count).ok_or_else(link_work_error)?);
        }
        let amount = match u64::try_from(count) {
            Ok(value) => BudgetCounter::exact(value),
            Err(_) => BudgetCounter {
                consumed: u64::MAX,
                saturated: true,
            },
        };
        let mut observed = BudgetCounter {
            consumed: self.observation.work_steps,
            saturated: self.observation.saturated,
        };
        observed.observe(amount);
        self.observation.work_steps = observed.consumed;
        self.observation.saturated |= observed.saturated;
        Ok(())
    }
}

fn observe_reuse_counter(value: &mut u64, amount: u64, saturated: &mut bool) {
    match value.checked_add(amount) {
        Some(next) => *value = next,
        None => {
            *value = u64::MAX;
            *saturated = true;
        }
    }
}

fn consume_final_work(reuse: &mut Option<LinkTransportReuse>) -> Result<(), Diagnostic> {
    if let Some(reuse) = reuse.as_mut() {
        reuse.charge_work(1)?;
    }
    Ok(())
}

struct Scope<'a> {
    view: &'a RepositoryView,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        // Also executes on a propagated error or unwind. A failed/cancelled
        // operation cannot leave either reuse or recording enabled on this view.
        self.view.link_transport_reuse.borrow_mut().take();
        self.view.store.take_link_read_set();
    }
}

impl RepositoryView {
    /// Keeps the original linker and its canonical re-lowering inside one bounded
    /// reuse lifetime. Exact dependency/representation selection remains in each
    /// caller. Success additionally rechecks physical and view-derived objects and
    /// the retained revision-to-transport choices; failures keep their original class.
    pub(crate) fn with_link_read_reuse<T>(
        &self,
        operation: impl FnOnce() -> Result<T, Diagnostic>,
    ) -> Result<(T, RepositoryReadWork, LinkReadReuseObservation), Diagnostic> {
        self.validation_checkpoint()?;
        if self.link_transport_reuse.borrow().is_some() {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_link_read_scope",
                "link read reuse cannot nest or outlive its exact repository view",
            ));
        }
        self.store.begin_link_read_set().map_err(store_diagnostic)?;
        *self.link_transport_reuse.borrow_mut() = Some(LinkTransportReuse::default());
        let scope = Scope { view: self };
        let result = operation();
        // Disable both facilities before final reads. The read-set recheck must
        // inspect the bound store/derived overlay, never satisfy itself from reuse.
        let mut reuse = self.link_transport_reuse.borrow_mut().take();
        let objects = self.store.take_link_read_set();
        let value = result?;
        let mut work = RepositoryReadWork::default();
        let mut admission = StoreReadAdmission::unbounded();
        for key in objects {
            self.validation_checkpoint()?;
            consume_final_work(&mut reuse)?;
            let bytes = self
                .store
                .read_admitted(
                    key,
                    key.domain.maximum_bytes(),
                    &mut admission,
                    &mut work.store,
                )
                .map_err(store_diagnostic)?
                .ok_or_else(|| {
                    read_error(
                        DiagnosticClass::Corrupt,
                        "publication_link_read_object_missing",
                        "reused link admission lost an original canonical or dependency object",
                    )
                })?;
            key.verify(&bytes).map_err(store_diagnostic)?;
            if let Some(reuse) = reuse.as_mut() {
                observe_reuse_counter(
                    &mut reuse.observation.rechecked_objects,
                    1,
                    &mut reuse.observation.saturated,
                );
            }
        }
        if reuse
            .as_ref()
            .is_some_and(|reuse| !reuse.entries.is_empty())
        {
            self.validation_checkpoint()?;
            consume_final_work(&mut reuse)?;
            let reuse = reuse.as_mut().ok_or_else(link_work_error)?;
            let ready =
                super::super::repository::read_store_readiness(self.store.persistent_base())?;
            for closure in reuse.entries.values() {
                for selection in &closure.closure.selections {
                    self.validation_checkpoint()?;
                    // Charge the shared allowance before processing each retained selection.
                    // Split fields below so the immutable entry borrow stays independent.
                    if let Some(remaining) = reuse.remaining_work {
                        reuse.remaining_work =
                            Some(remaining.checked_sub(1).ok_or_else(link_work_error)?);
                    }
                    observe_reuse_counter(
                        &mut reuse.observation.work_steps,
                        1,
                        &mut reuse.observation.saturated,
                    );
                    if ready.bindings.get(&selection.package_revision) != Some(&selection.transport)
                    {
                        return Err(read_error(
                            DiagnosticClass::Semantic,
                            "publication_link_read_selection_changed",
                            "an exact retained package transport selection changed during linking",
                        ));
                    }
                    let (entries, overflow) = work.map.entries_visited.overflowing_add(1);
                    work.map.entries_visited = if overflow { u64::MAX } else { entries };
                    if overflow {
                        work.map
                            .mark_counter_saturated(MapWorkCounter::EntriesVisited);
                    }
                }
            }
        }
        self.validation_checkpoint()?;
        drop(scope);
        let observation = reuse.map(|reuse| reuse.observation).unwrap_or_default();
        Ok((value, work, observation))
    }

    fn link_reuse_step(&self, count: usize) -> Result<(), Diagnostic> {
        self.validation_checkpoint()?;
        if let Some(reuse) = self.link_transport_reuse.borrow_mut().as_mut() {
            reuse.charge_work(count)?;
        }
        Ok(())
    }

    pub(super) fn admit_link_memory_records(
        &self,
        admission: &mut RepositoryReadAdmission,
        work: &mut RepositoryReadWork,
        count: u64,
    ) -> Result<(), Diagnostic> {
        self.link_reuse_step(usize::try_from(count).map_err(|_| {
            read_error(
                DiagnosticClass::Resource,
                "publication_link_read_work",
                "copy work overflowed",
            )
        })?)?;
        // Reserve one returned retained record under the existing logical
        // canonical-record allowance. Variable nested-copy weight is charged
        // above to the cumulative reuse ledger, not reported as N decoded records.
        if let Some(reuse) = self.link_transport_reuse.borrow_mut().as_mut() {
            observe_reuse_counter(
                &mut reuse.observation.copy_work,
                count,
                &mut reuse.observation.saturated,
            );
        }
        admission.admit_canonical_records(1)?;
        work.observe_canonical_records(BudgetCounter::exact(1));
        Ok(())
    }

    pub(super) fn admit_link_lookup(
        &self,
        admission: &mut RepositoryReadAdmission,
        work: &mut RepositoryReadWork,
    ) -> Result<(), Diagnostic> {
        self.link_reuse_step(1)?;
        admission.map.maximum_entries_visited = admission
            .map
            .maximum_entries_visited
            .map(|remaining| {
                remaining.checked_sub(1).ok_or_else(|| {
                    read_error(
                        DiagnosticClass::Resource,
                        "publication_link_read_map_entries",
                        "link read reuse exhausted canonical map-entry admission",
                    )
                })
            })
            .transpose()?;
        let (entries, overflow) = work.map.entries_visited.overflowing_add(1);
        work.map.entries_visited = if overflow { u64::MAX } else { entries };
        if overflow {
            work.map
                .mark_counter_saturated(MapWorkCounter::EntriesVisited);
        }
        Ok(())
    }

    pub(super) fn admit_cached_owner_copy(
        &self,
        revision: PackageRevisionDigest,
        owner: OwnerKey,
        admission: &mut RepositoryReadAdmission,
        work: &mut RepositoryReadWork,
    ) -> Result<(), Diagnostic> {
        self.admit_link_lookup(admission, work)?;
        let weight = self
            .link_transport_reuse
            .borrow()
            .as_ref()
            .and_then(|reuse| reuse.entries.get(&revision))
            .and_then(|entry| entry.owner_copy_work.get(&owner))
            .copied()
            .ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_link_read_copy_model",
                    "retained owner is missing its admitted copy weight",
                )
            })?;
        self.admit_link_memory_records(admission, work, weight)
    }

    /// The cold path is the existing complete resolver. Only a successful selector
    /// and a final checkpoint can publish that complete entry into this invocation.
    pub(super) fn with_link_transport<T>(
        &self,
        revision: PackageRevisionDigest,
        admission: &mut RepositoryReadAdmission,
        select: impl FnOnce(
            &PackageTransportClosureValidation,
            &mut RepositoryReadAdmission,
            &mut RepositoryReadWork,
            bool,
        ) -> Result<T, Diagnostic>,
    ) -> Result<RevisionRead<T>, Diagnostic> {
        self.validation_checkpoint()?;
        let active = self.link_transport_reuse.borrow().is_some();
        let cached = if active {
            // Debit the catalogue lookup before cloning its Arc handle.
            self.link_reuse_step(1)?;
            self.link_transport_reuse
                .borrow()
                .as_ref()
                .and_then(|reuse| reuse.entries.get(&revision).map(Arc::clone))
        } else {
            None
        };
        if let Some(closure) = cached {
            let mut work = RepositoryReadWork::default();
            self.admit_link_lookup(admission, &mut work)?;
            if let Some(reuse) = self.link_transport_reuse.borrow_mut().as_mut() {
                observe_reuse_counter(
                    &mut reuse.observation.transport_hits,
                    1,
                    &mut reuse.observation.saturated,
                );
            }
            let value = select(&closure.closure, admission, &mut work, true)?;
            self.validation_checkpoint()?;
            return Ok(self.read(value, work));
        }
        if let Some(reuse) = self.link_transport_reuse.borrow_mut().as_mut() {
            observe_reuse_counter(
                &mut reuse.observation.transport_misses,
                1,
                &mut reuse.observation.saturated,
            );
        }
        let resolved = self.resolve_package_transport_admitted(revision, admission)?;
        let mut work = resolved.work;
        let value = select(&resolved.value, admission, &mut work, false)?;
        self.validation_checkpoint()?;
        if active {
            // The existing current-validation cache uses the same conservative
            // decoded-layout model: read bytes*8 plus256 bytes per retained record.
            // An oversized entry remains on the original cold path; capacity
            // cannot turn a valid semantic closure into an invalid one.
            let mut records = resolved
                .value
                .root_interface
                .owners
                .len()
                .checked_add(resolved.value.root_interface.type_objects.len());
            for interface in resolved.value.representation_interfaces.values() {
                self.link_reuse_step(1)?;
                records = records
                    .and_then(|n| n.checked_add(interface.owners.len()))
                    .and_then(|n| n.checked_add(interface.type_objects.len()));
            }
            let estimate = records
                .and_then(|n| u64::try_from(n).ok())
                .and_then(|n| n.checked_mul(256))
                .and_then(|n| {
                    work.store
                        .bytes_read
                        .checked_mul(8)
                        .and_then(|bytes| bytes.checked_add(n))
                })
                .and_then(|n| {
                    n.checked_add(std::mem::size_of::<PackageTransportClosureValidation>() as u64)
                })
                .and_then(|n| n.checked_add(64));
            self.link_reuse_step(1)?;
            let retain = {
                let mut cache = self.link_transport_reuse.borrow_mut();
                let reuse = cache.as_mut().ok_or_else(link_work_error)?;
                reuse.observation.maximum_miss_physical_bytes = reuse
                    .observation
                    .maximum_miss_physical_bytes
                    .max(work.store.bytes_read);
                if let Some(estimate) = estimate {
                    reuse.observation.maximum_entry_modeled_bytes =
                        reuse.observation.maximum_entry_modeled_bytes.max(estimate);
                } else {
                    observe_reuse_counter(
                        &mut reuse.observation.estimate_overflows,
                        1,
                        &mut reuse.observation.saturated,
                    );
                }
                estimate
                    .and_then(|n| reuse.retained_bytes.checked_add(n))
                    .filter(|total| {
                        *total <= MAXIMUM_RETAINED_BYTES
                            && reuse.entries.len()
                                < crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE
                    })
            };
            if let Some(total) = retain {
                // Reserve the conservative decoded-layout and cost-map model
                // before any cost-map node or Arc/catalogue allocation grows.
                // Any later failure leaves no entry and the scope guard clears
                // this invocation, including its prepaid reservation.
                {
                    let mut cache = self.link_transport_reuse.borrow_mut();
                    let reuse = cache.as_mut().ok_or_else(link_work_error)?;
                    reuse.retained_bytes = total;
                    reuse.observation.peak_modeled_retained_bytes =
                        reuse.observation.peak_modeled_retained_bytes.max(total);
                }
                // Reuse the exact admitted canonical object length, including its
                // uncompressed envelope. Logical DTO Encode intentionally refuses
                // nonempty associated inventories and is not a wire-size oracle.
                // Each source lookup/read stays on the exact root transport map,
                // under the existing shared admissions and this controlled scope.
                let mut owner_copy_work = BTreeMap::new();
                for owner in resolved.value.root_interface.owners.keys() {
                    self.link_reuse_step(1)?;
                    let binding = self
                        .lookup_map_admitted(
                            resolved.value.root_transport.interface_owners,
                            &owner_map_key(*owner),
                            &mut work,
                            admission,
                        )?
                        .ok_or_else(|| {
                            read_error(
                                DiagnosticClass::Corrupt,
                                "publication_link_read_copy_model",
                                "admitted interface owner has no exact root transport binding",
                            )
                        })?;
                    let digest =
                        crate::platform::package_interface::decode_package_interface_binding(
                            &binding,
                        )?;
                    self.link_reuse_step(1)?;
                    let bytes = self.read_required_object_admitted(
                        ObjectDomain::PackageInterface,
                        digest.bytes(),
                        "admitted interface owner lost its exact canonical bytes",
                        &mut work,
                        admission,
                    )?;
                    self.link_reuse_step(1)?;
                    ObjectKey::from_digest(ObjectDomain::PackageInterface, digest.bytes())
                        .verify(&bytes)
                        .map_err(store_diagnostic)?;
                    let weight = u64::try_from(bytes.len()).map_err(|_| link_work_error())?;
                    // The cost-map node was reserved above. No canonical byte Vec
                    // is retained; only its fixed-size weight accompanies the proof.
                    owner_copy_work.insert(*owner, weight.max(1));
                }
                self.validation_checkpoint()?;
                let mut cache = self.link_transport_reuse.borrow_mut();
                let reuse = cache.as_mut().ok_or_else(link_work_error)?;
                // The prepaid256-byte per-record model includes cost-map nodes;
                // the closure becomes reusable only after all costs succeed.
                observe_reuse_counter(
                    &mut reuse.observation.retained_closures,
                    1,
                    &mut reuse.observation.saturated,
                );
                reuse.entries.insert(
                    revision,
                    Arc::new(RetainedTransport {
                        closure: resolved.value,
                        owner_copy_work,
                    }),
                );
            } else {
                let mut cache = self.link_transport_reuse.borrow_mut();
                let reuse = cache.as_mut().ok_or_else(link_work_error)?;
                observe_reuse_counter(
                    &mut reuse.observation.capacity_skips,
                    1,
                    &mut reuse.observation.saturated,
                );
            }
        }
        if let Some(reuse) = self.link_transport_reuse.borrow_mut().as_mut() {
            // Include canonical copy-weight reads in physical miss observations.
            reuse.observation.maximum_miss_physical_bytes = reuse
                .observation
                .maximum_miss_physical_bytes
                .max(work.store.bytes_read);
        }
        Ok(self.read(value, work))
    }
}

#[cfg(test)]
#[path = "link_read_reuse_tests.rs"]
mod tests;

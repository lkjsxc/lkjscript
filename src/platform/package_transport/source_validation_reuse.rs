//! Complete local semantic proofs for one publication supplier-admission invocation.
//! Canonical source, interfaces and complete composed relations are still read and checked.
use super::*;
use crate::platform::witness::CompletedSourceValidation;

const MAXIMUM_RETAINED_VALIDATION_BYTES: u64 = 8 * 1_048_576;

/// Only this source reader can construct the binding, after independently checking and
/// reconstructing every current canonical input. Exact transports bind raw encoding as well
/// as meaning; sibling packages do not enter a package's own representation inventory.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SourceValidationBinding {
    package: PackageTransportBinding,
    semantic_root: SemanticRootDigest,
    graph_contract_version: u16,
    validator: crate::platform::witness::ValidatorContractDigest,
    representations: Vec<PackageTransportBinding>,
}

impl SourceValidationBinding {
    pub(super) fn from_current_source<S: ?Sized>(
        package: PackageTransportBinding,
        semantic_root: SemanticRootDigest,
        snapshot: &KernelSnapshot,
        selections: &BTreeMap<PackageRevisionDigest, PackageTransportDigest>,
        store: &CollectingStore<'_, S>,
        control: &ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let count = snapshot.representation_dependencies.len();
        let lookup = selection_lookup_work(selections.len());
        let work = u64::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(lookup + 3))
            .and_then(|count| count.checked_add(8))
            .ok_or_else(|| limit("source proof binding work"))?;
        store.charge_visits(work)?;
        let mut representations = Vec::with_capacity(count);
        for dependency in snapshot.representation_dependencies.values() {
            control.check().map_err(execution_diagnostic)?;
            let transport = selections
                .get(&dependency.package_revision)
                .ok_or_else(|| {
                    corrupt(
                        "package_source_validation_selection",
                        "a complete source proof lost an exact representation transport",
                    )
                })?;
            representations.push(PackageTransportBinding {
                package_revision: dependency.package_revision,
                transport: *transport,
            });
        }
        Ok(Self {
            package,
            semantic_root,
            graph_contract_version: snapshot.root.graph_contract_version,
            validator: crate::platform::witness::contract::validator_contract_digest(),
            representations,
        })
    }

    pub(super) fn retained_bytes(&self) -> u64 {
        // Conservatively reserve a whole possible BTree node and a pending token slot per
        // entry, using the actual typed payload size, plus fixed node/Vec metadata. The flat
        // selection Vec reserves exact capacity. No snapshots, pages or nested records remain.
        12 * std::mem::size_of::<(PackageRevisionDigest, CompletedSourceValidation)>() as u64
            + 256
            + self.representations.len() as u64 * 64
    }
}

pub(super) fn selection_lookup_work(count: usize) -> u64 {
    // A whole possible node comparison at each conservatively bounded tree level.
    12 * u64::from(usize::BITS - count.max(1).leading_zeros()) + 2
}

pub(super) struct SourceValidationReuse {
    completed: BTreeMap<PackageRevisionDigest, CompletedSourceValidation>,
    retained_bytes: u64,
    maximum_retained_bytes: u64,
}

impl Default for SourceValidationReuse {
    fn default() -> Self {
        Self {
            completed: BTreeMap::new(),
            retained_bytes: 0,
            maximum_retained_bytes: MAXIMUM_RETAINED_VALIDATION_BYTES,
        }
    }
}

impl SourceValidationReuse {
    pub(super) fn lookup<S: ?Sized>(
        &self,
        binding: &SourceValidationBinding,
        store: &CollectingStore<'_, S>,
        control: &ExecutionControl,
    ) -> Result<Option<&CompletedSourceValidation>, Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        // Reserve the map lookup, both complete equality comparisons (here and at the
        // witness boundary) and the fixed seven-field validation report copy before work.
        store.charge_visits(
            selection_lookup_work(self.completed.len())
                + 24
                + binding.representations.len() as u64 * 4,
        )?;
        Ok(self
            .completed
            .get(&binding.package.package_revision)
            .filter(|proof| proof.source_binding() == binding))
    }

    pub(super) fn can_retain(&self, binding: &SourceValidationBinding, pending_bytes: u64) -> bool {
        self.retained_bytes
            .checked_add(pending_bytes)
            .and_then(|bytes| bytes.checked_add(binding.retained_bytes()))
            .is_some_and(|bytes| bytes <= self.maximum_retained_bytes)
    }

    pub(super) fn reserve_pending<S: ?Sized>(
        &self,
        store: &CollectingStore<'_, S>,
        control: &ExecutionControl,
    ) -> Result<(), Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        // Token mint copies the fixed report and derives a scalar construction reservation;
        // the exact binding is moved, never cloned.
        store.charge_visits(24)
    }

    pub(super) fn reserve_reconstruction<S: ?Sized>(
        &self,
        proof: &CompletedSourceValidation,
        store: &CollectingStore<'_, S>,
        control: &ExecutionControl,
    ) -> Result<(), Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        store.charge_visits(proof.modeled_rebuild_work())
    }

    pub(super) fn commit<S: ?Sized>(
        &mut self,
        pending: Vec<CompletedSourceValidation>,
        store: &CollectingStore<'_, S>,
        control: &ExecutionControl,
    ) -> Result<(), Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let commit_work = pending.len() as u64
            * (selection_lookup_work(self.completed.len() + pending.len()) + 4);
        // Reserve both the complete preflight traversal and all lookup/insertion work before
        // inspection or growth. Check cancellation per entry while mutation is still absent.
        store.charge_visits(commit_work)?;
        let mut pending_bytes = 0_u64;
        for proof in &pending {
            control.check().map_err(execution_diagnostic)?;
            pending_bytes = pending_bytes
                .checked_add(proof.source_binding().retained_bytes())
                .ok_or_else(|| limit("retained source proof bytes"))?;
        }
        if self
            .retained_bytes
            .checked_add(pending_bytes)
            .is_none_or(|bytes| bytes > self.maximum_retained_bytes)
        {
            return Err(limit("retained source proof bytes"));
        }
        control.check().map_err(execution_diagnostic)?;
        // All fallible checks and reservation precede mutation. Failed/cancelled closures
        // cannot install a prefix, and the complete cache drops with its owning session.
        for proof in pending {
            let binding = proof.source_binding();
            let revision = binding.package.package_revision;
            self.retained_bytes += binding.retained_bytes();
            if let Some(previous) = self.completed.insert(revision, proof) {
                self.retained_bytes -= previous.source_binding().retained_bytes();
            }
        }
        Ok(())
    }
}

/// Store and actual control borrows bind the cache to exactly one supplier admission call.
/// It exposes admitted closures only; completed tokens and their bindings never escape.
pub(crate) struct SourceAdmissionSession<'a, S: ?Sized> {
    base: &'a S,
    control: &'a ExecutionControl,
    reuse: SourceValidationReuse,
}

impl<'a, S: ImmutableObjectStore + ?Sized> SourceAdmissionSession<'a, S> {
    pub(crate) fn new(base: &'a S, control: &'a ExecutionControl) -> Self {
        Self {
            base,
            control,
            reuse: SourceValidationReuse::default(),
        }
    }

    pub(crate) fn collect_preplanned_with_budget(
        &mut self,
        prior: super::super::PackageTransportClosureValidation,
        maximum_visits: u64,
        maximum_read_bytes: u64,
    ) -> Result<AdmittedClosure, Diagnostic> {
        let root = PackageTransportBinding {
            package_revision: prior.logical.root,
            transport: prior.root_transport_digest,
        };
        collect_admitted(
            self.base,
            root,
            &prior.selections,
            maximum_visits,
            maximum_read_bytes,
            self.control,
            Some(&prior.logical),
            Some(&mut self.reuse),
        )
        .map_err(|error| source_context(root, error))
    }

    #[cfg(test)]
    pub(super) fn completed_count(&self) -> usize {
        self.reuse.completed.len()
    }

    #[cfg(test)]
    pub(super) fn with_retained_byte_limit(
        base: &'a S,
        control: &'a ExecutionControl,
        maximum_retained_bytes: u64,
    ) -> Self {
        assert!(maximum_retained_bytes <= MAXIMUM_RETAINED_VALIDATION_BYTES);
        let mut session = Self::new(base, control);
        session.reuse.maximum_retained_bytes = maximum_retained_bytes;
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_validation_reuse_warm_witness_reservation_has_exact_boundary() {
        let source = super::super::tests::source_validation_reuse_fixture();
        let control = ExecutionControl::uncancelled();
        let mut session = SourceAdmissionSession::new(&source.container.objects, &control);
        session
            .collect_preplanned_with_budget(
                super::super::tests::source_validation_reuse_plan(&source.container),
                MAXIMUM_VALIDATION_VISITS,
                MAXIMUM_VALIDATION_READ_BYTES,
            )
            .unwrap();
        let completed = session.reuse.completed.values().next().unwrap();
        let required = completed.modeled_rebuild_work();
        assert!(required > 0);
        for (maximum, succeeds) in [(required, true), (required - 1, false)] {
            let store = CollectingStore {
                base: &source.container.objects,
                admission: RefCell::new(StoreReadAdmission::new(StoreReadLimits {
                    maximum_catalog_lookups: Some(maximum),
                    maximum_objects: Some(maximum),
                    maximum_bytes: Some(MAXIMUM_VALIDATION_READ_BYTES),
                })),
                objects: RefCell::new(BTreeMap::new()),
                retained_bytes: std::cell::Cell::new(84),
                maximum_visits: maximum,
                semantic_visits: std::cell::Cell::new(0),
            };
            let result = session
                .reuse
                .reserve_reconstruction(completed, &store, &control);
            assert_eq!(result.is_ok(), succeeds);
            assert_eq!(store.visits(), if succeeds { required } else { 0 });
            if let Err(error) = result {
                assert_eq!(error.class, DiagnosticClass::Resource);
                assert_eq!(error.code, "package_source_budget");
            }
        }
    }
}

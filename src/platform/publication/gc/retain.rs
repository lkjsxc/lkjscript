//! Operation-local canonical reads. No accepted validation cache supplies reachability.

use super::*;

const OBJECT_OVERHEAD: u64 = 256;

pub(super) struct Retained<'a, S: ?Sized> {
    base: &'a S,
    objects: RefCell<BTreeMap<ObjectKey, Vec<u8>>>,
    observation: Cell<PreparationObservation>,
    control: &'a ExecutionControl,
}

impl<'a, S: ImmutableObjectStore + ?Sized> Retained<'a, S> {
    pub(super) fn new(base: &'a S, control: &'a ExecutionControl) -> Self {
        Self {
            base,
            objects: RefCell::new(BTreeMap::new()),
            observation: Cell::new(PreparationObservation::default()),
            control,
        }
    }

    pub(super) fn required(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
    ) -> Result<Vec<u8>, Diagnostic> {
        checkpoint(self.control)?;
        self.read(
            ObjectKey::from_digest(domain, digest),
            domain.maximum_bytes(),
            &mut StoreWork::default(),
        )
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            gc_error(
                DiagnosticClass::Corrupt,
                "gc_missing_object",
                format!("reachable {} object is missing", domain.name()),
            )
        })
    }

    pub(super) fn map(
        &self,
        root: MapRoot,
        mut visit: impl FnMut(&[u8], &[u8]) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        checkpoint(self.control)?;
        let reader = ObjectPageReader::new(self);
        let mut error = None;
        let result = PersistentMap::from_root(root).for_each(
            &reader,
            &mut MapWork::default(),
            |key, value| {
                if let Err(failure) = checkpoint(self.control).and_then(|()| visit(key, value)) {
                    error = Some(failure);
                    return Err(MapError {
                        class: MapErrorClass::Store,
                        code: "gc_map_visit",
                        message: "reachable map visit failed".into(),
                    });
                }
                Ok(())
            },
        );
        if let Some(error) = error {
            return Err(error);
        }
        result.map_err(|error| {
            gc_error(
                match error.class {
                    MapErrorClass::Resource => DiagnosticClass::Resource,
                    MapErrorClass::Store => DiagnosticClass::Infrastructure,
                    _ => DiagnosticClass::Corrupt,
                },
                error.code,
                error.message,
            )
        })
    }

    pub(super) fn observation(&self) -> PreparationObservation {
        self.observation.get()
    }

    fn read_inner(
        &self,
        key: ObjectKey,
        maximum: Option<usize>,
        external: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let mut observed = StoreWork::default();
        let result = match external {
            Some(admission) => self
                .base
                .read_admitted(key, maximum, admission, &mut observed),
            None => self.base.read(key, maximum, &mut observed),
        };
        work.add(observed);
        let mut observation = self.observation.get();
        observation.saturated |= observed.saturated;
        observe(
            &mut observation.catalog_lookups,
            observed.catalog_lookups,
            &mut observation.saturated,
        );
        observe(
            &mut observation.objects_read,
            observed.objects_read,
            &mut observation.saturated,
        );
        observe(
            &mut observation.bytes_read,
            observed.bytes_read,
            &mut observation.saturated,
        );
        self.observation.set(observation);
        let bytes = result?;
        if let Some(bytes) = &bytes {
            key.verify(bytes)?;
            let mut objects = self.objects.borrow_mut();
            if let Some(previous) = objects.get(&key) {
                if previous != bytes {
                    return Err(StoreError::new(
                        StoreErrorClass::Corrupt,
                        "gc_changed_object",
                        "reachable canonical bytes changed during compaction",
                    ));
                }
            } else {
                let mut retained = Vec::new();
                retained.try_reserve_exact(bytes.len()).map_err(|_| {
                    StoreError::new(
                        StoreErrorClass::Resource,
                        "gc_retention_allocation",
                        "cannot allocate reachable canonical object retention",
                    )
                })?;
                retained.extend_from_slice(bytes);
                objects.insert(key, retained);
                observe(
                    &mut observation.retained_modeled_bytes,
                    bytes.len() as u64,
                    &mut observation.saturated,
                );
                observe(
                    &mut observation.retained_modeled_bytes,
                    OBJECT_OVERHEAD,
                    &mut observation.saturated,
                );
                self.observation.set(observation);
            }
        }
        Ok(bytes)
    }

    pub(super) fn finish(self) -> BTreeMap<ObjectKey, Vec<u8>> {
        self.objects.into_inner()
    }
}

impl<S: ImmutableObjectStore + ?Sized> ImmutableObjectStore for Retained<'_, S> {
    fn read(
        &self,
        key: ObjectKey,
        maximum: Option<usize>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_inner(key, maximum, None, work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum: Option<usize>,
        external: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_inner(key, maximum, Some(external), work)
    }

    fn stage(
        &mut self,
        _: ObjectKey,
        _: &[u8],
        _: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "gc_read_only",
            "reachability cannot stage meaning",
        ))
    }
}

#[cfg(test)]
#[path = "retain_tests.rs"]
mod tests;

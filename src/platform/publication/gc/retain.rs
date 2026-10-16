//! Bounded, operation-local canonical reads. No accepted validation cache supplies reachability.

use super::*;

pub(super) const MAXIMUM_RETAINED_BYTES: usize = 512 * 1_048_576;
pub(super) const MAXIMUM_OBJECTS: usize = 1_000_000;
const OBJECT_OVERHEAD: usize = 256;
const MAXIMUM_READS: u64 = 16_000_000;
const MAXIMUM_READ_BYTES: u64 = 4_294_967_296;

pub(super) struct Retained<'a, S: ?Sized> {
    base: &'a S,
    objects: RefCell<BTreeMap<ObjectKey, Vec<u8>>>,
    bytes: Cell<usize>,
    admission: RefCell<StoreReadAdmission>,
    control: &'a ExecutionControl,
}

impl<'a, S: ImmutableObjectStore + ?Sized> Retained<'a, S> {
    pub(super) fn new(base: &'a S, control: &'a ExecutionControl) -> Self {
        Self {
            base,
            objects: RefCell::new(BTreeMap::new()),
            bytes: Cell::new(0),
            admission: RefCell::new(StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: MAXIMUM_READS,
                maximum_objects: MAXIMUM_READS,
                maximum_bytes: MAXIMUM_READ_BYTES,
            })),
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
        if root.entries() > MAXIMUM_OBJECTS as u64 {
            return Err(gc_error(
                DiagnosticClass::Resource,
                "gc_map_entries",
                "reachable map exceeds compaction capacity",
            ));
        }
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

    pub(super) fn finish(self) -> BTreeMap<ObjectKey, Vec<u8>> {
        self.objects.into_inner()
    }
}

impl<S: ImmutableObjectStore + ?Sized> ImmutableObjectStore for Retained<'_, S> {
    fn read(
        &self,
        key: ObjectKey,
        maximum: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_admitted(key, maximum, &mut StoreReadAdmission::unbounded(), work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum: usize,
        external: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let fresh = !self.objects.borrow().contains_key(&key);
        if fresh
            && (self.objects.borrow().len() == MAXIMUM_OBJECTS
                || self.bytes.get() > MAXIMUM_RETAINED_BYTES - OBJECT_OVERHEAD)
        {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "gc_retention_capacity",
                "reachable object retention exceeds compaction capacity",
            ));
        }
        let maximum = if fresh {
            maximum.min(MAXIMUM_RETAINED_BYTES - self.bytes.get() - OBJECT_OVERHEAD)
        } else {
            maximum
        };
        let inner = self.admission.borrow().remaining();
        let outer = external.remaining();
        let combined = StoreReadLimits {
            maximum_catalog_lookups: inner
                .maximum_catalog_lookups
                .min(outer.maximum_catalog_lookups),
            maximum_objects: inner.maximum_objects.min(outer.maximum_objects),
            maximum_bytes: inner.maximum_bytes.min(outer.maximum_bytes),
        };
        let mut admission = StoreReadAdmission::new(combined);
        let result = self.base.read_admitted(key, maximum, &mut admission, work);
        let remaining = admission.remaining();
        let charged = |budget: StoreReadLimits| {
            StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: budget.maximum_catalog_lookups
                    - (combined.maximum_catalog_lookups - remaining.maximum_catalog_lookups),
                maximum_objects: budget.maximum_objects
                    - (combined.maximum_objects - remaining.maximum_objects),
                maximum_bytes: budget.maximum_bytes
                    - (combined.maximum_bytes - remaining.maximum_bytes),
            })
        };
        *self.admission.borrow_mut() = charged(inner);
        *external = charged(outer);
        let bytes = result?;
        if let Some(bytes) = &bytes {
            key.verify(bytes)?;
            if fresh {
                self.bytes
                    .set(self.bytes.get() + bytes.len() + OBJECT_OVERHEAD);
                self.objects.borrow_mut().insert(key, bytes.clone());
            } else if self.objects.borrow().get(&key) != Some(bytes) {
                return Err(StoreError::new(
                    StoreErrorClass::Corrupt,
                    "gc_changed_object",
                    "reachable canonical bytes changed during compaction",
                ));
            }
        }
        Ok(bytes)
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
mod tests {
    use super::*;

    #[test]
    fn retention_and_read_allowances_refuse_before_payload_growth() {
        let bytes = b"bounded".to_vec();
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
        let base = BTreeMap::from([(key, bytes)]);
        let control = ExecutionControl::uncancelled();
        let store = Retained::new(&base, &control);
        store.bytes.set(MAXIMUM_RETAINED_BYTES - OBJECT_OVERHEAD);
        let mut work = StoreWork::default();
        assert_eq!(
            store.read(key, 7, &mut work).unwrap_err().class,
            StoreErrorClass::Resource
        );
        assert_eq!(work.objects_read, 0);
        assert!(store.objects.borrow().is_empty());
        store.bytes.set(0);
        let mut outer = StoreReadAdmission::new(StoreReadLimits {
            maximum_catalog_lookups: 1,
            maximum_objects: 1,
            maximum_bytes: 6,
        });
        assert_eq!(
            store
                .read_admitted(key, 7, &mut outer, &mut work)
                .unwrap_err()
                .class,
            StoreErrorClass::Resource
        );
        assert_eq!(work.objects_read, 0);
        assert!(store.objects.borrow().is_empty());
    }
}

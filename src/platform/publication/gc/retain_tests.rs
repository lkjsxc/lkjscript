use super::*;

/// A finite source avoids retaining a second 528 MiB copy of the fixture.
struct BlobSource {
    keys: BTreeMap<ObjectKey, u8>,
    length: usize,
}

impl BlobSource {
    fn new(count: u8, length: usize) -> Self {
        let keys = (0..count)
            .map(|marker| {
                (
                    ObjectKey::for_bytes(ObjectDomain::Blob, &vec![marker; length]),
                    marker,
                )
            })
            .collect();
        Self { keys, length }
    }
}

impl ImmutableObjectStore for BlobSource {
    fn read(
        &self,
        key: ObjectKey,
        maximum: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        let Some(marker) = self.keys.get(&key) else {
            return Ok(None);
        };
        if self.length > maximum {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "fixture_object_limit",
                "explicit object limit",
            ));
        }
        work.objects_read = work.objects_read.saturating_add(1);
        work.bytes_read = work.bytes_read.saturating_add(self.length as u64);
        Ok(Some(vec![*marker; self.length]))
    }

    fn stage(
        &mut self,
        _: ObjectKey,
        _: &[u8],
        _: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "fixture_read_only",
            "read-only fixture",
        ))
    }
}

#[test]
fn gc_retains_real_payloads_beyond_the_former_512_mib_ceiling() {
    let base = BlobSource::new(33, 16 * 1_048_576);
    let control = ExecutionControl::uncancelled();
    let store = Retained::new(&base, &control);
    for key in base.keys.keys() {
        let bytes = store.required(key.domain, key.digest.bytes()).unwrap();
        assert_eq!(bytes.len(), base.length);
    }
    let observation = store.observation();
    assert!(observation.retained_modeled_bytes > 512 * 1_048_576);
    assert_eq!(
        observation.retained_modeled_bytes,
        33 * (base.length as u64 + OBJECT_OVERHEAD)
    );
    assert_eq!(observation.objects_read, 33);
    let objects = store.finish();
    assert_eq!(objects.len(), 33);
    assert_eq!(
        objects.values().map(Vec::len).sum::<usize>(),
        528 * 1_048_576
    );
}

#[test]
fn gc_repeated_reads_cross_the_former_four_gib_aggregate_ceiling() {
    let base = BlobSource::new(1, 16 * 1_048_576);
    let key = *base.keys.keys().next().unwrap();
    let control = ExecutionControl::uncancelled();
    let store = Retained::new(&base, &control);
    for _ in 0..257 {
        store.required(key.domain, key.digest.bytes()).unwrap();
    }
    let observation = store.observation();
    assert_eq!(observation.objects_read, 257);
    assert!(observation.bytes_read > 4_294_967_296);
    assert_eq!(
        observation.retained_modeled_bytes,
        base.length as u64 + OBJECT_OVERHEAD
    );
    assert_eq!(store.finish().len(), 1);
}

#[test]
fn gc_explicit_read_allowances_still_refuse_before_payload_growth() {
    let bytes = b"bounded".to_vec();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
    let base = BTreeMap::from([(key, bytes)]);
    let control = ExecutionControl::uncancelled();
    let store = Retained::new(&base, &control);
    let mut work = StoreWork::default();
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
    let mut exact = StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: 1,
        maximum_objects: 1,
        maximum_bytes: 7,
    });
    assert_eq!(
        store.read_admitted(key, 7, &mut exact, &mut work).unwrap(),
        Some(b"bounded".to_vec())
    );
}

#[test]
fn gc_saturated_telemetry_does_not_authorize_or_refuse_reads() {
    let bytes = b"observed".to_vec();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
    let base = BTreeMap::from([(key, bytes)]);
    let control = ExecutionControl::uncancelled();
    let store = Retained::new(&base, &control);
    store.observation.set(PreparationObservation {
        retained_modeled_bytes: u64::MAX,
        catalog_lookups: u64::MAX,
        objects_read: u64::MAX,
        bytes_read: u64::MAX,
        references_visited: u64::MAX,
    });
    assert_eq!(
        store.required(key.domain, key.digest.bytes()).unwrap(),
        b"observed"
    );
    assert_eq!(store.observation().retained_modeled_bytes, u64::MAX);
    assert_eq!(store.observation().bytes_read, u64::MAX);
    control.cancel();
    assert_eq!(
        store
            .required(key.domain, key.digest.bytes())
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(store.finish().len(), 1);
}

#[test]
fn gc_retention_still_verifies_each_canonical_object() {
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, b"valid");
    let base = BTreeMap::from([(key, b"invalid".to_vec())]);
    let control = ExecutionControl::uncancelled();
    let store = Retained::new(&base, &control);
    assert_eq!(
        store
            .required(key.domain, key.digest.bytes())
            .unwrap_err()
            .class,
        DiagnosticClass::Corrupt
    );
    assert!(store.finish().is_empty());
}

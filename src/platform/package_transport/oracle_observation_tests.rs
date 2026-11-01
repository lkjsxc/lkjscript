use super::*;

#[test]
fn independent_oracle_observations_cross_former_defaults_without_refusal() -> Result<(), Diagnostic>
{
    let payload = b"abc";
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, payload);
    let objects = BTreeMap::from([(key, payload.to_vec())]);
    let reader = Reader::new(&objects);
    reader.visits.set(SourceObservation::exact(16_000_000));
    reader.bytes.set(SourceObservation::exact(4_294_967_296));
    reader
        .copy_bytes
        .set(SourceObservation::exact(256 * 1_048_576));
    reader.charge(1)?;
    assert_eq!(reader.visits.get(), SourceObservation::exact(16_000_001));
    assert_eq!(reader.object(key.domain, key.digest.bytes())?, payload);
    assert_eq!(reader.visits.get(), SourceObservation::exact(16_000_002));
    assert_eq!(reader.bytes.get(), SourceObservation::exact(4_294_967_299));
    reader.observe_copy(SourceObservation::exact(65), SourceObservation::exact(3));
    assert_eq!(reader.visits.get(), SourceObservation::exact(16_000_007));
    assert_eq!(
        reader.copy_bytes.get(),
        SourceObservation::exact(256 * 1_048_576 + 65)
    );
    assert_eq!(reader.bytes.get(), SourceObservation::exact(4_294_967_299));

    reader.visits.set(SourceObservation::exact(u64::MAX));
    reader.bytes.set(SourceObservation::exact(u64::MAX));
    reader.copy_bytes.set(SourceObservation::exact(u64::MAX));
    assert_eq!(reader.object(key.domain, key.digest.bytes())?, payload);
    reader.observe_copy(SourceObservation::exact(1), SourceObservation::exact(1));
    reader.charge(1)?;
    for observed in [
        reader.visits.get(),
        reader.bytes.get(),
        reader.copy_bytes.get(),
    ] {
        assert_eq!(observed.consumed, u64::MAX);
        assert!(observed.saturated);
    }
    assert_eq!(
        objects.get(&key).map(Vec::as_slice),
        Some(payload.as_slice())
    );
    Ok(())
}

#[test]
fn shared_builtin_oracle_copy_work_is_passive_and_separate_from_reads() -> Result<(), Diagnostic> {
    let standard = crate::platform::builtin_standard::BuiltinStandard::load()
        .map_err(|error| failure(format!("builtin source fixture: {error:?}")))?;
    assert!(!standard.independent_source().validation_visits_saturated);
    assert!(
        !standard
            .independent_source()
            .validation_read_bytes_saturated
    );
    let encoded = standard.admitted_source().container.objects.values().fold(
        SourceObservation::default(),
        |mut total, bytes| {
            total.observe(observed_count(bytes.len()));
            total
        },
    );
    let (bytes, records) = standard
        .independent_source()
        .package_copy_material(standard.package, encoded)
        .ok_or_else(|| failure("builtin source fixture has no independent package"))?;
    assert!(!encoded.saturated && !bytes.saturated && !records.saturated);
    assert!(bytes.consumed > encoded.consumed);
    let objects = BTreeMap::new();
    let reader = Reader::new(&objects);
    reader.bytes.set(SourceObservation::exact(7));
    reader.observe_copy(bytes, records);
    assert_eq!(reader.copy_bytes.get(), bytes);
    assert_eq!(
        reader.visits.get(),
        SourceObservation::exact(records.consumed + bytes.consumed.div_ceil(64))
    );
    assert_eq!(reader.bytes.get(), SourceObservation::exact(7));

    let (overflowed, _) = standard
        .independent_source()
        .package_copy_material(standard.package, SourceObservation::exact(u64::MAX))
        .ok_or_else(|| failure("builtin source fixture lost its independent package"))?;
    assert_eq!(overflowed.consumed, u64::MAX);
    assert!(overflowed.saturated);
    reader.observe_copy(overflowed, records);
    assert!(reader.copy_bytes.get().saturated);
    assert!(reader.visits.get().saturated);
    assert_eq!(reader.bytes.get(), SourceObservation::exact(7));
    Ok(())
}

#[test]
fn saturated_oracle_observation_preserves_object_integrity_and_explicit_read_size()
-> Result<(), Diagnostic> {
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, b"valid");
    let objects = BTreeMap::from([(key, b"valid".to_vec())]);
    let reader = Reader::new(&objects);
    reader.visits.set(SourceObservation {
        consumed: u64::MAX,
        saturated: true,
    });
    reader.bytes.set(SourceObservation {
        consumed: u64::MAX,
        saturated: true,
    });
    let refused = reader.read(key, Some(0), &mut StoreWork::default());
    assert!(matches!(
        refused,
        Err(StoreError {
            class: StoreErrorClass::Resource,
            ..
        })
    ));
    assert!(reader.seen.borrow().is_empty());
    assert_eq!(reader.object(key.domain, key.digest.bytes())?, b"valid");

    let corrupt_objects = BTreeMap::from([(key, b"changed".to_vec())]);
    let corrupt_reader = Reader::new(&corrupt_objects);
    corrupt_reader.visits.set(SourceObservation {
        consumed: u64::MAX,
        saturated: true,
    });
    assert!(
        corrupt_reader
            .object(key.domain, key.digest.bytes())
            .is_err()
    );
    assert!(corrupt_reader.seen.borrow().is_empty());
    Ok(())
}

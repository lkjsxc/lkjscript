//! Candidate overlays are atomic private storage, without an aggregate logical byte quota.

use super::*;
use crate::platform::MemProjectStore;
use crate::platform::publication::GraphRepository;
use std::cell::Cell;

type TestResult<T = ()> = Result<T, String>;

fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> TestResult<T> {
    result.map_err(|error| format!("{error:?}"))
}

fn failed<T, E>(result: Result<T, E>) -> TestResult<E> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("operation unexpectedly succeeded".to_owned()),
    }
}

fn empty_view() -> TestResult<ViewStore> {
    let memory = checked(MemProjectStore::new(None))?;
    let base = checked(memory.graph.lock())?.store.clone();
    Ok(ViewStore::new(StorageBackend::Memory(base)))
}

fn payload(length: usize, byte: u8) -> TestResult<Vec<u8>> {
    let mut bytes = Vec::new();
    checked(bytes.try_reserve_exact(length))?;
    bytes.resize(length, byte);
    Ok(bytes)
}

fn read_overlay(store: &ViewStore, key: ObjectKey) -> TestResult<Vec<u8>> {
    checked(store.read(key, key.domain.maximum_bytes(), &mut StoreWork::default()))?
        .ok_or_else(|| "installed overlay object is missing".to_owned())
}

#[test]
fn preview_overlay_round_trips_more_than_the_former_64_mib_aggregate() -> TestResult {
    const OBJECT_BYTES: usize = 4 * 1_048_576;
    const COUNT: usize = 17;
    let total = OBJECT_BYTES * COUNT;
    assert!(total > 64 * 1_048_576);
    let mut candidate = BTreeMap::new();
    for ordinal in 0..COUNT {
        let bytes = payload(OBJECT_BYTES, checked(u8::try_from(ordinal))?)?;
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
        checked(key.verify(&bytes))?;
        candidate.insert(key, bytes);
    }
    assert_eq!(candidate.len(), COUNT);
    let mut store = empty_view()?;
    checked(store.install_candidate(&candidate, &|| Ok(())))?;
    let mut admission = StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: Some(2 * COUNT as u64),
        maximum_objects: Some(COUNT as u64),
        maximum_bytes: Some(checked(u64::try_from(total))?),
    });
    let mut work = StoreWork::default();
    for (key, expected) in &candidate {
        let actual = checked(store.read_admitted(
            *key,
            key.domain.maximum_bytes(),
            &mut admission,
            &mut work,
        ))?
        .ok_or_else(|| "large candidate object was not installed".to_owned())?;
        assert_eq!(&actual, expected);
        checked(key.verify(&actual))?;
    }
    assert_eq!(
        admission.remaining(),
        StoreReadLimits {
            maximum_catalog_lookups: Some(0),
            maximum_objects: Some(0),
            maximum_bytes: Some(0),
        }
    );
    assert_eq!(work.catalog_lookups, 2 * COUNT as u64);
    assert_eq!(work.objects_read, COUNT as u64);
    assert_eq!(work.bytes_read, checked(u64::try_from(total))?);
    assert!(!work.saturated);
    assert_eq!(store.persistent_base().staged_len(), 0);
    assert_eq!(store.persistent_base().catalog_observation().entries, 0);
    Ok(())
}

#[test]
fn preview_overlay_merges_full_keys_reuses_duplicates_and_rejects_conflicts() -> TestResult {
    let package = b"package revision".as_slice();
    let backup = b"backup manifest".as_slice();
    let blob = b"blob".as_slice();
    let manifest = b"compilation manifest".as_slice();
    let package_key = ObjectKey::for_bytes(ObjectDomain::PackageRevision, package);
    let backup_key = ObjectKey::for_bytes(ObjectDomain::BackupManifest, backup);
    let blob_key = ObjectKey::for_bytes(ObjectDomain::Blob, blob);
    let manifest_key = ObjectKey::for_bytes(ObjectDomain::CompilationManifest, manifest);
    assert!(package_key < backup_key);
    assert!(package_key.domain.tag() > backup_key.domain.tag());
    let mut store = empty_view()?;
    checked(store.install_batch(vec![(package_key, package)], &|| Ok(())))?;
    let original_pointer = store
        .derived_bytes(package_key)
        .ok_or_else(|| "seed overlay is missing".to_owned())?
        .as_ptr();
    checked(store.install_batch(
        vec![
            (manifest_key, manifest),
            (backup_key, backup),
            (package_key, package),
            (blob_key, blob),
            (backup_key, backup),
        ],
        &|| Ok(()),
    ))?;
    let expected = BTreeMap::from([
        (package_key, package),
        (backup_key, backup),
        (blob_key, blob),
        (manifest_key, manifest),
    ]);
    assert_eq!(
        store
            .derived
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>(),
        expected.keys().copied().collect::<Vec<_>>()
    );
    for (key, expected) in expected {
        assert_eq!(read_overlay(&store, key)?.as_slice(), expected);
    }
    checked(
        store.install_batch(vec![(backup_key, backup), (package_key, package)], &|| {
            Ok(())
        }),
    )?;
    assert_eq!(store.derived.len(), 4);
    assert_eq!(
        store.derived_bytes(package_key).map(<[u8]>::as_ptr),
        Some(original_pointer)
    );

    // A conflicting existing binding must be detected even if the incoming
    // payload itself has the correct content digest. Fault injection avoids
    // pretending to manufacture a cryptographic collision.
    let position = checked(
        store
            .derived
            .binary_search_by_key(&package_key, |(key, _)| *key),
    )?;
    store.derived[position].1[0] ^= 1;
    let before = store.derived.clone();
    let error = failed(store.install_batch(vec![(package_key, package)], &|| Ok(())))?;
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "object_digest_collision");
    assert_eq!(store.derived, before);
    Ok(())
}

#[test]
fn preview_overlay_late_invalid_candidate_preserves_the_previous_overlay() -> TestResult {
    let mut store = empty_view()?;
    let old = b"retained private object".as_slice();
    let old_key = ObjectKey::for_bytes(ObjectDomain::PackageRevision, old);
    checked(store.install_batch(vec![(old_key, old)], &|| Ok(())))?;
    let before = store.derived.clone();
    let valid = b"valid new prefix".as_slice();
    let new_key = ObjectKey::for_bytes(ObjectDomain::Blob, valid);
    let invalid_key = ObjectKey::for_bytes(ObjectDomain::CompilationManifest, b"claimed bytes");
    assert!(new_key < invalid_key);
    let candidate = BTreeMap::from([
        (
            new_key,
            checked(crate::platform::storage::object::copy_object_bytes(valid))?,
        ),
        (invalid_key, payload(12, 3)?),
    ]);
    let error = failed(store.install_candidate(&candidate, &|| Ok(())))?;
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "object_digest_mismatch");
    assert_eq!(store.derived, before);
    assert_eq!(read_overlay(&store, old_key)?.as_slice(), old);
    assert!(checked(store.read(new_key, None, &mut StoreWork::default()))?.is_none());
    assert_eq!(store.persistent_base().staged_len(), 0);
    Ok(())
}

#[test]
fn preview_overlay_invalid_rebuilt_facts_do_not_publish_a_valid_prefix() -> TestResult {
    let mut validation = checked(RevalidatedBase::rebuild(
        crate::platform::kernel::tests::witness_snapshot(),
        &|| Ok(()),
    ))?;
    checked(validation.require_current())?;
    let (_, last) = validation
        .facts
        .summary_objects
        .iter_mut()
        .next_back()
        .ok_or_else(|| "fixture has no owner summary".to_owned())?;
    last.push(0x5a);
    let old = b"previous derived context".as_slice();
    let old_key = ObjectKey::for_bytes(ObjectDomain::Blob, old);
    let mut store = empty_view()?;
    checked(store.install_batch(vec![(old_key, old)], &|| Ok(())))?;
    let before = store.derived.clone();
    let error = failed(store.install(&validation))?;
    assert_eq!(error.code, "object_digest_mismatch");
    assert_eq!(store.derived, before);
    assert_eq!(read_overlay(&store, old_key)?.as_slice(), old);
    Ok(())
}

#[test]
fn preview_overlay_reads_keep_exact_caller_and_rebuild_allowances() -> TestResult {
    let bytes = b"bounded overlay read".as_slice();
    let length = checked(u64::try_from(bytes.len()))?;
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
    let mut store = empty_view()?;
    checked(store.install_batch(vec![(key, bytes)], &|| Ok(())))?;
    for (limits, code) in [
        (
            StoreReadLimits {
                maximum_catalog_lookups: Some(0),
                ..StoreReadLimits::unbounded()
            },
            "object_read_catalog_lookups_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_catalog_lookups: Some(1),
                ..StoreReadLimits::unbounded()
            },
            "object_read_catalog_lookups_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_objects: Some(0),
                ..StoreReadLimits::unbounded()
            },
            "object_read_objects_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_bytes: Some(length - 1),
                ..StoreReadLimits::unbounded()
            },
            "object_read_bytes_exhausted",
        ),
    ] {
        let error = failed(store.read_admitted(
            key,
            None,
            &mut StoreReadAdmission::new(limits),
            &mut StoreWork::default(),
        ))?;
        assert_eq!(error.class, StoreErrorClass::Resource);
        assert_eq!(error.code, code);
        assert_eq!(store.derived_bytes(key), Some(bytes));
    }
    let error = failed(store.read(key, Some(bytes.len() - 1), &mut StoreWork::default()))?;
    assert_eq!(error.code, "publication_validation_object_size");
    let exact = StoreReadLimits {
        maximum_catalog_lookups: Some(2),
        maximum_objects: Some(1),
        maximum_bytes: Some(length),
    };
    let mut admission = StoreReadAdmission::new(exact);
    assert_eq!(
        checked(store.read_admitted(
            key,
            Some(bytes.len()),
            &mut admission,
            &mut StoreWork::default(),
        ))?
        .as_deref(),
        Some(bytes)
    );
    let zero = StoreReadLimits {
        maximum_catalog_lookups: Some(0),
        maximum_objects: Some(0),
        maximum_bytes: Some(0),
    };
    assert_eq!(admission.remaining(), zero);

    // The enclosing rebuild policy intersects the caller policy, while MAX
    // remains a finite allowance with an exact decrement.
    store
        .rebuild_reads
        .set(StoreReadAdmission::new(StoreReadLimits {
            maximum_bytes: Some(length - 1),
            ..exact
        }));
    let error = failed(store.read(key, None, &mut StoreWork::default()))?;
    assert_eq!(error.code, "object_read_bytes_exhausted");
    store.rebuild_reads.set(StoreReadAdmission::new(exact));
    let mut maximum = StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: Some(u64::MAX),
        maximum_objects: Some(u64::MAX),
        maximum_bytes: Some(u64::MAX),
    });
    let mut work = StoreWork::default();
    assert!(checked(store.read_admitted(key, None, &mut maximum, &mut work))?.is_some());
    assert_eq!(store.rebuild_reads.get().remaining(), zero);
    assert_eq!(
        maximum.remaining(),
        StoreReadLimits {
            maximum_catalog_lookups: Some(u64::MAX - 2),
            maximum_objects: Some(u64::MAX - 1),
            maximum_bytes: Some(u64::MAX - length),
        }
    );
    assert_eq!(work.catalog_lookups, 2);
    assert_eq!(work.objects_read, 1);
    assert_eq!(work.bytes_read, length);
    Ok(())
}

#[test]
fn preview_overlay_never_masks_physical_bytes_corruption_or_read_refusal() -> TestResult {
    let bytes = b"authoritative physical bytes".as_slice();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
    let mut store = empty_view()?;
    checked(store.install_batch(vec![(key, bytes)], &|| Ok(())))?;
    assert_eq!(read_overlay(&store, key)?.as_slice(), bytes);
    checked(store.base.stage(key, bytes, &mut StoreWork::default()))?;
    store.derived[0].1[0] ^= 1;
    let mut work = StoreWork::default();
    assert_eq!(
        checked(store.read(key, None, &mut work))?.as_deref(),
        Some(bytes)
    );
    assert_eq!(work.catalog_lookups, 1);
    let error = failed(store.read(key, Some(bytes.len() - 1), &mut StoreWork::default()))?;
    assert_eq!(error.code, "memory_read_limit");
    store.derived[0].1[0] ^= 1;
    let StorageBackend::Memory(memory) = &mut store.base else {
        return Err("fixture did not create a memory backend".to_owned());
    };
    checked(memory.objects.insert(key, b"corrupt physical replacement"))?;
    let error = failed(store.read(key, None, &mut StoreWork::default()))?;
    assert_eq!(error.class, StoreErrorClass::Corrupt);
    assert_eq!(error.code, "object_digest_mismatch");
    assert_eq!(store.derived_bytes(key), Some(bytes));
    Ok(())
}

#[test]
fn preview_overlay_only_cache_reads_use_content_proof_instead_of_memory_identity() -> TestResult {
    let memory = checked(MemProjectStore::new(None))?;
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let created = checked(GraphRepository::create_memory(&memory, &snapshot, None))?;
    let current = checked(created.repository.current())?;
    let key = CurrentValidationKey {
        root: current.accepted.semantic_root,
        revision: current.head.revision,
        publication: current.head.record,
        validator: crate::platform::witness::contract::validator_contract_digest(),
        implementation: crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH,
    };
    let context = Arc::new(checked(RevalidatedBase::rebuild(snapshot, &|| Ok(())))?);
    checked(context.require_current())?;
    let mut store = ViewStore::new(checked(created.repository.object_store())?);
    let bytes = b"derived-only cache read".as_slice();
    let object = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
    assert!(checked(store.base.read(object, None, &mut StoreWork::default()))?.is_none());
    checked(store.install_batch(vec![(object, bytes)], &|| Ok(())))?;
    let entry = cache_entry(
        key,
        checked(u64::try_from(bytes.len()))?,
        BTreeSet::from([object]),
        Vec::new(),
        context,
        &store,
    )
    .ok_or_else(|| "small derived-only proof was not cacheable".to_owned())?;
    assert_eq!(entry.objects.len(), 1);
    assert!(entry.objects[0].memory.is_none());
    let work = checked(recheck(
        &entry,
        &store,
        &|| Ok(()),
        StoreReadLimits {
            maximum_catalog_lookups: Some(2),
            maximum_objects: Some(1),
            maximum_bytes: Some(checked(u64::try_from(bytes.len()))?),
        },
    ))?;
    assert_eq!(work.catalog_lookups, 2);
    assert_eq!(work.objects_read, 1);
    assert_eq!(work.objects_reused, 0);
    assert_eq!(work.bytes_read, checked(u64::try_from(bytes.len()))?);
    Ok(())
}

#[test]
fn preview_overlay_cancellation_at_each_checkpoint_keeps_the_old_overlay() -> TestResult {
    let bytes = b"old private preview".as_slice();
    let old_key = ObjectKey::for_bytes(ObjectDomain::PackageRevision, bytes);
    let candidate = BTreeMap::from([
        (
            ObjectKey::for_bytes(ObjectDomain::Blob, b"new blob"),
            b"new blob".to_vec(),
        ),
        (
            ObjectKey::for_bytes(ObjectDomain::BackupManifest, b"new backup"),
            b"new backup".to_vec(),
        ),
    ]);
    let mut probe = empty_view()?;
    checked(probe.install_batch(vec![(old_key, bytes)], &|| Ok(())))?;
    let total = Cell::new(0_usize);
    checked(probe.install_candidate(&candidate, &|| {
        total.set(total.get() + 1);
        Ok(())
    }))?;
    assert!(total.get() > 1);
    for stop in 1..=total.get() {
        let mut store = empty_view()?;
        checked(store.install_batch(vec![(old_key, bytes)], &|| Ok(())))?;
        let before = store.derived.clone();
        let pointer = store.derived[0].1.as_ptr();
        let seen = Cell::new(0_usize);
        let error = failed(store.install_candidate(&candidate, &|| {
            seen.set(seen.get() + 1);
            if seen.get() == stop {
                Err(Diagnostic::new(
                    DiagnosticClass::Cancelled,
                    "preview_test_cancel",
                    "cancelled before private overlay publication",
                ))
            } else {
                Ok(())
            }
        }))?;
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(seen.get(), stop);
        assert_eq!(store.derived, before);
        assert_eq!(store.derived[0].1.as_ptr(), pointer);
        assert_eq!(read_overlay(&store, old_key)?.as_slice(), bytes);
        assert_eq!(store.persistent_base().staged_len(), 0);
    }
    Ok(())
}

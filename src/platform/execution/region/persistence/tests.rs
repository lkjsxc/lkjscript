use super::*;
use crate::platform::data::{DataCommitOutcome, DataKeyPart, DataLimits, DataStore};
use crate::platform::execution::region::{RegionLayout, with_region};

fn key() -> DataKey {
    DataKey::new(vec![DataKeyPart::I64(7)], &DataLimits::default()).unwrap()
}

fn memory_store() -> DataStore {
    crate::platform::MemProjectStore::new(1_048_576)
        .unwrap()
        .data_store("region-tests", DataLimits::default())
        .unwrap()
}

fn image() -> Vec<u8> {
    let control = ExecutionControl::uncancelled();
    with_region(
        256,
        &control,
        |_| Ok(()),
        |mut region| {
            let layout = RegionLayout::<i64>::new([1; 32], 8, 8)?;
            region.allocate(layout, &42_i64.to_le_bytes(), &control)?;
            Ok(region.freeze(&control)?.image().to_vec())
        },
    )
    .unwrap()
}

#[test]
fn relative_cycles_round_trip_through_memory_and_disk_data_transactions() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("data");
    DataStore::initialize(&root).unwrap();
    let disk = DataStore::open(&root, "region-tests", DataLimits::default()).unwrap();
    for store in [memory_store(), disk] {
        let control = ExecutionControl::uncancelled();
        let prior = store.begin().unwrap();
        let mut transaction = store.begin().unwrap();
        let layout = RegionLayout::<()>::new([2; 32], 16, 8).unwrap();
        let (expected, offset, exact_bytes) = with_region(
            512,
            &control,
            |_| Ok(()),
            |mut region| {
                let a = region.allocate(layout, &[0; 16], &control)?;
                let b = region.allocate(layout, &[0; 16], &control)?;
                for (pointer, next, value) in [(a, b, 11_i64), (b, a, 29_i64)] {
                    let mut node = [0; 16];
                    node[..8].copy_from_slice(&value.to_le_bytes());
                    node[8..16].copy_from_slice(&next.offset().to_le_bytes());
                    region.write(layout, pointer, &node, &control)?;
                }
                let frozen = region.freeze(&control)?;
                let mut charges = Vec::new();
                let expected = save(
                    &mut transaction,
                    "graphs",
                    &key(),
                    frozen.read_cap(),
                    DataExpectation::Missing,
                    &control,
                    |bytes| {
                        charges.push(bytes);
                        Ok(())
                    },
                )?
                .unwrap();
                assert_eq!(
                    charges,
                    [frozen.image().len() as u64 + std::mem::size_of::<Vec<u8>>() as u64]
                );
                assert_eq!(
                    transaction
                        .get_ref("graphs", &key())
                        .unwrap()
                        .unwrap()
                        .value,
                    frozen.image()
                );
                Ok((expected, a.offset(), frozen.image().to_vec()))
            },
        )
        .unwrap();
        assert!(prior.get_ref("graphs", &key()).unwrap().is_none());
        assert!(matches!(
            transaction.commit().unwrap(),
            DataCommitOutcome::Committed { .. }
        ));
        assert!(prior.get_ref("graphs", &key()).unwrap().is_none());
        let snapshot = store.begin().unwrap();
        let mut charges = Vec::new();
        let loaded = load(
            &snapshot,
            "graphs",
            &key(),
            expected,
            1024,
            &control,
            |bytes| {
                charges.push(bytes);
                Ok(())
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(loaded.image(), exact_bytes);
        assert_eq!(loaded.digest(), expected);
        assert_eq!(
            charges,
            [exact_bytes.len() as u64 + 63 + std::mem::size_of::<RegionImage>() as u64]
        );
        let address = loaded.image().as_ptr() as usize;
        assert_eq!(address % 64, 0);
        drop(snapshot);
        let loaded = std::thread::spawn(move || {
            assert_eq!(loaded.image().as_ptr() as usize, address);
            let control = ExecutionControl::uncancelled();
            loaded
                .with_region(&control, |read| {
                    let mut current = read.resolve(layout, offset, &control)?;
                    let mut sum = 0;
                    for _ in 0..100 {
                        let node = read.read(layout, current, &control)?;
                        assert_eq!(node.as_ptr() as usize, address + current.offset() as usize);
                        sum += i64::from_le_bytes(node[..8].try_into().unwrap());
                        current = read.resolve(
                            layout,
                            u64::from_le_bytes(node[8..16].try_into().unwrap()),
                            &control,
                        )?;
                    }
                    assert_eq!(sum, 2000);
                    assert_eq!(current.offset(), offset);
                    Ok(())
                })
                .unwrap();
            loaded
        })
        .join()
        .unwrap();
        assert_eq!(loaded.image(), exact_bytes);
        store.verify().unwrap();
    }
}

#[test]
fn digest_layout_and_capacity_are_independent_load_requirements() {
    let control = ExecutionControl::uncancelled();
    let image = image();
    let expected = digest(&image, &control).unwrap();
    let mut changed = image.clone();
    changed[192] ^= 1;
    let mut charged = false;
    let error = RegionImage::from_bytes(&changed, expected, 1024, &control, |_| {
        charged = true;
        Ok(())
    })
    .err()
    .unwrap();
    assert_eq!(error.code, "region_image_digest");
    assert!(!charged);
    let error = RegionImage::from_bytes(&image, expected, image.len() - 1, &control, |_| {
        charged = true;
        Ok(())
    })
    .err()
    .unwrap();
    assert_eq!(error.code, "region_capacity");
    assert!(!charged);
    for (offset, byte) in [(0, 0), (8, image[8] ^ 1), (24, 0), (32, 0), (65, 0), (184, 1)] {
        let mut corrupted = image.clone();
        corrupted[offset] = byte;
        let digest = digest(&corrupted, &control).unwrap();
        assert!(RegionImage::from_bytes(&corrupted, digest, 1024, &control, |_| Ok(())).is_err());
    }
    let error = RegionImage::from_bytes(&image, expected, 1024, &control, |_| {
        Err(ExecutionError::resource("quota", "test quota"))
    })
    .err()
    .unwrap();
    assert_eq!(error.code, "quota");
    let error = RegionImage::from_bytes(&image, expected, 1024, &control, |_| {
        control.cancel();
        Ok(())
    })
    .err()
    .unwrap();
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
}

#[test]
fn cancelled_or_unfunded_saves_do_not_stage_data() {
    for cancel in [false, true] {
        let store = memory_store();
        let control = ExecutionControl::uncancelled();
        let mut transaction = store.begin().unwrap();
        with_region(
            256,
            &control,
            |_| Ok(()),
            |mut region| {
                let layout = RegionLayout::<i64>::new([1; 32], 8, 8)?;
                region.allocate(layout, &42_i64.to_le_bytes(), &control)?;
                let frozen = region.freeze(&control)?;
                let error = save(
                    &mut transaction,
                    "graphs",
                    &key(),
                    frozen.read_cap(),
                    DataExpectation::Missing,
                    &control,
                    |_| {
                        if cancel {
                            control.cancel();
                            Ok(())
                        } else {
                            Err(ExecutionError::resource("quota", "test quota"))
                        }
                    },
                )
                .unwrap_err();
                assert_eq!(
                    error.class,
                    if cancel {
                        ExecutionFailureClass::Cancelled
                    } else {
                        ExecutionFailureClass::Resource
                    }
                );
                Ok(())
            },
        )
        .unwrap();
        assert!(transaction.get_ref("graphs", &key()).unwrap().is_none());
        assert!(matches!(
            transaction.commit().unwrap(),
            DataCommitOutcome::Unchanged { .. }
        ));
    }
}

#[test]
fn changed_keys_cannot_satisfy_an_older_image_digest() {
    let store = memory_store();
    let control = ExecutionControl::uncancelled();
    let first = image();
    let expected = digest(&first, &control).unwrap();
    let mut transaction = store.begin().unwrap();
    transaction
        .put("graphs", &key(), first.clone(), DataExpectation::Missing)
        .unwrap();
    transaction.commit().unwrap();
    let prior = store.begin().unwrap();
    let entry = prior.get_ref("graphs", &key()).unwrap().unwrap();
    let mut second = first.clone();
    second[192..200].copy_from_slice(&99_i64.to_le_bytes());
    let mut next = store.begin().unwrap();
    assert!(
        next.put(
            "graphs",
            &key(),
            second,
            DataExpectation::Exact(entry.revision)
        )
        .unwrap()
    );
    next.commit().unwrap();
    let current = store.begin().unwrap();
    assert_eq!(
        load(
            &current,
            "graphs",
            &key(),
            expected,
            1024,
            &control,
            |_| Ok(())
        )
        .err()
        .unwrap()
        .code,
        "region_image_digest"
    );
    assert_eq!(
        load(&prior, "graphs", &key(), expected, 1024, &control, |_| Ok(
            ()
        ))
        .unwrap()
        .unwrap()
        .image(),
        first
    );
    let missing = DataKey::new(vec![DataKeyPart::I64(99)], &DataLimits::default()).unwrap();
    assert!(
        load(
            &current,
            "graphs",
            &missing,
            expected,
            1024,
            &control,
            |_| Ok(())
        )
        .unwrap()
        .is_none()
    );
}

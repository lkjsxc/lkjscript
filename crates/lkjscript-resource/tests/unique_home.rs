mod common;

use common::*;
use lkjscript_core::UniqueStoreLimits;
use lkjscript_resource::*;

#[test]
fn partitioned_unique_store_moves_homes_and_drains_remote_release() -> ResourceResult<()> {
    let limits = UniqueStoreLimits::new(4, 1024, 4, 8, u32::MAX)
        .map_err(|error| ResourceError::new("test-limits", format!("{error:?}")))?;
    let store = PartitionedUniqueStore::new(9, limits, 2, 4, 4)?;
    let first = WorkerId::new(0, 1);
    let second = WorkerId::new(1, 1);
    let value = store.allocate_byte_vector(owner(7), first, vec![1, 2, 3])?;
    assert_eq!(store.checksum(value)?, 6);
    store.begin_loan(value)?;
    assert!(store.prove_no_live_loan(value).is_err());
    store.end_loan(value)?;
    let proof = store.prove_no_live_loan(value)?;
    store.move_home(value, second, proof)?;
    store.fill(value, 4)?;
    assert_eq!(store.checksum(value)?, 12);
    store.release(first, id(0), value)?;
    assert!(store.verify_empty().is_err());
    assert_eq!(store.drain_remote(second, 1)?, 1);
    store.verify_empty()?;
    let (owner_metrics, unique_metrics) = store.metrics()?;
    assert_eq!(owner_metrics.transfers, 1);
    assert_eq!(owner_metrics.remote_releases, 1);
    assert_eq!(unique_metrics.live_objects, 0);
    Ok(())
}

#[test]
fn independent_partitions_mutate_and_release_concurrently() -> ResourceResult<()> {
    let limits = UniqueStoreLimits::new(16, 4096, 16, 8, u32::MAX)
        .map_err(|error| ResourceError::new("test-limits", format!("{error:?}")))?;
    let store = PartitionedUniqueStore::new(20, limits, 4, 16, 16)?;
    let values = (0..8_u32)
        .map(|slot| {
            let home = WorkerId::new(slot % 4, 1);
            store
                .allocate_byte_vector(owner(slot + 20), home, vec![0; 64])
                .map(|value| (value, home, slot as u8))
        })
        .collect::<ResourceResult<Vec<_>>>()?;
    std::thread::scope(|scope| -> ResourceResult<()> {
        let handles: Vec<_> = values
            .iter()
            .map(|(value, home, byte)| {
                scope.spawn(|| -> ResourceResult<()> {
                    store.fill(*value, *byte)?;
                    assert_eq!(store.checksum(*value)?, u64::from(*byte) * 64);
                    store.release(*home, id(u32::from(*byte)), *value)
                })
            })
            .collect();
        for handle in handles {
            handle
                .join()
                .map_err(|_| ResourceError::new("test-panic", "partition worker panicked"))??;
        }
        Ok(())
    })?;
    store.verify_empty()?;
    let (_, metrics) = store.metrics()?;
    assert_eq!(metrics.allocations, 8);
    assert_eq!(metrics.frees, 8);
    assert_eq!(metrics.live_objects, 0);
    assert_eq!(metrics.peak_live_objects, 8);
    Ok(())
}

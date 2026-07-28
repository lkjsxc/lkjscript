mod common;

use common::*;
use lkjscript_core::UniqueStoreLimits;
use lkjscript_resource::*;

#[test]
fn partitioned_unique_store_moves_homes_and_drains_remote_release() -> ResourceResult<()> {
    let limits = UniqueStoreLimits::new(4, 1024, 4, 8, u32::MAX)
        .map_err(|error| ResourceError::new("test-limits", format!("{error:?}")))?;
    let store = PartitionedUniqueStore::new(9, limits, 4, 4)?;
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

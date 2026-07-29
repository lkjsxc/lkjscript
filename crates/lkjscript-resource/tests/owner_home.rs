use lkjscript_resource::{
    DataOwnerId, OwnerHomeTable, OwnerMetrics, RemoteRelease, ResourceError, ResourceResult,
    TaskId, WorkerId,
};

#[test]
fn owner_homes_require_fresh_proof_and_hold_release_authority() -> ResourceResult<()> {
    let mut homes = OwnerHomeTable::new(2, 1);
    let data = DataOwnerId::new(0, 1);
    let first = WorkerId::new(0, 1);
    let second = WorkerId::new(1, 1);
    homes.insert(data, first)?;
    let stale = homes.prove_no_live_loan(data)?;
    homes.begin_loan(data)?;
    assert!(homes.prove_no_live_loan(data).is_err());
    homes.end_loan(data)?;
    assert_eq!(
        homes
            .move_owner(data, second, stale)
            .map_err(|error| error.code),
        Err("owner-proof")
    );
    let proof = homes.prove_no_live_loan(data)?;
    homes.move_owner(data, second, proof)?;
    let release = RemoteRelease::new(data, TaskId::new(0, 1));
    let proof = homes.prove_no_live_loan(data)?;
    homes.remote_release(second, release, proof)?;
    assert!(homes.prove_no_live_loan(data).is_err());
    assert!(homes.begin_loan(data).is_err());
    let drained = homes.drain_releases(second, 1)?;
    assert_eq!(drained.len(), 1);
    assert_eq!(homes.release_queue_count(), 0);
    assert_eq!(homes.pending_release_count(), 1);
    assert!(homes
        .complete_release(drained[0], || {
            Err(ResourceError::new("teardown", "injected failure"))
        })
        .is_err());
    assert_eq!(homes.pending_release_count(), 1);
    homes.complete_release(drained[0], || Ok(()))?;
    assert_eq!(homes.pending_release_count(), 0);
    assert!(homes.home(data).is_err());
    assert_eq!(
        homes.metrics(),
        OwnerMetrics {
            transfers: 1,
            remote_releases: 1,
        }
    );
    Ok(())
}

#[test]
fn zero_capacity_and_failed_processing_preserve_exact_state() -> ResourceResult<()> {
    let owner = DataOwnerId::new(0, 1);
    let home = WorkerId::new(0, 1);
    let release = RemoteRelease::new(owner, TaskId::new(0, 1));
    let mut zero = OwnerHomeTable::new(1, 0);
    zero.insert(owner, home)?;
    let proof = zero.prove_no_live_loan(owner)?;
    assert_eq!(
        zero.remote_release(home, release, proof)
            .map_err(|error| error.code),
        Err("release-capacity")
    );
    assert_eq!(zero.release_queue_count(), 0);
    assert_eq!(zero.pending_release_count(), 0);

    let mut homes = OwnerHomeTable::new(1, 1);
    homes.insert(owner, home)?;
    let proof = homes.prove_no_live_loan(owner)?;
    homes.remote_release(home, release, proof)?;
    assert!(homes
        .process_releases(home, 1, |_| {
            Err(ResourceError::new("teardown", "injected failure"))
        })
        .is_err());
    assert_eq!(homes.release_queue_count(), 1);
    assert_eq!(homes.pending_release_count(), 1);
    assert_eq!(homes.process_releases(home, 1, |_| Ok(()))?, 1);
    assert_eq!(homes.release_queue_count(), 0);
    assert_eq!(homes.pending_release_count(), 0);
    Ok(())
}

#[test]
fn drained_worker_queues_do_not_accumulate() -> ResourceResult<()> {
    let mut homes = OwnerHomeTable::new(8, 1);
    for slot in 0..8 {
        let owner = DataOwnerId::new(slot, 1);
        let home = WorkerId::new(slot, 1);
        homes.insert(owner, home)?;
        let proof = homes.prove_no_live_loan(owner)?;
        homes.remote_release(home, RemoteRelease::new(owner, TaskId::new(slot, 1)), proof)?;
        let release = homes.drain_releases(home, 1)?.pop().ok_or_else(|| {
            ResourceError::new("release-missing", "queued release was not drained")
        })?;
        homes.complete_release(release, || Ok(()))?;
        assert_eq!(homes.release_queue_count(), 0);
    }
    assert_eq!(homes.pending_release_count(), 0);
    Ok(())
}

use lkjscript_resource::*;

#[test]
fn unrelated_owner_activity_does_not_stale_a_transfer_proof() -> ResourceResult<()> {
    let first = DataOwnerId::new(1, 1);
    let second = DataOwnerId::new(2, 1);
    let worker = WorkerId::new(0, 1);
    let destination = WorkerId::new(1, 1);
    let mut homes = OwnerHomeTable::new(2, 2);
    homes.insert(first, worker)?;
    homes.insert(second, worker)?;
    let proof = homes.prove_no_live_loan(first)?;
    homes.begin_loan(second)?;
    homes.end_loan(second)?;
    homes.move_owner(first, destination, proof)?;
    assert_eq!(homes.home(first)?, destination);
    assert_eq!(homes.metrics().transfers, 1);
    Ok(())
}

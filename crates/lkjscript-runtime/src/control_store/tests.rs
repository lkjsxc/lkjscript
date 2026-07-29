use std::error::Error;

use lkjscript_host::{DurableStorage, FakeDurableStorage, StorageFault};

use super::*;

#[test]
fn first_boot_commit_checkpoint_and_reopen_are_exact() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let mut store = ControlStore::open(storage.clone())?;
    assert_eq!(store.sequence(), 0);
    assert_eq!(store.put("system/clean".into(), b"false".to_vec()), Ok(1));
    assert_eq!(
        store.put("application/1/owner".into(), b"1000".to_vec()),
        Ok(2)
    );
    assert_eq!(store.checkpoint(), Ok(()));
    storage.crash();
    let reopened = ControlStore::open(storage)?;
    assert_eq!(reopened.sequence(), 2);
    assert_eq!(reopened.get("system/clean"), Some(b"false".as_slice()));
    assert_eq!(
        reopened.get("application/1/owner"),
        Some(b"1000".as_slice())
    );
    Ok(())
}

#[test]
fn truncated_final_record_is_repaired_before_future_append() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let mut store = ControlStore::open(storage.clone())?;
    assert_eq!(
        store.put("application/1/state".into(), b"installed".to_vec()),
        Ok(1)
    );
    assert_eq!(
        store.put("application/1/quota".into(), b"bounded".to_vec()),
        Ok(2)
    );
    drop(store);
    storage.truncate_durable(JOURNAL, 7);
    storage.crash();
    let mut recovered = ControlStore::open(storage.clone())?;
    assert!(recovered.recovery_report().repaired_truncated_tail);
    assert_eq!(recovered.sequence(), 1);
    assert_eq!(recovered.get("application/1/quota"), None);
    assert_eq!(
        recovered.put("application/1/health".into(), b"ready".to_vec()),
        Ok(2)
    );
    storage.crash();
    let reopened = ControlStore::open(storage)?;
    assert_eq!(reopened.sequence(), 2);
    assert_eq!(
        reopened.get("application/1/health"),
        Some(b"ready".as_slice())
    );
    Ok(())
}

#[test]
fn corrupt_complete_record_fails_closed() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let mut store = ControlStore::open(storage.clone())?;
    assert_eq!(store.put("system/clean".into(), b"false".to_vec()), Ok(1));
    drop(store);
    storage.corrupt_durable_tail(JOURNAL);
    storage.crash();
    assert!(matches!(
        ControlStore::open(storage),
        Err(ControlStoreError::Corrupt("checksum"))
    ));
    Ok(())
}

#[test]
fn sync_failure_publishes_no_fact_and_retry_is_not_duplicated() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let mut store = ControlStore::open(storage.clone())?;
    storage.inject(StorageFault::SyncFailure);
    assert!(matches!(
        store.put("system/clean".into(), b"false".to_vec()),
        Err(ControlStoreError::Host(_))
    ));
    assert_eq!(store.sequence(), 0);
    assert_eq!(store.get("system/clean"), None);
    assert_eq!(store.put("system/clean".into(), b"true".to_vec()), Ok(1));
    storage.crash();
    let reopened = ControlStore::open(storage)?;
    assert_eq!(reopened.sequence(), 1);
    assert_eq!(reopened.get("system/clean"), Some(b"true".as_slice()));
    Ok(())
}

#[test]
fn stale_platform_revision_is_rejected_with_recovery_diagnostic() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let mut store = ControlStore::open(storage.clone())?;
    assert_eq!(store.put("system/clean".into(), b"true".to_vec()), Ok(1));
    drop(store);
    let mut bytes = storage
        .read(JOURNAL)?
        .ok_or("journal must exist after committed mutation")?;
    bytes[8..16].copy_from_slice(&1_u64.to_le_bytes());
    assert_eq!(storage.replace(JOURNAL, &bytes), Ok(()));
    assert!(matches!(
        ControlStore::open(storage),
        Err(ControlStoreError::StaleRevision { found: 1 })
    ));
    Ok(())
}

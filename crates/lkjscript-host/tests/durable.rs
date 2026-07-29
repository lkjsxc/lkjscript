#![allow(clippy::expect_used)]

use lkjscript_host::{DurableStorage, FakeDurableStorage, StorageFault};

#[test]
fn fake_models_short_write_sync_and_crash() {
    let storage = FakeDurableStorage::new();
    storage.inject(StorageFault::ShortWrite(2));
    assert_eq!(storage.append("wal", b"abcd"), Ok(2));
    assert_eq!(storage.sync("wal"), Ok(()));
    assert_eq!(storage.append("wal", b"ef"), Ok(2));
    storage.crash();
    assert_eq!(storage.read("wal"), Ok(Some(b"ab".to_vec())));
}

#[test]
fn fake_models_disk_full_and_sync_failure() {
    let storage = FakeDurableStorage::new();
    storage.inject(StorageFault::DiskFull);
    assert!(storage.append("wal", b"x").is_err());
    storage.append("wal", b"x").expect("append succeeds");
    storage.inject(StorageFault::SyncFailure);
    assert!(storage.sync("wal").is_err());
    storage.crash();
    assert_eq!(storage.read("wal"), Ok(Some(Vec::new())));
}

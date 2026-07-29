#![allow(clippy::expect_used)]

use std::sync::Arc;

use lkjscript_database::{Database, DatabaseLimits, Key, TenantId, Value};
use lkjscript_host::{FakeDurableStorage, StorageFault};

fn tenant() -> TenantId {
    TenantId::new(b"tenant".to_vec()).expect("tenant")
}
fn key() -> Key {
    Key::new(b"key".to_vec()).expect("key")
}
fn value() -> Value {
    Value::new(b"value".to_vec()).expect("value")
}

fn commit_one(database: &Database) {
    let mut write = database.begin_write().expect("writer");
    write.put(tenant(), key(), value()).expect("put");
    write.commit().expect("commit");
}

#[test]
fn short_write_is_completed_and_reopen_is_idempotent() {
    let storage = Arc::new(FakeDurableStorage::new());
    let database =
        Database::create(storage.clone(), "short", DatabaseLimits::default()).expect("create");
    storage.inject(StorageFault::ShortWrite(3));
    commit_one(&database);
    drop(database);
    storage.crash();
    let reopened =
        Database::open(storage.clone(), "short", DatabaseLimits::default()).expect("open");
    assert_eq!(
        reopened.begin_read().expect("read").get(&tenant(), &key()),
        Some(value())
    );
    drop(reopened);
    let again = Database::open(storage, "short", DatabaseLimits::default()).expect("open again");
    assert_eq!(
        again.begin_read().expect("read").get(&tenant(), &key()),
        Some(value())
    );
}

#[test]
fn sync_failure_and_disk_full_never_publish() {
    for fault in [StorageFault::SyncFailure, StorageFault::DiskFull] {
        let storage = Arc::new(FakeDurableStorage::new());
        let database =
            Database::create(storage.clone(), "fault", DatabaseLimits::default()).expect("create");
        storage.inject(fault);
        let mut write = database.begin_write().expect("writer");
        write.put(tenant(), key(), value()).expect("put");
        assert!(write.commit().is_err());
        storage.crash();
        drop(database);
        let reopened = Database::open(storage, "fault", DatabaseLimits::default()).expect("reopen");
        assert_eq!(
            reopened.begin_read().expect("read").get(&tenant(), &key()),
            None
        );
    }
}

#[test]
fn torn_and_truncated_commit_frames_discard_uncommitted_work() {
    for truncate in [false, true] {
        let storage = Arc::new(FakeDurableStorage::new());
        let name = if truncate { "truncated" } else { "torn" };
        let database =
            Database::create(storage.clone(), name, DatabaseLimits::default()).expect("create");
        commit_one(&database);
        drop(database);
        let wal = format!("{name}.wal");
        if truncate {
            storage.truncate_durable(&wal, 2);
        } else {
            storage.corrupt_durable_tail(&wal);
        }
        let reopened = Database::open(storage, name, DatabaseLimits::default()).expect("reopen");
        assert!(reopened.recovery_report().damaged_tail_discarded);
        assert_eq!(
            reopened
                .recovery_report()
                .uncommitted_transactions_discarded,
            1
        );
        assert_eq!(
            reopened.begin_read().expect("read").get(&tenant(), &key()),
            None
        );
    }
}

#[test]
fn close_checkpoints_and_clears_wal_atomically() {
    let storage = Arc::new(FakeDurableStorage::new());
    let database =
        Database::create(storage.clone(), "checkpoint", DatabaseLimits::default()).expect("create");
    commit_one(&database);
    database.close().expect("close");
    storage.crash();
    let reopened =
        Database::open(storage, "checkpoint", DatabaseLimits::default()).expect("reopen");
    assert_eq!(
        reopened.begin_read().expect("read").get(&tenant(), &key()),
        Some(value())
    );
    assert_eq!(reopened.recovery_report(), Default::default());
}

#![allow(clippy::expect_used)]

use std::sync::Arc;

use lkjscript_database::{Database, DatabaseLimits, Key, TenantId, Value};
use lkjscript_host::FakeDurableStorage;

fn tenant(name: &[u8]) -> TenantId {
    TenantId::new(name.to_vec()).expect("valid tenant")
}
fn key(name: &[u8]) -> Key {
    Key::new(name.to_vec()).expect("valid key")
}
fn value(bytes: &[u8]) -> Value {
    Value::new(bytes.to_vec()).expect("valid value")
}

#[test]
fn snapshots_single_writer_and_tenant_ranges_are_exact() {
    let storage = Arc::new(FakeDurableStorage::new());
    let database = Database::create(storage, "main", DatabaseLimits::default()).expect("create");
    let alpha = tenant(b"alpha");
    let beta = tenant(b"beta");
    let before = database.begin_read().expect("snapshot");
    let mut write = database.begin_write().expect("writer");
    assert!(database.begin_write().is_err());
    write
        .put(alpha.clone(), key(b"b"), value(b"2"))
        .expect("put");
    write
        .put(alpha.clone(), key(b"a"), value(b"1"))
        .expect("put");
    write
        .put(beta.clone(), key(b"a"), value(b"other"))
        .expect("put");
    write.commit().expect("commit");
    assert_eq!(before.get(&alpha, &key(b"a")), None);
    let after = database.begin_read().expect("snapshot");
    let range = after
        .range(&alpha, Some(b"a"), Some(b"c"), 10)
        .expect("range");
    let pairs: Vec<_> = range
        .iter()
        .map(|(key, value)| (key.as_bytes(), value.as_bytes()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            (b"a".as_slice(), b"1".as_slice()),
            (b"b".as_slice(), b"2".as_slice())
        ]
    );
}

#[test]
fn abort_releases_writer_without_publishing() {
    let storage = Arc::new(FakeDurableStorage::new());
    let database = Database::create(storage, "abort", DatabaseLimits::default()).expect("create");
    let tenant = tenant(b"tenant");
    let mut write = database.begin_write().expect("writer");
    write
        .put(tenant.clone(), key(b"key"), value(b"value"))
        .expect("put");
    write.abort().expect("abort");
    assert_eq!(
        database
            .begin_read()
            .expect("read")
            .get(&tenant, &key(b"key")),
        None
    );
    database
        .begin_write()
        .expect("new writer")
        .abort()
        .expect("abort");
}

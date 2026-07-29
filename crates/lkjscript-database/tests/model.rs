#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::sync::Arc;

use lkjscript_database::{Database, DatabaseLimits, Key, TenantId, Value};
use lkjscript_host::FakeDurableStorage;

#[test]
fn deterministic_operations_match_independent_btree_model() {
    let storage = Arc::new(FakeDurableStorage::new());
    let database = Database::create(storage, "model", DatabaseLimits::default()).expect("create");
    let tenant = TenantId::new(b"model".to_vec()).expect("tenant");
    let mut model = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    let mut state = 0x5eed_u64;
    for step in 0..300u64 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let raw_key = format!("k{:02}", (state >> 32) % 40).into_bytes();
        let key = Key::new(raw_key.clone()).expect("key");
        let mut write = database.begin_write().expect("write");
        if state & 3 == 0 {
            write.delete(tenant.clone(), key).expect("delete");
            model.remove(&raw_key);
        } else {
            let raw_value = step.to_le_bytes().to_vec();
            write
                .put(
                    tenant.clone(),
                    key,
                    Value::new(raw_value.clone()).expect("value"),
                )
                .expect("put");
            model.insert(raw_key, raw_value);
        }
        write.commit().expect("commit");
        let actual = database
            .begin_read()
            .expect("read")
            .range(&tenant, None, None, 4096)
            .expect("range");
        let actual: Vec<_> = actual
            .into_iter()
            .map(|(key, value)| (key.as_bytes().to_vec(), value.as_bytes().to_vec()))
            .collect();
        let expected: Vec<_> = model
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        assert_eq!(actual, expected);
    }
}

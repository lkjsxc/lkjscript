//! Physical payload identity is independent of the map's work counters.
use super::{NormalizedMapKey as Key, NormalizedValue as Value};
use crate::platform::execution::normalized::map::Map;

fn bytes(value: &Value) -> &[u8] {
    match value {
        Value::Bytes(value) => value,
        Value::Text(value) | Value::StaticText(value) => value.as_bytes(),
        _ => panic!("expected a byte-backed primitive"),
    }
}

fn key_bytes(key: &Key) -> &[u8] {
    match key {
        Key::Bytes(value) => value,
        Key::Text(value) => value.as_bytes(),
        _ => panic!("expected a byte-backed key"),
    }
}

#[test]
fn map_keys_reuse_immutable_payload_across_value_roundtrip() {
    for input in [
        Value::bytes(vec![0, 1, 255, 128]),
        Value::text("shared 日本語".repeat(128)),
        Value::static_text("static 日本語"),
    ] {
        let retained = input.clone();
        let address = bytes(&retained).as_ptr();
        let key = Key::from_value(input).unwrap();
        assert_eq!(key_bytes(&key), bytes(&retained));
        assert_eq!(key_bytes(&key).as_ptr(), address, "value to key copied");
        let cloned = key.clone();
        assert_eq!(key_bytes(&cloned).as_ptr(), address, "key clone copied");
        let projected = key.to_value();
        assert_eq!(bytes(&projected), bytes(&retained));
        assert_eq!(bytes(&projected).as_ptr(), address, "key projection copied");
        drop(key);
        drop(cloned);
        assert_eq!(bytes(&projected), bytes(&retained));
    }
}

#[test]
fn map_keys_share_payload_across_persistent_updates() {
    for input in [
        Value::bytes(vec![17; 65_536]),
        Value::text("日本語".repeat(8_192)),
    ] {
        let address = bytes(&input).as_ptr();
        let key = Key::from_value(input.clone()).unwrap();
        let original = Map::default()
            .insert(key.clone(), Value::I64(11), 1, &mut |_| Ok(()))
            .unwrap();
        let replacement = original
            .insert(key.clone(), Value::I64(22), 1, &mut |_| Ok(()))
            .unwrap();
        let removed = replacement.remove(&key, &mut |_| Ok(())).unwrap();
        assert!(removed.is_empty());
        assert_eq!(original.get(&key), Some(&Value::I64(11)));
        assert_eq!(replacement.get(&key), Some(&Value::I64(22)));
        for map in [&original, &replacement] {
            let stored = map.keys().next().unwrap();
            assert_eq!(key_bytes(stored), bytes(&input));
            assert_eq!(key_bytes(stored).as_ptr(), address, "map copied its key");
            assert_eq!(bytes(&stored.to_value()).as_ptr(), address);
        }
    }
}

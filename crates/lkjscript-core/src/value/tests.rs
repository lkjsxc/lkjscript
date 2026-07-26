#![allow(clippy::expect_used)]

use super::{CapabilityKind, Value, MAX_SMALL_I64, MIN_SMALL_I64};

#[test]
fn semantic_singletons_are_distinct_from_invalid() {
    assert!(Value::INVALID.is_invalid());
    assert!(Value::UNIT.is_unit());
    assert!(!Value::UNIT.is_invalid());
    assert!(Value::EMPTY_LIST.is_empty_list());
    assert!(!Value::EMPTY_LIST.is_unit());
    assert!(!Value::EMPTY_LIST.is_invalid());
    assert_ne!(Value::UNIT, Value::EMPTY_LIST);
}

#[test]
fn closed_capabilities_round_trip_without_aliasing_other_values() {
    for kind in CapabilityKind::ALL {
        let value = Value::from_capability(kind);
        assert_eq!(value.as_capability(), Some(kind));
        assert!(value.as_small_i64().is_none());
        assert!(value.as_handle().is_none());
        assert!(value.as_heap().is_none());
    }
}

#[test]
fn small_integer_boundaries_round_trip_without_truncation() {
    for number in [MIN_SMALL_I64, -1, 0, 1, MAX_SMALL_I64] {
        let value = Value::from_small_i64(number).expect("representable small I64");
        assert_eq!(value.as_small_i64(), Some(number));
    }
    assert!(Value::from_small_i64(MIN_SMALL_I64 - 1).is_none());
    assert!(Value::from_small_i64(MAX_SMALL_I64 + 1).is_none());
    assert!(Value::from_small_i64(i64::MIN).is_none());
    assert!(Value::from_small_i64(i64::MAX).is_none());
}

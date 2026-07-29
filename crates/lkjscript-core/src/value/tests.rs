use super::{CapabilityKind, Value};

#[test]
fn closed_value_representation_is_sixteen_bytes() {
    assert_eq!(std::mem::size_of::<Value>(), 16);
    assert_eq!(std::mem::align_of::<Value>(), 8);
}

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
fn closed_capabilities_round_trip_without_aliasing_other_categories() {
    for kind in CapabilityKind::ALL {
        let value = Value::from_capability(kind);
        assert_eq!(value.as_capability(), Some(kind));
        assert!(value.as_i64().is_none());
        assert!(value.as_resource().is_none());
        assert!(value.as_legacy_traced().is_none());
    }
}

#[test]
fn reference_and_key_categories_remain_disjoint_for_equal_payloads() {
    let resource = Value::from_resource(7);
    let traced = Value::from_legacy_traced(7);
    let vector = Value::from_byte_vector_key(7);
    assert_eq!(resource.as_resource(), Some(7));
    assert_eq!(traced.as_legacy_traced(), Some(7));
    assert_eq!(vector.as_byte_vector_key(), Some(7));
    assert_ne!(resource, traced);
    assert_ne!(resource, vector);
    assert_ne!(traced, vector);
}

#[test]
fn function_prototypes_round_trip_inline_without_category_aliasing() {
    for prototype in [0, 1, u32::MAX] {
        let value = Value::from_function(prototype);
        assert_eq!(value.as_function(), Some(prototype));
        assert!(value.as_i64().is_none());
        assert!(value.as_resource().is_none());
        assert!(value.as_legacy_traced().is_none());
        assert_eq!(format!("{value:?}"), format!("function#{prototype}"));
    }
}

#[test]
fn symbol_constants_round_trip_without_reference_aliasing() {
    for constant in [0, 1, u32::MAX] {
        let value = Value::from_symbol(constant);
        assert_eq!(value.as_symbol(), Some(constant));
        assert!(value.as_i64().is_none());
        assert!(value.as_legacy_traced().is_none());
        assert_eq!(format!("{value:?}"), format!("symbol#{constant}"));
    }
}

#[test]
fn complete_i64_range_round_trips_inline() {
    for number in [
        i64::MIN,
        i64::MIN + 1,
        -9_007_199_254_740_993,
        -1,
        0,
        1,
        9_007_199_254_740_993,
        i64::MAX - 1,
        i64::MAX,
    ] {
        let value = Value::from_i64(number);
        assert_eq!(value.as_i64(), Some(number));
        assert!(value.as_f64_bits().is_none());
        assert!(value.as_legacy_traced().is_none());
    }
}

#[test]
fn exact_f64_bits_round_trip_inline() {
    for bits in [
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0x7ff8_0000_0000_0042,
        0xfff8_dead_beef_cafe,
        u64::MAX,
    ] {
        let value = Value::from_f64_bits(bits);
        assert_eq!(value.as_f64_bits(), Some(bits));
        assert_eq!(value.as_f64().map(f64::to_bits), Some(bits));
        assert!(value.as_i64().is_none());
        assert!(value.as_legacy_traced().is_none());
    }
}

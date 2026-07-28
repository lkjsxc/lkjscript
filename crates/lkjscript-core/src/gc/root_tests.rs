#![allow(clippy::expect_used)]

use super::GcHeap;
use crate::{CapabilityKind, HeapObj, Value};

#[test]
fn only_explicit_legacy_traced_values_retain_heap_objects() {
    let non_roots = [
        Value::from_i64(0),
        Value::from_f64_bits(0),
        Value::from_resource(0),
        Value::from_capability(CapabilityKind::Arguments),
        Value::from_opaque_unique_key(0),
        Value::UNIT,
        Value::FALSE,
        Value::EMPTY_LIST,
    ];
    for root in non_roots {
        let mut heap = GcHeap::default();
        let object = heap
            .alloc(HeapObj::Str("not-rooted".into()))
            .expect("object allocation");
        assert_eq!(object.as_legacy_traced(), Some(0));
        heap.collect(&[root]);
        assert!(heap.get(object).is_err(), "unexpected root {root:?}");
    }

    let mut heap = GcHeap::default();
    let object = heap
        .alloc(HeapObj::Str("rooted".into()))
        .expect("object allocation");
    heap.collect(&[object]);
    assert!(heap.get(object).is_ok());
}

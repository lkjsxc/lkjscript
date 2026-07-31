#![allow(clippy::expect_used)]

use super::*;
use crate::{HeapObj, RuntimeLayoutId};

#[test]
fn boxed_enum_traces_only_active_initialized_payload() {
    let mut heap = GcHeap::default();
    let active = heap
        .alloc(HeapObj::Pair {
            car: crate::Value::from_i64(1),
            cdr: crate::Value::EMPTY_LIST,
        })
        .expect("active child");
    let inactive = heap
        .alloc(HeapObj::Pair {
            car: crate::Value::from_i64(2),
            cdr: crate::Value::EMPTY_LIST,
        })
        .expect("inactive child");
    let value = heap
        .alloc(HeapObj::Enum {
            layout: RuntimeLayoutId::new([7; 32]),
            physical_tag: 3,
            active_payload: vec![active],
        })
        .expect("enum");
    heap.collect(&[value]);
    assert!(heap.get(active).is_ok());
    assert!(heap.get(inactive).is_err());
    assert!(matches!(
        heap.get(value),
        Ok(HeapObj::Enum { physical_tag: 3, active_payload, .. }) if active_payload == &[active]
    ));
}

#[test]
fn enum_preflight_reserves_exactly_one_object() {
    let allowed = GcHeap::new(GcConfig {
        max_allocations: 1,
        ..GcConfig::default()
    });
    assert_eq!(allowed.preflight_enum_allocation(1), Ok(()));
    assert_eq!(allowed.total_allocations(), 0);

    let denied = GcHeap::new(GcConfig {
        max_allocations: 0,
        ..GcConfig::default()
    });
    assert_eq!(
        denied.preflight_enum_allocation(1),
        Err(GcLimit::Allocations)
    );
    assert_eq!(denied.total_allocations(), 0);
}

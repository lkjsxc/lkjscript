#![allow(clippy::expect_used)]

use super::*;
use crate::{HeapObj, RuntimeLayoutId};

#[test]
fn boxed_enum_traces_only_active_initialized_payload() {
    let mut heap = GcHeap::default();
    let active = heap
        .alloc(HeapObj::Str("active".into()))
        .expect("active child");
    let inactive = heap
        .alloc(HeapObj::Str("inactive".into()))
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
fn enum_preflight_reserves_complete_scalar_box_and_object_count() {
    let heap = GcHeap::new(GcConfig {
        max_allocations: 2,
        ..GcConfig::default()
    });
    assert_eq!(heap.preflight_enum_allocations(1, 1), Ok(()));
    assert_eq!(
        heap.preflight_enum_allocations(2, 1),
        Err(GcLimit::Allocations)
    );
    assert_eq!(heap.total_allocations(), 0);
}

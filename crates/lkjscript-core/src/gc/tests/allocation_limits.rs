#![allow(clippy::expect_used)]

use super::*;

#[test]
fn bounded_publication_rejects_without_partial_object() {
    let mut heap = GcHeap::new(GcConfig {
        max_allocations: 1,
        ..GcConfig::default()
    });
    assert!(heap
        .try_alloc(HeapObj::Pair {
            car: Value::from_i64(1),
            cdr: Value::EMPTY_LIST,
        })
        .is_ok());
    assert_eq!(
        heap.try_alloc(HeapObj::Pair {
            car: Value::from_i64(2),
            cdr: Value::EMPTY_LIST,
        }),
        Err(GcLimit::Allocations)
    );
    assert_eq!(heap.total_allocations(), 1);
    assert!(matches!(
        heap.alloc(HeapObj::Pair {
            car: Value::from_i64(3),
            cdr: Value::EMPTY_LIST,
        }),
        Err(ref error)
            if error.class()
                == crate::ErrorClass::Resource(ResourceLimitKind::Allocations)
    ));
    assert_eq!(heap.total_allocations(), 1);
}

#[test]
fn swept_same_layout_handle_is_never_reused() {
    let mut heap = GcHeap::default();
    let stale = heap
        .try_alloc_with_layout(
            HeapObj::Pair {
                car: Value::from_i64(1),
                cdr: Value::EMPTY_LIST,
            },
            77,
        )
        .expect("first typed allocation");
    heap.collect(&[]);
    let current = heap
        .try_alloc_with_layout(
            HeapObj::Pair {
                car: Value::from_i64(2),
                cdr: Value::EMPTY_LIST,
            },
            77,
        )
        .expect("second typed allocation");
    assert_ne!(stale.as_legacy_traced(), current.as_legacy_traced());
    assert!(heap.get(stale).is_err());
    assert!(matches!(
        heap.get(current),
        Ok(HeapObj::Pair { car, cdr })
            if car.as_i64() == Some(2) && cdr.is_empty_list()
    ));
}

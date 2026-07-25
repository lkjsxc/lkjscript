#![allow(clippy::expect_used)]

use super::{allocation::stable_index_available, *};
use crate::{Error, HeapObj, ResourceLimitKind, Result, Value};

#[test]
fn collection_preserves_nested_graph_and_reports_exact_counters() {
    let mut heap = GcHeap::new(GcConfig {
        collect_before_every_allocation: true,
        ..GcConfig::default()
    });
    let payload = heap
        .alloc(HeapObj::Str("nested".into()))
        .expect("payload allocation");
    let list = heap
        .alloc(HeapObj::Pair {
            car: payload,
            cdr: Value::EMPTY_LIST,
        })
        .expect("list allocation");
    let result = heap
        .alloc(HeapObj::ResultOk(list))
        .expect("result allocation");
    let option = heap
        .alloc(HeapObj::OptionSome(result))
        .expect("option allocation");
    let product = heap
        .alloc(HeapObj::Product {
            product: crate::ProductId::new(0),
            fields: vec![option],
        })
        .expect("product allocation");
    heap.collect(&[product]);
    for value in [payload, list, result, option, product] {
        assert!(heap.get(value).is_ok());
    }
    assert_eq!(heap.total_allocations(), 5);
    assert_eq!(heap.collections(), 1);
    assert!(heap.total_allocated_bytes() >= heap.heap_bytes() as u64);
    assert!(heap.peak_live_heap_bytes() >= heap.heap_bytes());
}

#[test]
fn stable_index_boundary_model_rejects_before_duplicate_u32_handles() {
    let last_valid_slot_count = u32::MAX as usize;
    assert!(stable_index_available(last_valid_slot_count));
    if let Some(exhausted_slot_count) = last_valid_slot_count.checked_add(1) {
        assert!(!stable_index_available(exhausted_slot_count));
    }
}

#[test]
fn bounded_publication_rejects_without_partial_object() {
    let mut heap = GcHeap::new(GcConfig {
        max_allocations: 1,
        ..GcConfig::default()
    });
    assert!(heap.try_alloc(HeapObj::Str("one".into())).is_ok());
    assert_eq!(
        heap.try_alloc(HeapObj::Str("two".into())),
        Err(GcLimit::Allocations)
    );
    assert_eq!(heap.total_allocations(), 1);
    assert!(matches!(
        heap.alloc(HeapObj::Str("compatibility".into())),
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
        .try_alloc_with_layout(HeapObj::Buf(vec![1]), 77)
        .expect("first typed allocation");
    heap.collect(&[]);
    let current = heap
        .try_alloc_with_layout(HeapObj::Buf(vec![2]), 77)
        .expect("second typed allocation");
    assert_ne!(stale.as_heap(), current.as_heap());
    assert!(heap.get(stale).is_err());
    assert!(matches!(heap.get(current), Ok(HeapObj::Buf(bytes)) if bytes == &[2]));
}

#[test]
fn mutation_growth_is_bounded_and_failure_rolls_back_every_object_kind() {
    for object in [
        HeapObj::Buf(vec![1]),
        HeapObj::Str("x".into()),
        HeapObj::Product {
            product: crate::ProductId::new(0),
            fields: vec![Value::UNIT],
        },
    ] {
        let mut heap = GcHeap::default();
        let value = heap.alloc(object.clone()).expect("test object allocation");
        heap.set_config(GcConfig {
            max_heap_bytes: heap.heap_bytes(),
            ..heap.config()
        });
        let result = heap.mutate(value, |current| {
            match current {
                HeapObj::Buf(bytes) => bytes.extend_from_slice(&[2; 128]),
                HeapObj::Str(text) => text.push_str(&"y".repeat(128)),
                HeapObj::Product { fields, .. } => fields.extend([Value::UNIT; 128]),
                _ => return Err(Error::msg("unexpected test object")),
            }
            Ok(())
        });
        assert!(matches!(
            result,
            Err(ref error)
                if error.class() == crate::ErrorClass::Resource(ResourceLimitKind::HeapBytes)
        ));
        assert_eq!(heap.get(value).ok(), Some(&object));
    }

    let mut heap = GcHeap::default();
    let value = heap
        .alloc(HeapObj::Buf(vec![1]))
        .expect("growth buffer allocation");
    let before_growth = heap.stats();
    heap.mutate(value, |object| {
        let HeapObj::Buf(bytes) = object else {
            return Err(Error::msg("unexpected test object"));
        };
        bytes.extend_from_slice(&[2; 128]);
        Ok(())
    })
    .expect("bounded mutation growth");
    let after_growth = heap.stats();
    assert!(after_growth.live_heap_bytes > before_growth.live_heap_bytes);
    assert!(after_growth.allocated_bytes > before_growth.allocated_bytes);
    assert!(after_growth.peak_live_heap_bytes >= after_growth.live_heap_bytes);

    let mut heap = GcHeap::default();
    let value = heap
        .alloc(HeapObj::Buf(vec![1, 2]))
        .expect("rollback buffer allocation");
    let before = heap.stats();
    let result: Result<()> = heap.mutate(value, |object| {
        let HeapObj::Buf(bytes) = object else {
            return Err(Error::msg("unexpected test object"));
        };
        bytes.push(3);
        Err(Error::msg("reject mutation"))
    });
    assert!(result.is_err());
    assert_eq!(heap.get(value).ok(), Some(&HeapObj::Buf(vec![1, 2])));
    assert_eq!(heap.stats(), before);

    let result = heap.mutate(value, |object| {
        *object = HeapObj::Str("wrong layout".into());
        Ok(())
    });
    assert!(
        matches!(result, Err(ref error) if error.as_str() == "heap mutation changed object layout")
    );
    assert_eq!(heap.get(value).ok(), Some(&HeapObj::Buf(vec![1, 2])));
    assert_eq!(heap.stats(), before);
}

#[test]
fn snapshot_clones_only_transitively_reachable_objects() {
    let mut heap = GcHeap::default();
    let child = heap
        .alloc(HeapObj::Str("child".into()))
        .expect("snapshot child allocation");
    let root = heap
        .alloc(HeapObj::OptionSome(child))
        .expect("snapshot root allocation");
    let _unreachable = heap
        .alloc(HeapObj::Str("unreachable".into()))
        .expect("snapshot unreachable allocation");
    let snapshot = heap.snapshot(root).expect("reachable snapshot");
    assert_eq!(snapshot.snapshot_object_count(), 2);
    assert!(heap.get(root).is_ok());
    assert_eq!(heap.total_allocations(), 3);
}

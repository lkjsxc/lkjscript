#![allow(clippy::expect_used)]

use super::*;

#[test]
fn collection_preserves_nested_graph_and_reports_exact_counters() {
    let mut heap = GcHeap::new(GcConfig {
        collect_before_every_allocation: true,
        ..GcConfig::default()
    });
    let payload = heap
        .alloc(HeapObj::Pair {
            car: Value::from_i64(7),
            cdr: Value::EMPTY_LIST,
        })
        .expect("payload allocation");
    let list = heap
        .alloc(HeapObj::Pair {
            car: payload,
            cdr: Value::EMPTY_LIST,
        })
        .expect("list allocation");
    let result = heap
        .alloc(HeapObj::Enum {
            layout: crate::RuntimeLayoutId::new(crate::RESULT_LAYOUT),
            physical_tag: 0,
            active_payload: vec![list],
        })
        .expect("generic Result allocation");
    let option = heap
        .alloc(HeapObj::Enum {
            layout: crate::RuntimeLayoutId::new(crate::OPTION_LAYOUT),
            physical_tag: 0,
            active_payload: vec![result],
        })
        .expect("generic Option allocation");
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

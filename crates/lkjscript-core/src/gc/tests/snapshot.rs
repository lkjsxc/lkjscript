#![allow(clippy::expect_used)]

use super::*;

#[test]
fn snapshot_clones_only_transitively_reachable_objects() {
    let mut heap = GcHeap::default();
    let child = heap
        .alloc(HeapObj::Pair {
            car: Value::from_i64(1),
            cdr: Value::EMPTY_LIST,
        })
        .expect("snapshot child allocation");
    let root = heap
        .alloc(HeapObj::Enum {
            layout: crate::RuntimeLayoutId::new(crate::OPTION_LAYOUT),
            physical_tag: 0,
            active_payload: vec![child],
        })
        .expect("generic snapshot root allocation");
    let _unreachable = heap
        .alloc(HeapObj::Pair {
            car: Value::from_i64(2),
            cdr: Value::EMPTY_LIST,
        })
        .expect("snapshot unreachable allocation");
    let snapshot = heap.snapshot(root).expect("reachable snapshot");
    assert_eq!(snapshot.snapshot_object_count(), 2);
    assert!(heap.get(root).is_ok());
    assert_eq!(heap.total_allocations(), 3);
}

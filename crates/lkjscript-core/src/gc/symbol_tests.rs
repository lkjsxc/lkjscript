#![allow(clippy::expect_used)]

use super::*;
use crate::{Error, HeapObj, Value};

#[test]
fn returned_snapshots_retain_reachable_symbol_text_without_heap_objects() {
    let heap = GcHeap::default();
    let symbol = Value::from_symbol(3);
    let owned = heap
        .snapshot(symbol)
        .and_then(|value| {
            value.retain_symbols(|index| {
                (index == 3)
                    .then_some("retained")
                    .ok_or_else(|| Error::msg("unexpected symbol"))
            })
        })
        .expect("owned symbol");
    assert_eq!(owned.as_str(), Some("retained"));
    assert_eq!(owned.snapshot_object_count(), 0);
    let same_text = heap
        .snapshot(Value::from_symbol(9))
        .and_then(|value| value.retain_symbols(|_| Ok("retained")))
        .expect("same owned symbol text");
    assert_eq!(owned, same_text);

    let mut heap = GcHeap::default();
    let root = heap
        .alloc(HeapObj::Enum {
            layout: crate::RuntimeLayoutId::new(crate::OPTION_LAYOUT),
            physical_tag: 0,
            active_payload: vec![symbol],
        })
        .expect("symbol enum");
    let mut resolved = Vec::new();
    let owned = heap
        .snapshot(root)
        .and_then(|value| {
            value.retain_symbols(|index| {
                resolved.push(index);
                Ok("retained")
            })
        })
        .expect("nested owned symbol");
    assert_eq!(resolved, vec![3]);
    assert_eq!(owned.snapshot_object_count(), 1);
}

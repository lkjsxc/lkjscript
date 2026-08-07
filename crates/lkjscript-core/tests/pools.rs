#![allow(clippy::expect_used)]

use std::cell::RefCell;
use std::num::NonZeroU64;
use std::rc::Rc;

use lkjscript_core::{
    LayoutIdentity, PoolId, SemanticTypeIdentity, StructuralError, StructuralRuntime, TypedPool,
};

fn ids(value: u64) -> (LayoutIdentity, SemanticTypeIdentity) {
    (
        LayoutIdentity::new(NonZeroU64::new(value).expect("nonzero")),
        SemanticTypeIdentity::new(NonZeroU64::new(value + 1).expect("nonzero")),
    )
}

fn runtime() -> StructuralRuntime {
    StructuralRuntime::new().expect("runtime")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Node {
    value: u32,
    next: Option<PoolId<Node>>,
}

#[test]
fn typed_pool_owns_a_cyclic_mutable_graph_and_rejects_stale_ids() {
    let mut runtime = runtime();
    let (layout, semantic_type) = ids(70);
    let mut pool = TypedPool::new(&mut runtime, layout, semantic_type).expect("pool");
    let first = pool
        .insert(Node {
            value: 1,
            next: None,
        })
        .expect("first");
    let second = pool
        .insert(Node {
            value: 2,
            next: None,
        })
        .expect("second");
    pool.get_mut(first).expect("first mut").next = Some(second);
    pool.get_mut(second).expect("second mut").next = Some(first);
    assert_eq!(pool.get(first).expect("first node").next, Some(second));
    assert_eq!(pool.remove(first).expect("remove").value, 1);
    assert!(pool.get(first).is_err());
    let replacement = pool
        .insert(Node {
            value: 3,
            next: None,
        })
        .expect("replacement");
    assert_eq!(first.root().slot(), replacement.root().slot());
    assert_ne!(first.root().generation(), replacement.root().generation());
    pool.destroy(&mut runtime).expect("destroy");
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn runtime_identity_and_partition_checks_reject_foreign_ids() {
    let mut first_runtime = runtime();
    let mut second_runtime = runtime();
    let (layout, semantic_type) = ids(80);
    let mut first = TypedPool::new(&mut first_runtime, layout, semantic_type).expect("first");
    let second = TypedPool::<u64>::new(&mut second_runtime, layout, semantic_type).expect("second");
    let id = first.insert(7_u64).expect("value");
    assert_eq!(second.get(id), Err(StructuralError::WrongPool));
    let outside = first.partition(1, 1).expect("empty partition");
    assert_eq!(
        first.get_in_partition(&outside, id),
        Err(StructuralError::WrongPartition)
    );
    first.destroy(&mut first_runtime).expect("destroy first");
    second.destroy(&mut second_runtime).expect("destroy second");
}

#[derive(Debug)]
struct DropTracked(u8, Rc<RefCell<Vec<u8>>>);

impl Drop for DropTracked {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}

#[test]
fn pool_destruction_is_slot_ordered_and_reports_zero_live_state() {
    let mut runtime = runtime();
    let (layout, semantic_type) = ids(90);
    let mut pool = TypedPool::new(&mut runtime, layout, semantic_type).expect("pool");
    let order = Rc::new(RefCell::new(Vec::new()));
    pool.insert(DropTracked(1, Rc::clone(&order))).expect("one");
    pool.insert(DropTracked(2, Rc::clone(&order))).expect("two");
    pool.insert(DropTracked(3, Rc::clone(&order)))
        .expect("three");
    let metrics = pool.destroy(&mut runtime).expect("destroy");
    assert_eq!(*order.borrow(), vec![1, 2, 3]);
    assert_eq!(metrics.live_slots, 0);
    assert_eq!(metrics.bytes_live, 0);
}

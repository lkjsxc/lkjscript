#![allow(clippy::expect_used)]

use std::cell::RefCell;
use std::num::NonZeroU64;
use std::rc::Rc;

use lkjscript_core::{
    LayoutIdentity, PoolId, SemanticTypeIdentity, StructuralError, StructuralLimits,
    StructuralRuntime, TypedPool,
};

fn ids(value: u64) -> (LayoutIdentity, SemanticTypeIdentity) {
    (
        LayoutIdentity::new(NonZeroU64::new(value).expect("nonzero")),
        SemanticTypeIdentity::new(NonZeroU64::new(value + 1).expect("nonzero")),
    )
}

fn runtime() -> StructuralRuntime {
    StructuralRuntime::new(StructuralLimits::default()).expect("runtime")
}

#[test]
fn stale_ids_and_generation_retirement_are_exact() {
    let mut runtime = runtime();
    let (layout, semantic_type) = ids(60);
    let limits = StructuralLimits {
        max_generation: 2,
        ..StructuralLimits::default()
    };
    let mut pool = TypedPool::new(&mut runtime, layout, semantic_type, limits).expect("pool");
    let first = pool.insert(10_u64).expect("first");
    assert_eq!(pool.remove(first).expect("remove first"), 10);
    assert!(pool.get(first).is_err());
    let second = pool.insert(20).expect("second");
    assert_eq!(first.root().slot(), second.root().slot());
    assert_ne!(first.root().generation(), second.root().generation());
    assert_eq!(pool.remove(second).expect("remove second"), 20);
    let third = pool.insert(30).expect("third");
    assert_ne!(second.root().slot(), third.root().slot());
    assert!(pool.get(second).is_err());
    pool.validate().expect("valid pool");
    let metrics = pool.destroy(&mut runtime).expect("destroy");
    assert_eq!(metrics.slots_retired, 1);
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Node {
    value: u32,
    next: Option<PoolId<Node>>,
}

#[test]
fn typed_pool_owns_a_cyclic_mutable_graph() {
    let mut runtime = runtime();
    let (layout, semantic_type) = ids(70);
    let mut pool = TypedPool::new(
        &mut runtime,
        layout,
        semantic_type,
        StructuralLimits::default(),
    )
    .expect("pool");
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
    assert_eq!(pool.get(second).expect("second node").next, Some(first));
    let order = pool.iter().map(|(_, node)| node.value).collect::<Vec<_>>();
    assert_eq!(order, vec![1, 2]);
    pool.destroy(&mut runtime).expect("destroy");
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Position(i32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Velocity(i32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Entity {
    position: PoolId<Position>,
    velocity: PoolId<Velocity>,
}

#[test]
fn runtime_identity_and_partition_checks_reject_foreign_ids() {
    let mut first_runtime = runtime();
    let mut second_runtime = runtime();
    assert_ne!(first_runtime.identity(), second_runtime.identity());
    let (layout, semantic_type) = ids(80);
    let limits = StructuralLimits::default();
    let mut first =
        TypedPool::new(&mut first_runtime, layout, semantic_type, limits).expect("first");
    let second =
        TypedPool::<u64>::new(&mut second_runtime, layout, semantic_type, limits).expect("second");
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
    let mut pool = TypedPool::new(
        &mut runtime,
        layout,
        semantic_type,
        StructuralLimits::default(),
    )
    .expect("pool");
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

#[test]
fn ecs_pools_update_in_deterministic_slot_order() {
    let mut runtime = runtime();
    let (position_layout, position_type) = ids(80);
    let (velocity_layout, velocity_type) = ids(82);
    let (entity_layout, entity_type) = ids(84);
    let limits = StructuralLimits::default();
    let mut positions =
        TypedPool::new(&mut runtime, position_layout, position_type, limits).expect("positions");
    let mut velocities =
        TypedPool::new(&mut runtime, velocity_layout, velocity_type, limits).expect("velocities");
    let mut entities =
        TypedPool::new(&mut runtime, entity_layout, entity_type, limits).expect("entities");
    for value in [3, 1, 2] {
        let position = positions.insert(Position(value)).expect("position");
        let velocity = velocities.insert(Velocity(value * 10)).expect("velocity");
        entities
            .insert(Entity { position, velocity })
            .expect("entity");
    }
    let entity_values = entities.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    for entity in entity_values {
        let delta = velocities.get(entity.velocity).expect("velocity").0;
        positions.get_mut(entity.position).expect("position").0 += delta;
    }
    let values = positions
        .iter()
        .map(|(_, position)| position.0)
        .collect::<Vec<_>>();
    assert_eq!(values, vec![33, 11, 22]);
    let partition = positions.partition(0, 2).expect("partition");
    let first = positions.iter().next().expect("first position").0;
    assert!(positions.get_in_partition(&partition, first).is_ok());
    entities.destroy(&mut runtime).expect("destroy entities");
    velocities
        .destroy(&mut runtime)
        .expect("destroy velocities");
    positions.destroy(&mut runtime).expect("destroy positions");
    assert_eq!(runtime.metrics().live_domains, 0);
}

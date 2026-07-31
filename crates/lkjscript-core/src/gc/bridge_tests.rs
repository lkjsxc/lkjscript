#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::{CapabilityKind, HeapObj, ProductId, RuntimeLayoutId, Value};

#[test]
fn every_legacy_family_rejects_deterministic_or_authority_payloads() {
    let cases = [
        HeapObj::Pair {
            car: Value::from_aggregate_adapter(1),
            cdr: Value::UNIT,
        },
        HeapObj::Product {
            product: ProductId::new(0),
            fields: vec![Value::from_resource(1)],
        },
        HeapObj::Enum {
            layout: RuntimeLayoutId::new([0; 32]),
            physical_tag: 0,
            active_payload: vec![Value::from_byte_vector_key(1)],
        },
        HeapObj::Pair {
            car: Value::from_capability(CapabilityKind::Stdio),
            cdr: Value::UNIT,
        },
    ];
    for object in cases {
        let error = GcHeap::default()
            .alloc(object)
            .expect_err("mixed legacy graph must fail before publication");
        assert!(error
            .to_string()
            .contains("cannot contain deterministic owners or capabilities"));
    }
    let typed = HeapObj::Pair {
        car: Value::from_aggregate_adapter(1),
        cdr: Value::UNIT,
    };
    assert_eq!(
        GcHeap::default().try_alloc_with_layout(typed, 1),
        Err(GcLimit::MixedOwnershipGraph)
    );
}

#[test]
fn mutation_rolls_back_a_mixed_owner_graph() {
    let mut heap = GcHeap::default();
    let owner = heap
        .alloc(HeapObj::Product {
            product: ProductId::new(0),
            fields: vec![Value::from_i64(7)],
        })
        .expect("legacy product");
    let error = heap
        .mutate(owner, |object| {
            let HeapObj::Product { fields, .. } = object else {
                return Err(crate::Error::msg("wrong family"));
            };
            fields[0] = Value::from_aggregate_adapter(1);
            Ok(())
        })
        .expect_err("mixed mutation must roll back");
    assert!(error
        .to_string()
        .contains("cannot contain deterministic owners or capabilities"));
    let HeapObj::Product { fields, .. } = heap.get(owner).expect("restored product") else {
        panic!("restored family changed")
    };
    assert_eq!(fields, &[Value::from_i64(7)]);
}

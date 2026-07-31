#![allow(clippy::expect_used)]

use super::*;
use crate::{Error, HeapObj, ResourceLimitKind, Result, Value};

#[test]
fn mutation_accounting_and_rollback_cover_every_traced_family() {
    for object in [
        HeapObj::Product {
            product: crate::ProductId::new(0),
            fields: vec![Value::UNIT],
        },
        HeapObj::Enum {
            layout: crate::RuntimeLayoutId::new(crate::OPTION_LAYOUT),
            physical_tag: 0,
            active_payload: vec![Value::UNIT],
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
                HeapObj::Product { fields, .. } => fields.extend([Value::UNIT; 128]),
                HeapObj::Enum { active_payload, .. } => {
                    active_payload.extend([Value::UNIT; 128]);
                }
                HeapObj::Pair { .. } => return Err(Error::msg("unexpected fixed-size object")),
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
    let product = heap
        .alloc(HeapObj::Product {
            product: crate::ProductId::new(0),
            fields: vec![Value::UNIT],
        })
        .expect("growth product allocation");
    let before_growth = heap.stats();
    heap.mutate(product, |object| {
        let HeapObj::Product { fields, .. } = object else {
            return Err(Error::msg("unexpected test object"));
        };
        fields.extend([Value::UNIT; 128]);
        Ok(())
    })
    .expect("bounded mutation growth");
    let after_growth = heap.stats();
    assert!(after_growth.live_heap_bytes > before_growth.live_heap_bytes);
    assert!(after_growth.allocated_bytes > before_growth.allocated_bytes);
    assert!(after_growth.peak_live_heap_bytes >= after_growth.live_heap_bytes);

    let pair_object = HeapObj::Pair {
        car: Value::from_i64(1),
        cdr: Value::EMPTY_LIST,
    };
    let pair = heap
        .alloc(pair_object.clone())
        .expect("rollback pair allocation");
    let before = heap.stats();
    let result: Result<()> = heap.mutate(pair, |object| {
        let HeapObj::Pair { car, .. } = object else {
            return Err(Error::msg("unexpected test object"));
        };
        *car = Value::from_i64(2);
        Err(Error::msg("reject mutation"))
    });
    assert!(result.is_err());
    assert_eq!(heap.get(pair).ok(), Some(&pair_object));
    assert_eq!(heap.stats(), before);

    let result = heap.mutate(pair, |object| {
        *object = HeapObj::Product {
            product: crate::ProductId::new(0),
            fields: Vec::new(),
        };
        Ok(())
    });
    assert!(
        matches!(result, Err(ref error) if error.as_str() == "heap mutation changed object layout")
    );
    assert_eq!(heap.get(pair).ok(), Some(&pair_object));
    assert_eq!(heap.stats(), before);
}

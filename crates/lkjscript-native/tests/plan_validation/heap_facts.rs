use super::*;

#[test]
fn public_heap_descriptors_reject_noncanonical_operation_facts() {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let string = ValueType::Reference(ReferenceType::Str);
    let product = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(0)));
    let list_i64 = ValueType::Reference(ReferenceType::List(
        LayoutIdentity::new(100),
        ValueType::I64.layout_identity(),
    ));
    let option_i64 = ValueType::Reference(ReferenceType::Option(
        LayoutIdentity::new(101),
        ValueType::I64.layout_identity(),
    ));

    let malformed = [
        HeapCallDescriptor::new(
            HeapOperation::EmptyList,
            vec![],
            string,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::BufSet,
            vec![string, ValueType::I64, ValueType::I64],
            ValueType::Unit,
            AllocationClass::None,
            StoreClass::Scalar,
        ),
        HeapCallDescriptor::new(
            HeapOperation::ProductField {
                product: 0,
                field: 15,
                field_type: ValueType::I64,
            },
            vec![product],
            ValueType::I64,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::ProductField {
                product: 1,
                field: 0,
                field_type: ValueType::I64,
            },
            vec![product],
            ValueType::I64,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::ProductField {
                product: 0,
                field: 0,
                field_type: string,
            },
            vec![product],
            ValueType::I64,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::Car,
            vec![list_i64],
            string,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::Some,
            vec![string],
            option_i64,
            AllocationClass::Bounded,
            StoreClass::Initialization,
        ),
        HeapCallDescriptor::new(
            HeapOperation::EqualValue,
            vec![buf, buf],
            ValueType::Bool,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::BufClone,
            vec![buf],
            buf,
            AllocationClass::None,
            StoreClass::Initialization,
        ),
        HeapCallDescriptor::new(
            HeapOperation::BufSet,
            vec![buf, ValueType::I64, ValueType::I64],
            ValueType::Unit,
            AllocationClass::None,
            StoreClass::Reference,
        ),
    ];
    assert!(malformed
        .into_iter()
        .all(|descriptor| descriptor == Err(PlanError::InvalidHeapCall)));
}

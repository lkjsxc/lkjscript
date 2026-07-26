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
    let result_layout = [
        0xad, 0x73, 0x35, 0x0f, 0xf0, 0x48, 0xd2, 0xf4, 0x87, 0xd7, 0xe6, 0x1e, 0x52, 0x6a, 0xb7,
        0x3c, 0x50, 0x20, 0xf6, 0x48, 0x37, 0xb8, 0xfd, 0xdf, 0xd1, 0x0e, 0xbb, 0x35, 0x09, 0x84,
        0xc9, 0xf0,
    ];
    let result = ValueType::Reference(ReferenceType::Enum(LayoutIdentity::new(200), result_layout));
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
        HeapCallDescriptor::new(
            HeapOperation::BufToStr {
                error_type: ReferenceType::Enum(LayoutIdentity::new(201), [0; 32]),
            },
            vec![buf],
            result,
            AllocationClass::Bounded,
            StoreClass::Initialization,
        ),
    ];
    assert!(malformed
        .into_iter()
        .all(|descriptor| descriptor == Err(PlanError::InvalidHeapCall)));
}

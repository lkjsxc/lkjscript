use super::*;

#[test]
fn public_heap_descriptors_reject_noncanonical_operation_facts_and_structural_owners() {
    let product = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(0)));
    let other_product = ValueType::Reference(ReferenceType::Product(LayoutIdentity::product(1)));
    let list_i64 = ValueType::Reference(ReferenceType::List(
        LayoutIdentity::new(100),
        ValueType::I64.layout_identity(),
    ));
    let enum_value = ValueType::Reference(ReferenceType::Enum(LayoutIdentity::new(101), [4; 32]));
    let structural = ValueType::StructuralOwner(StructuralTypeIdentity::new(
        201,
        202,
        StructuralKind::String,
    ));
    let malformed = [
        HeapCallDescriptor::new(
            HeapOperation::EmptyList,
            vec![],
            product,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::ProductValue {
                product: 0,
                fields: 1,
            },
            vec![structural],
            product,
            AllocationClass::Bounded,
            StoreClass::Initialization,
        ),
        HeapCallDescriptor::new(
            HeapOperation::EnumValue {
                enum_id: [1; 32],
                variant: [2; 32],
                layout: [3; 32],
                physical_tag: 0,
                substitutions: Vec::new(),
                fields: 1,
            },
            vec![structural],
            enum_value,
            AllocationClass::Bounded,
            StoreClass::Initialization,
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
                field_type: ValueType::I64,
            },
            vec![product],
            ValueType::Bool,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::Car,
            vec![list_i64],
            product,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::EqualValue,
            vec![product, other_product],
            ValueType::Bool,
            AllocationClass::None,
            StoreClass::None,
        ),
        HeapCallDescriptor::new(
            HeapOperation::WithProductField {
                product: 0,
                field: 0,
                field_type: ValueType::I64,
            },
            vec![product, ValueType::I64],
            product,
            AllocationClass::None,
            StoreClass::Initialization,
        ),
        HeapCallDescriptor::new(
            HeapOperation::WithProductField {
                product: 0,
                field: 0,
                field_type: ValueType::I64,
            },
            vec![product, ValueType::I64],
            product,
            AllocationClass::Bounded,
            StoreClass::Reference,
        ),
    ];
    assert!(malformed
        .into_iter()
        .all(|descriptor| descriptor == Err(PlanError::InvalidHeapCall)));
}

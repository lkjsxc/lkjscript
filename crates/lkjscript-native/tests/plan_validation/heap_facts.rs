use super::*;

#[test]
fn public_heap_descriptors_reject_noncanonical_facts_and_structural_product_fields() {
    let product = ValueType::Reference(ReferenceType::RegionProduct(
        LayoutIdentity::product(0),
        [10; 32],
    ));
    let list_i64 = ValueType::Reference(ReferenceType::List(
        LayoutIdentity::new(100),
        101,
        ValueType::I64.layout_identity(),
        0x856c_7aee_ed9b_2587,
    ));
    let structural = ValueType::StructuralOwner(StructuralTypeIdentity::new(
        201,
        202,
        StructuralKind::String,
        false,
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
            StoreClass::None,
        ),
    ];
    assert!(malformed
        .into_iter()
        .all(|descriptor| descriptor == Err(PlanError::InvalidHeapCall)));
}

#[test]
fn structural_list_descriptors_require_copyable_exact_element_identity() {
    let owner_type = StructuralTypeIdentity::new(201, 202, StructuralKind::String, true);
    let owner = ValueType::StructuralOwner(owner_type);
    let list = ValueType::Reference(ReferenceType::List(
        LayoutIdentity::new(300),
        301,
        owner.layout_identity(),
        owner_type.semantic_type(),
    ));
    for descriptor in [
        HeapCallDescriptor::new(
            HeapOperation::Cons,
            vec![owner, list],
            list,
            AllocationClass::Bounded,
            StoreClass::Initialization,
        ),
        HeapCallDescriptor::new(
            HeapOperation::Car,
            vec![list],
            owner,
            AllocationClass::None,
            StoreClass::None,
        ),
    ] {
        assert!(descriptor.is_ok());
    }

    let stale = ValueType::Reference(ReferenceType::List(
        LayoutIdentity::new(300),
        301,
        owner.layout_identity(),
        203,
    ));
    let affine = ValueType::StructuralOwner(StructuralTypeIdentity::new(
        201,
        202,
        StructuralKind::Product,
        false,
    ));
    for (element, list) in [(owner, stale), (affine, list)] {
        assert_eq!(
            HeapCallDescriptor::new(
                HeapOperation::Cons,
                vec![element, list],
                list,
                AllocationClass::Bounded,
                StoreClass::Initialization,
            ),
            Err(PlanError::InvalidHeapCall),
        );
    }
}

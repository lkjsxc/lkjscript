use super::*;

fn enum_type() -> ValueType {
    ValueType::Reference(ReferenceType::Enum(LayoutIdentity::new(100), [3; 32]))
}

fn construction(substitutions: usize) -> HeapOperation {
    HeapOperation::EnumValue {
        enum_id: [1; 32],
        variant: [2; 32],
        layout: [3; 32],
        physical_tag: 0,
        substitutions: vec![LayoutIdentity::new(4); substitutions],
        fields: 1,
    }
}

#[test]
fn enum_substitution_metadata_accepts_limit_and_rejects_plus_one() {
    assert!(HeapCallDescriptor::new(
        construction(16),
        vec![ValueType::I64],
        enum_type(),
        AllocationClass::Bounded,
        StoreClass::Initialization,
    )
    .is_ok());
    assert_eq!(
        HeapCallDescriptor::new(
            construction(17),
            vec![ValueType::I64],
            enum_type(),
            AllocationClass::Bounded,
            StoreClass::Initialization,
        ),
        Err(PlanError::InvalidHeapCall)
    );
}

#[test]
fn enum_descriptors_reject_unresolved_identity_and_field_plus_one() {
    let unresolved = HeapOperation::EnumIsVariant {
        enum_id: [0; 32],
        variant: [2; 32],
        layout: [3; 32],
        physical_tag: 0,
    };
    assert_eq!(
        HeapCallDescriptor::new(
            unresolved,
            vec![enum_type()],
            ValueType::Bool,
            AllocationClass::None,
            StoreClass::None,
        ),
        Err(PlanError::InvalidHeapCall)
    );
    let field = HeapOperation::EnumField {
        enum_id: [1; 32],
        variant: [2; 32],
        field: [3; 32],
        layout: [4; 32],
        physical_tag: 0,
        field_index: 16,
        field_type: ValueType::I64,
    };
    assert_eq!(
        HeapCallDescriptor::new(
            field,
            vec![enum_type()],
            ValueType::I64,
            AllocationClass::None,
            StoreClass::None,
        ),
        Err(PlanError::InvalidHeapCall)
    );
}

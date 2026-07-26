use crate::heap::enum_metadata::enum_facts;
use crate::{HeapOperation, ValueType};
use lkjscript_ir::{
    EnumFieldMetadata, EnumId, EnumLayoutFacts, EnumMetadata, EnumVariantMetadata, RuntimeLayoutId,
    SsaType, VariantFieldId, VariantId,
};

fn metadata() -> Vec<EnumMetadata> {
    vec![EnumMetadata {
        id: EnumId::new([1; 32]),
        name: "Choice".into(),
        type_parameters: Vec::new(),
        variants: vec![EnumVariantMetadata {
            id: VariantId::new([2; 32]),
            name: "Only".into(),
            physical_tag: 0,
            fields: vec![EnumFieldMetadata {
                id: VariantFieldId::new([3; 32]),
                name: "value".into(),
                ty: SsaType::I64,
                indirect: false,
                traced: false,
            }],
        }],
        layout: EnumLayoutFacts {
            identity: RuntimeLayoutId::new([4; 32]),
            recursive: false,
        },
    }]
}

fn construction(tag: u16, layout: [u8; 32]) -> HeapOperation {
    HeapOperation::EnumValue {
        enum_id: [1; 32],
        variant: [2; 32],
        layout,
        physical_tag: tag,
        substitutions: Vec::new(),
        fields: 1,
    }
}

#[test]
fn safe_enum_metadata_preflight_rejects_malformed_layout_and_tag() {
    let metadata = metadata();
    assert!(enum_facts(&metadata, &construction(0, [4; 32])).is_ok());
    assert!(enum_facts(&metadata, &construction(1, [4; 32])).is_err());
    assert!(enum_facts(&metadata, &construction(0, [5; 32])).is_err());
}

#[test]
fn safe_enum_metadata_preflight_rejects_wrong_active_field_identity() {
    let operation = HeapOperation::EnumField {
        enum_id: [1; 32],
        variant: [2; 32],
        field: [9; 32],
        layout: [4; 32],
        physical_tag: 0,
        field_index: 0,
        field_type: ValueType::I64,
    };
    assert!(enum_facts(&metadata(), &operation).is_err());
}

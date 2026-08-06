use super::*;
use crate::{
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, RuntimeLayoutId, VariantFieldId, VariantId,
};

fn prelude_option_chunk() -> Chunk {
    let mut chunk = unit_chunk();
    chunk.enums.push(EnumMetadata {
        id: EnumId::new(crate::OPTION_ID),
        name: "option".into(),
        type_parameter_count: 1,
        layout: RuntimeLayoutId::new(crate::OPTION_LAYOUT),
        variants: vec![
            EnumVariantMetadata {
                id: VariantId::new(crate::OPTION_NONE_ID),
                name: "none".into(),
                source_order: 0,
                physical_tag: 1,
                fields: Vec::new(),
            },
            EnumVariantMetadata {
                id: VariantId::new(crate::OPTION_SOME_ID),
                name: "some".into(),
                source_order: 1,
                physical_tag: 0,
                fields: vec![EnumFieldMetadata {
                    id: VariantFieldId::new(crate::OPTION_VALUE_ID),
                    name: "value".into(),
                }],
            },
        ],
    });
    chunk
}

fn enum_chunk() -> Chunk {
    let enum_id = EnumId::new([1; 32]);
    let a = VariantId::new([2; 32]);
    let b = VariantId::new([3; 32]);
    let field = VariantFieldId::new([4; 32]);
    let layout = RuntimeLayoutId::new([5; 32]);
    let mut chunk = Chunk::new();
    chunk.enums.push(EnumMetadata {
        id: enum_id,
        name: "Choice".into(),
        type_parameter_count: 0,
        layout,
        variants: vec![
            EnumVariantMetadata {
                id: a,
                name: "a".into(),
                source_order: 0,
                physical_tag: 1,
                fields: Vec::new(),
            },
            EnumVariantMetadata {
                id: b,
                name: "b".into(),
                source_order: 1,
                physical_tag: 0,
                fields: vec![EnumFieldMetadata {
                    id: field,
                    name: "value".into(),
                }],
            },
        ],
    });
    chunk.enum_constructions.push(EnumConstructionRef {
        enum_id,
        variant: a,
        layout,
        substitution_arity: 0,
    });
    chunk.enum_fields.push(EnumFieldRef {
        enum_id,
        variant: b,
        field,
        layout,
    });
    chunk.main.emit_op_u64(Op::MakeEnum, 0);
    chunk.main.emit_op_u64(Op::LoadEnumField, 0);
    chunk.main.emit(Op::Return);
    chunk
}

#[test]
fn canonical_prelude_identity_accepts_only_its_exact_metadata() {
    assert!(validate_chunk(prelude_option_chunk(), ValidationPolicy::Unrestricted).is_ok());
    let mut forged = prelude_option_chunk();
    forged.enums[0].layout = RuntimeLayoutId::new([9; 32]);
    assert!(error(forged).contains("invalid identity/name/layout"));
}

#[test]
fn duplicate_enum_identity_collision_is_rejected() {
    let mut collided = prelude_option_chunk();
    let mut duplicate = collided.enums[0].clone();
    duplicate.name = "ForgedOption".into();
    duplicate.layout = RuntimeLayoutId::new([8; 32]);
    collided.enums.push(duplicate);
    assert!(error(collided).contains("invalid identity/name/layout"));
}

#[test]
fn enum_construction_without_structural_operations_is_rejected() {
    assert!(error(enum_chunk())
        .contains("enum construction is unsupported without structural metadata and operations"));
}

#[test]
fn malformed_physical_tag_is_rejected_before_execution() {
    let mut chunk = enum_chunk();
    chunk.enums[0].variants[0].physical_tag = 9;
    assert!(error(chunk).contains("variant metadata is malformed"));
}

#[test]
fn mismatched_layout_descriptor_is_rejected_before_execution() {
    let mut chunk = enum_chunk();
    chunk.enum_constructions[0].layout = RuntimeLayoutId::new([9; 32]);
    assert!(error(chunk).contains("construction descriptor is malformed"));
}

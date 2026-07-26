use super::*;
use crate::{
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, RuntimeLayoutId, VariantFieldId, VariantId,
};

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
                name: "A".into(),
                physical_tag: 1,
                fields: Vec::new(),
            },
            EnumVariantMetadata {
                id: b,
                name: "B".into(),
                physical_tag: 0,
                fields: vec![EnumFieldMetadata {
                    id: field,
                    name: "value".into(),
                    traced: false,
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
    chunk.main.emit_op_u16(Op::MakeEnum, 0);
    chunk.main.emit_op_u16(Op::LoadEnumField, 0);
    chunk.main.emit(Op::Return);
    chunk
}

#[test]
fn inactive_projection_is_rejected_by_bytecode_validation() {
    assert!(error(enum_chunk()).contains("inactive enum projection rejected before access"));
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

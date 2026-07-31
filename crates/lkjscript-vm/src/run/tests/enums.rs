#![allow(clippy::panic)]

use super::*;
use lkjscript_core::{
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, RuntimeLayoutId, VariantFieldId, VariantId,
};

#[test]
fn hand_built_nonstructural_enum_rejects_validation() {
    let enum_id = EnumId::new([1; 32]);
    let variant = VariantId::new([2; 32]);
    let field = VariantFieldId::new([3; 32]);
    let layout = RuntimeLayoutId::new([4; 32]);
    let mut chunk = Chunk::new();
    chunk.enums.push(EnumMetadata {
        id: enum_id,
        name: "Boxed".into(),
        type_parameter_count: 0,
        layout,
        variants: vec![EnumVariantMetadata {
            id: variant,
            name: "Value".into(),
            physical_tag: 0,
            fields: vec![EnumFieldMetadata {
                id: field,
                name: "value".into(),
            }],
        }],
    });
    chunk.enum_constructions.push(EnumConstructionRef {
        enum_id,
        variant,
        layout,
        substitution_arity: 0,
    });
    chunk.enum_fields.push(EnumFieldRef {
        enum_id,
        variant,
        field,
        layout,
    });
    chunk.constants.push(Constant::I64(42));
    chunk.main.emit_op_u16(Op::LoadConst, 0);
    chunk.main.emit_op_u16(Op::MakeEnum, 0);
    chunk.main.emit_op_u16(Op::LoadEnumField, 0);
    chunk.main.emit(Op::Return);
    let error = lkjscript_core::validate_chunk(chunk, &lkjscript_core::ValidationLimits::default())
        .expect_err("enum construction without structural metadata rejects");
    assert!(error
        .as_str()
        .contains("enum construction is unsupported without structural metadata"));
}

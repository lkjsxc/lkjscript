use super::super::super::{runtime, types};
use super::super::{enum_header, ids, product_field, scalar, Encoder};
use crate::*;

pub(super) fn encode(out: &mut Encoder, value: &InstructionKind) {
    match value {
        InstructionKind::FunctionRef(value) => {
            out.tag(18);
            out.u32(value.raw());
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            signature,
        } => {
            out.tag(19);
            runtime::runtime_op(out, *operation);
            ids(out, arguments);
            types::signature_value(out, signature);
        }
        InstructionKind::F64FromI64Exact { value } => scalar(out, 20, *value),
        InstructionKind::F64FromI64Rounded { value } => scalar(out, 21, *value),
        InstructionKind::I64FromF64Exact { value } => scalar(out, 22, *value),
        InstructionKind::I64FromF64Trunc { value } => scalar(out, 23, *value),
        InstructionKind::Call {
            target,
            arguments,
            consuming,
            signature,
            instantiation,
        } => {
            out.tag(24);
            match target {
                CallTarget::Direct(id) => {
                    out.tag(0);
                    out.u32(id.raw());
                }
                CallTarget::Indirect(id) => {
                    out.tag(1);
                    out.u32(id.raw());
                }
            }
            ids(out, arguments);
            out.sequence(consuming, |out, value| out.bool(*value));
            types::signature_value(out, signature);
            out.option(instantiation.as_ref(), types::instantiation);
        }
        InstructionKind::ProductValue { product, fields } => {
            out.tag(25);
            out.u16(product.raw());
            ids(out, fields);
        }
        InstructionKind::ProductField {
            product,
            field,
            value,
        } => product_field(out, 26, *product, *field, *value, None),
        InstructionKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => product_field(out, 27, *product, *field, *value, Some(*replacement)),
        InstructionKind::EnumValue {
            enum_id,
            variant,
            layout,
            fields,
        } => {
            out.tag(28);
            enum_header(out, *enum_id, *variant, *layout);
            ids(out, fields);
        }
        InstructionKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => {
            out.tag(29);
            enum_header(out, *enum_id, *variant, *layout);
            out.u32(value.raw());
        }
        InstructionKind::EnumField {
            enum_id,
            variant,
            field,
            layout,
            value,
        } => {
            out.tag(30);
            enum_header(out, *enum_id, *variant, *layout);
            out.fixed(&field.bytes());
            out.u32(value.raw());
        }
        _ => out.fail("verified SSA identity execution instruction partition failed"),
    }
}

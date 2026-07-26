use super::{functions, operations, types, writer::Writer, IdentityError};
use crate::*;

type IdentityResult<T = ()> = std::result::Result<T, IdentityError>;

pub(super) fn instruction(out: &mut Writer, value: &Instruction) -> IdentityResult {
    out.u32(value.id.raw())?;
    types::ssa_type(out, &value.ty)?;
    kind(out, &value.kind)?;
    metadata(out, &value.metadata)
}

fn metadata(out: &mut Writer, value: &InstructionMetadata) -> IdentityResult {
    types::origin(out, value.origin)?;
    out.u16(value.effects.bits())?;
    out.u8(match value.safepoint {
        Safepoint::None => 0,
        Safepoint::Required => 1,
    })?;
    out.u8(match value.failure {
        FailureBehavior::None => 0,
        FailureBehavior::Trap => 1,
        FailureBehavior::StructuredOutcome => 2,
        FailureBehavior::TrapOrOutcome => 3,
    })?;
    out.option(value.frame_state.as_ref(), functions::frame_state)
}

fn kind(out: &mut Writer, value: &InstructionKind) -> IdentityResult {
    match value {
        InstructionKind::Constant(value) => {
            out.u8(0)?;
            super::instruction_values::constant(out, value)
        }
        InstructionKind::Copy(value) => super::instruction_values::tagged_value(out, 1, *value),
        InstructionKind::PlaceInit { place, value } => {
            super::instruction_values::place_value(out, 2, *place, *value)
        }
        InstructionKind::PlaceEnd { place } => {
            out.u8(3)?;
            out.u32(place.raw())
        }
        InstructionKind::Move { place, value } => {
            super::instruction_values::place_value(out, 4, *place, *value)
        }
        InstructionKind::Borrow {
            place,
            loan,
            kind,
            value,
        } => {
            out.u8(5)?;
            out.u32(place.raw())?;
            out.u32(loan.raw())?;
            out.u8(match kind {
                BorrowKind::Shared => 0,
                BorrowKind::Mutable => 1,
            })?;
            out.u32(value.raw())
        }
        InstructionKind::FunctionRef(function) => {
            out.u8(6)?;
            out.u32(function.raw())
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            signature,
        } => {
            out.u8(7)?;
            operations::runtime(out, *operation)?;
            functions::values(out, arguments)?;
            types::signature(out, signature)
        }
        InstructionKind::F64FromI64Exact { value } => {
            super::instruction_values::tagged_value(out, 8, *value)
        }
        InstructionKind::F64FromI64Rounded { value } => {
            super::instruction_values::tagged_value(out, 9, *value)
        }
        InstructionKind::I64FromF64Exact { value } => {
            super::instruction_values::tagged_value(out, 10, *value)
        }
        InstructionKind::I64FromF64Trunc { value } => {
            super::instruction_values::tagged_value(out, 11, *value)
        }
        InstructionKind::Call {
            target,
            arguments,
            signature,
            instantiation,
        } => {
            out.u8(12)?;
            super::instruction_values::call_target(out, target)?;
            functions::values(out, arguments)?;
            types::signature(out, signature)?;
            out.option(instantiation.as_ref(), types::instantiation)
        }
        InstructionKind::ProductValue { product, fields } => {
            out.u8(13)?;
            out.u16(product.raw())?;
            functions::values(out, fields)
        }
        InstructionKind::ProductField {
            product,
            field,
            value,
        } => super::instruction_values::product_field(out, 14, *product, *field, *value),
        InstructionKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => {
            super::instruction_values::product_field(out, 15, *product, *field, *value)?;
            out.u32(replacement.raw())
        }
        InstructionKind::EnumValue {
            enum_id,
            variant,
            layout,
            fields,
        } => {
            super::instruction_values::enum_prefix(out, 16, *enum_id, *variant, *layout)?;
            functions::values(out, fields)
        }
        InstructionKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => {
            super::instruction_values::enum_prefix(out, 17, *enum_id, *variant, *layout)?;
            out.u32(value.raw())
        }
        InstructionKind::EnumField {
            enum_id,
            variant,
            field,
            layout,
            value,
        } => {
            super::instruction_values::enum_prefix(out, 18, *enum_id, *variant, *layout)?;
            out.fixed(&field.bytes())?;
            out.u32(value.raw())
        }
    }
}

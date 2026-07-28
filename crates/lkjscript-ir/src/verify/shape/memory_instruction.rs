use crate::verify::*;
use crate::{EffectSet, Function, Instruction, InstructionKind, SsaType};

pub(super) fn verify(
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
) -> crate::Result<EffectSet> {
    match &instruction.kind {
        InstructionKind::PlaceInit { place, value } => {
            let declared = place_by_id(function, *place)?;
            if declared.drop_glue.is_none()
                || !is_owned_value(&declared.ty)
                || declared.ty != *value_type(types, *value)?
                || instruction.ty != SsaType::Unit
            {
                return fail("SSA PlaceInit requires an exact affine place and owner value");
            }
        }
        InstructionKind::PlaceEnd { place } => {
            let declared = place_by_id(function, *place)?;
            if declared.drop_glue.is_none() || instruction.ty != SsaType::Unit {
                return fail("SSA PlaceEnd requires an exact obligated place and Unit result");
            }
        }
        InstructionKind::EndBorrow { place, value, .. } => {
            let declared = place_by_id(function, *place)?;
            if !is_owned_buf(&declared.ty)
                || !matches!(
                    value_type(types, *value)?,
                    SsaType::ByteSlice | SsaType::ByteSliceMut
                )
                || instruction.ty != SsaType::Unit
            {
                return fail("SSA EndBorrow requires a matching byte loan and Unit result");
            }
        }
        InstructionKind::Drop {
            place,
            value,
            glue,
            kind,
        } => {
            let declared = place_by_id(function, *place)?;
            let bad_kind = matches!(
                (kind, glue),
                (
                    crate::DropEventKind::ImplicitCleanup,
                    crate::DropGlueIdentity::Resource(_)
                ) | (
                    crate::DropEventKind::ExplicitClose,
                    crate::DropGlueIdentity::ByteVector
                )
            );
            if declared.drop_glue != Some(*glue)
                || expected_drop_glue(value_type(types, *value)?) != Some(*glue)
                || declared.ty != *value_type(types, *value)?
                || instruction.ty != SsaType::Unit
                || bad_kind
            {
                return fail("SSA Drop has a mismatched place, type, kind, or glue identity");
            }
        }
        InstructionKind::Move { value, .. } => {
            if value_type(types, *value)? != &instruction.ty || !is_owned_value(&instruction.ty) {
                return fail("SSA Move requires matching exact affine input and result");
            }
        }
        InstructionKind::Borrow { kind, value, .. } => {
            let expected = match kind {
                crate::BorrowKind::Shared => SsaType::ByteSlice,
                crate::BorrowKind::Mutable => SsaType::ByteSliceMut,
            };
            if !is_owned_buf(value_type(types, *value)?) || instruction.ty != expected {
                return fail("SSA Borrow ownership or reference type mismatch");
            }
        }
        _ => return fail("non-memory SSA instruction reached memory verifier"),
    }
    Ok(EffectSet::PURE)
}

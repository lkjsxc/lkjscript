use crate::verify::*;
use crate::{EffectSet, Function, Instruction, InstructionKind, Program, SsaType};

pub(super) fn verify(
    program: &Program,
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
) -> crate::Result<EffectSet> {
    match &instruction.kind {
        InstructionKind::PlaceInit { place, value } => {
            let declared = place_by_id(function, *place)?;
            if declared.drop_glue.is_none()
                || !is_owned_value(program, &declared.ty)
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
        InstructionKind::EndBorrow { place, loan, value } => {
            let declared = place_by_id(function, *place)?;
            let vector_loan = is_byte_vector(&declared.ty)
                && matches!(
                    value_type(types, *value)?,
                    SsaType::ByteSlice | SsaType::ByteSliceMut
                );
            let bytes_loan =
                declared.ty == SsaType::Bytes && value_type(types, *value)? == &SsaType::Bytes;
            let structural_loan =
                program.memory.is_owned(&declared.ty) && value_type(types, *value)? == &declared.ty;
            let structural_projection = program.memory.is_owned(&declared.ty)
                && function.blocks.iter().any(|block| {
                    block.instructions.iter().any(|candidate| {
                        candidate.id == *value
                            && matches!(
                                candidate.kind,
                                InstructionKind::AggregateFieldBorrow {
                                    place: source,
                                    loan: source_loan,
                                    ..
                                } if source == *place && source_loan == *loan
                            )
                    })
                });
            if (!vector_loan && !bytes_loan && !structural_loan && !structural_projection)
                || instruction.ty != SsaType::Unit
            {
                return fail("SSA EndBorrow requires a matching exact loan and Unit result");
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
                    crate::DropEventKind::ExplicitClose,
                    crate::DropGlueIdentity::ByteVector
                        | crate::DropGlueIdentity::Bytes
                        | crate::DropGlueIdentity::Structural(_)
                )
            ) || matches!(
                (kind, glue),
                (
                    crate::DropEventKind::ImplicitCleanup,
                    crate::DropGlueIdentity::Resource(
                        lkjscript_contracts::ResourceKind::InputStream
                    )
                )
            );
            if declared.drop_glue != Some(*glue)
                || expected_drop_glue(program, value_type(types, *value)?) != Some(*glue)
                || declared.ty != *value_type(types, *value)?
                || instruction.ty != SsaType::Unit
                || bad_kind
            {
                return fail("SSA Drop has a mismatched place, type, kind, or glue identity");
            }
        }
        InstructionKind::Move { value, .. } => {
            if value_type(types, *value)? != &instruction.ty
                || !is_owned_value(program, &instruction.ty)
            {
                return fail("SSA Move requires matching exact affine input and result");
            }
        }
        InstructionKind::Borrow {
            place, kind, value, ..
        } => {
            let source = value_type(types, *value)?;
            if program.memory.is_owned(source) {
                let declared = place_by_id(function, *place)?;
                if &declared.ty != source
                    || instruction.ty != *source
                    || *kind == crate::BorrowKind::Mutable
                {
                    return fail("SSA structural borrow requires an exact shared whole owner");
                }
                return Ok(EffectSet::PURE);
            }
            if source == &SsaType::Bytes {
                if *kind != crate::BorrowKind::Shared || instruction.ty != SsaType::Bytes {
                    return fail("SSA immutable bytes borrow must be shared exact bytes");
                }
                return Ok(EffectSet::PURE);
            }
            let expected = match kind {
                crate::BorrowKind::Shared => SsaType::ByteSlice,
                crate::BorrowKind::Mutable => SsaType::ByteSliceMut,
            };
            if !is_byte_vector(value_type(types, *value)?) || instruction.ty != expected {
                return fail("SSA Borrow ownership or reference type mismatch");
            }
        }
        _ => return fail("non-memory SSA instruction reached memory verifier"),
    }
    Ok(EffectSet::PURE)
}

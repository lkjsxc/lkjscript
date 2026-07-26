use super::enum_instruction;
use crate::verify::*;
use crate::{EffectSet, Function, Instruction, InstructionKind, Program, SsaType};

pub(crate) fn expected_instruction_effects(
    program: &Program,
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
    type_parameters: &[&str],
) -> crate::Result<EffectSet> {
    let effects = match &instruction.kind {
        InstructionKind::Constant(constant) => {
            if !constant.ty(&instruction.ty) {
                return fail(format!(
                    "SSA value {} constant type mismatch",
                    instruction.id.raw()
                ));
            }
            EffectSet::PURE
        }
        InstructionKind::Copy(value) => {
            if value_type(types, *value)? != &instruction.ty {
                return fail(format!(
                    "SSA value {} copy type mismatch",
                    instruction.id.raw()
                ));
            }
            if is_affine(&instruction.ty) {
                return fail("SSA ordinary Copy cannot copy an affine ownership value");
            }
            EffectSet::PURE
        }
        InstructionKind::PlaceInit { place, value } => {
            let declared = place_by_id(function, *place)?;
            if !is_owned_buf(&declared.ty)
                || !is_owned_buf(value_type(types, *value)?)
                || instruction.ty != SsaType::Unit
            {
                return fail("SSA PlaceInit requires an exact Owned Buf place and owner value");
            }
            EffectSet::PURE
        }
        InstructionKind::PlaceEnd { place } => {
            let declared = place_by_id(function, *place)?;
            if !is_owned_buf(&declared.ty) || instruction.ty != SsaType::Unit {
                return fail("SSA PlaceEnd requires an exact Owned Buf place and Unit result");
            }
            EffectSet::PURE
        }
        InstructionKind::Move { value, .. } => {
            if value_type(types, *value)? != &instruction.ty || !is_owned_buf(&instruction.ty) {
                return fail("SSA Move requires matching exact Owned Buf input and result");
            }
            EffectSet::PURE
        }
        InstructionKind::Borrow { kind, value, .. } => {
            if !is_owned_buf(value_type(types, *value)?)
                || instruction.ty
                    != match kind {
                        crate::BorrowKind::Shared => SsaType::Ref(Box::new(SsaType::Buf)),
                        crate::BorrowKind::Mutable => SsaType::RefMut(Box::new(SsaType::Buf)),
                    }
            {
                return fail("SSA Borrow ownership or reference type mismatch");
            }
            EffectSet::PURE
        }
        InstructionKind::FunctionRef(target) => {
            let callee = function_by_id(program, *target)?;
            if !callee.signature.bounds.is_empty() {
                return fail("SSA bounded generic function cannot be a first-class value");
            }
            if instruction.ty != SsaType::Function(Box::new(callee.signature.clone())) {
                return fail(format!(
                    "SSA value {} function-reference type mismatch",
                    instruction.id.raw()
                ));
            }
            EffectSet::PURE
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            signature,
        } => {
            verify_resolved_signature(signature, arguments, &instruction.ty, types)?;
            verify_runtime_signature(*operation, signature)?;
            operation.effects()
        }
        InstructionKind::Call {
            target,
            arguments,
            signature,
            instantiation,
        } => {
            if matches!(instruction.ty, SsaType::Ref(_) | SsaType::RefMut(_)) {
                return fail("SSA user-call result cannot be a lexical reference in this slice");
            }
            verify_resolved_signature(signature, arguments, &instruction.ty, types)?;
            match target {
                CallTarget::Direct(target) => {
                    let callee = function_by_id(program, *target)?;
                    verify_call_compatibility(
                        program,
                        &callee.signature,
                        signature,
                        instantiation.as_ref(),
                        type_parameters,
                    )?;
                    callee.effects
                }
                CallTarget::Indirect(target) => {
                    let target_ty = value_type(types, *target)?;
                    let SsaType::Function(target_signature) = target_ty else {
                        return fail(format!(
                            "SSA value {} has a non-function indirect call target",
                            instruction.id.raw()
                        ));
                    };
                    if !target_signature.bounds.is_empty() {
                        return fail("SSA indirect call target has unsupported marker bounds");
                    }
                    verify_call_compatibility(
                        program,
                        target_signature,
                        signature,
                        instantiation.as_ref(),
                        type_parameters,
                    )?;
                    EffectSet::CONSERVATIVE_CALL
                }
            }
        }
        InstructionKind::ProductValue { product, fields } => {
            let metadata = product_by_id(program, *product)?;
            if instruction.ty != SsaType::Product(*product) || fields.len() != metadata.fields.len()
            {
                return fail(format!(
                    "SSA value {} malformed product construction",
                    instruction.id.raw()
                ));
            }
            for (value, field) in fields.iter().zip(&metadata.fields) {
                if value_type(types, *value)? != &field.ty {
                    return fail(format!(
                        "SSA value {} product field type mismatch",
                        instruction.id.raw()
                    ));
                }
            }
            EffectSet::ALLOCATES
        }
        InstructionKind::ProductField {
            product,
            field,
            value,
        } => {
            let metadata = product_by_id(program, *product)?;
            let Some(field_metadata) = metadata.fields.get(usize::from(*field)) else {
                return fail("SSA product field index is out of range");
            };
            if value_type(types, *value)? != &SsaType::Product(*product)
                || instruction.ty != field_metadata.ty
            {
                return fail("SSA product field type or identity mismatch");
            }
            EffectSet::READS_MEMORY
        }
        InstructionKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => {
            let metadata = product_by_id(program, *product)?;
            let Some(field_metadata) = metadata.fields.get(usize::from(*field)) else {
                return fail("SSA replacement field index is out of range");
            };
            if value_type(types, *value)? != &SsaType::Product(*product)
                || value_type(types, *replacement)? != &field_metadata.ty
                || instruction.ty != SsaType::Product(*product)
            {
                return fail("SSA product replacement type or identity mismatch");
            }
            EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES)
        }
        kind @ (InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. }) => {
            enum_instruction::verify(program, instruction, types, kind)?
        }
    };
    Ok(effects)
}

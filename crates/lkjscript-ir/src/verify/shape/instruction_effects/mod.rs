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
            if is_affine(program, &instruction.ty) {
                return fail("SSA ordinary Copy cannot copy an affine ownership value");
            }
            EffectSet::PURE
        }
        InstructionKind::PlaceInit { .. }
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::EndBorrow { .. }
        | InstructionKind::Drop { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. } => {
            super::memory_instruction::verify(program, function, instruction, types)?
        }
        InstructionKind::StructuralPublish { .. }
        | InstructionKind::DestinationCreate { .. }
        | InstructionKind::DestinationFieldInit { .. }
        | InstructionKind::DestinationFinish { .. }
        | InstructionKind::DestinationAbort { .. }
        | InstructionKind::AggregateFieldBorrow { .. }
        | InstructionKind::AggregateTag { .. }
        | InstructionKind::AggregateConsumePayload { .. }
        | InstructionKind::StringUtf8View { .. }
        | InstructionKind::StructuralCopy { .. } => {
            super::structural_instruction::verify(program, function, instruction, types)?
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
        InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. } => {
            super::numeric_conversion::verify(program, instruction, types)?
        }
        InstructionKind::Call {
            target,
            arguments,
            consuming,
            signature,
            instantiation,
        } => {
            if matches!(instruction.ty, SsaType::ByteSlice | SsaType::ByteSliceMut) {
                return fail("SSA user-call result cannot be a lexical reference in this slice");
            }
            verify_resolved_signature(signature, arguments, &instruction.ty, types)?;
            if consuming.len() != arguments.len() {
                return fail("SSA call ownership modes do not match arity");
            }
            match target {
                CallTarget::Direct(target) => {
                    let callee = function_by_id(program, *target)?;
                    let entry = block_by_id(callee, callee.entry)?;
                    if entry.parameters.len() != consuming.len()
                        || entry
                            .parameters
                            .iter()
                            .zip(consuming)
                            .any(|(parameter, consuming)| {
                                parameter.owner_place.is_some() != *consuming
                                    && is_affine(program, &parameter.ty)
                            })
                    {
                        return fail(
                            "SSA direct call ownership modes disagree with callee parameters",
                        );
                    }
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
        kind @ (InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. }) => {
            expected_product_instruction_effects(program, instruction, types, kind)?
        }
        kind @ (InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. }) => {
            enum_instruction::verify(program, instruction, types, kind)?
        }
    };
    Ok(effects)
}

include!("product.rs");

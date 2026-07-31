use super::*;

mod borrow;
use borrow::*;

pub(super) fn consume_instruction(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    values: &mut [bool],
    locals: &mut [bool],
) -> Result<(), VerificationError> {
    consume(function, instruction, values, locals, true)
}

pub(super) fn transfer_instruction(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    values: &mut [bool],
    locals: &mut [bool],
) -> Result<(), VerificationError> {
    consume(function, instruction, values, locals, false)
}

fn consume(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    values: &mut [bool],
    locals: &mut [bool],
    checked: bool,
) -> Result<(), VerificationError> {
    if checked {
        check_borrow_conflicts(function, instruction, values)?;
    }
    for (index, operand) in instruction.operation.operands().into_iter().enumerate() {
        if consumes_argument(function, &instruction.operation, index, operand)? {
            if checked {
                reject_live_borrow(function, operand, values)?;
            }
            values[operand.index as usize] = false;
        }
    }
    match instruction.operation {
        Operation::ReadLocal(local) if local_type(function, local)?.is_affine() => {
            locals[local.index as usize] = false;
        }
        Operation::WriteLocal(local, _) => locals[local.index as usize] = true,
        _ => {}
    }
    Ok(())
}

pub(super) fn verify_affine_exit(
    function: &FunctionPlan,
    terminator: &Terminator,
    values: &[bool],
    locals: &[bool],
) -> Result<(), VerificationError> {
    if matches!(
        terminator,
        Terminator::Branch(_) | Terminator::BranchIf { .. }
    ) {
        return Ok(());
    }
    let returned = match terminator {
        Terminator::Return(value) if value_type(function, *value)?.is_affine() => Some(*value),
        _ => None,
    };
    for fact in &function.values {
        if values.get(fact.id.index as usize).copied().unwrap_or(false)
            && fact.value_type.is_affine()
            && Some(fact.id) != returned
        {
            return Err(
                if matches!(fact.value_type, ValueType::StructuralDestination(_)) {
                    VerificationError::IncompleteStructuralDestination(fact.id)
                } else {
                    VerificationError::LiveAffineValue(fact.id)
                },
            );
        }
    }
    for (index, fact) in function.locals.iter().enumerate() {
        if locals.get(index).copied().unwrap_or(false) && fact.value_type.is_affine() {
            return Err(VerificationError::LiveAffineLocal(fact.id));
        }
    }
    Ok(())
}

fn consumes_argument(
    function: &FunctionPlan,
    operation: &Operation,
    index: usize,
    value: ValueId,
) -> Result<bool, VerificationError> {
    if !value_type(function, value)?.is_affine() {
        return Ok(false);
    }
    let consumes = match operation {
        Operation::RuntimeCall(slot, _) => !matches!(
            slot,
            RuntimeCallSlot::ByteVectorBorrowShared
                | RuntimeCallSlot::ByteVectorBorrowExclusive
                | RuntimeCallSlot::ByteSliceLength
                | RuntimeCallSlot::ByteSliceByteAt
                | RuntimeCallSlot::ByteSliceReadU32Le
                | RuntimeCallSlot::ByteSliceMutSetByte
                | RuntimeCallSlot::ByteSliceMutWriteU32Le
                | RuntimeCallSlot::BytesBorrowShared
                | RuntimeCallSlot::BytesLength
                | RuntimeCallSlot::BytesByteAt
                | RuntimeCallSlot::BytesClone
                | RuntimeCallSlot::BytesCopySlice
        ),
        Operation::StructuralCall(descriptor, _) => {
            if descriptor.operation().is_observation()
                || matches!(descriptor.operation(), StructuralOperation::Borrow { .. })
            {
                observed_local_alias(function, value)
            } else {
                true
            }
        }
        Operation::WriteLocal(_, _) | Operation::Call(_, _) => true,
        Operation::HeapCall(_, _) => true,
        _ => index == usize::MAX,
    };
    Ok(consumes)
}

fn observed_local_alias(function: &FunctionPlan, value: ValueId) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            instruction.output == value
                && matches!(instruction.operation, Operation::ObserveLocal(_))
        })
    })
}

fn local_type(function: &FunctionPlan, local: LocalId) -> Result<ValueType, VerificationError> {
    Ok(function.locals[local_index(function, local)?].value_type)
}

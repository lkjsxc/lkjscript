use super::*;

pub(super) fn verify_arguments(
    function: &FunctionPlan,
    arguments: &[ValueId],
    signature: &Signature,
    context: &'static str,
) -> Result<(), VerificationError> {
    if arguments.len() != signature.parameters().len() {
        return Err(VerificationError::TypeMismatch(context));
    }
    for (argument, expected) in arguments.iter().zip(signature.parameters()) {
        let actual = value_type(function, *argument)?;
        let compatible = actual == *expected
            || (*expected == ValueType::StructuralKey
                && matches!(actual, ValueType::StructuralOwner(_)));
        if !compatible {
            return Err(VerificationError::TypeMismatch(context));
        }
    }
    Ok(())
}

pub(super) fn verify_terminator(
    function: &FunctionPlan,
    terminator: &Terminator,
) -> Result<(), VerificationError> {
    match terminator {
        Terminator::Branch(target) => {
            block_index(function, *target)?;
        }
        Terminator::BranchIf {
            condition,
            when_true,
            when_false,
        } => {
            block_index(function, *when_true)?;
            block_index(function, *when_false)?;
            if value_type(function, *condition)? != ValueType::Bool {
                return Err(VerificationError::TypeMismatch("conditional branch"));
            }
        }
        Terminator::Return(value) => {
            if value_type(function, *value)? != function.signature.result() {
                return Err(VerificationError::InvalidReturn(function.id));
            }
        }
        Terminator::Trap { .. } => {}
        Terminator::Outcome(_) => {}
        Terminator::Exit(code) => {
            if value_type(function, *code)? != ValueType::I64 {
                return Err(VerificationError::TypeMismatch("exit status"));
            }
        }
    }
    Ok(())
}

pub(super) fn require_output(
    instruction: &crate::plan::Instruction,
    expected: ValueType,
    context: &'static str,
) -> Result<(), VerificationError> {
    if instruction.output_type != expected {
        return Err(VerificationError::TypeMismatch(context));
    }
    Ok(())
}

pub(super) fn require_types<const N: usize>(
    function: &FunctionPlan,
    values: [ValueId; N],
    expected: ValueType,
    context: &'static str,
) -> Result<(), VerificationError> {
    for value in values {
        if value_type(function, value)? != expected {
            return Err(VerificationError::TypeMismatch(context));
        }
    }
    Ok(())
}

pub(super) fn require_available(
    function: &FunctionPlan,
    value: ValueId,
    available: &[bool],
) -> Result<(), VerificationError> {
    let index = value_index(function, value)?;
    if !available.get(index).copied().unwrap_or(false) {
        return Err(VerificationError::ValueNotAvailable(value));
    }
    Ok(())
}

pub(super) fn value_type(
    function: &FunctionPlan,
    value: ValueId,
) -> Result<ValueType, VerificationError> {
    let index = value_index(function, value)?;
    function
        .values
        .get(index)
        .map(|fact| fact.value_type)
        .ok_or(VerificationError::InvalidValue(value))
}

pub(super) fn value_index(
    function: &FunctionPlan,
    value: ValueId,
) -> Result<usize, VerificationError> {
    if value.function != function.id {
        return Err(VerificationError::InvalidValue(value));
    }
    let index = value.host_index().unwrap_or(usize::MAX);
    if function.values.get(index).map(|fact| fact.id) != Some(value) {
        return Err(VerificationError::InvalidValue(value));
    }
    Ok(index)
}

pub(super) fn local_index(
    function: &FunctionPlan,
    local: LocalId,
) -> Result<usize, VerificationError> {
    if local.function != function.id {
        return Err(VerificationError::InvalidLocal(local));
    }
    let index = local.host_index().unwrap_or(usize::MAX);
    if function.locals.get(index).map(|fact| fact.id) != Some(local) {
        return Err(VerificationError::InvalidLocal(local));
    }
    Ok(index)
}

pub(super) fn block_index(
    function: &FunctionPlan,
    block: BlockId,
) -> Result<usize, VerificationError> {
    if block.function != function.id {
        return Err(VerificationError::InvalidTarget(block));
    }
    let index = block.host_index().unwrap_or(usize::MAX);
    if function.blocks.get(index).map(|item| item.id) != Some(block) {
        return Err(VerificationError::InvalidTarget(block));
    }
    Ok(index)
}

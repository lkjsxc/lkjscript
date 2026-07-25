use crate::optimize::*;
use crate::{
    BlockId, EffectSet, FailureBehavior, Instruction, InstructionKind, RuntimeOp, Safepoint,
    Signature, SsaType, ValueId,
};

pub(crate) fn checker_allowed_gvn(
    indexes: &CheckerFunctionIndexes<'_>,
    use_block: BlockId,
    use_index: usize,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let Some(key) = checker_exact_expression_key(indexes, instruction, budget)? else {
        return Ok(None);
    };
    let Some(candidates) = indexes.expressions.get(&key) else {
        return Ok(None);
    };
    let mut best: Option<CheckerPosition> = None;
    for candidate in candidates {
        budget.charge(1)?;
        if candidate.value == instruction.id
            || !indexes.dominance.proves_definition_before(
                candidate.block,
                candidate.instruction_index,
                use_block,
                use_index,
            )
        {
            continue;
        }
        let candidate_order = (
            candidate.block.raw(),
            candidate.instruction_index,
            candidate.value.raw(),
        );
        if best.is_none_or(|old| {
            candidate_order < (old.block.raw(), old.instruction_index, old.value.raw())
        }) {
            best = Some(*candidate);
        }
    }
    let Some(best) = best else {
        return Ok(None);
    };
    let InstructionKind::Runtime {
        operation,
        arguments,
        ..
    } = &instruction.kind
    else {
        return Ok(None);
    };
    let exact_checked_i64 = instruction.ty == SsaType::I64
        && matches!(
            operation,
            RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
        )
        && arguments.iter().all(|argument| {
            indexes
                .definitions
                .get(argument.index().unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .is_some_and(|definition| definition.ty == &SsaType::I64)
        })
        && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    let kind = if exact_checked_i64 {
        OptimizationEditKind::CheckedI64GlobalValueNumbering
    } else {
        OptimizationEditKind::GlobalValueNumbering
    };
    Ok(Some((kind, *operation, arguments.clone(), best.value)))
}

pub(crate) fn checker_exact_expression_key<'a>(
    indexes: &CheckerFunctionIndexes<'a>,
    instruction: &'a Instruction,
    budget: &mut Budget,
) -> Result<Option<CheckerExpressionKey<'a>>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return Ok(None);
    };
    budget.charge(arguments.len() as u64)?;
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || !checker_is_nonownership_scalar(&instruction.ty)
        || !checker_signature_and_operand_types(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let exact_pure = match operation {
        RuntimeOp::Less | RuntimeOp::LessEqual | RuntimeOp::Greater | RuntimeOp::GreaterEqual => {
            instruction.ty == SsaType::Bool
                && arguments.len() == 2
                && arguments.iter().all(|argument| {
                    indexes
                        .definitions
                        .get(argument.index().unwrap_or(usize::MAX))
                        .and_then(Option::as_ref)
                        .is_some_and(|definition| {
                            matches!(definition.ty, SsaType::I64 | SsaType::F64)
                        })
                })
        }
        RuntimeOp::Not => instruction.ty == SsaType::Bool && arguments.len() == 1,
        RuntimeOp::BitAnd | RuntimeOp::BitOr | RuntimeOp::BitXor => {
            instruction.ty == SsaType::I64
                && arguments.len() == 2
                && arguments.iter().all(|argument| {
                    indexes
                        .definitions
                        .get(argument.index().unwrap_or(usize::MAX))
                        .and_then(Option::as_ref)
                        .is_some_and(|definition| definition.ty == &SsaType::I64)
                })
        }
        _ => false,
    } && instruction.metadata.effects == EffectSet::PURE
        && instruction.metadata.failure == FailureBehavior::None;
    let exact_arithmetic = matches!(
        operation,
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
    ) && arguments.len() == 2
        && arguments.iter().all(|argument| {
            indexes
                .definitions
                .get(argument.index().unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .is_some_and(|definition| matches!(definition.ty, SsaType::I64 | SsaType::F64))
        })
        && matches!(instruction.ty, SsaType::I64 | SsaType::F64)
        && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    if !exact_pure && !exact_arithmetic {
        return Ok(None);
    }
    Ok(Some(CheckerExpressionKey {
        operation: *operation,
        arguments,
        signature,
        result_type: &instruction.ty,
    }))
}

pub(crate) fn checker_signature_and_operand_types(
    indexes: &CheckerFunctionIndexes<'_>,
    signature: &Signature,
    arguments: &[ValueId],
    result: &SsaType,
) -> bool {
    if !signature.type_parameters.is_empty()
        || !signature.bounds.is_empty()
        || signature.result.as_ref() != result
        || signature.parameters.len() != arguments.len()
    {
        return false;
    }
    signature
        .parameters
        .iter()
        .zip(arguments)
        .all(|(expected, argument)| {
            checker_is_nonownership_scalar(expected)
                && indexes
                    .definitions
                    .get(argument.index().unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                    .is_some_and(|definition| definition.ty == expected)
        })
}

pub(crate) fn checker_i64_constant(
    indexes: &CheckerFunctionIndexes<'_>,
    value: ValueId,
) -> Option<i64> {
    indexes.constants.get(value.index()?)?.as_ref().copied()
}

pub(crate) fn checker_is_nonownership_scalar(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    )
}

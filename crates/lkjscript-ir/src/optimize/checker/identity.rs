use crate::optimize::*;
use crate::{
    BlockId, EffectSet, FailureBehavior, Instruction, InstructionKind, RuntimeOp, Safepoint,
    SsaType, ValueId,
};

pub(crate) fn checker_allowed_identity(
    indexes: &CheckerFunctionIndexes<'_>,
    use_block: BlockId,
    use_index: usize,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return Ok(None);
    };
    budget.charge(arguments.len() as u64)?;
    let metadata_legal = instruction.metadata.effects == EffectSet::PURE
        && instruction.metadata.failure == FailureBehavior::None
        && instruction.metadata.safepoint == Safepoint::None
        && instruction.metadata.frame_state.is_none();
    if !metadata_legal
        || !checker_signature_and_operand_types(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let replacement = match (*operation, arguments.as_slice(), &instruction.ty) {
        (RuntimeOp::BitXor | RuntimeOp::BitOr, [left, right], SsaType::I64) => {
            if checker_i64_constant(indexes, *left) == Some(0) {
                Some(*right)
            } else if checker_i64_constant(indexes, *right) == Some(0)
                || *operation == RuntimeOp::BitOr && left == right
            {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::BitAnd, [left, right], SsaType::I64) => {
            if checker_i64_constant(indexes, *left) == Some(-1) {
                Some(*right)
            } else if checker_i64_constant(indexes, *right) == Some(-1) || left == right {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::Not, [inner], SsaType::Bool) => {
            let Some(inner_definition) = indexes
                .definitions
                .get(inner.index().unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
            else {
                return Ok(None);
            };
            let Some(inner_instruction) = inner_definition.instruction else {
                return Ok(None);
            };
            if inner_instruction.ty != SsaType::Bool
                || inner_instruction.metadata.effects != EffectSet::PURE
                || inner_instruction.metadata.failure != FailureBehavior::None
                || inner_instruction.metadata.safepoint != Safepoint::None
                || inner_instruction.metadata.frame_state.is_some()
            {
                None
            } else {
                match &inner_instruction.kind {
                    InstructionKind::Runtime {
                        operation: RuntimeOp::Not,
                        arguments: inner_arguments,
                        signature: inner_signature,
                    } if inner_arguments.len() == 1
                        && checker_signature_and_operand_types(
                            indexes,
                            inner_signature,
                            inner_arguments,
                            &inner_instruction.ty,
                        ) =>
                    {
                        inner_arguments.first().copied()
                    }
                    _ => None,
                }
            }
        }
        _ => None,
    };
    let Some(replacement) = replacement else {
        return Ok(None);
    };
    if !checker_value_precedes(indexes, replacement, use_block, use_index) {
        return Ok(None);
    }
    Ok(Some((
        OptimizationEditKind::AlgebraicIdentity,
        *operation,
        arguments.clone(),
        replacement,
    )))
}

pub(crate) fn checker_value_precedes(
    indexes: &CheckerFunctionIndexes<'_>,
    value: ValueId,
    use_block: BlockId,
    use_index: usize,
) -> bool {
    let Some(definition) = indexes
        .definitions
        .get(value.index().unwrap_or(usize::MAX))
        .and_then(Option::as_ref)
    else {
        return false;
    };
    if definition.block == use_block {
        return definition
            .instruction_index
            .is_none_or(|definition_index| definition_index < use_index);
    }
    indexes.dominance.proves_definition_before(
        definition.block,
        definition.instruction_index.unwrap_or(0),
        use_block,
        use_index,
    )
}

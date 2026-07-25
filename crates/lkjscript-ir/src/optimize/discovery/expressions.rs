use crate::optimize::*;
use crate::{
    BlockId, EffectSet, FailureBehavior, Instruction, InstructionKind, RuntimeOp, Safepoint,
    Signature, SsaType, ValueId,
};

pub(crate) fn discovery_gvn_edit(
    indexes: &DiscoveryFunctionIndexes<'_>,
    block: BlockId,
    instruction_index: usize,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let Some(key) = discovery_expression_key(indexes, instruction, budget)? else {
        return Ok(None);
    };
    let Some(candidates) = indexes.expressions.get(&key) else {
        return Ok(None);
    };
    let mut selected: Option<DiscoveryPosition> = None;
    for candidate in candidates {
        budget.charge(1)?;
        if candidate.value == instruction.id
            || !indexes.dominance.definition_dominates(
                candidate.block,
                candidate.instruction_index,
                block,
                instruction_index,
            )
        {
            continue;
        }
        let key = (
            candidate.block.raw(),
            candidate.instruction_index,
            candidate.value.raw(),
        );
        if selected.is_none_or(|current| {
            key < (
                current.block.raw(),
                current.instruction_index,
                current.value.raw(),
            )
        }) {
            selected = Some(*candidate);
        }
    }
    let Some(candidate) = selected else {
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
    let kind = if discovery_checked_i64(indexes, instruction, *operation) {
        OptimizationEditKind::CheckedI64GlobalValueNumbering
    } else {
        OptimizationEditKind::GlobalValueNumbering
    };
    Ok(Some((kind, *operation, arguments.clone(), candidate.value)))
}

pub(crate) fn discovery_expression_key<'a>(
    indexes: &DiscoveryFunctionIndexes<'a>,
    instruction: &'a Instruction,
    budget: &mut Budget,
) -> Result<Option<DiscoveryExpressionKey<'a>>, OptimizationError> {
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
        || !discovery_scalar(&instruction.ty)
        || !discovery_signature_matches(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let pure = matches!(
        operation,
        RuntimeOp::Less
            | RuntimeOp::LessEqual
            | RuntimeOp::Greater
            | RuntimeOp::GreaterEqual
            | RuntimeOp::Not
            | RuntimeOp::BitAnd
            | RuntimeOp::BitOr
            | RuntimeOp::BitXor
    ) && instruction.metadata.effects == EffectSet::PURE
        && instruction.metadata.failure == FailureBehavior::None;
    let arithmetic = matches!(
        operation,
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
    ) && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    if !pure && !arithmetic {
        return Ok(None);
    }
    Ok(Some(DiscoveryExpressionKey {
        operation: *operation,
        arguments,
        signature,
        ty: &instruction.ty,
    }))
}

pub(crate) fn discovery_signature_matches(
    indexes: &DiscoveryFunctionIndexes<'_>,
    signature: &Signature,
    arguments: &[ValueId],
    result: &SsaType,
) -> bool {
    signature.type_parameters.is_empty()
        && signature.bounds.is_empty()
        && signature.result.as_ref() == result
        && signature.parameters.len() == arguments.len()
        && signature
            .parameters
            .iter()
            .zip(arguments)
            .all(|(expected, value)| {
                indexes
                    .definitions
                    .get(value.index().unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                    .is_some_and(|definition| {
                        definition.ty == expected && discovery_scalar(expected)
                    })
            })
}

pub(crate) fn discovery_constant_i64(
    indexes: &DiscoveryFunctionIndexes<'_>,
    value: ValueId,
) -> Option<i64> {
    indexes.constants_i64.get(value.index()?)?.as_ref().copied()
}

pub(crate) fn discovery_checked_i64(
    indexes: &DiscoveryFunctionIndexes<'_>,
    instruction: &Instruction,
    operation: RuntimeOp,
) -> bool {
    instruction.ty == SsaType::I64
        && matches!(
            operation,
            RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
        )
        && match &instruction.kind {
            InstructionKind::Runtime { arguments, .. } => arguments.iter().all(|value| {
                indexes
                    .definitions
                    .get(value.index().unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                    .is_some_and(|definition| definition.ty == &SsaType::I64)
            }),
            _ => false,
        }
}

pub(crate) fn discovery_scalar(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    )
}

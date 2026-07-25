use crate::optimize::*;
use crate::{
    EffectSet, FailureBehavior, Instruction, InstructionKind, Program, RuntimeOp, Safepoint,
    SsaType,
};

pub(crate) fn discover_edits(
    program: &Program,
    indexes: &DiscoveryIndexes<'_>,
    budget: &mut Budget,
) -> Result<Vec<OptimizationCertificateRecord>, OptimizationError> {
    let mut builder = CertificateBuilder::new(budget)?;
    for function in &program.functions {
        let function_index = function
            .id
            .index()
            .ok_or_else(|| input_index_error("discovery FunctionId cannot index verified input"))?;
        let index = indexes
            .functions
            .get(function_index)
            .ok_or_else(|| input_index_error("discovery function index is incomplete"))?;
        for block in &function.blocks {
            if !index.dominance.is_reachable(block.id) {
                continue;
            }
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                budget.charge(1)?;
                let edit = match discovery_identity_edit(index, instruction, budget)? {
                    Some(edit) => Some(edit),
                    None => {
                        discovery_gvn_edit(index, block.id, instruction_index, instruction, budget)?
                    }
                };
                builder.push(function.id, block.id, instruction.id, edit, budget)?;
            }
        }
    }
    Ok(builder.finish())
}

pub(crate) fn discovery_identity_edit(
    indexes: &DiscoveryFunctionIndexes<'_>,
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
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || instruction.metadata.effects != EffectSet::PURE
        || instruction.metadata.failure != FailureBehavior::None
        || !discovery_signature_matches(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let replacement = match (*operation, arguments.as_slice()) {
        (RuntimeOp::BitXor | RuntimeOp::BitOr, [left, right]) if instruction.ty == SsaType::I64 => {
            if discovery_constant_i64(indexes, *left) == Some(0) {
                Some(*right)
            } else if discovery_constant_i64(indexes, *right) == Some(0)
                || operation == &RuntimeOp::BitOr && left == right
            {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::BitAnd, [left, right]) if instruction.ty == SsaType::I64 => {
            if discovery_constant_i64(indexes, *left) == Some(-1) {
                Some(*right)
            } else if discovery_constant_i64(indexes, *right) == Some(-1) || left == right {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::Not, [inner]) if instruction.ty == SsaType::Bool => indexes
            .definitions
            .get(inner.index().unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .and_then(|definition| definition.instruction)
            .and_then(|inner_instruction| {
                if inner_instruction.ty != SsaType::Bool
                    || inner_instruction.metadata.effects != EffectSet::PURE
                    || inner_instruction.metadata.failure != FailureBehavior::None
                    || inner_instruction.metadata.safepoint != Safepoint::None
                    || inner_instruction.metadata.frame_state.is_some()
                {
                    return None;
                }
                match &inner_instruction.kind {
                    InstructionKind::Runtime {
                        operation: RuntimeOp::Not,
                        arguments,
                        signature,
                    } if arguments.len() == 1
                        && discovery_signature_matches(
                            indexes,
                            signature,
                            arguments,
                            &inner_instruction.ty,
                        ) =>
                    {
                        arguments.first().copied()
                    }
                    _ => None,
                }
            }),
        _ => None,
    };
    Ok(replacement.map(|replacement| {
        (
            OptimizationEditKind::AlgebraicIdentity,
            *operation,
            arguments.clone(),
            replacement,
        )
    }))
}

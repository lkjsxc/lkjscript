use std::collections::HashMap;

use crate::{CallTarget, FrameState, InstructionKind, Program, Terminator, ValueId};

pub(crate) fn compact_values(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        let mut mapping = HashMap::new();
        let mut next = 0_u32;
        for block in &function.blocks {
            for parameter in &block.parameters {
                mapping.insert(parameter.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value count exceeds u32"))?;
            }
            for instruction in &block.instructions {
                mapping.insert(instruction.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value count exceeds u32"))?;
            }
        }
        for block in &mut function.blocks {
            for parameter in &mut block.parameters {
                parameter.id = mapped_value(&mapping, parameter.id)?;
            }
            for instruction in &mut block.instructions {
                instruction.id = mapped_value(&mapping, instruction.id)?;
            }
        }
        rewrite_function_values(function, |value| {
            mapping.get(&value).copied().unwrap_or(value)
        });
    }
    Ok(())
}

pub(crate) fn mapped_value(
    mapping: &HashMap<ValueId, ValueId>,
    id: ValueId,
) -> crate::Result<ValueId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA value {}", id.raw())))
}

pub(crate) fn rewrite_function_values(
    function: &mut crate::Function,
    mut rewrite: impl FnMut(ValueId) -> ValueId,
) {
    for block in &mut function.blocks {
        if let Some(frame) = &mut block.metadata.frame_state {
            rewrite_frame(frame, &mut rewrite);
        }
        for instruction in &mut block.instructions {
            match &mut instruction.kind {
                InstructionKind::Constant(_)
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::FunctionRef(_) => {}
                InstructionKind::Copy(value)
                | InstructionKind::PlaceInit { value, .. }
                | InstructionKind::Move { value, .. }
                | InstructionKind::Borrow { value, .. }
                | InstructionKind::F64FromI64Exact { value }
                | InstructionKind::F64FromI64Rounded { value }
                | InstructionKind::I64FromF64Exact { value }
                | InstructionKind::I64FromF64Trunc { value } => {
                    *value = rewrite(*value);
                }
                InstructionKind::Runtime { arguments, .. }
                | InstructionKind::Call { arguments, .. } => {
                    for argument in arguments {
                        *argument = rewrite(*argument);
                    }
                    if let InstructionKind::Call {
                        target: CallTarget::Indirect(target),
                        ..
                    } = &mut instruction.kind
                    {
                        *target = rewrite(*target);
                    }
                }
                InstructionKind::ProductValue { fields, .. }
                | InstructionKind::EnumValue { fields, .. } => {
                    for field in fields {
                        *field = rewrite(*field);
                    }
                }
                InstructionKind::ProductField { value, .. }
                | InstructionKind::EnumIsVariant { value, .. }
                | InstructionKind::EnumField { value, .. } => *value = rewrite(*value),
                InstructionKind::WithProductField {
                    value, replacement, ..
                } => {
                    *value = rewrite(*value);
                    *replacement = rewrite(*replacement);
                }
            }
            if let Some(frame) = &mut instruction.metadata.frame_state {
                rewrite_frame(frame, &mut rewrite);
            }
        }
        match &mut block.terminator {
            Terminator::Branch { arguments, .. } => {
                for argument in arguments {
                    *argument = rewrite(*argument);
                }
            }
            Terminator::ConditionalBranch {
                condition,
                true_arguments,
                false_arguments,
                ..
            } => {
                *condition = rewrite(*condition);
                for argument in true_arguments.iter_mut().chain(false_arguments) {
                    *argument = rewrite(*argument);
                }
            }
            Terminator::Return(value) | Terminator::Trap { value } => {
                *value = rewrite(*value);
            }
            Terminator::Exit { code } => *code = rewrite(*code),
            Terminator::Outcome { detail, .. } => {
                if let Some(detail) = detail {
                    *detail = rewrite(*detail);
                }
            }
        }
    }
}

pub(crate) fn rewrite_frame(frame: &mut FrameState, rewrite: &mut impl FnMut(ValueId) -> ValueId) {
    for local in &mut frame.locals {
        local.value = rewrite(local.value);
    }
    for value in &mut frame.operand_stack {
        *value = rewrite(*value);
    }
}

use super::*;

pub(crate) fn rewrite_function_values(
    function: &mut crate::Function,
    mut rewrite: impl FnMut(ValueId) -> ValueId,
) {
    for node in &mut function.failure_cleanups {
        match &mut node.action {
            FailureCleanupAction::EndBorrow { value, .. }
            | FailureCleanupAction::DropOwner { value, .. } => *value = rewrite(*value),
        }
    }
    for block in &mut function.blocks {
        if let Some(frame) = &mut block.metadata.frame_state {
            rewrite_frame(frame, &mut rewrite);
        }
        for instruction in &mut block.instructions {
            match &mut instruction.kind {
                InstructionKind::Constant(_)
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::DestinationCreate { .. }
                | InstructionKind::FunctionRef(_) => {}
                InstructionKind::Copy(value)
                | InstructionKind::PlaceInit { value, .. }
                | InstructionKind::EndBorrow { value, .. }
                | InstructionKind::Drop { value, .. }
                | InstructionKind::Move { value, .. }
                | InstructionKind::Borrow { value, .. }
                | InstructionKind::StructuralPublish { value, .. }
                | InstructionKind::DestinationFinish { destination: value }
                | InstructionKind::DestinationAbort { destination: value }
                | InstructionKind::AggregateFieldBorrow { value, .. }
                | InstructionKind::AggregateTag { value, .. }
                | InstructionKind::AggregateConsumePayload { value, .. }
                | InstructionKind::StringUtf8View { value, .. }
                | InstructionKind::StructuralCopy { value, .. }
                | InstructionKind::MemoryWitnessIndependentOwner { value, .. }
                | InstructionKind::MemoryWitnessDispose { value, .. }
                | InstructionKind::F64FromI64Exact { value }
                | InstructionKind::F64FromI64Rounded { value }
                | InstructionKind::I64FromF64Exact { value }
                | InstructionKind::I64FromF64Trunc { value } => {
                    *value = rewrite(*value);
                }
                InstructionKind::DestinationFieldInit {
                    destination, value, ..
                } => {
                    *destination = rewrite(*destination);
                    *value = rewrite(*value);
                }
                InstructionKind::MemoryWitnessCompare { left, right, .. } => {
                    *left = rewrite(*left);
                    *right = rewrite(*right);
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

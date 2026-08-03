use std::collections::HashMap;

use crate::optimize::passes::*;
use crate::{InstructionKind, ValueId, VerifiedProgram};

pub fn copy_propagate(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    if verified.program().functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    InstructionKind::StructuralPublish { .. }
                        | InstructionKind::DestinationCreate { .. }
                        | InstructionKind::DestinationFieldInit { .. }
                        | InstructionKind::DestinationFinish { .. }
                        | InstructionKind::DestinationAbort { .. }
                        | InstructionKind::AggregateFieldBorrow { .. }
                        | InstructionKind::AggregateTag { .. }
                        | InstructionKind::AggregateConsumePayload { .. }
                        | InstructionKind::StringUtf8View { .. }
                        | InstructionKind::StructuralCopy { .. }
                        | InstructionKind::MemoryWitnessIndependentOwner { .. }
                        | InstructionKind::MemoryWitnessDispose { .. }
                )
            })
        })
    }) {
        return Ok(verified.clone());
    }
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let mut copies = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let InstructionKind::Copy(source) = instruction.kind {
                    copies.insert(instruction.id, resolve_copy(source, &copies));
                }
            }
        }
        rewrite_function_values(function, |value| resolve_copy(value, &copies));
        for block in &mut function.blocks {
            block
                .instructions
                .retain(|instruction| !matches!(instruction.kind, InstructionKind::Copy(_)));
        }
    }
    compact_values(&mut program)?;
    finish(program)
}

pub(crate) fn resolve_copy(mut value: ValueId, copies: &HashMap<ValueId, ValueId>) -> ValueId {
    let mut remaining = copies.len().saturating_add(1);
    while remaining > 0 {
        let Some(next) = copies.get(&value).copied() else {
            break;
        };
        value = next;
        remaining -= 1;
    }
    value
}

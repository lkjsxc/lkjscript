use std::collections::HashMap;

use crate::verify::*;
use crate::{Block, Function, Program, SsaType, ValueId};

pub(crate) fn verify_block(
    program: &Program,
    function: &Function,
    block: &Block,
    types: &[SsaType],
    definitions: &HashMap<ValueId, Definition>,
    dominators: &Dominators,
    type_parameters: &[&str],
) -> crate::Result<()> {
    let frame_context = FrameVerificationContext {
        program,
        function,
        types,
        definitions,
        dominators,
    };
    if let Some(frame) = &block.metadata.frame_state {
        verify_frame_state(&frame_context, block.id, None, frame)?;
    }
    for (index, instruction) in block.instructions.iter().enumerate() {
        for operand in instruction.kind.operands() {
            verify_available(
                function,
                block.id,
                Some(index),
                operand,
                definitions,
                dominators,
            )?;
        }
        if let Some(frame) = &instruction.metadata.frame_state {
            verify_frame_state(&frame_context, block.id, Some(index), frame)?;
        }
        super::active_enum::projection(function, block, instruction)?;
        verify_instruction(program, function, instruction, types, type_parameters)?;
    }
    for operand in block.terminator.operands() {
        verify_available(
            function,
            block.id,
            Some(block.instructions.len()),
            operand,
            definitions,
            dominators,
        )?;
    }
    verify_terminator(program, function, block, types)
}

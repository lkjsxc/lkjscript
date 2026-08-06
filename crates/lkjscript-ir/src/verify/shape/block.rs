use std::collections::HashMap;

use crate::verify::*;
use crate::{Block, Function, Program, SsaType, ValueId};

pub(crate) struct BlockVerificationContext<'a> {
    pub(crate) program: &'a Program,
    pub(crate) function: &'a Function,
    pub(crate) types: &'a [SsaType],
    pub(crate) definitions: &'a HashMap<ValueId, Definition>,
    pub(crate) dominators: &'a Dominators,
    pub(crate) active_enum: &'a super::active_enum::Graph<'a>,
    pub(crate) type_parameters: &'a [&'a str],
}

pub(crate) fn verify_block(
    context: &BlockVerificationContext<'_>,
    block: &Block,
) -> crate::Result<()> {
    let BlockVerificationContext {
        program,
        function,
        types,
        definitions,
        dominators,
        active_enum,
        type_parameters,
    } = context;
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
        super::active_enum::projection(program, active_enum, block, instruction)?;
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

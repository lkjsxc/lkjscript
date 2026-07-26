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
    if let Some(frame) = &block.metadata.frame_state {
        verify_frame_state(
            function,
            block.id,
            None,
            frame,
            types,
            definitions,
            dominators,
        )?;
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
            verify_frame_state(
                function,
                block.id,
                Some(index),
                frame,
                types,
                definitions,
                dominators,
            )?;
        }
        verify_enum_projection_provenance(function, instruction)?;
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

fn verify_enum_projection_provenance(
    function: &Function,
    instruction: &crate::Instruction,
) -> crate::Result<()> {
    let crate::InstructionKind::EnumField {
        enum_id,
        variant,
        layout,
        value,
        ..
    } = instruction.kind
    else {
        return Ok(());
    };
    let constructor = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|candidate| candidate.id == value)
        .ok_or_else(|| crate::IrError::new("SSA enum projection provenance is missing"))?;
    match constructor.kind {
        crate::InstructionKind::EnumValue {
            enum_id: active_enum,
            variant: active_variant,
            layout: active_layout,
            ..
        } if active_enum == enum_id && active_variant == variant && active_layout == layout => {
            Ok(())
        }
        crate::InstructionKind::EnumValue { .. } => {
            fail("SSA inactive enum projection rejected before access")
        }
        _ => fail("SSA enum projection lacks verified active-variant provenance"),
    }
}

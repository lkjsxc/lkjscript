use std::collections::{HashMap, HashSet};

use crate::verify::*;
use crate::{Block, BlockId, Function, Program, SsaType, Terminator, ValueId};

pub(crate) fn verify_terminator(
    _program: &Program,
    function: &Function,
    block: &Block,
    types: &[SsaType],
) -> crate::Result<()> {
    match &block.terminator {
        Terminator::Branch { target, arguments } => {
            verify_edge(function, *target, arguments, types)?;
        }
        Terminator::ConditionalBranch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            if value_type(types, *condition)? != &SsaType::Bool {
                return fail("SSA conditional branch condition is not Bool");
            }
            verify_edge(function, *true_target, true_arguments, types)?;
            verify_edge(function, *false_target, false_arguments, types)?;
        }
        Terminator::Return(value) => {
            if value_type(types, *value)? != function.signature.result.as_ref() {
                return fail(format!(
                    "SSA function {} returns the wrong type",
                    function.name
                ));
            }
        }
        Terminator::Trap { message } => {
            if message.is_empty() {
                return fail("SSA trap terminator has an empty diagnostic");
            }
        }
        Terminator::Exit { code } => {
            if value_type(types, *code)? != &SsaType::I64 {
                return fail("SSA exit terminator code is not I64");
            }
        }
        Terminator::Outcome { detail, .. } => {
            if let Some(detail) = detail {
                if value_type(types, *detail)? != &SsaType::Str {
                    return fail("SSA structured-outcome detail is not Str");
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn verify_edge(
    function: &Function,
    target: BlockId,
    arguments: &[ValueId],
    types: &[SsaType],
) -> crate::Result<()> {
    let target = block(function, target)?;
    if target.parameters.len() != arguments.len() {
        return fail(format!(
            "SSA edge to block {} has {} arguments for {} parameters",
            target.id.raw(),
            arguments.len(),
            target.parameters.len()
        ));
    }
    for (argument, parameter) in arguments.iter().zip(&target.parameters) {
        if value_type(types, *argument)? != &parameter.ty {
            return fail(format!(
                "SSA edge to block {} has a block-argument type mismatch",
                target.id.raw()
            ));
        }
    }
    Ok(())
}

pub(crate) fn verify_frame_state(
    function: &Function,
    block: BlockId,
    instruction: Option<usize>,
    frame: &crate::FrameState,
    types: &[SsaType],
    definitions: &HashMap<ValueId, Definition>,
    dominators: &Dominators,
) -> crate::Result<()> {
    let mut bindings = HashSet::new();
    let mut slots = HashSet::new();
    let mut affine_values = HashSet::new();
    let mut previous_binding = None;
    for local in &frame.locals {
        if previous_binding.is_some_and(|previous| previous >= local.binding) {
            return fail("SSA frame locals are not in stable BindingId order");
        }
        previous_binding = Some(local.binding);
        if !bindings.insert(local.binding) || !slots.insert(local.slot) {
            return fail("SSA frame state has duplicate bindings or local slots");
        }
        let ty = value_type(types, local.value)?;
        if is_affine(ty) && !affine_values.insert(local.value) {
            return fail("SSA frame state duplicates an affine local value");
        }
        verify_available(
            function,
            block,
            instruction,
            local.value,
            definitions,
            dominators,
        )?;
    }
    for value in &frame.operand_stack {
        let ty = value_type(types, *value)?;
        if is_affine(ty) && !affine_values.insert(*value) {
            return fail("SSA frame state duplicates an affine value");
        }
        verify_available(
            function,
            block,
            instruction,
            *value,
            definitions,
            dominators,
        )?;
    }
    Ok(())
}

pub(crate) fn verify_available(
    function: &Function,
    use_block: BlockId,
    use_instruction: Option<usize>,
    value: ValueId,
    definitions: &HashMap<ValueId, Definition>,
    dominators: &Dominators,
) -> crate::Result<()> {
    let Some(definition) = definitions.get(&value).copied() else {
        return fail(format!(
            "SSA function {} uses missing ValueId {}",
            function.name,
            value.raw()
        ));
    };
    if definition.block == use_block {
        match (definition.instruction, use_instruction) {
            (None, _) => Ok(()),
            (Some(definition), Some(usage)) if definition < usage => Ok(()),
            _ => fail(format!(
                "SSA function {} uses ValueId {} before definition",
                function.name,
                value.raw()
            )),
        }
    } else {
        if dominates(dominators, use_block, definition.block)? {
            Ok(())
        } else {
            fail(format!(
                "SSA ValueId {} does not dominate its use in function {}",
                value.raw(),
                function.name
            ))
        }
    }
}

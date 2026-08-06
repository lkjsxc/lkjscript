mod boolean;
mod equivalence;
mod graph;
mod universal;

use crate::verify::*;
use crate::{Block, BlockId, Function, InstructionKind, RuntimeLayoutId, ValueId, VariantId};
use equivalence::equivalent;
use graph::{charge, edge_argument, find_instruction, incoming};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct ActiveVariant {
    enum_id: crate::EnumId,
    variant: VariantId,
    layout: RuntimeLayoutId,
}

pub(super) fn projection(
    program: &crate::Program,
    function: &Function,
    block: &Block,
    instruction: &crate::Instruction,
) -> crate::Result<()> {
    let InstructionKind::EnumField {
        enum_id,
        variant,
        layout,
        value,
        ..
    } = instruction.kind
    else {
        return Ok(());
    };
    let active = ActiveVariant {
        enum_id,
        variant,
        layout,
    };
    let mut work = 0_usize;
    if matches!(
        find_instruction(function, value).map(|item| &item.kind),
        Some(InstructionKind::EnumValue { .. })
    ) && !universal::value(function, value, active, &mut HashSet::new(), &mut work)?
    {
        return fail("SSA inactive enum projection rejected before access");
    }
    let mut visited = HashSet::new();
    if active_at_entry(
        program,
        function,
        block.id,
        value,
        active,
        &mut visited,
        &mut work,
    )? || universal::value(function, value, active, &mut HashSet::new(), &mut work)?
    {
        Ok(())
    } else {
        fail(format!(
            concat!(
                "SSA enum projection lacks verified active-variant provenance: ",
                "block {:?}, value {:?}, enum {:?}, variant {:?}"
            ),
            block.id, value, active.enum_id, active.variant
        ))
    }
}

fn active_at_entry(
    program: &crate::Program,
    function: &Function,
    block: BlockId,
    value: ValueId,
    active: ActiveVariant,
    visited: &mut HashSet<(BlockId, ValueId, ActiveVariant)>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if !visited.insert((block, value, active)) {
        return Ok(false);
    }
    let current = block_by_id(function, block)?;
    if let Some(index) = current
        .parameters
        .iter()
        .position(|parameter| parameter.id == value)
    {
        let predecessors = incoming(function, block);
        if predecessors.is_empty() {
            return Ok(false);
        }
        for predecessor in predecessors {
            let argument = edge_argument(predecessor, block, index)?;
            if !active_on_edge(
                program,
                function,
                predecessor,
                block,
                argument,
                active,
                visited,
                work,
            )? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if universal::value(function, value, active, &mut HashSet::new(), work)? {
        return Ok(true);
    }
    let predecessors = incoming(function, block);
    if predecessors.is_empty() {
        return Ok(false);
    }
    for predecessor in predecessors {
        if !active_on_edge(
            program,
            function,
            predecessor,
            block,
            value,
            active,
            visited,
            work,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn active_on_edge(
    program: &crate::Program,
    function: &Function,
    predecessor: &Block,
    target: BlockId,
    value: ValueId,
    active: ActiveVariant,
    visited: &mut HashSet<(BlockId, ValueId, ActiveVariant)>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if let crate::Terminator::ConditionalBranch {
        condition,
        true_target,
        ..
    } = predecessor.terminator
    {
        if true_target == target {
            if condition_tests_variant(program, function, condition, value, active, work)? {
                return Ok(true);
            }
            if let Some(possible) =
                boolean::true_predecessors(function, predecessor, condition, value, work)?
            {
                if possible.is_empty() {
                    return Ok(false);
                }
                for (source, argument) in possible {
                    if !active_on_edge(
                        program,
                        function,
                        source,
                        predecessor.id,
                        argument,
                        active,
                        visited,
                        work,
                    )? {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
        }
    }
    active_at_entry(
        program,
        function,
        predecessor.id,
        value,
        active,
        visited,
        work,
    )
}

fn condition_tests_variant(
    program: &crate::Program,
    function: &Function,
    condition: ValueId,
    value: ValueId,
    active: ActiveVariant,
    work: &mut usize,
) -> crate::Result<bool> {
    let Some(test) = find_instruction(function, condition) else {
        return Ok(false);
    };
    if let InstructionKind::EnumIsVariant {
        enum_id,
        variant,
        layout,
        value: tested,
    } = test.kind
    {
        return Ok(ActiveVariant {
            enum_id,
            variant,
            layout,
        } == active
            && equivalent(function, tested, value, &mut HashSet::new(), work)?);
    }
    let InstructionKind::Runtime {
        operation: crate::RuntimeOp::EqualValue,
        arguments,
        ..
    } = &test.kind
    else {
        return Ok(false);
    };
    let [left, right] = arguments.as_slice() else {
        return Ok(false);
    };
    let Some(expected_tag) = program
        .enums
        .iter()
        .find(|definition| {
            definition.id == active.enum_id && definition.layout.identity == active.layout
        })
        .and_then(|definition| {
            definition
                .variants
                .iter()
                .find(|variant| variant.id == active.variant)
        })
        .and_then(|variant| i64::try_from(variant.physical_tag).ok())
    else {
        return Ok(false);
    };
    for (tag_value, constant_value) in [(*left, *right), (*right, *left)] {
        let Some(InstructionKind::AggregateTag { value: tested, .. }) =
            find_instruction(function, tag_value).map(|item| &item.kind)
        else {
            continue;
        };
        if !matches!(
            find_instruction(function, constant_value).map(|item| &item.kind),
            Some(InstructionKind::Constant(crate::Constant::I64(tag))) if *tag == expected_tag
        ) {
            continue;
        }
        if equivalent(function, *tested, value, &mut HashSet::new(), work)? {
            return Ok(true);
        }
    }
    Ok(false)
}

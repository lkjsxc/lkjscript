mod boolean;
mod equivalence;
mod graph;
mod universal;

use crate::verify::*;
use crate::{Block, BlockId, InstructionKind, RuntimeLayoutId, ValueId, VariantId};
use equivalence::equivalent;
use graph::{push, visit, Observation};
use std::collections::HashSet;

pub(super) use graph::Graph;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct ActiveVariant {
    enum_id: crate::EnumId,
    variant: VariantId,
    layout: RuntimeLayoutId,
}

#[derive(Clone, Copy)]
enum ProvenanceTask {
    Entry {
        block: BlockId,
        value: ValueId,
    },
    Edge {
        predecessor: BlockId,
        target: BlockId,
        value: ValueId,
    },
}

pub(super) fn projection(
    program: &crate::Program,
    graph: &Graph<'_>,
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
    let mut observation = Observation::default();
    if matches!(
        graph.instruction(value).map(|item| &item.kind),
        Some(InstructionKind::EnumValue { .. })
    ) && !universal::value(graph, value, active, &mut observation)?
    {
        return fail("SSA inactive enum projection rejected before access");
    }
    if active_at_entry(program, graph, block.id, value, active, &mut observation)?
        || universal::value(graph, value, active, &mut observation)?
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
    graph: &Graph<'_>,
    block: BlockId,
    value: ValueId,
    active: ActiveVariant,
    observation: &mut Observation,
) -> crate::Result<bool> {
    let mut visited = HashSet::new();
    let mut pending = Vec::new();
    push(
        &mut pending,
        ProvenanceTask::Entry { block, value },
        "SSA active-variant provenance worklist allocation failed",
    )?;
    while let Some(task) = pending.pop() {
        match task {
            ProvenanceTask::Entry { block, value } => {
                observation.observe()?;
                if !visit(
                    &mut visited,
                    (block, value, active),
                    "SSA active-variant provenance visited allocation failed",
                )? {
                    return Ok(false);
                }
                if let Some((parameter_block, index)) = graph.parameter(value) {
                    if parameter_block == block {
                        let predecessors = graph.predecessors(block)?;
                        if predecessors.is_empty() {
                            return Ok(false);
                        }
                        for predecessor in predecessors.iter().rev() {
                            push(
                                &mut pending,
                                ProvenanceTask::Edge {
                                    predecessor: *predecessor,
                                    target: block,
                                    value: graph.edge_argument(*predecessor, block, index)?,
                                },
                                "SSA active-variant provenance worklist allocation failed",
                            )?;
                        }
                        continue;
                    }
                }
                if universal::value(graph, value, active, observation)? {
                    continue;
                }
                let predecessors = graph.predecessors(block)?;
                if predecessors.is_empty() {
                    return Ok(false);
                }
                for predecessor in predecessors.iter().rev() {
                    push(
                        &mut pending,
                        ProvenanceTask::Edge {
                            predecessor: *predecessor,
                            target: block,
                            value,
                        },
                        "SSA active-variant provenance worklist allocation failed",
                    )?;
                }
            }
            ProvenanceTask::Edge {
                predecessor,
                target,
                value,
            } => {
                observation.observe()?;
                let predecessor_block = graph.block(predecessor)?;
                if let crate::Terminator::ConditionalBranch {
                    condition,
                    true_target,
                    ..
                } = predecessor_block.terminator
                {
                    if true_target == target {
                        if condition_tests_variant(
                            program,
                            graph,
                            condition,
                            value,
                            active,
                            observation,
                        )? {
                            continue;
                        }
                        if let Some(possible) = boolean::true_predecessors(
                            graph,
                            predecessor,
                            condition,
                            value,
                            observation,
                        )? {
                            if possible.is_empty() {
                                return Ok(false);
                            }
                            for (source, argument) in possible.into_iter().rev() {
                                push(
                                    &mut pending,
                                    ProvenanceTask::Edge {
                                        predecessor: source,
                                        target: predecessor,
                                        value: argument,
                                    },
                                    "SSA active-variant provenance worklist allocation failed",
                                )?;
                            }
                            continue;
                        }
                    }
                }
                push(
                    &mut pending,
                    ProvenanceTask::Entry {
                        block: predecessor,
                        value,
                    },
                    "SSA active-variant provenance worklist allocation failed",
                )?;
            }
        }
    }
    Ok(true)
}

fn condition_tests_variant(
    program: &crate::Program,
    graph: &Graph<'_>,
    condition: ValueId,
    value: ValueId,
    active: ActiveVariant,
    observation: &mut Observation,
) -> crate::Result<bool> {
    let Some(test) = graph.instruction(condition) else {
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
            && equivalent(graph, tested, value, observation)?);
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
            graph.instruction(tag_value).map(|item| &item.kind)
        else {
            continue;
        };
        if !matches!(
            graph.instruction(constant_value).map(|item| &item.kind),
            Some(InstructionKind::Constant(crate::Constant::I64(tag))) if *tag == expected_tag
        ) {
            continue;
        }
        if equivalent(graph, *tested, value, observation)? {
            return Ok(true);
        }
    }
    Ok(false)
}

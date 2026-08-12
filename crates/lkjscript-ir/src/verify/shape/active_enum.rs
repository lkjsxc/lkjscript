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
                            for source in possible.into_iter().rev() {
                                push(
                                    &mut pending,
                                    ProvenanceTask::Edge {
                                        predecessor: source.predecessor,
                                        target: source.target,
                                        value: source.value,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlockMetadata, BlockParameter, EffectSet, FailureBehavior, Function, FunctionId,
        Instruction, InstructionMetadata, Origin, Program, Signature, SsaType, Terminator,
        VariantFieldId,
    };

    const ENUM: crate::EnumId = crate::EnumId::new([1; 32]);
    const VARIANT: VariantId = VariantId::new([2; 32]);
    const FIELD: VariantFieldId = VariantFieldId::new([3; 32]);
    const LAYOUT: RuntimeLayoutId = RuntimeLayoutId::new([4; 32]);

    fn parameter(id: u64, ty: SsaType) -> BlockParameter {
        BlockParameter {
            id: ValueId::new(id),
            ty,
            owner_place: None,
            origin: Origin::SYNTHETIC,
        }
    }

    fn instruction(id: u64, ty: SsaType, kind: InstructionKind) -> Instruction {
        Instruction {
            id: ValueId::new(id),
            ty,
            kind,
            metadata: InstructionMetadata {
                origin: Origin::SYNTHETIC,
                effects: EffectSet::PURE,
                failure: FailureBehavior::None,
                failure_cleanup: None,
                frame_state: None,
            },
        }
    }

    fn block(
        id: u64,
        parameters: Vec<BlockParameter>,
        instructions: Vec<Instruction>,
        terminator: Terminator,
    ) -> Block {
        Block {
            id: BlockId::new(id),
            parameters,
            instructions,
            terminator,
            metadata: BlockMetadata {
                loop_header: false,
                origin: Origin::SYNTHETIC,
                failure_cleanup: None,
                frame_state: None,
            },
        }
    }

    fn chained_boolean_program(inactive_path_can_succeed: bool) -> Program {
        let enum_ty = SsaType::Enum {
            id: ENUM,
            arguments: Vec::new(),
        };
        let inactive_condition = if inactive_path_can_succeed {
            InstructionKind::Copy(ValueId::new(6))
        } else {
            InstructionKind::Constant(crate::Constant::Bool(false))
        };
        let blocks = vec![
            block(
                0,
                vec![parameter(0, enum_ty.clone()), parameter(1, SsaType::Bool)],
                vec![instruction(
                    2,
                    SsaType::Bool,
                    InstructionKind::EnumIsVariant {
                        enum_id: ENUM,
                        variant: VARIANT,
                        layout: LAYOUT,
                        value: ValueId::new(0),
                    },
                )],
                Terminator::ConditionalBranch {
                    condition: ValueId::new(2),
                    true_target: BlockId::new(1),
                    true_arguments: vec![ValueId::new(0), ValueId::new(1)],
                    false_target: BlockId::new(2),
                    false_arguments: vec![ValueId::new(0), ValueId::new(1)],
                },
            ),
            block(
                1,
                vec![parameter(3, enum_ty.clone()), parameter(4, SsaType::Bool)],
                Vec::new(),
                Terminator::ConditionalBranch {
                    condition: ValueId::new(4),
                    true_target: BlockId::new(3),
                    true_arguments: vec![ValueId::new(3)],
                    false_target: BlockId::new(4),
                    false_arguments: vec![ValueId::new(3)],
                },
            ),
            block(
                2,
                vec![parameter(5, enum_ty.clone()), parameter(6, SsaType::Bool)],
                vec![instruction(7, SsaType::Bool, inactive_condition)],
                Terminator::Branch {
                    target: BlockId::new(6),
                    arguments: vec![ValueId::new(7), ValueId::new(5)],
                },
            ),
            block(
                3,
                vec![parameter(8, enum_ty.clone())],
                vec![instruction(
                    9,
                    SsaType::Bool,
                    InstructionKind::Constant(crate::Constant::Bool(true)),
                )],
                Terminator::Branch {
                    target: BlockId::new(5),
                    arguments: vec![ValueId::new(9), ValueId::new(8)],
                },
            ),
            block(
                4,
                vec![parameter(10, enum_ty.clone())],
                vec![instruction(
                    11,
                    SsaType::Bool,
                    InstructionKind::Constant(crate::Constant::Bool(false)),
                )],
                Terminator::Branch {
                    target: BlockId::new(5),
                    arguments: vec![ValueId::new(11), ValueId::new(10)],
                },
            ),
            block(
                5,
                vec![parameter(12, SsaType::Bool), parameter(13, enum_ty.clone())],
                Vec::new(),
                Terminator::Branch {
                    target: BlockId::new(6),
                    arguments: vec![ValueId::new(12), ValueId::new(13)],
                },
            ),
            block(
                6,
                vec![parameter(14, SsaType::Bool), parameter(15, enum_ty.clone())],
                Vec::new(),
                Terminator::ConditionalBranch {
                    condition: ValueId::new(14),
                    true_target: BlockId::new(7),
                    true_arguments: vec![ValueId::new(15)],
                    false_target: BlockId::new(8),
                    false_arguments: vec![ValueId::new(15)],
                },
            ),
            block(
                7,
                vec![parameter(16, enum_ty.clone())],
                vec![instruction(
                    17,
                    SsaType::I64,
                    InstructionKind::EnumField {
                        enum_id: ENUM,
                        variant: VARIANT,
                        field: FIELD,
                        field_index: 0,
                        layout: LAYOUT,
                        value: ValueId::new(16),
                    },
                )],
                Terminator::Return(ValueId::new(17)),
            ),
            block(
                8,
                vec![parameter(18, enum_ty.clone())],
                vec![instruction(
                    19,
                    SsaType::I64,
                    InstructionKind::Constant(crate::Constant::I64(0)),
                )],
                Terminator::Return(ValueId::new(19)),
            ),
        ];
        Program {
            memory: crate::StructuralMemoryMetadata::default(),
            region_products: Vec::new(),
            sources: Vec::new(),
            products: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            implementations: Vec::new(),
            functions: vec![Function {
                id: FunctionId::new(0),
                name: "active-variant-chain".into(),
                signature: Signature::monomorphic(vec![enum_ty, SsaType::Bool], SsaType::I64),
                places: Vec::new(),
                failure_cleanups: Vec::new(),
                effects: EffectSet::PURE,
                entry: BlockId::new(0),
                blocks,
                origin: Origin::SYNTHETIC,
            }],
            main: FunctionId::new(0),
        }
    }

    fn verify_projection(program: &Program) -> crate::Result<()> {
        let function = &program.functions[0];
        let cfg = ControlFlowGraph::build(function)?;
        let graph = Graph::build(function, &cfg, 20)?;
        projection(
            program,
            &graph,
            &function.blocks[7],
            &function.blocks[7].instructions[0],
        )
    }

    fn verify_all_projections(program: &Program) -> crate::Result<()> {
        let function = &program.functions[0];
        let cfg = ControlFlowGraph::build(function)?;
        let graph = Graph::build(function, &cfg, 20)?;
        for block in &function.blocks {
            for instruction in &block.instructions {
                projection(program, &graph, block, instruction)?;
            }
        }
        Ok(())
    }

    #[test]
    fn chained_boolean_merges_preserve_active_variant_provenance() -> crate::Result<()> {
        verify_all_projections(&chained_boolean_program(false))
    }

    #[test]
    fn chained_boolean_merges_reject_possible_inactive_variant_paths() -> crate::Result<()> {
        let Err(error) = verify_projection(&chained_boolean_program(true)) else {
            return Err(crate::IrError::new(
                "possible inactive path accepted an enum projection",
            ));
        };
        if error
            .to_string()
            .contains("lacks verified active-variant provenance")
        {
            Ok(())
        } else {
            Err(crate::IrError::new(format!(
                "unexpected active-variant rejection: {error}"
            )))
        }
    }
}

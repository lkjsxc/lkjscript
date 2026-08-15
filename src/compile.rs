use crate::core_ir::{
    self, BlockId, CoreBlock, CoreFunction, CoreProgram, FunctionId, Instruction, Terminator,
    ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::query;
use crate::schema::{Node, OperationKind, SemanticType, ValueRef};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn compile(snapshot: &Snapshot, entry: NodeId) -> Result<CoreProgram> {
    let blockers = query::entry_blockers(snapshot, entry)?;
    if !blockers.is_empty() {
        return Err(LkError::new(
            ErrorCode::CompileIncomplete,
            "entry direct-call closure contains completeness blockers",
        )
        .for_workspace(snapshot.workspace())
        .at_revision(snapshot.revision())
        .for_node(entry)
        .with_related(
            blockers
                .iter()
                .map(|blocker| blocker.target.unwrap_or(blocker.owner)),
        ));
    }
    let reachable = reachable_functions(snapshot, entry)?;
    let mut function_ids = BTreeMap::new();
    for function in &reachable {
        function_ids.insert(
            *function,
            FunctionId(dense_u32(function_ids.len(), *function, "function")?),
        );
    }
    let mut functions = Vec::with_capacity(reachable.len());
    for function in reachable {
        functions.push(lower_function(snapshot, function, &function_ids)?);
    }
    let program = CoreProgram {
        entry: *function_ids
            .get(&entry)
            .ok_or_else(|| invalid(entry, "entry function was not allocated"))?,
        functions,
    };
    core_ir::verify(&program)?;
    Ok(program)
}

fn reachable_functions(snapshot: &Snapshot, entry: NodeId) -> Result<Vec<NodeId>> {
    if !matches!(snapshot.node(entry)?, Node::Function { .. }) {
        return Err(
            LkError::new(ErrorCode::WrongKind, "compile entry must be a function").for_node(entry),
        );
    }
    let mut pending = BTreeSet::from([entry]);
    let mut visited = BTreeSet::new();
    while let Some(function) = pending.pop_first() {
        if !visited.insert(function) {
            continue;
        }
        let Node::Function { body, .. } = snapshot.node(function)? else {
            unreachable!()
        };
        let Some(body) = body else {
            continue;
        };
        let mut nodes = vec![*body];
        while let Some(id) = nodes.pop() {
            let node = snapshot.node(id)?;
            if let Node::Operation {
                operation:
                    OperationKind::Call {
                        function: target, ..
                    },
                ..
            } = node
                && !visited.contains(target)
            {
                pending.insert(*target);
            }
            for index in (0..node.owned_child_count()).rev() {
                if let Some(child) = node.owned_child(index) {
                    nodes.push(child);
                }
            }
        }
    }
    Ok(visited.into_iter().collect())
}

struct BuildBlock {
    origin: NodeId,
    parameters: Vec<ValueId>,
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
}
struct FunctionBuilder {
    origin: NodeId,
    result: SemanticType,
    parameters: Vec<ValueId>,
    value_types: Vec<SemanticType>,
    blocks: Vec<BuildBlock>,
    entry: BlockId,
}
impl FunctionBuilder {
    fn value(&mut self, ty: SemanticType, origin: NodeId) -> Result<ValueId> {
        let id = ValueId(dense_u32(self.value_types.len(), origin, "value")?);
        self.value_types.push(ty);
        Ok(id)
    }
    fn block(
        &mut self,
        origin: NodeId,
        parameter_types: &[SemanticType],
    ) -> Result<(BlockId, Vec<ValueId>)> {
        let id = BlockId(dense_u32(self.blocks.len(), origin, "block")?);
        let mut parameters = Vec::with_capacity(parameter_types.len());
        for ty in parameter_types {
            parameters.push(self.value(*ty, origin)?);
        }
        self.blocks.push(BuildBlock {
            origin,
            parameters: parameters.clone(),
            instructions: Vec::new(),
            terminator: None,
        });
        Ok((id, parameters))
    }
    fn instruction(&mut self, block: BlockId, instruction: Instruction) -> Result<()> {
        self.block_mut(block)?.instructions.push(instruction);
        Ok(())
    }
    fn terminate(&mut self, block: BlockId, terminator: Terminator) -> Result<()> {
        let target = self.block_mut(block)?;
        if target.terminator.replace(terminator).is_some() {
            return Err(invalid(
                target.origin,
                "Core lowering terminated a block twice",
            ));
        }
        Ok(())
    }
    fn block_mut(&mut self, block: BlockId) -> Result<&mut BuildBlock> {
        self.blocks
            .get_mut(
                usize::try_from(block.0)
                    .map_err(|_| invalid(self.origin, "block index overflows host"))?,
            )
            .ok_or_else(|| invalid(self.origin, "lowering referenced an absent block"))
    }
    fn finish(self) -> Result<CoreFunction> {
        let mut blocks = Vec::with_capacity(self.blocks.len());
        for block in self.blocks {
            blocks.push(CoreBlock {
                origin: block.origin,
                parameters: block.parameters,
                instructions: block.instructions,
                terminator: block
                    .terminator
                    .ok_or_else(|| invalid(block.origin, "lowered block has no terminator"))?,
            });
        }
        Ok(CoreFunction {
            origin: self.origin,
            parameters: self.parameters,
            result: self.result,
            value_types: self.value_types,
            blocks,
            entry: self.entry,
        })
    }
}

#[derive(Clone)]
enum EndAction {
    Return,
    YieldBranch {
        target: BlockId,
        captures: Vec<ValueRef>,
    },
    LoopYield {
        target: BlockId,
        captures: Vec<ValueRef>,
        index: ValueRef,
        step: i64,
        origin: NodeId,
    },
}
struct Task {
    operations: Vec<NodeId>,
    index: usize,
    terminator: NodeId,
    block: BlockId,
    environment: BTreeMap<ValueRef, ValueId>,
    end: EndAction,
}

fn lower_function(
    snapshot: &Snapshot,
    function: NodeId,
    function_ids: &BTreeMap<NodeId, FunctionId>,
) -> Result<CoreFunction> {
    let Node::Function {
        parameters,
        result,
        body,
        ..
    } = snapshot.node(function)?
    else {
        return Err(invalid(function, "reachable definition is not a function"));
    };
    let body = body.ok_or_else(|| {
        LkError::new(
            ErrorCode::CompileIncomplete,
            "reachable function has no body",
        )
        .for_node(function)
    })?;
    let semantic_block = region_block(snapshot, body)?;
    let mut builder = FunctionBuilder {
        origin: function,
        result: *result,
        parameters: Vec::new(),
        value_types: Vec::new(),
        blocks: Vec::new(),
        entry: BlockId(0),
    };
    let mut parameter_types = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let Node::Parameter { ty, .. } = snapshot.node(*parameter)? else {
            return Err(invalid(
                *parameter,
                "function parameter slot is not a parameter",
            ));
        };
        parameter_types.push(*ty);
    }
    let (entry, core_parameters) = builder.block(semantic_block, &parameter_types)?;
    builder.entry = entry;
    builder.parameters.clone_from(&core_parameters);
    let mut environment = BTreeMap::new();
    for (parameter, core) in parameters.iter().zip(core_parameters) {
        environment.insert(ValueRef::FunctionParameter(*parameter), core);
    }
    let (operations, terminator, arguments) = semantic_block_parts(snapshot, semantic_block)?;
    if !arguments.is_empty() {
        return Err(invalid(
            semantic_block,
            "function entry semantic block has arguments",
        ));
    }
    let mut tasks = vec![Task {
        operations,
        index: 0,
        terminator,
        block: entry,
        environment,
        end: EndAction::Return,
    }];
    while let Some(mut task) = tasks.pop() {
        if task.index == task.operations.len() {
            finish_task(snapshot, &mut builder, task)?;
            continue;
        }
        let operation_id = task.operations[task.index];
        let Node::Operation { operation, .. } = snapshot.node(operation_id)? else {
            return Err(invalid(
                operation_id,
                "semantic body item is not an operation",
            ));
        };
        match operation {
            OperationKind::If {
                condition,
                result,
                then_region,
                else_region,
            } => {
                let condition = lower_value(&task.environment, *condition)?;
                let captures: Vec<_> = task.environment.keys().copied().collect();
                let current_arguments = capture_values(&task.environment, &captures)?;
                let then_semantic = region_block(snapshot, *then_region)?;
                let else_semantic = region_block(snapshot, *else_region)?;
                let (then_block, then_env, then_args) = captured_block(
                    &mut builder,
                    then_semantic,
                    &task.environment,
                    &captures,
                    &[],
                )?;
                let (else_block, else_env, else_args) = captured_block(
                    &mut builder,
                    else_semantic,
                    &task.environment,
                    &captures,
                    &[],
                )?;
                debug_assert_eq!(current_arguments.len(), then_args.len());
                debug_assert_eq!(current_arguments.len(), else_args.len());
                let (join, mut join_env, join_parameters) = captured_block(
                    &mut builder,
                    operation_id,
                    &task.environment,
                    &captures,
                    &[*result],
                )?;
                join_env.insert(
                    ValueRef::OperationResult {
                        operation: operation_id,
                        output: 0,
                    },
                    *join_parameters.last().ok_or_else(|| {
                        invalid(operation_id, "if join result parameter is missing")
                    })?,
                );
                builder.terminate(
                    task.block,
                    Terminator::CondBranch {
                        origin: operation_id,
                        condition,
                        then_target: then_block,
                        then_arguments: current_arguments.clone(),
                        else_target: else_block,
                        else_arguments: current_arguments,
                    },
                )?;
                task.index += 1;
                task.block = join;
                task.environment = join_env;
                tasks.push(task);
                let (else_ops, else_term, else_block_args) =
                    semantic_block_parts(snapshot, else_semantic)?;
                if !else_block_args.is_empty() {
                    return Err(invalid(
                        else_semantic,
                        "if arm block has unexpected arguments",
                    ));
                }
                tasks.push(Task {
                    operations: else_ops,
                    index: 0,
                    terminator: else_term,
                    block: else_block,
                    environment: else_env,
                    end: EndAction::YieldBranch {
                        target: join,
                        captures: captures.clone(),
                    },
                });
                let (then_ops, then_term, then_block_args) =
                    semantic_block_parts(snapshot, then_semantic)?;
                if !then_block_args.is_empty() {
                    return Err(invalid(
                        then_semantic,
                        "if arm block has unexpected arguments",
                    ));
                }
                tasks.push(Task {
                    operations: then_ops,
                    index: 0,
                    terminator: then_term,
                    block: then_block,
                    environment: then_env,
                    end: EndAction::YieldBranch {
                        target: join,
                        captures,
                    },
                });
            }
            OperationKind::ForI64 {
                start,
                end_exclusive,
                step,
                initial,
                carried,
                body_region,
            } => {
                let start_value = lower_value(&task.environment, *start)?;
                let initial_value = lower_value(&task.environment, *initial)?;
                let captures: Vec<_> = task.environment.keys().copied().collect();
                let current_captures = capture_values(&task.environment, &captures)?;
                let body_semantic = region_block(snapshot, *body_region)?;
                let (_, _, body_arguments) = semantic_block_parts(snapshot, body_semantic)?;
                if body_arguments.len() != 2 {
                    return Err(invalid(
                        body_semantic,
                        "for body requires index and carried arguments",
                    ));
                }
                let capture_types = capture_types(&builder, &task.environment, &captures)?;
                let mut header_types = capture_types.clone();
                header_types.extend([SemanticType::I64, *carried]);
                let (header, header_parameters) = builder.block(operation_id, &header_types)?;
                let header_env = environment_from_parameters(&captures, &header_parameters)?;
                let header_index = header_parameters[captures.len()];
                let header_carried = header_parameters[captures.len() + 1];
                let mut body_types = capture_types.clone();
                body_types.extend([SemanticType::I64, *carried]);
                let (body_block, body_parameters) = builder.block(body_semantic, &body_types)?;
                let mut body_env = environment_from_parameters(&captures, &body_parameters)?;
                body_env.insert(
                    ValueRef::BlockArgument(body_arguments[0]),
                    body_parameters[captures.len()],
                );
                body_env.insert(
                    ValueRef::BlockArgument(body_arguments[1]),
                    body_parameters[captures.len() + 1],
                );
                let mut exit_types = capture_types;
                exit_types.push(*carried);
                let (exit, exit_parameters) = builder.block(operation_id, &exit_types)?;
                let mut exit_env = environment_from_parameters(&captures, &exit_parameters)?;
                exit_env.insert(
                    ValueRef::OperationResult {
                        operation: operation_id,
                        output: 0,
                    },
                    exit_parameters[captures.len()],
                );
                let mut initial_arguments = current_captures;
                initial_arguments.extend([start_value, initial_value]);
                builder.terminate(
                    task.block,
                    Terminator::Branch {
                        origin: operation_id,
                        target: header,
                        arguments: initial_arguments,
                    },
                )?;
                let end_value = lower_value(&header_env, *end_exclusive)?;
                let condition = builder.value(SemanticType::Bool, operation_id)?;
                builder.instruction(
                    header,
                    Instruction::LtI64 {
                        origin: operation_id,
                        result: condition,
                        lhs: header_index,
                        rhs: end_value,
                    },
                )?;
                let mut body_edge = capture_values(&header_env, &captures)?;
                body_edge.extend([header_index, header_carried]);
                let mut exit_edge = capture_values(&header_env, &captures)?;
                exit_edge.push(header_carried);
                builder.terminate(
                    header,
                    Terminator::CondBranch {
                        origin: operation_id,
                        condition,
                        then_target: body_block,
                        then_arguments: body_edge,
                        else_target: exit,
                        else_arguments: exit_edge,
                    },
                )?;
                task.index += 1;
                task.block = exit;
                task.environment = exit_env;
                tasks.push(task);
                let (body_ops, body_term, _) = semantic_block_parts(snapshot, body_semantic)?;
                tasks.push(Task {
                    operations: body_ops,
                    index: 0,
                    terminator: body_term,
                    block: body_block,
                    environment: body_env,
                    end: EndAction::LoopYield {
                        target: header,
                        captures,
                        index: ValueRef::BlockArgument(body_arguments[0]),
                        step: *step,
                        origin: operation_id,
                    },
                });
            }
            OperationKind::Return { .. } | OperationKind::Yield { .. } => {
                return Err(invalid(
                    operation_id,
                    "terminator appeared in semantic operation list",
                ));
            }
            OperationKind::Hole { .. } => {
                return Err(LkError::new(
                    ErrorCode::CompileIncomplete,
                    "hole reached Core lowering",
                )
                .for_node(operation_id));
            }
            _ => {
                let result_type = semantic_result_type(snapshot, operation)?;
                let result = builder.value(result_type, operation_id)?;
                let instruction = lower_instruction(
                    operation_id,
                    operation,
                    result,
                    &task.environment,
                    function_ids,
                )?;
                builder.instruction(task.block, instruction)?;
                task.environment.insert(
                    ValueRef::OperationResult {
                        operation: operation_id,
                        output: 0,
                    },
                    result,
                );
                task.index += 1;
                tasks.push(task);
            }
        }
    }
    builder.finish()
}

fn finish_task(snapshot: &Snapshot, builder: &mut FunctionBuilder, task: Task) -> Result<()> {
    let Node::Operation { operation, .. } = snapshot.node(task.terminator)? else {
        return Err(invalid(
            task.terminator,
            "semantic terminator is not an operation",
        ));
    };
    let value = match operation {
        OperationKind::Return { value } | OperationKind::Yield { value } => *value,
        _ => {
            return Err(invalid(
                task.terminator,
                "unexpected semantic terminator kind",
            ));
        }
    };
    let yielded = lower_value(&task.environment, value)?;
    match task.end {
        EndAction::Return => {
            if !matches!(operation, OperationKind::Return { .. }) {
                return Err(invalid(task.terminator, "function body must return"));
            }
            builder.terminate(
                task.block,
                Terminator::Return {
                    origin: task.terminator,
                    value: yielded,
                },
            )
        }
        EndAction::YieldBranch { target, captures } => {
            if !matches!(operation, OperationKind::Yield { .. }) {
                return Err(invalid(task.terminator, "structured region must yield"));
            }
            let mut arguments = capture_values(&task.environment, &captures)?;
            arguments.push(yielded);
            builder.terminate(
                task.block,
                Terminator::Branch {
                    origin: task.terminator,
                    target,
                    arguments,
                },
            )
        }
        EndAction::LoopYield {
            target,
            captures,
            index,
            step,
            origin,
        } => {
            if !matches!(operation, OperationKind::Yield { .. }) {
                return Err(invalid(task.terminator, "loop body must yield"));
            }
            let index_value = lower_value(&task.environment, index)?;
            let step_value = builder.value(SemanticType::I64, origin)?;
            builder.instruction(
                task.block,
                Instruction::ConstI64 {
                    origin,
                    result: step_value,
                    value: step,
                },
            )?;
            let next_index = builder.value(SemanticType::I64, origin)?;
            builder.instruction(
                task.block,
                Instruction::AddI64 {
                    origin,
                    result: next_index,
                    lhs: index_value,
                    rhs: step_value,
                },
            )?;
            let mut arguments = capture_values(&task.environment, &captures)?;
            arguments.extend([next_index, yielded]);
            builder.terminate(
                task.block,
                Terminator::Branch {
                    origin: task.terminator,
                    target,
                    arguments,
                },
            )
        }
    }
}

fn lower_instruction(
    origin: NodeId,
    operation: &OperationKind,
    result: ValueId,
    environment: &BTreeMap<ValueRef, ValueId>,
    function_ids: &BTreeMap<NodeId, FunctionId>,
) -> Result<Instruction> {
    Ok(match operation {
        OperationKind::ConstUnit => Instruction::ConstUnit { origin, result },
        OperationKind::ConstBool(value) => Instruction::ConstBool {
            origin,
            result,
            value: *value,
        },
        OperationKind::ConstI64(value) => Instruction::ConstI64 {
            origin,
            result,
            value: *value,
        },
        OperationKind::AddI64 { lhs, rhs } => Instruction::AddI64 {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::LtI64 { lhs, rhs } => Instruction::LtI64 {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::Call {
            function,
            arguments,
        } => Instruction::Call {
            origin,
            result,
            function: *function_ids.get(function).ok_or_else(|| {
                invalid(
                    *function,
                    "call target is absent from reachable function map",
                )
            })?,
            arguments: arguments
                .iter()
                .map(|value| lower_value(environment, *value))
                .collect::<Result<_>>()?,
        },
        OperationKind::Hole { .. }
        | OperationKind::If { .. }
        | OperationKind::ForI64 { .. }
        | OperationKind::Return { .. }
        | OperationKind::Yield { .. } => {
            return Err(invalid(
                origin,
                "structured operation entered scalar instruction lowering",
            ));
        }
    })
}

fn semantic_result_type(snapshot: &Snapshot, operation: &OperationKind) -> Result<SemanticType> {
    Ok(match operation {
        OperationKind::ConstUnit => SemanticType::Unit,
        OperationKind::ConstBool(_) | OperationKind::LtI64 { .. } => SemanticType::Bool,
        OperationKind::ConstI64(_) | OperationKind::AddI64 { .. } => SemanticType::I64,
        OperationKind::Call { function, .. } => match snapshot.node(*function)? {
            Node::Function { result, .. } => *result,
            _ => return Err(invalid(*function, "call target is not a function")),
        },
        OperationKind::Hole { expected } => *expected,
        OperationKind::If { result, .. } => *result,
        OperationKind::ForI64 { carried, .. } => *carried,
        OperationKind::Return { .. } | OperationKind::Yield { .. } => {
            return Err(invalid(
                operation
                    .definition_target()
                    .unwrap_or_else(|| snapshot.root()),
                "terminator has no result",
            ));
        }
    })
}

fn captured_block(
    builder: &mut FunctionBuilder,
    origin: NodeId,
    environment: &BTreeMap<ValueRef, ValueId>,
    captures: &[ValueRef],
    extra_types: &[SemanticType],
) -> Result<(BlockId, BTreeMap<ValueRef, ValueId>, Vec<ValueId>)> {
    let mut types = capture_types(builder, environment, captures)?;
    types.extend_from_slice(extra_types);
    let (block, parameters) = builder.block(origin, &types)?;
    let env = environment_from_parameters(captures, &parameters)?;
    Ok((block, env, parameters))
}
fn capture_types(
    builder: &FunctionBuilder,
    environment: &BTreeMap<ValueRef, ValueId>,
    captures: &[ValueRef],
) -> Result<Vec<SemanticType>> {
    captures
        .iter()
        .map(|key| {
            let value = environment.get(key).ok_or_else(|| {
                invalid(
                    builder.origin,
                    "capture is absent from semantic environment",
                )
            })?;
            builder
                .value_types
                .get(
                    usize::try_from(value.0)
                        .map_err(|_| invalid(builder.origin, "value index overflows host"))?,
                )
                .copied()
                .ok_or_else(|| invalid(builder.origin, "capture value type is absent"))
        })
        .collect()
}
fn environment_from_parameters(
    captures: &[ValueRef],
    parameters: &[ValueId],
) -> Result<BTreeMap<ValueRef, ValueId>> {
    if parameters.len() < captures.len() {
        return Err(LkError::new(
            ErrorCode::CoreIrInvalid,
            "capture parameter list is truncated",
        ));
    }
    Ok(captures
        .iter()
        .copied()
        .zip(parameters.iter().copied())
        .collect())
}
fn capture_values(
    environment: &BTreeMap<ValueRef, ValueId>,
    captures: &[ValueRef],
) -> Result<Vec<ValueId>> {
    captures
        .iter()
        .map(|key| {
            environment.get(key).copied().ok_or_else(|| {
                LkError::new(ErrorCode::CoreIrInvalid, "capture is not visible at branch")
            })
        })
        .collect()
}
fn region_block(snapshot: &Snapshot, region: NodeId) -> Result<NodeId> {
    let Node::Region { blocks, .. } = snapshot.node(region)? else {
        return Err(invalid(region, "body does not reference a region"));
    };
    if blocks.len() != 1 {
        return Err(invalid(
            region,
            "structured region must contain exactly one block",
        ));
    }
    Ok(blocks[0])
}
fn semantic_block_parts(
    snapshot: &Snapshot,
    block: NodeId,
) -> Result<(Vec<NodeId>, NodeId, Vec<NodeId>)> {
    let Node::Block {
        arguments,
        operations,
        terminator,
        ..
    } = snapshot.node(block)?
    else {
        return Err(invalid(block, "region child is not a block"));
    };
    Ok((
        operations.clone(),
        terminator.ok_or_else(|| {
            LkError::new(
                ErrorCode::CompileIncomplete,
                "reachable block has no terminator",
            )
            .for_node(block)
        })?,
        arguments.clone(),
    ))
}
fn lower_value(environment: &BTreeMap<ValueRef, ValueId>, value: ValueRef) -> Result<ValueId> {
    environment.get(&value).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::CoreIrInvalid,
            "semantic value is not visible in the current lowered block",
        )
        .for_node(value.referenced_node())
    })
}
fn dense_u32(value: usize, origin: NodeId, category: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("Core IR {category} count exceeds dense representation"),
        )
        .for_node(origin)
    })
}
fn invalid(origin: NodeId, message: &str) -> LkError {
    LkError::new(ErrorCode::CoreIrInvalid, message).for_node(origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Workspace;
    use crate::interpret::{RunPolicy, RuntimeValue, compile_and_run};
    use crate::query::{PageRequest, Query, QueryResult};
    use crate::transaction::{
        ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
        FunctionParameterDraft, NodeTarget, Transaction, TransactionMode, TransactionOp,
        TransactionResponseSpec, YieldingBodyDraft,
    };
    use crate::{LocalHandle, Revision, ValueDraft, WorkspaceId};

    fn local(value: u32) -> NodeTarget {
        NodeTarget::Local(LocalHandle::new(value))
    }
    fn result(value: u32) -> ValueDraft {
        ValueDraft::OperationResult {
            operation: local(value),
            output: 0,
        }
    }
    fn parameter(value: u32) -> ValueDraft {
        ValueDraft::FunctionParameter(local(value))
    }
    fn argument(value: u32) -> ValueDraft {
        ValueDraft::BlockArgument(local(value))
    }
    fn expression(handle: u32, operation: ExpressionKindDraft) -> ExpressionDraft {
        ExpressionDraft {
            handle: LocalHandle::new(handle),
            operation,
        }
    }
    fn function(
        handle: u32,
        module: u32,
        name: &str,
        parameters: Vec<(u32, &str, SemanticType)>,
        result_type: SemanticType,
        body: Option<FunctionBodyDraft>,
    ) -> TransactionOp {
        TransactionOp::CreateFunction {
            handle: LocalHandle::new(handle),
            module: local(module),
            name: name.into(),
            parameters: parameters
                .into_iter()
                .map(|(handle, name, ty)| FunctionParameterDraft {
                    handle: LocalHandle::new(handle),
                    name: name.into(),
                    ty,
                })
                .collect(),
            result: result_type,
            body,
        }
    }
    fn body(
        operations: Vec<ExpressionDraft>,
        return_value: ValueDraft,
    ) -> Option<FunctionBodyDraft> {
        Some(FunctionBodyDraft {
            operations,
            return_value,
        })
    }
    fn yielding(operations: Vec<ExpressionDraft>, yield_value: ValueDraft) -> YieldingBodyDraft {
        YieldingBodyDraft {
            operations,
            yield_value,
        }
    }
    fn run(snapshot: &Snapshot, entry: NodeId, arguments: &[RuntimeValue]) -> Result<RuntimeValue> {
        Ok(compile_and_run(
            snapshot,
            entry,
            arguments,
            RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 10_000,
            },
        )?
        .value)
    }

    #[test]
    fn compile_incomplete_diagnostic_is_bounded_while_blocker_query_remains_paginated() {
        let workspace_id = WorkspaceId::from_bytes([0x92; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let mut expressions = (0..100_u32)
            .map(|index| {
                expression(
                    100 + index,
                    ExpressionKindDraft::Hole {
                        expected: SemanticType::I64,
                    },
                )
            })
            .collect::<Vec<_>>();
        expressions.push(expression(300, ExpressionKindDraft::ConstI64(0)));
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: workspace_id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            handle: LocalHandle::new(1),
                            name: "app".into(),
                        },
                        TransactionOp::CreateModule {
                            handle: LocalHandle::new(2),
                            package: local(1),
                            name: "root".into(),
                        },
                        function(
                            3,
                            2,
                            "main",
                            Vec::new(),
                            SemanticType::I64,
                            body(expressions, result(300)),
                        ),
                        TransactionOp::SetEntryFunction {
                            package: local(1),
                            function: local(3),
                        },
                    ],
                },
                response: TransactionResponseSpec {
                    return_handles: vec![LocalHandle::new(3)],
                },
            })
            .expect("incomplete transaction");
        let entry = prepared.receipt.returned_bindings[0].1;
        let snapshot = &prepared.snapshot;
        let blockers = query::entry_blockers(snapshot, entry).expect("blockers");
        assert_eq!(blockers.len(), 100);
        let error = compile(snapshot, entry).expect_err("incomplete");
        assert_eq!(error.code, ErrorCode::CompileIncomplete);
        assert_eq!(error.target, Some(entry));
        assert_eq!(error.related.len(), crate::error::MAX_ERROR_RELATED_IDS);
        let expected = blockers
            .iter()
            .map(|blocker| blocker.target.unwrap_or(blocker.owner))
            .take(crate::error::MAX_ERROR_RELATED_IDS)
            .collect::<Vec<_>>();
        assert_eq!(&*error.related, expected);

        let first = match query::execute(
            snapshot,
            &Query::Blockers {
                page: PageRequest {
                    limit: 64,
                    after: None,
                },
            },
            None,
        )
        .expect("first page")
        {
            QueryResult::Blockers(page) => page,
            _ => panic!("blocker result"),
        };
        assert_eq!(first.total, Some(100));
        assert_eq!(first.items.len(), 64);
        let second = match query::execute(
            snapshot,
            &Query::Blockers {
                page: PageRequest {
                    limit: 64,
                    after: first.next,
                },
            },
            None,
        )
        .expect("second page")
        {
            QueryResult::Blockers(page) => page,
            _ => panic!("blocker result"),
        };
        assert_eq!(second.total, Some(100));
        assert_eq!(second.items.len(), 36);
    }

    fn structured_snapshot() -> (
        crate::transaction::PreparedTransaction,
        BTreeMap<u32, NodeId>,
    ) {
        let workspace_id = WorkspaceId::from_bytes([0x91; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let mut operations = vec![
            TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "app".into(),
            },
            TransactionOp::CreateModule {
                handle: LocalHandle::new(2),
                package: local(1),
                name: "root".into(),
            },
        ];
        operations.push(function(
            10,
            2,
            "range_sum",
            vec![
                (11, "start", SemanticType::I64),
                (12, "end", SemanticType::I64),
            ],
            SemanticType::I64,
            body(
                vec![
                    expression(13, ExpressionKindDraft::ConstI64(0)),
                    expression(
                        14,
                        ExpressionKindDraft::ForI64 {
                            start: parameter(11),
                            end_exclusive: parameter(12),
                            step: 1,
                            initial: result(13),
                            carried: SemanticType::I64,
                            index_handle: LocalHandle::new(15),
                            carried_handle: LocalHandle::new(16),
                            body: yielding(
                                vec![expression(
                                    17,
                                    ExpressionKindDraft::AddI64 {
                                        lhs: argument(16),
                                        rhs: argument(15),
                                    },
                                )],
                                result(17),
                            ),
                        },
                    ),
                ],
                result(14),
            ),
        ));
        operations.push(function(
            20,
            2,
            "normalize_and_sum",
            vec![(21, "n", SemanticType::I64)],
            SemanticType::I64,
            body(
                vec![
                    expression(22, ExpressionKindDraft::ConstI64(0)),
                    expression(
                        23,
                        ExpressionKindDraft::LtI64 {
                            lhs: parameter(21),
                            rhs: result(22),
                        },
                    ),
                    expression(
                        24,
                        ExpressionKindDraft::If {
                            condition: result(23),
                            result: SemanticType::I64,
                            then_body: yielding(vec![], result(22)),
                            else_body: yielding(
                                vec![expression(
                                    25,
                                    ExpressionKindDraft::Call {
                                        function: local(10),
                                        arguments: vec![result(22), parameter(21)],
                                    },
                                )],
                                result(25),
                            ),
                        },
                    ),
                ],
                result(24),
            ),
        ));
        operations.push(function(
            30,
            2,
            "main",
            vec![],
            SemanticType::I64,
            body(
                vec![
                    expression(31, ExpressionKindDraft::ConstI64(0)),
                    expression(32, ExpressionKindDraft::ConstI64(101)),
                    expression(
                        33,
                        ExpressionKindDraft::Call {
                            function: local(10),
                            arguments: vec![result(31), result(32)],
                        },
                    ),
                ],
                result(33),
            ),
        ));
        operations.push(function(
            40,
            2,
            "choose",
            vec![(41, "condition", SemanticType::Bool)],
            SemanticType::I64,
            body(
                vec![expression(
                    42,
                    ExpressionKindDraft::If {
                        condition: parameter(41),
                        result: SemanticType::I64,
                        then_body: yielding(
                            vec![expression(43, ExpressionKindDraft::ConstI64(7))],
                            result(43),
                        ),
                        else_body: yielding(
                            vec![expression(44, ExpressionKindDraft::ConstI64(9))],
                            result(44),
                        ),
                    },
                )],
                result(42),
            ),
        ));
        operations.push(function(
            50,
            2,
            "lazy",
            vec![(51, "condition", SemanticType::Bool)],
            SemanticType::I64,
            body(
                vec![expression(
                    52,
                    ExpressionKindDraft::If {
                        condition: parameter(51),
                        result: SemanticType::I64,
                        then_body: yielding(
                            vec![expression(53, ExpressionKindDraft::ConstI64(1))],
                            result(53),
                        ),
                        else_body: yielding(
                            vec![
                                expression(54, ExpressionKindDraft::ConstI64(i64::MAX)),
                                expression(55, ExpressionKindDraft::ConstI64(1)),
                                expression(
                                    56,
                                    ExpressionKindDraft::AddI64 {
                                        lhs: result(54),
                                        rhs: result(55),
                                    },
                                ),
                            ],
                            result(56),
                        ),
                    },
                )],
                result(52),
            ),
        ));
        operations.push(function(
            60,
            2,
            "unit_if",
            vec![(61, "condition", SemanticType::Bool)],
            SemanticType::Unit,
            body(
                vec![expression(
                    62,
                    ExpressionKindDraft::If {
                        condition: parameter(61),
                        result: SemanticType::Unit,
                        then_body: yielding(
                            vec![expression(63, ExpressionKindDraft::ConstUnit)],
                            result(63),
                        ),
                        else_body: yielding(
                            vec![expression(64, ExpressionKindDraft::ConstUnit)],
                            result(64),
                        ),
                    },
                )],
                result(62),
            ),
        ));
        operations.push(function(
            70,
            2,
            "step_two",
            vec![],
            SemanticType::I64,
            body(
                vec![
                    expression(71, ExpressionKindDraft::ConstI64(0)),
                    expression(72, ExpressionKindDraft::ConstI64(6)),
                    expression(
                        73,
                        ExpressionKindDraft::ForI64 {
                            start: result(71),
                            end_exclusive: result(72),
                            step: 2,
                            initial: result(71),
                            carried: SemanticType::I64,
                            index_handle: LocalHandle::new(74),
                            carried_handle: LocalHandle::new(75),
                            body: yielding(
                                vec![expression(
                                    76,
                                    ExpressionKindDraft::AddI64 {
                                        lhs: argument(75),
                                        rhs: argument(74),
                                    },
                                )],
                                result(76),
                            ),
                        },
                    ),
                ],
                result(73),
            ),
        ));
        operations.push(function(
            80,
            2,
            "capture",
            vec![],
            SemanticType::I64,
            body(
                vec![
                    expression(81, ExpressionKindDraft::ConstI64(0)),
                    expression(82, ExpressionKindDraft::ConstI64(4)),
                    expression(83, ExpressionKindDraft::ConstI64(10)),
                    expression(
                        84,
                        ExpressionKindDraft::ForI64 {
                            start: result(81),
                            end_exclusive: result(82),
                            step: 1,
                            initial: result(81),
                            carried: SemanticType::I64,
                            index_handle: LocalHandle::new(85),
                            carried_handle: LocalHandle::new(86),
                            body: yielding(
                                vec![
                                    expression(
                                        87,
                                        ExpressionKindDraft::AddI64 {
                                            lhs: argument(85),
                                            rhs: result(83),
                                        },
                                    ),
                                    expression(
                                        88,
                                        ExpressionKindDraft::AddI64 {
                                            lhs: argument(86),
                                            rhs: result(87),
                                        },
                                    ),
                                ],
                                result(88),
                            ),
                        },
                    ),
                ],
                result(84),
            ),
        ));
        operations.push(function(
            90,
            2,
            "recurse_once",
            vec![(91, "again", SemanticType::Bool)],
            SemanticType::I64,
            body(
                vec![expression(
                    92,
                    ExpressionKindDraft::If {
                        condition: parameter(91),
                        result: SemanticType::I64,
                        then_body: yielding(
                            vec![
                                expression(93, ExpressionKindDraft::ConstBool(false)),
                                expression(
                                    94,
                                    ExpressionKindDraft::Call {
                                        function: local(90),
                                        arguments: vec![result(93)],
                                    },
                                ),
                            ],
                            result(94),
                        ),
                        else_body: yielding(
                            vec![expression(95, ExpressionKindDraft::ConstI64(1))],
                            result(95),
                        ),
                    },
                )],
                result(92),
            ),
        ));
        operations.push(function(
            100,
            2,
            "mutual_a",
            vec![(101, "again", SemanticType::Bool)],
            SemanticType::I64,
            body(
                vec![expression(
                    102,
                    ExpressionKindDraft::If {
                        condition: parameter(101),
                        result: SemanticType::I64,
                        then_body: yielding(
                            vec![
                                expression(103, ExpressionKindDraft::ConstBool(false)),
                                expression(
                                    104,
                                    ExpressionKindDraft::Call {
                                        function: local(110),
                                        arguments: vec![result(103)],
                                    },
                                ),
                            ],
                            result(104),
                        ),
                        else_body: yielding(
                            vec![expression(105, ExpressionKindDraft::ConstI64(2))],
                            result(105),
                        ),
                    },
                )],
                result(102),
            ),
        ));
        operations.push(function(
            110,
            2,
            "mutual_b",
            vec![(111, "again", SemanticType::Bool)],
            SemanticType::I64,
            body(
                vec![expression(
                    112,
                    ExpressionKindDraft::If {
                        condition: parameter(111),
                        result: SemanticType::I64,
                        then_body: yielding(
                            vec![
                                expression(113, ExpressionKindDraft::ConstBool(false)),
                                expression(
                                    114,
                                    ExpressionKindDraft::Call {
                                        function: local(100),
                                        arguments: vec![result(113)],
                                    },
                                ),
                            ],
                            result(114),
                        ),
                        else_body: yielding(
                            vec![expression(115, ExpressionKindDraft::ConstI64(3))],
                            result(115),
                        ),
                    },
                )],
                result(112),
            ),
        ));
        operations.push(function(
            120,
            2,
            "index_overflow",
            vec![],
            SemanticType::I64,
            body(
                vec![
                    expression(121, ExpressionKindDraft::ConstI64(i64::MAX - 1)),
                    expression(122, ExpressionKindDraft::ConstI64(i64::MAX)),
                    expression(123, ExpressionKindDraft::ConstI64(0)),
                    expression(
                        124,
                        ExpressionKindDraft::ForI64 {
                            start: result(121),
                            end_exclusive: result(122),
                            step: 2,
                            initial: result(123),
                            carried: SemanticType::I64,
                            index_handle: LocalHandle::new(125),
                            carried_handle: LocalHandle::new(126),
                            body: yielding(vec![], argument(126)),
                        },
                    ),
                ],
                result(124),
            ),
        ));
        operations.push(function(
            130,
            2,
            "bodyless",
            vec![],
            SemanticType::I64,
            None,
        ));
        operations.push(function(
            140,
            2,
            "unreachable_hole",
            vec![],
            SemanticType::I64,
            body(
                vec![expression(
                    141,
                    ExpressionKindDraft::Hole {
                        expected: SemanticType::I64,
                    },
                )],
                result(141),
            ),
        ));
        operations.push(function(
            150,
            2,
            "nested_loops",
            vec![],
            SemanticType::I64,
            body(
                vec![
                    expression(151, ExpressionKindDraft::ConstI64(0)),
                    expression(152, ExpressionKindDraft::ConstI64(3)),
                    expression(153, ExpressionKindDraft::ConstI64(2)),
                    expression(
                        154,
                        ExpressionKindDraft::ForI64 {
                            start: result(151),
                            end_exclusive: result(152),
                            step: 1,
                            initial: result(151),
                            carried: SemanticType::I64,
                            index_handle: LocalHandle::new(155),
                            carried_handle: LocalHandle::new(156),
                            body: yielding(
                                vec![expression(
                                    157,
                                    ExpressionKindDraft::ForI64 {
                                        start: result(151),
                                        end_exclusive: result(153),
                                        step: 1,
                                        initial: argument(156),
                                        carried: SemanticType::I64,
                                        index_handle: LocalHandle::new(158),
                                        carried_handle: LocalHandle::new(159),
                                        body: yielding(
                                            vec![expression(
                                                160,
                                                ExpressionKindDraft::AddI64 {
                                                    lhs: argument(159),
                                                    rhs: argument(158),
                                                },
                                            )],
                                            result(160),
                                        ),
                                    },
                                )],
                                result(157),
                            ),
                        },
                    ),
                ],
                result(154),
            ),
        ));
        operations.push(function(
            170,
            2,
            "loop_in_if",
            vec![(171, "condition", SemanticType::Bool)],
            SemanticType::I64,
            body(
                vec![
                    expression(172, ExpressionKindDraft::ConstI64(0)),
                    expression(173, ExpressionKindDraft::ConstI64(4)),
                    expression(
                        174,
                        ExpressionKindDraft::If {
                            condition: parameter(171),
                            result: SemanticType::I64,
                            then_body: yielding(
                                vec![expression(
                                    175,
                                    ExpressionKindDraft::ForI64 {
                                        start: result(172),
                                        end_exclusive: result(173),
                                        step: 1,
                                        initial: result(172),
                                        carried: SemanticType::I64,
                                        index_handle: LocalHandle::new(176),
                                        carried_handle: LocalHandle::new(177),
                                        body: yielding(
                                            vec![expression(
                                                178,
                                                ExpressionKindDraft::AddI64 {
                                                    lhs: argument(177),
                                                    rhs: argument(176),
                                                },
                                            )],
                                            result(178),
                                        ),
                                    },
                                )],
                                result(175),
                            ),
                            else_body: yielding(vec![], result(172)),
                        },
                    ),
                ],
                result(174),
            ),
        ));
        operations.push(function(
            200,
            2,
            "zero_callee",
            vec![],
            SemanticType::I64,
            body(
                vec![expression(201, ExpressionKindDraft::ConstI64(5))],
                result(201),
            ),
        ));
        operations.push(function(
            210,
            2,
            "zero_caller",
            vec![],
            SemanticType::I64,
            body(
                vec![expression(
                    211,
                    ExpressionKindDraft::Call {
                        function: local(200),
                        arguments: vec![],
                    },
                )],
                result(211),
            ),
        ));
        operations.push(function(
            220,
            2,
            "infinite",
            vec![],
            SemanticType::I64,
            body(
                vec![expression(
                    221,
                    ExpressionKindDraft::Call {
                        function: local(220),
                        arguments: vec![],
                    },
                )],
                result(221),
            ),
        ));
        operations.push(function(
            230,
            2,
            "calls_bodyless",
            vec![],
            SemanticType::I64,
            body(
                vec![expression(
                    231,
                    ExpressionKindDraft::Call {
                        function: local(130),
                        arguments: vec![],
                    },
                )],
                result(231),
            ),
        ));
        operations.push(function(
            240,
            2,
            "calls_hole",
            vec![],
            SemanticType::I64,
            body(
                vec![expression(
                    241,
                    ExpressionKindDraft::Call {
                        function: local(140),
                        arguments: vec![],
                    },
                )],
                result(241),
            ),
        ));
        operations.push(function(
            260,
            2,
            "if_in_loop",
            vec![],
            SemanticType::I64,
            body(
                vec![
                    expression(261, ExpressionKindDraft::ConstI64(0)),
                    expression(262, ExpressionKindDraft::ConstI64(3)),
                    expression(263, ExpressionKindDraft::ConstBool(true)),
                    expression(
                        264,
                        ExpressionKindDraft::ForI64 {
                            start: result(261),
                            end_exclusive: result(262),
                            step: 1,
                            initial: result(261),
                            carried: SemanticType::I64,
                            index_handle: LocalHandle::new(265),
                            carried_handle: LocalHandle::new(266),
                            body: yielding(
                                vec![expression(
                                    267,
                                    ExpressionKindDraft::If {
                                        condition: result(263),
                                        result: SemanticType::I64,
                                        then_body: yielding(
                                            vec![expression(
                                                268,
                                                ExpressionKindDraft::AddI64 {
                                                    lhs: argument(266),
                                                    rhs: argument(265),
                                                },
                                            )],
                                            result(268),
                                        ),
                                        else_body: yielding(vec![], argument(266)),
                                    },
                                )],
                                result(267),
                            ),
                        },
                    ),
                ],
                result(264),
            ),
        ));
        operations.push(function(
            250,
            2,
            "bool_if",
            vec![(251, "condition", SemanticType::Bool)],
            SemanticType::Bool,
            body(
                vec![expression(
                    252,
                    ExpressionKindDraft::If {
                        condition: parameter(251),
                        result: SemanticType::Bool,
                        then_body: yielding(
                            vec![expression(253, ExpressionKindDraft::ConstBool(true))],
                            result(253),
                        ),
                        else_body: yielding(
                            vec![expression(254, ExpressionKindDraft::ConstBool(false))],
                            result(254),
                        ),
                    },
                )],
                result(252),
            ),
        ));
        operations.push(TransactionOp::SetEntryFunction {
            package: local(1),
            function: local(30),
        });
        let handles: Vec<_> = [
            10, 20, 30, 31, 32, 40, 50, 56, 60, 70, 73, 76, 80, 90, 94, 100, 110, 120, 124, 130,
            140, 150, 170, 200, 210, 220, 221, 230, 240, 250, 260,
        ]
        .into_iter()
        .map(LocalHandle::new)
        .collect();
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: workspace_id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::ValidateOnly,
                    operations,
                },
                response: TransactionResponseSpec {
                    return_handles: handles,
                },
            })
            .expect("structured program");
        let ids = prepared
            .receipt
            .returned_bindings
            .iter()
            .map(|(handle, node)| (handle.get(), *node))
            .collect();
        (prepared, ids)
    }

    #[test]
    fn structured_lowering_executes_calls_if_loops_captures_recursion_and_unreachable_incomplete_definitions()
     {
        let (prepared, ids) = structured_snapshot();
        let snapshot = &prepared.snapshot;
        assert_eq!(
            run(snapshot, ids[&30], &[]).expect("main"),
            RuntimeValue::I64(5050)
        );
        assert_eq!(
            run(snapshot, ids[&20], &[RuntimeValue::I64(-3)]).expect("negative"),
            RuntimeValue::I64(0)
        );
        assert_eq!(
            run(snapshot, ids[&20], &[RuntimeValue::I64(11)]).expect("eleven"),
            RuntimeValue::I64(55)
        );
        assert_eq!(
            run(
                snapshot,
                ids[&10],
                &[RuntimeValue::I64(5), RuntimeValue::I64(5)]
            )
            .expect("zero loop"),
            RuntimeValue::I64(0)
        );
        assert_eq!(
            run(
                snapshot,
                ids[&10],
                &[RuntimeValue::I64(6), RuntimeValue::I64(5)]
            )
            .expect("greater"),
            RuntimeValue::I64(0)
        );
        assert_eq!(
            run(
                snapshot,
                ids[&10],
                &[RuntimeValue::I64(4), RuntimeValue::I64(5)]
            )
            .expect("one"),
            RuntimeValue::I64(4)
        );
        assert_eq!(
            run(
                snapshot,
                ids[&10],
                &[RuntimeValue::I64(2), RuntimeValue::I64(5)]
            )
            .expect("ordered parameters"),
            RuntimeValue::I64(9)
        );
        assert_eq!(
            run(snapshot, ids[&40], &[RuntimeValue::Bool(true)]).expect("then"),
            RuntimeValue::I64(7)
        );
        assert_eq!(
            run(snapshot, ids[&40], &[RuntimeValue::Bool(false)]).expect("else"),
            RuntimeValue::I64(9)
        );
        assert_eq!(
            run(snapshot, ids[&50], &[RuntimeValue::Bool(true)]).expect("lazy"),
            RuntimeValue::I64(1)
        );
        let selected_overflow = compile_and_run(
            snapshot,
            ids[&50],
            &[RuntimeValue::Bool(false)],
            RunPolicy {
                fuel: 100,
                maximum_frames: 10,
            },
        )
        .expect_err("selected overflow");
        assert_eq!(selected_overflow.code, ErrorCode::RuntimeTrap);
        assert_eq!(selected_overflow.target, Some(ids[&56]));
        assert_eq!(
            run(snapshot, ids[&60], &[RuntimeValue::Bool(false)]).expect("unit"),
            RuntimeValue::Unit
        );
        assert_eq!(
            run(snapshot, ids[&70], &[]).expect("step"),
            RuntimeValue::I64(6)
        );
        assert_eq!(
            run(snapshot, ids[&80], &[]).expect("capture"),
            RuntimeValue::I64(46)
        );
        assert_eq!(
            run(snapshot, ids[&150], &[]).expect("nested loops"),
            RuntimeValue::I64(3)
        );
        assert_eq!(
            run(snapshot, ids[&170], &[RuntimeValue::Bool(true)]).expect("loop in if"),
            RuntimeValue::I64(6)
        );
        assert_eq!(
            run(snapshot, ids[&170], &[RuntimeValue::Bool(false)]).expect("lazy loop arm"),
            RuntimeValue::I64(0)
        );
        assert_eq!(
            run(snapshot, ids[&210], &[]).expect("zero argument call"),
            RuntimeValue::I64(5)
        );
        assert_eq!(
            run(snapshot, ids[&250], &[RuntimeValue::Bool(true)]).expect("bool if true"),
            RuntimeValue::Bool(true)
        );
        assert_eq!(
            run(snapshot, ids[&250], &[RuntimeValue::Bool(false)]).expect("bool if false"),
            RuntimeValue::Bool(false)
        );
        assert_eq!(
            run(snapshot, ids[&260], &[]).expect("if in loop"),
            RuntimeValue::I64(3)
        );
        assert_eq!(
            run(snapshot, ids[&90], &[RuntimeValue::Bool(true)]).expect("direct recursion"),
            RuntimeValue::I64(1)
        );
        assert_eq!(
            run(snapshot, ids[&100], &[RuntimeValue::Bool(true)]).expect("mutual recursion"),
            RuntimeValue::I64(3)
        );
        let trap = compile_and_run(
            snapshot,
            ids[&120],
            &[],
            RunPolicy {
                fuel: 100,
                maximum_frames: 10,
            },
        )
        .expect_err("index overflow");
        assert_eq!(trap.code, ErrorCode::RuntimeTrap);
        assert_eq!(trap.target, Some(ids[&124]));
        assert_eq!(
            run(snapshot, ids[&30], &[]).expect("usable after trap"),
            RuntimeValue::I64(5050)
        );
    }

    #[test]
    fn core_lowering_is_exact_and_stable_across_snapshot_insertion_order() {
        let (prepared, ids) = structured_snapshot();
        let snapshot = &prepared.snapshot;
        let program = compile(snapshot, ids[&30]).expect("compile main closure");
        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.entry, FunctionId(1));
        assert_eq!(
            program
                .functions
                .iter()
                .map(|function| function.origin)
                .collect::<Vec<_>>(),
            vec![ids[&10], ids[&30]]
        );

        let range = &program.functions[0];
        assert_eq!(range.parameters, vec![ValueId(0), ValueId(1)]);
        assert_eq!(range.entry, BlockId(0));
        assert_eq!(
            range
                .blocks
                .iter()
                .map(|block| block.parameters.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![ValueId(0), ValueId(1)],
                vec![ValueId(3), ValueId(4), ValueId(5), ValueId(6), ValueId(7)],
                vec![
                    ValueId(8),
                    ValueId(9),
                    ValueId(10),
                    ValueId(11),
                    ValueId(12)
                ],
                vec![ValueId(13), ValueId(14), ValueId(15), ValueId(16)],
            ]
        );
        assert!(
            matches!(&range.blocks[0].terminator, Terminator::Branch { target: BlockId(1), arguments, .. } if arguments == &vec![ValueId(0), ValueId(1), ValueId(2), ValueId(0), ValueId(2)])
        );
        assert!(
            matches!(&range.blocks[1].terminator, Terminator::CondBranch { then_target: BlockId(2), then_arguments, else_target: BlockId(3), else_arguments, .. } if then_arguments == &vec![ValueId(3), ValueId(4), ValueId(5), ValueId(6), ValueId(7)] && else_arguments == &vec![ValueId(3), ValueId(4), ValueId(5), ValueId(7)])
        );
        assert!(
            matches!(&range.blocks[2].terminator, Terminator::Branch { target: BlockId(1), arguments, .. } if arguments == &vec![ValueId(8), ValueId(9), ValueId(10), ValueId(20), ValueId(18)])
        );

        let mut reversed = snapshot
            .nodes()
            .map(|(id, node)| (id, node.clone()))
            .collect::<Vec<_>>();
        reversed.reverse();
        let perturbed = Snapshot::from_parts(
            snapshot.workspace(),
            snapshot.revision(),
            snapshot.root(),
            snapshot.next_serial(),
            BTreeSet::new(),
            reversed.into_iter().collect(),
        )
        .expect("equivalent snapshot");
        assert_eq!(
            compile(&perturbed, ids[&30]).expect("perturbed compile"),
            program
        );
    }

    #[test]
    fn run_arguments_fuel_frames_and_reachable_completeness_are_exact() {
        let (prepared, ids) = structured_snapshot();
        let snapshot = &prepared.snapshot;
        let policy = compile_and_run(
            snapshot,
            ids[&20],
            &[RuntimeValue::I64(1)],
            RunPolicy {
                fuel: 0,
                maximum_frames: 10,
            },
        )
        .expect_err("zero fuel policy");
        assert_eq!(policy.code, ErrorCode::PolicyExceeded);
        let policy = compile_and_run(
            snapshot,
            ids[&20],
            &[RuntimeValue::I64(1)],
            RunPolicy {
                fuel: 10,
                maximum_frames: 0,
            },
        )
        .expect_err("zero frame policy");
        assert_eq!(policy.code, ErrorCode::PolicyExceeded);
        let policy = compile_and_run(
            snapshot,
            ids[&20],
            &[RuntimeValue::I64(1)],
            RunPolicy {
                fuel: crate::interpret::MAX_RUN_FUEL + 1,
                maximum_frames: 1,
            },
        )
        .expect_err("fuel maximum");
        assert_eq!(policy.code, ErrorCode::PolicyExceeded);
        let policy = compile_and_run(
            snapshot,
            ids[&20],
            &[RuntimeValue::I64(1)],
            RunPolicy {
                fuel: 1,
                maximum_frames: crate::interpret::MAX_RUN_FRAMES + 1,
            },
        )
        .expect_err("frame maximum");
        assert_eq!(policy.code, ErrorCode::PolicyExceeded);
        let excessive = compile_and_run(
            snapshot,
            ids[&20],
            &vec![RuntimeValue::I64(1); crate::interpret::MAX_RUN_ARGUMENTS + 1],
            RunPolicy {
                fuel: 100,
                maximum_frames: 10,
            },
        )
        .expect_err("argument boundary");
        assert_eq!(excessive.code, ErrorCode::RunArgumentMismatch);
        let mismatch = compile_and_run(
            snapshot,
            ids[&20],
            &[],
            RunPolicy {
                fuel: 100,
                maximum_frames: 10,
            },
        )
        .expect_err("arity");
        assert_eq!(mismatch.code, ErrorCode::RunArgumentMismatch);
        let mismatch = compile_and_run(
            snapshot,
            ids[&20],
            &[RuntimeValue::Bool(true)],
            RunPolicy {
                fuel: 100,
                maximum_frames: 10,
            },
        )
        .expect_err("type");
        assert_eq!(mismatch.code, ErrorCode::RunArgumentMismatch);
        let loop_fuel = compile_and_run(
            snapshot,
            ids[&70],
            &[],
            RunPolicy {
                fuel: 5,
                maximum_frames: 10,
            },
        )
        .expect_err("loop fuel");
        assert_eq!(loop_fuel.code, ErrorCode::ExecutionFuelExhausted);
        assert_eq!(loop_fuel.target, Some(ids[&76]));
        let fuel = compile_and_run(
            snapshot,
            ids[&30],
            &[],
            RunPolicy {
                fuel: 1,
                maximum_frames: 10,
            },
        )
        .expect_err("fuel");
        assert_eq!(fuel.code, ErrorCode::ExecutionFuelExhausted);
        assert_eq!(fuel.target, Some(ids[&32]));
        let frames = compile_and_run(
            snapshot,
            ids[&90],
            &[RuntimeValue::Bool(true)],
            RunPolicy {
                fuel: 100,
                maximum_frames: 1,
            },
        )
        .expect_err("frames");
        assert_eq!(frames.code, ErrorCode::ExecutionFrameExhausted);
        assert_eq!(frames.target, Some(ids[&94]));
        assert_eq!(
            compile(snapshot, ids[&130]).expect_err("bodyless").code,
            ErrorCode::CompileIncomplete
        );
        assert_eq!(
            compile(snapshot, ids[&140]).expect_err("hole").code,
            ErrorCode::CompileIncomplete
        );
        let bodyless = compile(snapshot, ids[&230]).expect_err("reachable bodyless callee");
        assert_eq!(bodyless.code, ErrorCode::CompileIncomplete);
        assert_eq!(&*bodyless.related, &[ids[&130]]);
        assert_eq!(
            compile(snapshot, ids[&240])
                .expect_err("reachable hole callee")
                .code,
            ErrorCode::CompileIncomplete
        );
        let deep = compile_and_run(
            snapshot,
            ids[&220],
            &[],
            RunPolicy {
                fuel: 100_000,
                maximum_frames: 5_000,
            },
        )
        .expect_err("explicit frame bound");
        assert_eq!(deep.code, ErrorCode::ExecutionFrameExhausted);
        assert_eq!(deep.target, Some(ids[&221]));
    }
}

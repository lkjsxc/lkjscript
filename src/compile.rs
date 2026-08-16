use crate::core_ir::{
    self, BOOL_TYPE, BlockId, CoreBlock, CoreField, CoreFunction, CoreProgram, CoreType,
    CoreTypeId, CoreTypeKind, CoreVariant, FunctionId, I64_TYPE, Instruction, SwitchArgument,
    SwitchArm, Terminator, UNIT_TYPE, ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::query;
use crate::schema::{DirectReference, Node, OperationKind, SemanticType, ValueRef};
use crate::type_layout::{self, DerivedLayout, LayoutShape, ValueLayout};
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
    let nominal_types = reachable_nominal_types(snapshot, &reachable)?;
    let (types, type_ids) = build_type_table(snapshot, &nominal_types)?;
    let mut function_ids = BTreeMap::new();
    for function in &reachable {
        function_ids.insert(
            *function,
            FunctionId(dense_u32(function_ids.len(), *function, "function")?),
        );
    }
    let mut functions = Vec::with_capacity(reachable.len());
    for function in reachable {
        functions.push(lower_function(
            snapshot,
            function,
            &function_ids,
            &type_ids,
            &types,
        )?);
    }
    let program = CoreProgram {
        types,
        entry: *function_ids
            .get(&entry)
            .ok_or_else(|| invalid(entry, "entry function was not allocated"))?,
        functions,
    };
    core_ir::verify(&program)?;
    Ok(program)
}

fn reachable_nominal_types(snapshot: &Snapshot, reachable: &[NodeId]) -> Result<Vec<NodeId>> {
    let mut declarations = BTreeSet::new();
    let mut pending_nodes = reachable.to_vec();
    while let Some(origin) = pending_nodes.pop() {
        let node = snapshot.node(origin)?;
        for index in 0..node.direct_reference_count() {
            match node.direct_reference(index) {
                Some(DirectReference::Type { target, .. }) => {
                    declarations.insert(target);
                }
                Some(DirectReference::Definition { target }) => match snapshot.node(target)? {
                    Node::ProductType { .. } | Node::SumType { .. } => {
                        declarations.insert(target);
                    }
                    Node::ProductField { owner, .. } | Node::SumVariant { owner, .. } => {
                        declarations.insert(*owner);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if let Node::Operation { operation, .. } = node
            && !operation.is_terminator()
            && let SemanticType::Nominal(target) = semantic_result_type(snapshot, operation)?
        {
            declarations.insert(target);
        }
        for index in (0..node.owned_child_count()).rev() {
            if let Some(child) = node.owned_child(index) {
                pending_nodes.push(child);
            }
        }
    }
    let mut pending = declarations.iter().copied().collect::<Vec<_>>();
    while let Some(declaration) = pending.pop() {
        match snapshot.node(declaration)? {
            Node::ProductType { fields, .. } => {
                for field in fields {
                    let Node::ProductField { ty, .. } = snapshot.node(*field)? else {
                        return Err(invalid(*field, "product member is not a field"));
                    };
                    if let SemanticType::Nominal(target) = ty
                        && declarations.insert(*target)
                    {
                        pending.push(*target);
                    }
                }
            }
            Node::SumType { variants, .. } => {
                for variant in variants {
                    let Node::SumVariant { payload, .. } = snapshot.node(*variant)? else {
                        return Err(invalid(*variant, "sum member is not a variant"));
                    };
                    if let Some(SemanticType::Nominal(target)) = payload
                        && declarations.insert(*target)
                    {
                        pending.push(*target);
                    }
                }
            }
            _ => {
                return Err(invalid(
                    declaration,
                    "reachable nominal type is not a declaration",
                ));
            }
        }
    }
    Ok(declarations.into_iter().collect())
}

fn build_type_table(
    snapshot: &Snapshot,
    declarations: &[NodeId],
) -> Result<(Vec<CoreType>, BTreeMap<SemanticType, CoreTypeId>)> {
    let primitive = |kind, size, align, cells| CoreType {
        origin: None,
        kind,
        layout: ValueLayout {
            size,
            align,
            cells,
            shape: LayoutShape::Primitive,
        },
    };
    let mut types = vec![
        primitive(CoreTypeKind::Unit, 0, 1, 0),
        primitive(CoreTypeKind::Bool, 1, 1, 1),
        primitive(CoreTypeKind::I64, 8, 8, 1),
    ];
    let mut ids = BTreeMap::from([
        (SemanticType::Unit, UNIT_TYPE),
        (SemanticType::Bool, BOOL_TYPE),
        (SemanticType::I64, I64_TYPE),
    ]);
    for declaration in declarations {
        let id = CoreTypeId(dense_u32(types.len(), *declaration, "type")?);
        ids.insert(SemanticType::Nominal(*declaration), id);
        types.push(CoreType {
            origin: Some(*declaration),
            kind: CoreTypeKind::Unit,
            layout: ValueLayout {
                size: 0,
                align: 1,
                cells: 0,
                shape: LayoutShape::Primitive,
            },
        });
    }
    let layouts = type_layout::derive_layouts(snapshot)?;
    for declaration in declarations {
        let id = *ids
            .get(&SemanticType::Nominal(*declaration))
            .ok_or_else(|| invalid(*declaration, "reachable nominal Core type ID is absent"))?;
        let DerivedLayout::Representable(layout) = layouts
            .get(declaration)
            .cloned()
            .ok_or_else(|| invalid(*declaration, "reachable nominal layout is absent"))?
        else {
            return Err(LkError::new(
                ErrorCode::TypeLayoutUnrepresentable,
                "reachable nominal type layout is unrepresentable",
            )
            .for_node(*declaration));
        };
        let kind = match snapshot.node(*declaration)? {
            Node::ProductType { fields, .. } => {
                let mut cell_offset = 0_u64;
                let mut core_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let Node::ProductField { ty, .. } = snapshot.node(*field)? else {
                        return Err(invalid(*field, "product member is not a field"));
                    };
                    let field_ty = *ids.get(ty).ok_or_else(|| {
                        invalid(*field, "product field type is absent from Core closure")
                    })?;
                    core_fields.push(CoreField {
                        origin: *field,
                        ty: field_ty,
                        cell_offset,
                    });
                    let DerivedLayout::Representable(field_layout) =
                        type_layout::layout_of(snapshot, *ty, &layouts)?
                    else {
                        return Err(LkError::new(
                            ErrorCode::TypeLayoutUnrepresentable,
                            "reachable product field layout is unrepresentable",
                        )
                        .for_node(*field));
                    };
                    cell_offset = cell_offset
                        .checked_add(field_layout.cells)
                        .ok_or_else(|| invalid(*field, "product cell offset overflowed"))?;
                }
                CoreTypeKind::Product {
                    fields: core_fields,
                }
            }
            Node::SumType { variants, .. } => {
                let mut core_variants = Vec::with_capacity(variants.len());
                for (ordinal, variant) in variants.iter().enumerate() {
                    let Node::SumVariant { payload, .. } = snapshot.node(*variant)? else {
                        return Err(invalid(*variant, "sum member is not a variant"));
                    };
                    core_variants.push(CoreVariant {
                        origin: *variant,
                        payload: payload
                            .map(|ty| {
                                ids.get(&ty).copied().ok_or_else(|| {
                                    invalid(
                                        *variant,
                                        "sum payload type is absent from Core closure",
                                    )
                                })
                            })
                            .transpose()?,
                        discriminant: u64::try_from(ordinal)
                            .map_err(|_| invalid(*variant, "variant discriminant overflows"))?,
                    });
                }
                CoreTypeKind::Sum {
                    variants: core_variants,
                }
            }
            _ => {
                return Err(invalid(
                    *declaration,
                    "Core nominal type is not a declaration",
                ));
            }
        };
        let slot = types
            .get_mut(
                usize::try_from(id.0)
                    .map_err(|_| invalid(*declaration, "Core type index overflows host"))?,
            )
            .ok_or_else(|| invalid(*declaration, "Core type slot is absent"))?;
        *slot = CoreType {
            origin: Some(*declaration),
            kind,
            layout,
        };
    }
    Ok((types, ids))
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
    result: CoreTypeId,
    parameters: Vec<ValueId>,
    value_types: Vec<CoreTypeId>,
    blocks: Vec<BuildBlock>,
    entry: BlockId,
}
impl FunctionBuilder {
    fn value(&mut self, ty: CoreTypeId, origin: NodeId) -> Result<ValueId> {
        let id = ValueId(dense_u32(self.value_types.len(), origin, "value")?);
        self.value_types.push(ty);
        Ok(id)
    }
    fn block(
        &mut self,
        origin: NodeId,
        parameter_types: &[CoreTypeId],
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
    fn finish(self, types: &[CoreType]) -> Result<CoreFunction> {
        let mut frame_cells = 0_u64;
        for ty in &self.value_types {
            let cells = types
                .get(
                    usize::try_from(ty.0)
                        .map_err(|_| invalid(self.origin, "Core type index overflows host"))?,
                )
                .ok_or_else(|| invalid(self.origin, "Core value type is absent"))?
                .layout
                .cells;
            frame_cells = frame_cells
                .checked_add(cells)
                .ok_or_else(|| invalid(self.origin, "Core frame cell footprint overflowed"))?;
        }
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
            frame_cells,
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
    type_ids: &BTreeMap<SemanticType, CoreTypeId>,
    types: &[CoreType],
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
        result: core_type(type_ids, *result, function)?,
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
        parameter_types.push(core_type(type_ids, *ty, *parameter)?);
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
                    &[core_type(type_ids, *result, operation_id)?],
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
            OperationKind::MatchSum {
                scrutinee,
                result,
                arms,
            } => {
                let scrutinee = lower_value(&task.environment, *scrutinee)?;
                let captures: Vec<_> = task.environment.keys().copied().collect();
                let capture_arguments = capture_values(&task.environment, &captures)?;
                let (join, mut join_env, join_parameters) = captured_block(
                    &mut builder,
                    operation_id,
                    &task.environment,
                    &captures,
                    &[core_type(type_ids, *result, operation_id)?],
                )?;
                join_env.insert(
                    ValueRef::OperationResult {
                        operation: operation_id,
                        output: 0,
                    },
                    *join_parameters.last().ok_or_else(|| {
                        invalid(operation_id, "match join result parameter is missing")
                    })?,
                );
                let mut switch_arms = Vec::with_capacity(arms.len());
                let mut arm_tasks = Vec::with_capacity(arms.len());
                for arm in arms {
                    let Node::SumVariant {
                        ordinal, payload, ..
                    } = snapshot.node(arm.variant)?
                    else {
                        return Err(invalid(
                            arm.variant,
                            "match arm target is not a sum variant",
                        ));
                    };
                    let semantic_block = region_block(snapshot, arm.region)?;
                    let (operations, terminator, block_arguments) =
                        semantic_block_parts(snapshot, semantic_block)?;
                    let extra_types = payload
                        .map(|ty| core_type(type_ids, ty, arm.variant))
                        .transpose()?
                        .into_iter()
                        .collect::<Vec<_>>();
                    if block_arguments.len() != extra_types.len() {
                        return Err(invalid(
                            semantic_block,
                            "match payload block argument count is malformed",
                        ));
                    }
                    let (block, mut environment, _) = captured_block(
                        &mut builder,
                        semantic_block,
                        &task.environment,
                        &captures,
                        &extra_types,
                    )?;
                    if let Some(argument) = block_arguments.first() {
                        let parameter = builder.block_mut(block)?.parameters[captures.len()];
                        environment.insert(ValueRef::BlockArgument(*argument), parameter);
                    }
                    let mut edge_arguments = capture_arguments
                        .iter()
                        .copied()
                        .map(SwitchArgument::Value)
                        .collect::<Vec<_>>();
                    if payload.is_some() {
                        edge_arguments.push(SwitchArgument::Payload);
                    }
                    switch_arms.push(SwitchArm {
                        variant: *ordinal,
                        target: block,
                        arguments: edge_arguments,
                    });
                    arm_tasks.push(Task {
                        operations,
                        index: 0,
                        terminator,
                        block,
                        environment,
                        end: EndAction::YieldBranch {
                            target: join,
                            captures: captures.clone(),
                        },
                    });
                }
                builder.terminate(
                    task.block,
                    Terminator::SwitchVariant {
                        origin: operation_id,
                        scrutinee,
                        arms: switch_arms,
                    },
                )?;
                task.index += 1;
                task.block = join;
                task.environment = join_env;
                tasks.push(task);
                for arm_task in arm_tasks.into_iter().rev() {
                    tasks.push(arm_task);
                }
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
                header_types.extend([I64_TYPE, core_type(type_ids, *carried, operation_id)?]);
                let (header, header_parameters) = builder.block(operation_id, &header_types)?;
                let header_env = environment_from_parameters(&captures, &header_parameters)?;
                let header_index = header_parameters[captures.len()];
                let header_carried = header_parameters[captures.len() + 1];
                let mut body_types = capture_types.clone();
                body_types.extend([I64_TYPE, core_type(type_ids, *carried, operation_id)?]);
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
                exit_types.push(core_type(type_ids, *carried, operation_id)?);
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
                let condition = builder.value(BOOL_TYPE, operation_id)?;
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
                let semantic_type = semantic_result_type(snapshot, operation)?;
                let result_type = core_type(type_ids, semantic_type, operation_id)?;
                let result = builder.value(result_type, operation_id)?;
                let instruction = lower_instruction(
                    snapshot,
                    operation_id,
                    operation,
                    result,
                    &task.environment,
                    function_ids,
                    type_ids,
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
    builder.finish(types)
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
            let step_value = builder.value(I64_TYPE, origin)?;
            builder.instruction(
                task.block,
                Instruction::ConstI64 {
                    origin,
                    result: step_value,
                    value: step,
                },
            )?;
            let next_index = builder.value(I64_TYPE, origin)?;
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
    snapshot: &Snapshot,
    origin: NodeId,
    operation: &OperationKind,
    result: ValueId,
    environment: &BTreeMap<ValueRef, ValueId>,
    function_ids: &BTreeMap<NodeId, FunctionId>,
    type_ids: &BTreeMap<SemanticType, CoreTypeId>,
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
        OperationKind::ConstructProduct { product, fields } => Instruction::ConstructProduct {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*product), origin)?,
            fields: fields
                .iter()
                .map(|field| lower_value(environment, field.value))
                .collect::<Result<_>>()?,
        },
        OperationKind::ProjectField { value, field } => {
            let Node::ProductField { ordinal, .. } = snapshot.node(*field)? else {
                return Err(invalid(*field, "projection target is not a product field"));
            };
            Instruction::ProjectField {
                origin,
                result,
                value: lower_value(environment, *value)?,
                field: *ordinal,
            }
        }
        OperationKind::ConstructVariant { variant, payload } => {
            let Node::SumVariant { owner, ordinal, .. } = snapshot.node(*variant)? else {
                return Err(invalid(*variant, "variant target is not a sum variant"));
            };
            Instruction::ConstructVariant {
                origin,
                result,
                sum: core_type(type_ids, SemanticType::Nominal(*owner), origin)?,
                variant: *ordinal,
                payload: payload
                    .map(|value| lower_value(environment, value))
                    .transpose()?,
            }
        }
        OperationKind::Hole { .. }
        | OperationKind::If { .. }
        | OperationKind::ForI64 { .. }
        | OperationKind::MatchSum { .. }
        | OperationKind::Return { .. }
        | OperationKind::Yield { .. } => {
            return Err(invalid(
                origin,
                "structured operation entered scalar instruction lowering",
            ));
        }
    })
}

fn core_type(
    type_ids: &BTreeMap<SemanticType, CoreTypeId>,
    ty: SemanticType,
    origin: NodeId,
) -> Result<CoreTypeId> {
    type_ids.get(&ty).copied().ok_or_else(|| {
        invalid(
            origin,
            "semantic type is absent from exact Core type closure",
        )
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
        OperationKind::ConstructProduct { product, .. } => SemanticType::Nominal(*product),
        OperationKind::ProjectField { field, .. } => match snapshot.node(*field)? {
            Node::ProductField { ty, .. } => *ty,
            _ => return Err(invalid(*field, "projection target is not a product field")),
        },
        OperationKind::ConstructVariant { variant, .. } => match snapshot.node(*variant)? {
            Node::SumVariant { owner, .. } => SemanticType::Nominal(*owner),
            _ => return Err(invalid(*variant, "variant target is not a sum variant")),
        },
        OperationKind::MatchSum { result, .. } => *result,
        OperationKind::Return { .. } | OperationKind::Yield { .. } => {
            return Err(invalid(
                operation
                    .definition_target(0)
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
    extra_types: &[CoreTypeId],
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
) -> Result<Vec<CoreTypeId>> {
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
    use crate::schema::ProductFieldValueDraft;
    use crate::transaction::{
        ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
        FunctionParameterDraft, MatchArmDraft, NodeTarget, ProductFieldDraft, SumVariantDraft,
        Transaction, TransactionMode, TransactionOp, TransactionResponseSpec, YieldingBodyDraft,
    };
    use crate::{DraftSymbol, Revision, TypeDraft, ValueDraft, WorkspaceId};

    fn local(value: u32) -> NodeTarget {
        NodeTarget::Draft(DraftSymbol::generated(value))
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
    fn expression(symbol: u32, operation: ExpressionKindDraft) -> ExpressionDraft {
        ExpressionDraft {
            symbol: Some(DraftSymbol::generated(symbol)),
            operation,
        }
    }
    fn function(
        symbol: u32,
        module: u32,
        name: &str,
        parameters: Vec<(u32, &str, SemanticType)>,
        result_type: SemanticType,
        body: Option<FunctionBodyDraft>,
    ) -> TransactionOp {
        TransactionOp::CreateFunction {
            symbol: DraftSymbol::generated(symbol),
            module: local(module),
            name: name.into(),
            parameters: parameters
                .into_iter()
                .map(|(symbol, name, ty)| FunctionParameterDraft {
                    symbol: DraftSymbol::generated(symbol),
                    name: name.into(),
                    ty: ty.into(),
                })
                .collect(),
            result: result_type.into(),
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
    fn complete_reachable_nominal_signature_lowers_and_runs_with_exact_public_value() {
        let workspace_id = WorkspaceId::from_bytes([0x95; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: workspace_id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(1),
                        name: "app".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: local(1),
                        name: "root".into(),
                    },
                    TransactionOp::CreateProductType {
                        symbol: DraftSymbol::generated(3),
                        module: local(2),
                        name: "Reading".into(),
                        fields: vec![
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(4),
                                name: "value".into(),
                                ty: TypeDraft::I64,
                            },
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(8),
                                name: "valid".into(),
                                ty: TypeDraft::Bool,
                            },
                        ],
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(5),
                        module: local(2),
                        name: "identity".into(),
                        parameters: vec![FunctionParameterDraft {
                            symbol: DraftSymbol::generated(6),
                            name: "value".into(),
                            ty: TypeDraft::Nominal(local(3)),
                        }],
                        result: TypeDraft::Nominal(local(3)),
                        body: body(Vec::new(), parameter(6)),
                    },
                    TransactionOp::SetEntryFunction {
                        package: local(1),
                        function: local(5),
                    },
                ],
            },
            response: TransactionResponseSpec {
                return_symbols: vec![
                    DraftSymbol::generated(3),
                    DraftSymbol::generated(4),
                    DraftSymbol::generated(5),
                    DraftSymbol::generated(8),
                ],
            },
        };
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("complete nominal function");
        let id = |symbol: u32| {
            prepared
                .receipt
                .returned_bindings
                .iter()
                .find_map(|(candidate, node)| {
                    (candidate.generated_number() == symbol).then_some(*node)
                })
                .expect("binding")
        };
        let product = id(3);
        let field = id(4);
        let valid = id(8);
        let entry = id(5);
        let program = compile(&prepared.snapshot, entry).expect("nominal Core");
        assert_eq!(program.types.len(), 4);
        assert_eq!(program.types[3].origin, Some(product));
        let value = RuntimeValue::Product {
            ty: product,
            fields: vec![
                crate::RuntimeFieldValue {
                    field: valid,
                    value: RuntimeValue::Bool(true),
                },
                crate::RuntimeFieldValue {
                    field,
                    value: RuntimeValue::I64(41),
                },
            ],
        };
        let canonical = RuntimeValue::Product {
            ty: product,
            fields: vec![
                crate::RuntimeFieldValue {
                    field,
                    value: RuntimeValue::I64(41),
                },
                crate::RuntimeFieldValue {
                    field: valid,
                    value: RuntimeValue::Bool(true),
                },
            ],
        };
        assert_eq!(
            run(&prepared.snapshot, entry, &[value]).expect("nominal identity"),
            canonical
        );
        let duplicate = RuntimeValue::Product {
            ty: product,
            fields: vec![
                crate::RuntimeFieldValue {
                    field,
                    value: RuntimeValue::I64(1),
                },
                crate::RuntimeFieldValue {
                    field,
                    value: RuntimeValue::I64(2),
                },
            ],
        };
        assert_eq!(
            compile_and_run(
                &prepared.snapshot,
                entry,
                &[duplicate],
                RunPolicy {
                    fuel: 100,
                    maximum_frames: 10
                }
            )
            .expect_err("duplicate field")
            .code,
            ErrorCode::RunArgumentMismatch
        );
    }

    #[test]
    fn scalar_signature_construct_and_project_lower_directly() {
        let workspace_id = WorkspaceId::from_bytes([0x96; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: workspace_id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            symbol: DraftSymbol::generated(1),
                            name: "app".into(),
                        },
                        TransactionOp::CreateModule {
                            symbol: DraftSymbol::generated(2),
                            package: local(1),
                            name: "root".into(),
                        },
                        TransactionOp::CreateProductType {
                            symbol: DraftSymbol::generated(3),
                            module: local(2),
                            name: "Reading".into(),
                            fields: vec![ProductFieldDraft {
                                symbol: DraftSymbol::generated(4),
                                name: "value".into(),
                                ty: TypeDraft::I64,
                            }],
                        },
                        TransactionOp::CreateFunction {
                            symbol: DraftSymbol::generated(5),
                            module: local(2),
                            name: "main".into(),
                            parameters: Vec::new(),
                            result: TypeDraft::I64,
                            body: body(
                                vec![
                                    expression(6, ExpressionKindDraft::ConstI64(7)),
                                    expression(
                                        7,
                                        ExpressionKindDraft::ConstructProduct {
                                            product: local(3),
                                            fields: vec![ProductFieldValueDraft {
                                                field: local(4),
                                                value: result(6),
                                            }],
                                        },
                                    ),
                                    expression(
                                        8,
                                        ExpressionKindDraft::ProjectField {
                                            value: result(7),
                                            field: local(4),
                                        },
                                    ),
                                ],
                                result(8),
                            ),
                        },
                        TransactionOp::SetEntryFunction {
                            package: local(1),
                            function: local(5),
                        },
                    ],
                },
                response: TransactionResponseSpec {
                    return_symbols: vec![
                        DraftSymbol::generated(3),
                        DraftSymbol::generated(5),
                        DraftSymbol::generated(7),
                    ],
                },
            })
            .expect("complete scalar-signature nominal body");
        let binding = |symbol: u32| {
            prepared
                .receipt
                .returned_bindings
                .iter()
                .find_map(|(candidate, node)| {
                    (candidate.generated_number() == symbol).then_some(*node)
                })
                .expect("selected binding")
        };
        let program = compile(&prepared.snapshot, binding(5)).expect("aggregate Core lowering");
        assert!(matches!(
            program.functions[0].blocks[0].instructions[1],
            Instruction::ConstructProduct { .. }
        ));
        assert!(matches!(
            program.functions[0].blocks[0].instructions[2],
            Instruction::ProjectField { .. }
        ));
        assert_eq!(
            run(&prepared.snapshot, binding(5), &[]).expect("aggregate execution"),
            RuntimeValue::I64(7)
        );
    }

    #[test]
    fn exhaustive_sum_match_executes_only_selected_arm_with_exact_payload_binding() {
        let workspace_id = WorkspaceId::from_bytes([0x97; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: workspace_id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            symbol: DraftSymbol::generated(1),
                            name: "app".into(),
                        },
                        TransactionOp::CreateModule {
                            symbol: DraftSymbol::generated(2),
                            package: local(1),
                            name: "root".into(),
                        },
                        TransactionOp::CreateSumType {
                            symbol: DraftSymbol::generated(3),
                            module: local(2),
                            name: "Maybe".into(),
                            variants: vec![
                                SumVariantDraft {
                                    symbol: DraftSymbol::generated(4),
                                    name: "none".into(),
                                    payload: None,
                                },
                                SumVariantDraft {
                                    symbol: DraftSymbol::generated(5),
                                    name: "some".into(),
                                    payload: Some(TypeDraft::I64),
                                },
                            ],
                        },
                        TransactionOp::CreateProductType {
                            symbol: DraftSymbol::generated(11),
                            module: local(2),
                            name: "Unreachable".into(),
                            fields: vec![ProductFieldDraft {
                                symbol: DraftSymbol::generated(12),
                                name: "value".into(),
                                ty: TypeDraft::Bool,
                            }],
                        },
                        TransactionOp::CreateFunction {
                            symbol: DraftSymbol::generated(6),
                            module: local(2),
                            name: "unwrap_or_zero".into(),
                            parameters: vec![FunctionParameterDraft {
                                symbol: DraftSymbol::generated(7),
                                name: "value".into(),
                                ty: TypeDraft::Nominal(local(3)),
                            }],
                            result: TypeDraft::I64,
                            body: body(
                                vec![expression(
                                    8,
                                    ExpressionKindDraft::MatchSum {
                                        scrutinee: parameter(7),
                                        result: TypeDraft::I64,
                                        arms: vec![
                                            MatchArmDraft {
                                                variant: local(5),
                                                payload_symbol: Some(DraftSymbol::generated(9)),
                                                body: yielding(vec![], argument(9)),
                                            },
                                            MatchArmDraft {
                                                variant: local(4),
                                                payload_symbol: None,
                                                body: yielding(
                                                    vec![expression(
                                                        10,
                                                        ExpressionKindDraft::ConstI64(0),
                                                    )],
                                                    result(10),
                                                ),
                                            },
                                        ],
                                    },
                                )],
                                result(8),
                            ),
                        },
                        TransactionOp::SetEntryFunction {
                            package: local(1),
                            function: local(6),
                        },
                    ],
                },
                response: TransactionResponseSpec {
                    return_symbols: vec![
                        DraftSymbol::generated(3),
                        DraftSymbol::generated(4),
                        DraftSymbol::generated(5),
                        DraftSymbol::generated(6),
                    ],
                },
            })
            .expect("nominal match transaction");
        let id = |symbol: u32| {
            prepared
                .receipt
                .returned_bindings
                .iter()
                .find_map(|(candidate, node)| {
                    (candidate.generated_number() == symbol).then_some(*node)
                })
                .expect("binding")
        };
        let program = compile(&prepared.snapshot, id(6)).expect("match Core");
        assert_eq!(
            program.types.len(),
            4,
            "unreachable nominal declaration omitted"
        );
        assert_eq!(program.types[3].origin, Some(id(3)));
        assert!(matches!(
            program.functions[0].blocks[0].terminator,
            Terminator::SwitchVariant { .. }
        ));
        let none = RuntimeValue::Sum {
            ty: id(3),
            variant: id(4),
            payload: None,
        };
        let some = RuntimeValue::Sum {
            ty: id(3),
            variant: id(5),
            payload: Some(Box::new(RuntimeValue::I64(37))),
        };
        assert_eq!(
            run(&prepared.snapshot, id(6), &[none]).expect("none arm"),
            RuntimeValue::I64(0)
        );
        assert_eq!(
            run(&prepared.snapshot, id(6), &[some]).expect("payload arm"),
            RuntimeValue::I64(37)
        );
        let missing_payload = RuntimeValue::Sum {
            ty: id(3),
            variant: id(5),
            payload: None,
        };
        assert_eq!(
            compile_and_run(
                &prepared.snapshot,
                id(6),
                &[missing_payload],
                RunPolicy {
                    fuel: 100,
                    maximum_frames: 10
                }
            )
            .expect_err("missing payload")
            .code,
            ErrorCode::RunArgumentMismatch
        );
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
                        expected: SemanticType::I64.into(),
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
                            symbol: DraftSymbol::generated(1),
                            name: "app".into(),
                        },
                        TransactionOp::CreateModule {
                            symbol: DraftSymbol::generated(2),
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
                    return_symbols: vec![DraftSymbol::generated(3)],
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
                symbol: DraftSymbol::generated(1),
                name: "app".into(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
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
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(15),
                            carried_symbol: DraftSymbol::generated(16),
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
                            result: SemanticType::I64.into(),
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
                        result: SemanticType::I64.into(),
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
                        result: SemanticType::I64.into(),
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
                        result: SemanticType::Unit.into(),
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
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(74),
                            carried_symbol: DraftSymbol::generated(75),
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
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(85),
                            carried_symbol: DraftSymbol::generated(86),
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
                        result: SemanticType::I64.into(),
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
                        result: SemanticType::I64.into(),
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
                        result: SemanticType::I64.into(),
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
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(125),
                            carried_symbol: DraftSymbol::generated(126),
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
                        expected: SemanticType::I64.into(),
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
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(155),
                            carried_symbol: DraftSymbol::generated(156),
                            body: yielding(
                                vec![expression(
                                    157,
                                    ExpressionKindDraft::ForI64 {
                                        start: result(151),
                                        end_exclusive: result(153),
                                        step: 1,
                                        initial: argument(156),
                                        carried: SemanticType::I64.into(),
                                        index_symbol: DraftSymbol::generated(158),
                                        carried_symbol: DraftSymbol::generated(159),
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
                            result: SemanticType::I64.into(),
                            then_body: yielding(
                                vec![expression(
                                    175,
                                    ExpressionKindDraft::ForI64 {
                                        start: result(172),
                                        end_exclusive: result(173),
                                        step: 1,
                                        initial: result(172),
                                        carried: SemanticType::I64.into(),
                                        index_symbol: DraftSymbol::generated(176),
                                        carried_symbol: DraftSymbol::generated(177),
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
                            carried: SemanticType::I64.into(),
                            index_symbol: DraftSymbol::generated(265),
                            carried_symbol: DraftSymbol::generated(266),
                            body: yielding(
                                vec![expression(
                                    267,
                                    ExpressionKindDraft::If {
                                        condition: result(263),
                                        result: SemanticType::I64.into(),
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
                        result: SemanticType::Bool.into(),
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
        .map(DraftSymbol::generated)
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
                    return_symbols: handles,
                },
            })
            .expect("structured program");
        let ids = prepared
            .receipt
            .returned_bindings
            .iter()
            .map(|(symbol, node)| (symbol.generated_number(), *node))
            .collect();
        (prepared, ids)
    }

    #[test]
    fn deeply_nested_match_lowering_uses_one_verified_core_route() {
        const DEPTH: u32 = 7;
        fn nested_match(depth: u32) -> ExpressionDraft {
            let match_handle = DraftSymbol::generated(100 + depth * 3);
            let payload_symbol = DraftSymbol::generated(101 + depth * 3);
            let selected = if depth == 1 {
                let constant = DraftSymbol::generated(102 + depth * 3);
                yielding(
                    vec![expression(
                        constant.generated_number(),
                        ExpressionKindDraft::ConstI64(7),
                    )],
                    result(constant.generated_number()),
                )
            } else {
                yielding(vec![nested_match(depth - 1)], result(100 + (depth - 1) * 3))
            };
            expression(
                match_handle.generated_number(),
                ExpressionKindDraft::MatchSum {
                    scrutinee: parameter(7),
                    result: TypeDraft::I64,
                    arms: vec![
                        MatchArmDraft {
                            variant: local(5),
                            payload_symbol: Some(payload_symbol),
                            body: selected,
                        },
                        MatchArmDraft {
                            variant: local(4),
                            payload_symbol: None,
                            body: yielding(
                                vec![expression(10_000 + depth, ExpressionKindDraft::ConstI64(0))],
                                result(10_000 + depth),
                            ),
                        },
                    ],
                },
            )
        }

        let workspace_id = WorkspaceId::from_bytes([0x98; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: workspace_id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            symbol: DraftSymbol::generated(1),
                            name: "app".into(),
                        },
                        TransactionOp::CreateModule {
                            symbol: DraftSymbol::generated(2),
                            package: local(1),
                            name: "root".into(),
                        },
                        TransactionOp::CreateSumType {
                            symbol: DraftSymbol::generated(3),
                            module: local(2),
                            name: "Maybe".into(),
                            variants: vec![
                                SumVariantDraft {
                                    symbol: DraftSymbol::generated(4),
                                    name: "none".into(),
                                    payload: None,
                                },
                                SumVariantDraft {
                                    symbol: DraftSymbol::generated(5),
                                    name: "some".into(),
                                    payload: Some(TypeDraft::I64),
                                },
                            ],
                        },
                        TransactionOp::CreateFunction {
                            symbol: DraftSymbol::generated(6),
                            module: local(2),
                            name: "deep".into(),
                            parameters: vec![FunctionParameterDraft {
                                symbol: DraftSymbol::generated(7),
                                name: "value".into(),
                                ty: TypeDraft::Nominal(local(3)),
                            }],
                            result: TypeDraft::I64,
                            body: body(vec![nested_match(DEPTH)], result(100 + DEPTH * 3)),
                        },
                    ],
                },
                response: TransactionResponseSpec {
                    return_symbols: vec![
                        DraftSymbol::generated(3),
                        DraftSymbol::generated(5),
                        DraftSymbol::generated(6),
                    ],
                },
            })
            .expect("deep nested match transaction");
        let id = |symbol: u32| {
            prepared
                .receipt
                .returned_bindings
                .iter()
                .find_map(|(candidate, node)| {
                    (candidate.generated_number() == symbol).then_some(*node)
                })
                .expect("binding")
        };
        let program = compile(&prepared.snapshot, id(6)).expect("deep match lowering");
        core_ir::verify(&program).expect("deep match Core verification");
        assert_eq!(
            program
                .functions
                .iter()
                .flat_map(|function| &function.blocks)
                .filter(|block| matches!(block.terminator, Terminator::SwitchVariant { .. }))
                .count(),
            usize::try_from(DEPTH).expect("depth")
        );
        let some = RuntimeValue::Sum {
            ty: id(3),
            variant: id(5),
            payload: Some(Box::new(RuntimeValue::I64(1))),
        };
        assert_eq!(
            run(&prepared.snapshot, id(6), &[some]).expect("deep selected match path"),
            RuntimeValue::I64(7)
        );
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
        assert_eq!(loop_fuel.target.expect("fuel origin").serial(), 89);
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

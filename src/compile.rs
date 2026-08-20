use crate::core_ir::{
    self, BOOL_TYPE, BlockId, CoreBlock, CoreField, CoreFunction, CoreProgram, CoreType,
    CoreTypeId, CoreTypeKind, CoreVariant, FunctionId, I64_TYPE, Instruction, SwitchArgument,
    SwitchArm, Terminator, ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::query;
use crate::schema::{DirectReference, Node, OperationKind, SemanticType, ValueRef};
use crate::type_layout::{self, DerivedLayout, LayoutShape, ValueLayout};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

pub(crate) fn compile(snapshot: &Snapshot, entry: NodeId) -> Result<CoreProgram> {
    compile_observed(snapshot, entry).map(|(program, _)| program)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CompileObservation {
    pub lowering_nanoseconds: u64,
    pub core_verification_nanoseconds: u64,
}

pub(crate) fn compile_observed(
    snapshot: &Snapshot,
    entry: NodeId,
) -> Result<(CoreProgram, CompileObservation)> {
    let lowering_started = Instant::now();
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
    let lowering_nanoseconds = elapsed_nanoseconds(lowering_started);
    let verification_started = Instant::now();
    core_ir::verify(&program)?;
    let core_verification_nanoseconds = elapsed_nanoseconds(verification_started);
    Ok((
        program,
        CompileObservation {
            lowering_nanoseconds,
            core_verification_nanoseconds,
        },
    ))
}

fn elapsed_nanoseconds(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
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
                    Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. } => {
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
            Node::SequenceType { element, .. } => {
                if let SemanticType::Nominal(target) = element
                    && declarations.insert(*target)
                {
                    pending.push(*target);
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
    let mut types = Vec::with_capacity(SemanticType::PRIMITIVES.len() + declarations.len());
    let mut ids = BTreeMap::new();
    for semantic in SemanticType::PRIMITIVES {
        let id = CoreTypeId(dense_u32(types.len(), snapshot.root(), "primitive type")?);
        let kind = match semantic {
            SemanticType::Unit => CoreTypeKind::Unit,
            SemanticType::Bool => CoreTypeKind::Bool,
            SemanticType::I64 => CoreTypeKind::I64,
            SemanticType::Bytes => CoreTypeKind::Bytes,
            SemanticType::Text => CoreTypeKind::Text,
            SemanticType::Nominal(_) => {
                return Err(invalid(snapshot.root(), "primitive type is nominal"));
            }
        };
        let layout = type_layout::primitive_layout(semantic)
            .ok_or_else(|| invalid(snapshot.root(), "primitive layout is absent"))?;
        ids.insert(semantic, id);
        types.push(CoreType {
            origin: None,
            kind,
            layout,
        });
    }
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
            Node::SequenceType { element, .. } => CoreTypeKind::Sequence {
                element: *ids.get(element).ok_or_else(|| {
                    invalid(
                        *declaration,
                        "sequence element type is absent from Core closure",
                    )
                })?,
            },
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
        OperationKind::ConstBytes(value) => Instruction::ConstBytes {
            origin,
            result,
            value: value.clone(),
        },
        OperationKind::ConstText(value) => Instruction::ConstText {
            origin,
            result,
            value: value.clone(),
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
        OperationKind::EqualI64 { lhs, rhs } => Instruction::EqualI64 {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::NotBool { value } => Instruction::NotBool {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::AndBool { lhs, rhs } => Instruction::AndBool {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::OrBool { lhs, rhs } => Instruction::OrBool {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::BytesLen { value } => Instruction::BytesLen {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::BytesAt { value, index } => Instruction::BytesAt {
            origin,
            result,
            value: lower_value(environment, *value)?,
            index: lower_value(environment, *index)?,
        },
        OperationKind::BytesSlice {
            value,
            start,
            length,
        } => Instruction::BytesSlice {
            origin,
            result,
            value: lower_value(environment, *value)?,
            start: lower_value(environment, *start)?,
            length: lower_value(environment, *length)?,
        },
        OperationKind::BytesEqual { lhs, rhs } => Instruction::BytesEqual {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::BytesConcat { lhs, rhs } => Instruction::BytesConcat {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::TextLen { value } => Instruction::TextLen {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextEqual { lhs, rhs } => Instruction::TextEqual {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::TextConcat { lhs, rhs } => Instruction::TextConcat {
            origin,
            result,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::TextScalarLen { value } => Instruction::TextScalarLen {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextGraphemeLen { value } => Instruction::TextGraphemeLen {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextLineCount { value } => Instruction::TextLineCount {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextScalarAt { value, index } => Instruction::TextScalarAt {
            origin,
            result,
            value: lower_value(environment, *value)?,
            index: lower_value(environment, *index)?,
        },
        OperationKind::TextPreviousGraphemeBoundary { value, index } => {
            Instruction::TextPreviousGraphemeBoundary {
                origin,
                result,
                value: lower_value(environment, *value)?,
                index: lower_value(environment, *index)?,
            }
        }
        OperationKind::TextNextGraphemeBoundary { value, index } => {
            Instruction::TextNextGraphemeBoundary {
                origin,
                result,
                value: lower_value(environment, *value)?,
                index: lower_value(environment, *index)?,
            }
        }
        OperationKind::TextLineStart { value, line } => Instruction::TextLineStart {
            origin,
            result,
            value: lower_value(environment, *value)?,
            line: lower_value(environment, *line)?,
        },
        OperationKind::TextLineEnd { value, line } => Instruction::TextLineEnd {
            origin,
            result,
            value: lower_value(environment, *value)?,
            line: lower_value(environment, *line)?,
        },
        OperationKind::TextByteToLine { value, index } => Instruction::TextByteToLine {
            origin,
            result,
            value: lower_value(environment, *value)?,
            index: lower_value(environment, *index)?,
        },
        OperationKind::TextSlice {
            value,
            start,
            end_exclusive,
        } => Instruction::TextSlice {
            origin,
            result,
            value: lower_value(environment, *value)?,
            start: lower_value(environment, *start)?,
            end_exclusive: lower_value(environment, *end_exclusive)?,
        },
        OperationKind::TextSplice {
            value,
            start,
            end_exclusive,
            replacement,
        } => Instruction::TextSplice {
            origin,
            result,
            value: lower_value(environment, *value)?,
            start: lower_value(environment, *start)?,
            end_exclusive: lower_value(environment, *end_exclusive)?,
            replacement: lower_value(environment, *replacement)?,
        },
        OperationKind::TextFindForward {
            value,
            query,
            start,
        } => Instruction::TextFindForward {
            origin,
            result,
            value: lower_value(environment, *value)?,
            query: lower_value(environment, *query)?,
            start: lower_value(environment, *start)?,
        },
        OperationKind::TextFindBackward {
            value,
            query,
            end_exclusive,
        } => Instruction::TextFindBackward {
            origin,
            result,
            value: lower_value(environment, *value)?,
            query: lower_value(environment, *query)?,
            end_exclusive: lower_value(environment, *end_exclusive)?,
        },
        OperationKind::TextLineEndingKind { value } => Instruction::TextLineEndingKind {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextDisplayWidth {
            value,
            start,
            end_exclusive,
            initial_column,
            tab_width,
        } => Instruction::TextDisplayWidth {
            origin,
            result,
            value: lower_value(environment, *value)?,
            start: lower_value(environment, *start)?,
            end_exclusive: lower_value(environment, *end_exclusive)?,
            initial_column: lower_value(environment, *initial_column)?,
            tab_width: lower_value(environment, *tab_width)?,
        },
        OperationKind::TextCellPrefixBoundary {
            value,
            start,
            end_exclusive,
            initial_column,
            maximum_cells,
            tab_width,
        } => Instruction::TextCellPrefixBoundary {
            origin,
            result,
            value: lower_value(environment, *value)?,
            start: lower_value(environment, *start)?,
            end_exclusive: lower_value(environment, *end_exclusive)?,
            initial_column: lower_value(environment, *initial_column)?,
            maximum_cells: lower_value(environment, *maximum_cells)?,
            tab_width: lower_value(environment, *tab_width)?,
        },
        OperationKind::TextFromScalar { value } => Instruction::TextFromScalar {
            origin,
            result,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextToScalars { sequence, value } => Instruction::TextToScalars {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
        },
        OperationKind::TextFromScalars { sequence, value } => Instruction::TextFromScalars {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
        },
        OperationKind::SequenceEmpty { sequence } => Instruction::SequenceEmpty {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
        },
        OperationKind::SequenceLen { sequence, value } => Instruction::SequenceLen {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
        },
        OperationKind::SequenceGet {
            sequence,
            value,
            index,
        } => Instruction::SequenceGet {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
            index: lower_value(environment, *index)?,
        },
        OperationKind::SequenceAppend {
            sequence,
            value,
            element,
        } => Instruction::SequenceAppend {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
            element: lower_value(environment, *element)?,
        },
        OperationKind::SequenceReplace {
            sequence,
            value,
            index,
            element,
        } => Instruction::SequenceReplace {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
            index: lower_value(environment, *index)?,
            element: lower_value(environment, *element)?,
        },
        OperationKind::SequenceSlice {
            sequence,
            value,
            start,
            end_exclusive,
        } => Instruction::SequenceSlice {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            value: lower_value(environment, *value)?,
            start: lower_value(environment, *start)?,
            end_exclusive: lower_value(environment, *end_exclusive)?,
        },
        OperationKind::SequenceConcat { sequence, lhs, rhs } => Instruction::SequenceConcat {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            lhs: lower_value(environment, *lhs)?,
            rhs: lower_value(environment, *rhs)?,
        },
        OperationKind::SequenceRepeat {
            sequence,
            element,
            count,
        } => Instruction::SequenceRepeat {
            origin,
            result,
            ty: core_type(type_ids, SemanticType::Nominal(*sequence), origin)?,
            element: lower_value(environment, *element)?,
            count: lower_value(environment, *count)?,
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
        OperationKind::ConstBool(_)
        | OperationKind::LtI64 { .. }
        | OperationKind::EqualI64 { .. }
        | OperationKind::NotBool { .. }
        | OperationKind::AndBool { .. }
        | OperationKind::OrBool { .. }
        | OperationKind::BytesEqual { .. }
        | OperationKind::TextEqual { .. } => SemanticType::Bool,
        OperationKind::ConstI64(_)
        | OperationKind::AddI64 { .. }
        | OperationKind::BytesLen { .. }
        | OperationKind::BytesAt { .. }
        | OperationKind::TextLen { .. }
        | OperationKind::TextScalarLen { .. }
        | OperationKind::TextGraphemeLen { .. }
        | OperationKind::TextLineCount { .. }
        | OperationKind::TextScalarAt { .. }
        | OperationKind::TextPreviousGraphemeBoundary { .. }
        | OperationKind::TextNextGraphemeBoundary { .. }
        | OperationKind::TextLineStart { .. }
        | OperationKind::TextLineEnd { .. }
        | OperationKind::TextByteToLine { .. }
        | OperationKind::TextFindForward { .. }
        | OperationKind::TextFindBackward { .. }
        | OperationKind::TextLineEndingKind { .. }
        | OperationKind::TextDisplayWidth { .. }
        | OperationKind::TextCellPrefixBoundary { .. }
        | OperationKind::SequenceLen { .. } => SemanticType::I64,
        OperationKind::ConstBytes(_)
        | OperationKind::BytesSlice { .. }
        | OperationKind::BytesConcat { .. } => SemanticType::Bytes,
        OperationKind::ConstText(_)
        | OperationKind::TextConcat { .. }
        | OperationKind::TextSlice { .. }
        | OperationKind::TextSplice { .. }
        | OperationKind::TextFromScalar { .. }
        | OperationKind::TextFromScalars { .. } => SemanticType::Text,
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
        OperationKind::SequenceEmpty { sequence }
        | OperationKind::SequenceAppend { sequence, .. }
        | OperationKind::SequenceReplace { sequence, .. }
        | OperationKind::SequenceSlice { sequence, .. }
        | OperationKind::SequenceConcat { sequence, .. }
        | OperationKind::SequenceRepeat { sequence, .. }
        | OperationKind::TextToScalars { sequence, .. } => SemanticType::Nominal(*sequence),
        OperationKind::SequenceGet { sequence, .. } => match snapshot.node(*sequence)? {
            Node::SequenceType { element, .. } => *element,
            _ => return Err(invalid(*sequence, "sequence target is not a sequence type")),
        },
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
mod tests;

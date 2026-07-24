//! Lower resolved typed HIR into backend-independent typed SSA.

use std::collections::{BTreeMap, HashMap};

use lkjscript_core::{Error, Result};
use lkjscript_ir::{
    verify, BindingId as SsaBindingId, Block, BlockId, BlockMetadata, BlockParameter, CallTarget,
    Constant, EffectSet, FailureBehavior, FrameLocal, FrameState, Function, FunctionId,
    Instruction, InstructionKind, InstructionMetadata, Origin, ProductField, ProductId,
    ProductMetadata, Program, RuntimeOp, Safepoint, Signature, SourceMetadata, SsaType, Terminator,
    ValueId, VerifiedProgram,
};

use crate::hir::{self, BindingId, BindingStorage, Expr, ExprKind, LocalDefinition, Operation};
use crate::types::Type;

pub(crate) fn lower_program(program: &hir::Program) -> Result<VerifiedProgram> {
    let product_ids: HashMap<String, ProductId> = program
        .products
        .iter()
        .map(|product| (product.name.clone(), ProductId::new(product.id.raw())))
        .collect();
    let function_ids: HashMap<BindingId, FunctionId> = program
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| {
            let raw = u32::try_from(index).unwrap_or(u32::MAX);
            (function.binding, FunctionId::new(raw))
        })
        .collect();
    let function_effects: HashMap<FunctionId, EffectSet> = program
        .functions
        .iter()
        .filter_map(|function| {
            function_ids
                .get(&function.binding)
                .copied()
                .map(|id| (id, effects(function.summary)))
        })
        .collect();

    let mut functions = Vec::with_capacity(program.functions.len().saturating_add(1));
    for function in &program.functions {
        let id = function_ids
            .get(&function.binding)
            .copied()
            .ok_or_else(|| {
                Error::msg(format!(
                    "HIR function binding {} has no SSA FunctionId",
                    function.binding.raw()
                ))
            })?;
        let binding = program.binding(function.binding).ok_or_else(|| {
            Error::msg(format!(
                "HIR function binding {} is missing",
                function.binding.raw()
            ))
        })?;
        let signature = signature_from_type(&binding.ty, &product_ids)?;
        let mut builder = FunctionBuilder::new(
            &product_ids,
            &function_ids,
            &function_effects,
            id,
            binding.name.clone(),
            signature,
            effects(function.summary),
            origin(function.origin.raw(), 0),
        );
        let entry = builder.new_block(origin(function.origin.raw(), 0), false)?;
        builder.entry = entry;
        builder.current = Some(entry);
        if function.params.len() != builder.signature.parameters.len() {
            return Err(Error::msg(format!(
                "HIR function {} parameter/signature mismatch",
                binding.name
            )));
        }
        for (index, (binding_id, ty)) in function
            .params
            .iter()
            .copied()
            .zip(builder.signature.parameters.clone())
            .enumerate()
        {
            let parameter =
                builder.add_block_parameter(entry, ty, origin(function.origin.raw(), 0))?;
            builder.env.insert(binding_id, parameter);
            let slot =
                u16::try_from(index).map_err(|_| Error::msg("SSA parameter slot exceeds u16"))?;
            builder.slots.insert(binding_id, slot);
        }
        let body = builder.lower_expr(&function.body)?;
        if let Some(result) = body {
            builder.terminate(Terminator::Return(result))?;
        }
        functions.push(builder.finish()?);
    }

    let main_id = FunctionId::new(
        u32::try_from(functions.len()).map_err(|_| Error::msg("too many SSA functions"))?,
    );
    let main_signature = Signature::monomorphic(
        Vec::new(),
        lower_type(&program.main.return_type, &product_ids)?,
    );
    let mut builder = FunctionBuilder::new(
        &product_ids,
        &function_ids,
        &function_effects,
        main_id,
        "main".into(),
        main_signature,
        effects(program.main.body.effects),
        origin(program.main.origin.raw(), 0),
    );
    let entry = builder.new_block(origin(program.main.origin.raw(), 0), false)?;
    builder.entry = entry;
    builder.current = Some(entry);
    if let Some(result) = builder.lower_expr(&program.main.body)? {
        builder.terminate(Terminator::Return(result))?;
    }
    functions.push(builder.finish()?);

    let ssa = Program {
        sources: program
            .sources
            .iter()
            .map(|source| SourceMetadata {
                id: source.id.raw(),
                path: source.path.to_string_lossy().into_owned(),
            })
            .collect(),
        products: program
            .products
            .iter()
            .map(|product| {
                Ok(ProductMetadata {
                    id: ProductId::new(product.id.raw()),
                    name: product.name.clone(),
                    fields: product
                        .fields
                        .iter()
                        .map(|field| {
                            Ok(ProductField {
                                name: field.name.clone(),
                                ty: lower_type(&field.ty, &product_ids)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        functions,
        main: main_id,
    };
    let verified = verify(ssa).map_err(ir_error)?;
    lkjscript_ir::normalize_baseline(&verified).map_err(ir_error)
}

struct PendingBlock {
    id: BlockId,
    parameters: Vec<BlockParameter>,
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
    metadata: BlockMetadata,
}

struct FunctionBuilder<'a> {
    product_ids: &'a HashMap<String, ProductId>,
    function_ids: &'a HashMap<BindingId, FunctionId>,
    function_effects: &'a HashMap<FunctionId, EffectSet>,
    id: FunctionId,
    name: String,
    signature: Signature,
    function_effect: EffectSet,
    function_origin: Origin,
    entry: BlockId,
    blocks: Vec<PendingBlock>,
    current: Option<BlockId>,
    next_value: u32,
    next_position: u32,
    value_types: Vec<SsaType>,
    env: BTreeMap<BindingId, ValueId>,
    slots: BTreeMap<BindingId, u16>,
}

impl<'a> FunctionBuilder<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        product_ids: &'a HashMap<String, ProductId>,
        function_ids: &'a HashMap<BindingId, FunctionId>,
        function_effects: &'a HashMap<FunctionId, EffectSet>,
        id: FunctionId,
        name: String,
        signature: Signature,
        function_effect: EffectSet,
        function_origin: Origin,
    ) -> Self {
        Self {
            product_ids,
            function_ids,
            function_effects,
            id,
            name,
            signature,
            function_effect,
            function_origin,
            entry: BlockId::new(0),
            blocks: Vec::new(),
            current: None,
            next_value: 0,
            next_position: 0,
            value_types: Vec::new(),
            env: BTreeMap::new(),
            slots: BTreeMap::new(),
        }
    }

    fn new_block(&mut self, block_origin: Origin, loop_header: bool) -> Result<BlockId> {
        let raw = u32::try_from(self.blocks.len())
            .map_err(|_| Error::msg("SSA block count exceeds u32"))?;
        let id = BlockId::new(raw);
        self.blocks.push(PendingBlock {
            id,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
            metadata: BlockMetadata {
                loop_header,
                origin: block_origin,
                frame_state: None,
            },
        });
        Ok(id)
    }

    fn block_mut(&mut self, id: BlockId) -> Result<&mut PendingBlock> {
        self.blocks
            .get_mut(id.index().unwrap_or(usize::MAX))
            .filter(|block| block.id == id)
            .ok_or_else(|| Error::msg(format!("missing SSA block {}", id.raw())))
    }

    fn add_block_parameter(
        &mut self,
        block: BlockId,
        ty: SsaType,
        parameter_origin: Origin,
    ) -> Result<ValueId> {
        let id = self.next_value(&ty)?;
        self.block_mut(block)?.parameters.push(BlockParameter {
            id,
            ty,
            origin: parameter_origin,
        });
        Ok(id)
    }

    fn next_value(&mut self, ty: &SsaType) -> Result<ValueId> {
        let id = ValueId::new(self.next_value);
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or_else(|| Error::msg("SSA value count exceeds u32"))?;
        self.value_types.push(ty.clone());
        Ok(id)
    }

    fn terminate(&mut self, terminator: Terminator) -> Result<()> {
        let current = self
            .current
            .ok_or_else(|| Error::msg("cannot terminate an ended SSA path"))?;
        let block = self.block_mut(current)?;
        if block.terminator.replace(terminator).is_some() {
            return Err(Error::msg("SSA block has duplicate terminators"));
        }
        self.current = None;
        Ok(())
    }

    fn switch_to(&mut self, block: BlockId) -> Result<()> {
        if self.block_mut(block)?.terminator.is_some() {
            return Err(Error::msg("cannot switch to terminated SSA block"));
        }
        self.current = Some(block);
        Ok(())
    }

    fn append(
        &mut self,
        ty: SsaType,
        kind: InstructionKind,
        effects: EffectSet,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        let current = self
            .current
            .ok_or_else(|| Error::msg("cannot append to an ended SSA path"))?;
        let id = self.next_value(&ty)?;
        let safepoint = if matches!(kind, InstructionKind::Call { .. })
            || effects.contains(EffectSet::ALLOCATES)
            || effects.contains(EffectSet::HOST_IO)
        {
            Safepoint::Required
        } else {
            Safepoint::None
        };
        let frame_state = if safepoint == Safepoint::Required {
            Some(self.frame_state())
        } else {
            None
        };
        let metadata = InstructionMetadata {
            origin: self.next_origin(expression_origin.raw()),
            effects,
            safepoint,
            failure: failure_behavior(effects),
            frame_state,
        };
        self.block_mut(current)?.instructions.push(Instruction {
            id,
            ty,
            kind,
            metadata,
        });
        Ok(id)
    }

    fn next_origin(&mut self, source: u32) -> Origin {
        let position = self.next_position;
        self.next_position = self.next_position.saturating_add(1);
        origin(source, position)
    }

    fn frame_state(&self) -> FrameState {
        FrameState {
            bytecode_position: self.next_position,
            locals: self
                .env
                .iter()
                .filter_map(|(binding, value)| {
                    self.slots.get(binding).map(|slot| FrameLocal {
                        binding: SsaBindingId::new(binding.raw()),
                        slot: *slot,
                        value: *value,
                    })
                })
                .collect(),
            operand_stack: Vec::new(),
        }
    }

    fn constant(
        &mut self,
        ty: SsaType,
        constant: Constant,
        expression_origin: hir::SourceId,
    ) -> Result<ValueId> {
        self.append(
            ty,
            InstructionKind::Constant(constant),
            EffectSet::PURE,
            expression_origin,
        )
    }

    fn lower_expr(&mut self, expression: &Expr) -> Result<Option<ValueId>> {
        let ty = lower_type(&expression.ty, self.product_ids)?;
        let value = match &expression.kind {
            ExprKind::LitI64(value) => {
                self.constant(SsaType::I64, Constant::I64(*value), expression.origin)?
            }
            ExprKind::LitF64(value) => {
                self.constant(SsaType::F64, Constant::F64(*value), expression.origin)?
            }
            ExprKind::LitBool(value) => {
                self.constant(SsaType::Bool, Constant::Bool(*value), expression.origin)?
            }
            ExprKind::LitUnit => self.constant(SsaType::Unit, Constant::Unit, expression.origin)?,
            ExprKind::EmptyList => self.constant(ty, Constant::EmptyList, expression.origin)?,
            ExprKind::LitNone => self.constant(ty, Constant::None, expression.origin)?,
            ExprKind::LitStr(value) => self.constant(
                SsaType::Str,
                Constant::Str(value.clone()),
                expression.origin,
            )?,
            ExprKind::QuoteSymbol(value) => self.constant(
                SsaType::Symbol,
                Constant::Symbol(value.clone()),
                expression.origin,
            )?,
            ExprKind::Load(binding) => return self.lower_load(*binding, expression),
            ExprKind::Call { callee, args } => {
                let Some(arguments) = self.lower_arguments(args)? else {
                    return Ok(None);
                };
                let signature = Signature::monomorphic(
                    args.iter()
                        .map(|argument| lower_type(&argument.ty, self.product_ids))
                        .collect::<Result<Vec<_>>>()?,
                    ty.clone(),
                );
                let (target, call_effects) = match callee.storage {
                    BindingStorage::Function => {
                        let function =
                            self.function_ids
                                .get(&callee.binding)
                                .copied()
                                .ok_or_else(|| {
                                    Error::msg(format!(
                                        "HIR call target {} has no SSA function",
                                        callee.binding.raw()
                                    ))
                                })?;
                        let effects = self
                            .function_effects
                            .get(&function)
                            .copied()
                            .ok_or_else(|| Error::msg("SSA call target has no effect summary"))?;
                        (CallTarget::Direct(function), effects)
                    }
                    BindingStorage::Local(_) => {
                        let target = self.env.get(&callee.binding).copied().ok_or_else(|| {
                            Error::msg(format!(
                                "HIR local call target {} is not in SSA environment",
                                callee.binding.raw()
                            ))
                        })?;
                        (CallTarget::Indirect(target), EffectSet::CONSERVATIVE_CALL)
                    }
                };
                self.append(
                    ty,
                    InstructionKind::Call {
                        target,
                        arguments,
                        signature,
                    },
                    call_effects,
                    expression.origin,
                )?
            }
            ExprKind::Operation {
                operation,
                resolved_signature,
                args,
                ..
            } => {
                if *operation == Operation::Exit {
                    let Some(arguments) = self.lower_arguments(args)? else {
                        return Ok(None);
                    };
                    let Some(code) = arguments.first().copied() else {
                        return Err(Error::msg("resolved exit has no code argument"));
                    };
                    self.terminate(Terminator::Exit { code })?;
                    return Ok(None);
                }
                if matches!(operation, Operation::And | Operation::Or) {
                    return self.lower_short_circuit(*operation, args, expression);
                }
                let Some(arguments) = self.lower_arguments(args)? else {
                    return Ok(None);
                };
                let runtime = runtime_operation(*operation)?;
                let signature = signature_from_type(resolved_signature, self.product_ids)?;
                self.append(
                    ty,
                    InstructionKind::Runtime {
                        operation: runtime,
                        arguments,
                        signature,
                    },
                    effects(operation.effects()),
                    expression.origin,
                )?
            }
            ExprKind::Do(expressions) => {
                return self.lower_sequence(expressions, expression.origin)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                return self.lower_if(condition, then_branch, else_branch, expression);
            }
            ExprKind::While { condition, body } => {
                return self.lower_while(condition, body, expression);
            }
            ExprKind::Let { bindings, body } => {
                return self.lower_let(bindings, body);
            }
            ExprKind::MutableLocal {
                binding,
                slot,
                initial,
                body,
            } => {
                let Some(initial) = self.lower_expr(initial)? else {
                    return Ok(None);
                };
                let previous_value = self.env.insert(*binding, initial);
                let previous_slot = self.slots.insert(*binding, u16::from(*slot));
                let result = self.lower_expr(body);
                restore(&mut self.env, *binding, previous_value);
                restore(&mut self.slots, *binding, previous_slot);
                return result;
            }
            ExprKind::SetLocal {
                target,
                slot,
                value,
            } => {
                let Some(value) = self.lower_expr(value)? else {
                    return Ok(None);
                };
                if !self.env.contains_key(target) {
                    return Err(Error::msg(format!(
                        "HIR set target {} is not in SSA environment",
                        target.raw()
                    )));
                }
                self.env.insert(*target, value);
                self.slots.insert(*target, u16::from(*slot));
                self.constant(SsaType::Unit, Constant::Unit, expression.origin)?
            }
            ExprKind::ProductValue { product, fields } => {
                let Some(fields) = self.lower_arguments(fields)? else {
                    return Ok(None);
                };
                let product = ProductId::new(product.raw());
                self.append(
                    SsaType::Product(product),
                    InstructionKind::ProductValue { product, fields },
                    EffectSet::ALLOCATES,
                    expression.origin,
                )?
            }
            ExprKind::ProductField {
                product,
                field,
                value,
            } => {
                let Some(value) = self.lower_expr(value)? else {
                    return Ok(None);
                };
                let product = ProductId::new(product.raw());
                self.append(
                    ty,
                    InstructionKind::ProductField {
                        product,
                        field: *field,
                        value,
                    },
                    EffectSet::READS_MEMORY,
                    expression.origin,
                )?
            }
            ExprKind::WithProductField {
                product,
                field,
                value,
                replacement,
            } => {
                let Some(value) = self.lower_expr(value)? else {
                    return Ok(None);
                };
                let Some(replacement) = self.lower_expr(replacement)? else {
                    return Ok(None);
                };
                let product = ProductId::new(product.raw());
                self.append(
                    SsaType::Product(product),
                    InstructionKind::WithProductField {
                        product,
                        field: *field,
                        value,
                        replacement,
                    },
                    EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES),
                    expression.origin,
                )?
            }
        };
        Ok(Some(value))
    }

    fn lower_load(
        &mut self,
        binding: hir::BindingRef,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        match binding.storage {
            BindingStorage::Local(_) => self
                .env
                .get(&binding.binding)
                .copied()
                .map(Some)
                .ok_or_else(|| {
                    Error::msg(format!(
                        "HIR binding {} is not in SSA environment",
                        binding.binding.raw()
                    ))
                }),
            BindingStorage::Function => {
                let target = self
                    .function_ids
                    .get(&binding.binding)
                    .copied()
                    .ok_or_else(|| {
                        Error::msg(format!(
                            "HIR function binding {} has no SSA FunctionId",
                            binding.binding.raw()
                        ))
                    })?;
                let ty = lower_type(&expression.ty, self.product_ids)?;
                self.append(
                    ty,
                    InstructionKind::FunctionRef(target),
                    EffectSet::PURE,
                    expression.origin,
                )
                .map(Some)
            }
        }
    }

    fn lower_arguments(&mut self, arguments: &[Expr]) -> Result<Option<Vec<ValueId>>> {
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let Some(value) = self.lower_expr(argument)? else {
                return Ok(None);
            };
            values.push(value);
        }
        Ok(Some(values))
    }

    fn lower_sequence(
        &mut self,
        expressions: &[Expr],
        sequence_origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
        let mut result = None;
        for expression in expressions {
            result = self.lower_expr(expression)?;
            if result.is_none() {
                return Ok(None);
            }
        }
        if let Some(result) = result {
            Ok(Some(result))
        } else {
            self.constant(SsaType::Unit, Constant::Unit, sequence_origin)
                .map(Some)
        }
    }

    fn lower_let(&mut self, bindings: &[LocalDefinition], body: &Expr) -> Result<Option<ValueId>> {
        let mut previous = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let Some(value) = self.lower_expr(&binding.value)? else {
                for (binding, previous_value, previous_slot) in previous.into_iter().rev() {
                    restore(&mut self.env, binding, previous_value);
                    restore(&mut self.slots, binding, previous_slot);
                }
                return Ok(None);
            };
            previous.push((
                binding.binding,
                self.env.insert(binding.binding, value),
                self.slots.insert(binding.binding, u16::from(binding.slot)),
            ));
        }
        let result = self.lower_expr(body);
        for (binding, previous_value, previous_slot) in previous.into_iter().rev() {
            restore(&mut self.env, binding, previous_value);
            restore(&mut self.slots, binding, previous_slot);
        }
        result
    }

    fn lower_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let Some(condition_value) = self.lower_expr(condition)? else {
            return Ok(None);
        };
        let branch_origin = origin(expression.origin.raw(), self.next_position);
        let then_block = self.new_block(branch_origin, false)?;
        let else_block = self.new_block(branch_origin, false)?;
        self.terminate(Terminator::ConditionalBranch {
            condition: condition_value,
            true_target: then_block,
            true_arguments: Vec::new(),
            false_target: else_block,
            false_arguments: Vec::new(),
        })?;
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();

        self.switch_to(then_block)?;
        self.env = incoming_env.clone();
        self.slots = incoming_slots.clone();
        let then_value = self.lower_expr(then_branch)?;
        let then_end = self.current;
        let then_env = self.env.clone();

        self.switch_to(else_block)?;
        self.env = incoming_env.clone();
        self.slots = incoming_slots.clone();
        let else_value = self.lower_expr(else_branch)?;
        let else_end = self.current;
        let else_env = self.env.clone();

        self.merge_branches(
            lower_type(&expression.ty, self.product_ids)?,
            expression.origin,
            incoming_env,
            incoming_slots,
            (then_value, then_end, then_env),
            (else_value, else_end, else_env),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn merge_branches(
        &mut self,
        result_type: SsaType,
        expression_origin: hir::SourceId,
        incoming_env: BTreeMap<BindingId, ValueId>,
        incoming_slots: BTreeMap<BindingId, u16>,
        then_result: (
            Option<ValueId>,
            Option<BlockId>,
            BTreeMap<BindingId, ValueId>,
        ),
        else_result: (
            Option<ValueId>,
            Option<BlockId>,
            BTreeMap<BindingId, ValueId>,
        ),
    ) -> Result<Option<ValueId>> {
        match (then_result.0, else_result.0) {
            (None, None) => {
                self.current = None;
                self.env = incoming_env;
                self.slots = incoming_slots;
                Ok(None)
            }
            (Some(value), None) => {
                self.current = then_result.1;
                self.env = then_result.2;
                self.slots = incoming_slots;
                Ok(Some(value))
            }
            (None, Some(value)) => {
                self.current = else_result.1;
                self.env = else_result.2;
                self.slots = incoming_slots;
                Ok(Some(value))
            }
            (Some(then_value), Some(else_value)) => {
                let merge =
                    self.new_block(origin(expression_origin.raw(), self.next_position), false)?;
                let result = self.add_block_parameter(
                    merge,
                    result_type,
                    origin(expression_origin.raw(), self.next_position),
                )?;
                let bindings: Vec<BindingId> = incoming_env.keys().copied().collect();
                let mut merge_env = BTreeMap::new();
                for binding in &bindings {
                    let ty = self.value_type(
                        *incoming_env
                            .get(binding)
                            .ok_or_else(|| Error::msg("SSA merge lost incoming binding"))?,
                    )?;
                    let parameter = self.add_block_parameter(
                        merge,
                        ty,
                        origin(expression_origin.raw(), self.next_position),
                    )?;
                    merge_env.insert(*binding, parameter);
                }
                let then_arguments = edge_arguments(then_value, &bindings, &then_result.2)?;
                let else_arguments = edge_arguments(else_value, &bindings, &else_result.2)?;
                self.current = then_result.1;
                self.terminate(Terminator::Branch {
                    target: merge,
                    arguments: then_arguments,
                })?;
                self.current = else_result.1;
                self.terminate(Terminator::Branch {
                    target: merge,
                    arguments: else_arguments,
                })?;
                self.switch_to(merge)?;
                self.env = merge_env;
                self.slots = incoming_slots;
                Ok(Some(result))
            }
        }
    }

    fn lower_short_circuit(
        &mut self,
        operation: Operation,
        arguments: &[Expr],
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let [left, right] = arguments else {
            return Err(Error::msg(
                "resolved short-circuit operation must have two arguments",
            ));
        };
        let Some(left) = self.lower_expr(left)? else {
            return Ok(None);
        };
        let branch_origin = origin(expression.origin.raw(), self.next_position);
        let evaluate_right = self.new_block(branch_origin, false)?;
        let skip_right = self.new_block(branch_origin, false)?;
        let (true_target, false_target, skipped) = if operation == Operation::And {
            (evaluate_right, skip_right, false)
        } else {
            (skip_right, evaluate_right, true)
        };
        self.terminate(Terminator::ConditionalBranch {
            condition: left,
            true_target,
            true_arguments: Vec::new(),
            false_target,
            false_arguments: Vec::new(),
        })?;
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();

        self.switch_to(evaluate_right)?;
        self.env = incoming_env.clone();
        self.slots = incoming_slots.clone();
        let right_value = self.lower_expr(right)?;
        let right_end = self.current;
        let right_env = self.env.clone();

        self.switch_to(skip_right)?;
        self.env = incoming_env.clone();
        self.slots = incoming_slots.clone();
        let skipped_value =
            self.constant(SsaType::Bool, Constant::Bool(skipped), expression.origin)?;
        let skipped_end = self.current;
        let skipped_env = self.env.clone();

        let (then_result, else_result) = if operation == Operation::And {
            (
                (right_value, right_end, right_env),
                (Some(skipped_value), skipped_end, skipped_env),
            )
        } else {
            (
                (Some(skipped_value), skipped_end, skipped_env),
                (right_value, right_end, right_env),
            )
        };
        self.merge_branches(
            SsaType::Bool,
            expression.origin,
            incoming_env,
            incoming_slots,
            then_result,
            else_result,
        )
    }

    fn lower_while(
        &mut self,
        condition: &Expr,
        body: &[Expr],
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        let preheader = self
            .current
            .ok_or_else(|| Error::msg("while has no live SSA preheader"))?;
        let incoming_env = self.env.clone();
        let incoming_slots = self.slots.clone();
        let bindings: Vec<BindingId> = incoming_env.keys().copied().collect();
        let header = self.new_block(origin(expression.origin.raw(), self.next_position), true)?;
        let mut header_env = BTreeMap::new();
        for binding in &bindings {
            let incoming = incoming_env
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA loop lost incoming binding"))?;
            let parameter = self.add_block_parameter(
                header,
                self.value_type(incoming)?,
                origin(expression.origin.raw(), self.next_position),
            )?;
            header_env.insert(*binding, parameter);
        }
        self.current = Some(preheader);
        self.terminate(Terminator::Branch {
            target: header,
            arguments: bindings
                .iter()
                .map(|binding| {
                    incoming_env
                        .get(binding)
                        .copied()
                        .ok_or_else(|| Error::msg("SSA loop preheader lost binding"))
                })
                .collect::<Result<Vec<_>>>()?,
        })?;
        self.switch_to(header)?;
        self.env = header_env;
        self.slots = incoming_slots.clone();
        let header_frame = self.frame_state();
        self.block_mut(header)?.metadata.frame_state = Some(header_frame);

        let Some(condition_value) = self.lower_expr(condition)? else {
            self.current = None;
            return Ok(None);
        };
        let condition_env = self.env.clone();
        let body_block =
            self.new_block(origin(expression.origin.raw(), self.next_position), false)?;
        let exit_block =
            self.new_block(origin(expression.origin.raw(), self.next_position), false)?;
        let mut body_env = BTreeMap::new();
        let mut exit_env = BTreeMap::new();
        for binding in &bindings {
            let value = condition_env
                .get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA loop condition lost binding"))?;
            let ty = self.value_type(value)?;
            body_env.insert(
                *binding,
                self.add_block_parameter(
                    body_block,
                    ty.clone(),
                    origin(expression.origin.raw(), self.next_position),
                )?,
            );
            exit_env.insert(
                *binding,
                self.add_block_parameter(
                    exit_block,
                    ty,
                    origin(expression.origin.raw(), self.next_position),
                )?,
            );
        }
        let condition_arguments: Vec<ValueId> = bindings
            .iter()
            .map(|binding| {
                condition_env
                    .get(binding)
                    .copied()
                    .ok_or_else(|| Error::msg("SSA loop edge lost binding"))
            })
            .collect::<Result<Vec<_>>>()?;
        self.terminate(Terminator::ConditionalBranch {
            condition: condition_value,
            true_target: body_block,
            true_arguments: condition_arguments.clone(),
            false_target: exit_block,
            false_arguments: condition_arguments,
        })?;

        self.switch_to(body_block)?;
        self.env = body_env;
        self.slots = incoming_slots.clone();
        let body_result = self.lower_sequence(body, expression.origin)?;
        let mut has_backedge = false;
        if body_result.is_some() {
            let arguments = bindings
                .iter()
                .map(|binding| {
                    self.env
                        .get(binding)
                        .copied()
                        .ok_or_else(|| Error::msg("SSA loop body lost binding"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.terminate(Terminator::Branch {
                target: header,
                arguments,
            })?;
            has_backedge = true;
        }
        if !has_backedge {
            self.block_mut(header)?.metadata.loop_header = false;
            self.block_mut(header)?.metadata.frame_state = None;
        }

        self.switch_to(exit_block)?;
        self.env = exit_env;
        self.slots = incoming_slots;
        self.constant(SsaType::Unit, Constant::Unit, expression.origin)
            .map(Some)
    }

    fn value_type(&self, value: ValueId) -> Result<SsaType> {
        self.value_types
            .get(value.index().unwrap_or(usize::MAX))
            .cloned()
            .ok_or_else(|| Error::msg(format!("missing SSA value type {}", value.raw())))
    }

    fn finish(self) -> Result<Function> {
        let blocks = self
            .blocks
            .into_iter()
            .map(|block| {
                Ok(Block {
                    id: block.id,
                    parameters: block.parameters,
                    instructions: block.instructions,
                    terminator: block.terminator.ok_or_else(|| {
                        Error::msg(format!("SSA block {} has no terminator", block.id.raw()))
                    })?,
                    metadata: block.metadata,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Function {
            id: self.id,
            name: self.name,
            signature: self.signature,
            effects: self.function_effect,
            entry: self.entry,
            blocks,
            origin: self.function_origin,
        })
    }
}

fn edge_arguments(
    result: ValueId,
    bindings: &[BindingId],
    env: &BTreeMap<BindingId, ValueId>,
) -> Result<Vec<ValueId>> {
    let mut arguments = Vec::with_capacity(bindings.len().saturating_add(1));
    arguments.push(result);
    for binding in bindings {
        arguments.push(
            env.get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA branch lost mutable environment binding"))?,
        );
    }
    Ok(arguments)
}

fn restore<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, previous: Option<V>) {
    if let Some(previous) = previous {
        map.insert(key, previous);
    } else {
        map.remove(&key);
    }
}

fn signature_from_type(ty: &Type, products: &HashMap<String, ProductId>) -> Result<Signature> {
    match ty {
        Type::Fn { params, ret } => Ok(Signature::monomorphic(
            params
                .iter()
                .map(|parameter| lower_type(parameter, products))
                .collect::<Result<Vec<_>>>()?,
            lower_type(ret, products)?,
        )),
        Type::Forall { vars, body } => {
            let Type::Fn { params, ret } = body.as_ref() else {
                return Err(Error::msg("HIR forall body is not a function"));
            };
            Ok(Signature {
                type_parameters: vars.clone(),
                parameters: params
                    .iter()
                    .map(|parameter| lower_type(parameter, products))
                    .collect::<Result<Vec<_>>>()?,
                result: Box::new(lower_type(ret, products)?),
            })
        }
        _ => Err(Error::msg("HIR callable does not have a function type")),
    }
}

fn lower_type(ty: &Type, products: &HashMap<String, ProductId>) -> Result<SsaType> {
    Ok(match ty {
        Type::Unit => SsaType::Unit,
        Type::Bool => SsaType::Bool,
        Type::I64 => SsaType::I64,
        Type::F64 => SsaType::F64,
        Type::Str => SsaType::Str,
        Type::Buf => SsaType::Buf,
        Type::Symbol => SsaType::Symbol,
        Type::Handle => SsaType::Handle,
        Type::Product(name) => SsaType::Product(
            *products
                .get(name)
                .ok_or_else(|| Error::msg(format!("HIR type references unknown product {name}")))?,
        ),
        Type::Param(name) => SsaType::TypeParameter(name.clone()),
        Type::List(item) => SsaType::List(Box::new(lower_type(item, products)?)),
        Type::Option(item) => SsaType::Option(Box::new(lower_type(item, products)?)),
        Type::Result(ok, err) => SsaType::Result(
            Box::new(lower_type(ok, products)?),
            Box::new(lower_type(err, products)?),
        ),
        Type::Fn { .. } | Type::Forall { .. } => {
            SsaType::Function(Box::new(signature_from_type(ty, products)?))
        }
    })
}

fn runtime_operation(operation: Operation) -> Result<RuntimeOp> {
    Ok(match operation {
        Operation::Add => RuntimeOp::Add,
        Operation::Subtract => RuntimeOp::Subtract,
        Operation::Multiply => RuntimeOp::Multiply,
        Operation::Divide => RuntimeOp::Divide,
        Operation::EqualValue => RuntimeOp::EqualValue,
        Operation::SameObject => RuntimeOp::SameObject,
        Operation::ListEqual => RuntimeOp::ListEqual,
        Operation::F64BitsEqual => RuntimeOp::F64BitsEqual,
        Operation::Less => RuntimeOp::Less,
        Operation::LessEqual => RuntimeOp::LessEqual,
        Operation::Greater => RuntimeOp::Greater,
        Operation::GreaterEqual => RuntimeOp::GreaterEqual,
        Operation::Not => RuntimeOp::Not,
        Operation::Cons => RuntimeOp::Cons,
        Operation::Car => RuntimeOp::Car,
        Operation::Cdr => RuntimeOp::Cdr,
        Operation::IsEmptyList => RuntimeOp::IsEmptyList,
        Operation::Print => RuntimeOp::Print,
        Operation::Flush => RuntimeOp::Flush,
        Operation::ReadByte => RuntimeOp::ReadByte,
        Operation::WriteByte => RuntimeOp::WriteByte,
        Operation::BitAnd => RuntimeOp::BitAnd,
        Operation::BitOr => RuntimeOp::BitOr,
        Operation::BitXor => RuntimeOp::BitXor,
        Operation::WriteStr => RuntimeOp::WriteStr,
        Operation::EmptyStr => RuntimeOp::EmptyStr,
        Operation::ArgCount => RuntimeOp::ArgCount,
        Operation::Arg => RuntimeOp::Arg,
        Operation::BufNew => RuntimeOp::BufNew,
        Operation::BufLen => RuntimeOp::BufLen,
        Operation::BufRef => RuntimeOp::BufRef,
        Operation::BufSet => RuntimeOp::BufSet,
        Operation::BufClone => RuntimeOp::BufClone,
        Operation::BufGetU32 => RuntimeOp::BufGetU32,
        Operation::BufSetU32 => RuntimeOp::BufSetU32,
        Operation::StrLen => RuntimeOp::StrLen,
        Operation::StrRef => RuntimeOp::StrRef,
        Operation::StrAppend => RuntimeOp::StrAppend,
        Operation::StrSlice => RuntimeOp::StrSlice,
        Operation::StrFromByte => RuntimeOp::StrFromByte,
        Operation::StrFromI64 => RuntimeOp::StrFromI64,
        Operation::StrFromF64 => RuntimeOp::StrFromF64,
        Operation::StdinHandle => RuntimeOp::StdinHandle,
        Operation::SysIsatty => RuntimeOp::SysIsatty,
        Operation::SysClose => RuntimeOp::SysClose,
        Operation::SysReadByte => RuntimeOp::SysReadByte,
        Operation::SysWriteByte => RuntimeOp::SysWriteByte,
        Operation::SysTtyGuardSave => RuntimeOp::SysTtyGuardSave,
        Operation::SysTtyGuardClear => RuntimeOp::SysTtyGuardClear,
        Operation::SysOpenRead => RuntimeOp::SysOpenRead,
        Operation::SysOpenWrite => RuntimeOp::SysOpenWrite,
        Operation::SysPathExists => RuntimeOp::SysPathExists,
        Operation::SysWaitMs => RuntimeOp::SysWaitMs,
        Operation::SysNowMs => RuntimeOp::SysNowMs,
        Operation::SysSocket => RuntimeOp::SysSocket,
        Operation::SysBind => RuntimeOp::SysBind,
        Operation::SysListen => RuntimeOp::SysListen,
        Operation::SysAccept => RuntimeOp::SysAccept,
        Operation::SysRecv => RuntimeOp::SysRecv,
        Operation::SysSend => RuntimeOp::SysSend,
        Operation::SysPoll => RuntimeOp::SysPoll,
        Operation::SysTtyGet => RuntimeOp::SysTtyGet,
        Operation::SysTtySet => RuntimeOp::SysTtySet,
        Operation::Ok => RuntimeOp::Ok,
        Operation::Err => RuntimeOp::Err,
        Operation::IsOk => RuntimeOp::IsOk,
        Operation::UnwrapOk => RuntimeOp::UnwrapOk,
        Operation::UnwrapErr => RuntimeOp::UnwrapErr,
        Operation::Some => RuntimeOp::Some,
        Operation::IsSome => RuntimeOp::IsSome,
        Operation::UnwrapSome => RuntimeOp::UnwrapSome,
        Operation::Exit | Operation::And | Operation::Or => {
            return Err(Error::msg(format!(
                "control operation {operation:?} cannot lower as an SSA runtime operation"
            )));
        }
    })
}

fn effects(effects: hir::EffectSet) -> EffectSet {
    EffectSet::from_bits(effects.bits())
}

fn failure_behavior(effects: EffectSet) -> FailureBehavior {
    match (
        effects.contains(EffectSet::MAY_TRAP),
        effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES),
    ) {
        (false, false) => FailureBehavior::None,
        (true, false) => FailureBehavior::Trap,
        (false, true) => FailureBehavior::StructuredOutcome,
        (true, true) => FailureBehavior::TrapOrOutcome,
    }
}

const fn origin(source: u32, node: u32) -> Origin {
    Origin { source, node }
}

fn ir_error(error: lkjscript_ir::IrError) -> Error {
    Error::msg(format!("typed SSA verification failed: {error}"))
}

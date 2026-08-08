use super::*;

impl<'a> Producer<'a> {
    pub(super) fn run(mut self) -> Result<HirMemoryPlan> {
        let function_count = self
            .program
            .functions
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::msg("HIR memory-plan function count overflow"))?;
        self.charge_functions(function_count)?;
        for (index, function) in self.program.functions.iter().enumerate() {
            let id = MemoryFunctionId::new(
                u64::try_from(index)
                    .map_err(|_| Error::msg("HIR memory-plan function identity exceeds u64"))?,
            );
            self.build_function(id, function)?;
        }
        let main_id = MemoryFunctionId::new(
            u64::try_from(self.program.functions.len())
                .map_err(|_| Error::msg("HIR memory-plan main identity exceeds u64"))?,
        );
        self.build_main(main_id)?;
        self.finish_loans()?;
        self.finish_drop_classes()?;
        let witness_groups = self.finalize_witness_groups()?;
        let type_facts = std::mem::take(&mut self.type_planner.facts);
        let witnesses = std::mem::take(&mut self.type_planner.witnesses);
        let (value_placements, placement_work) = derive_value_placements(
            self.program,
            &self.entries,
            &type_facts,
            &witnesses,
            &self.uses,
        )?;
        self.work.value_placements = u64::try_from(value_placements.len())
            .map_err(|_| Error::msg("HIR value placement count exceeds u64"))?;
        self.work.placement_work = placement_work;
        let drop_paths = std::mem::take(&mut self.type_planner.drop_paths);
        let drop_glues = std::mem::take(&mut self.type_planner.glues);
        let mut plan = HirMemoryPlan {
            schema: HIR_MEMORY_PLAN_SCHEMA,
            id: MemoryPlanId::from_bytes([0; 32]),
            functions: self.functions,
            entries: self.entries,
            uses: self.uses,
            loans: self.loans,
            constants: self.constants,
            calls: self.calls,
            obligations: self.obligations,
            type_facts,
            witness_groups,
            witnesses,
            destinations: self.destinations,
            value_placements,
            borrow_scopes: self.borrow_scopes,
            drop_paths,
            drop_glues,
            work: self.work,
        };
        plan.id = compute_plan_id(&plan)?;
        Ok(plan)
    }
    pub(super) fn build_signatures(&mut self) -> Result<()> {
        for (index, function) in self.program.functions.iter().enumerate() {
            let function_id = MemoryFunctionId::new(
                u64::try_from(index)
                    .map_err(|_| Error::msg("HIR memory signature identity exceeds u64"))?,
            );
            let binding = self.program.binding(function.binding).ok_or_else(|| {
                Error::msg("HIR memory signature references unknown function binding")
            })?;
            let result_ty = function_result_type(&binding.ty)?.clone();
            let witness_parameters = memory_witness_parameters(&binding.ty, Some(&function.body))?;
            let mut parameters = Vec::with_capacity(function.params.len());
            for parameter in &function.params {
                let parameter_ty = &self
                    .program
                    .binding(*parameter)
                    .ok_or_else(|| {
                        Error::msg("HIR memory signature references unknown parameter binding")
                    })?
                    .ty;
                let parameter_ty = parameter_ty.clone();
                let dispose_parameter = match &parameter_ty {
                    Type::Param(name) => witness_parameters.iter().any(|requirement| {
                        requirement.parameter == *name
                            && requirement
                                .operations
                                .contains(&MemoryWitnessOperation::Dispose)
                    }),
                    _ => false,
                };
                parameters.push(if dispose_parameter {
                    MemoryParameterMode::Consume
                } else {
                    self.planned_parameter_mode(
                        &parameter_ty,
                        resource_parameter_consumed(&function.body, *parameter),
                    )?
                });
            }
            let result = self.planned_result_mode(&result_ty)?;
            self.signatures.push(FunctionMemorySignature {
                function: function_id,
                witness_parameters,
                parameters,
                result,
            });
        }
        let main_id = MemoryFunctionId::new(
            u64::try_from(self.program.functions.len())
                .map_err(|_| Error::msg("HIR main memory signature identity exceeds u64"))?,
        );
        let main_parameters = self
            .program
            .main
            .param_types
            .clone()
            .into_iter()
            .map(|ty| self.planned_parameter_mode(&ty, false))
            .collect::<Result<Vec<_>>>()?;
        let main_result = self.planned_result_mode(&self.program.main.return_type.clone())?;
        self.signatures.push(FunctionMemorySignature {
            function: main_id,
            witness_parameters: Vec::new(),
            parameters: main_parameters,
            result: main_result,
        });
        Ok(())
    }
    pub(super) fn build_function(
        &mut self,
        id: MemoryFunctionId,
        function: &hir::Function,
    ) -> Result<()> {
        self.current_function = id;
        self.next_place = 0;
        let signature = self.signature(id)?.clone();
        let mut parameter_entries = Vec::with_capacity(function.params.len());
        for (index, ((binding, place), mode)) in function
            .params
            .iter()
            .copied()
            .zip(function.param_places.iter().copied())
            .zip(signature.parameters.iter().copied())
            .enumerate()
        {
            let ty = self.binding_type(binding)?.clone();
            let parameter_entry = self.add_entry(
                MemorySubject::Parameter {
                    function: id,
                    index: u64::try_from(index)
                        .map_err(|_| Error::msg("HIR memory parameter index exceeds u64"))?,
                    binding: binding.raw(),
                    place: place.raw(),
                },
                &ty,
                0,
                MemoryEscape::Caller,
                MemoryOrigin {
                    source: crate::memory_plan::source_origin(function.origin),
                    expression: None,
                },
            )?;
            parameter_entries.push(parameter_entry);
            self.add_place(
                id,
                binding,
                place.raw(),
                &ty,
                crate::memory_plan::source_origin(function.origin),
                mode == MemoryParameterMode::Consume,
            )?;
        }
        let result_ty = function_result_type(self.binding_type(function.binding)?)?.clone();
        let result_entry = self.add_entry(
            MemorySubject::Result { function: id },
            &result_ty,
            function.summary.bits(),
            MemoryEscape::Returned,
            MemoryOrigin {
                source: crate::memory_plan::source_origin(function.origin),
                expression: None,
            },
        )?;
        let body = self.walk_expr(&function.body, None, 0, MemoryEscape::Returned, None)?;
        let name = self
            .program
            .binding(function.binding)
            .ok_or_else(|| Error::msg("HIR memory plan lost function binding"))?
            .name
            .clone();
        self.functions.push(FunctionMemoryPlan {
            id,
            name,
            binding: Some(function.binding.raw()),
            source: crate::memory_plan::source_origin(function.origin),
            signature,
            parameter_entries,
            result_entry,
            body,
        });
        Ok(())
    }
}

impl<'a> Producer<'a> {
    pub(super) fn build_main(&mut self, id: MemoryFunctionId) -> Result<()> {
        self.current_function = id;
        self.next_place = 0;
        let signature = self.signature(id)?.clone();
        let mut parameter_entries = Vec::with_capacity(self.program.main.params.len());
        for (index, (((binding, place), ty), mode)) in self
            .program
            .main
            .params
            .iter()
            .copied()
            .zip(self.program.main.param_places.iter().copied())
            .zip(self.program.main.param_types.iter())
            .zip(signature.parameters.iter().copied())
            .enumerate()
        {
            let entry = self.add_entry(
                MemorySubject::Parameter {
                    function: id,
                    index: u64::try_from(index)
                        .map_err(|_| Error::msg("HIR main memory parameter index exceeds u64"))?,
                    binding: binding.raw(),
                    place: place.raw(),
                },
                ty,
                0,
                MemoryEscape::Caller,
                MemoryOrigin {
                    source: crate::memory_plan::source_origin(self.program.main.origin),
                    expression: None,
                },
            )?;
            parameter_entries.push(entry);
            self.add_place(
                id,
                binding,
                place.raw(),
                ty,
                crate::memory_plan::source_origin(self.program.main.origin),
                mode == MemoryParameterMode::Consume,
            )?;
        }
        let result_entry = self.add_entry(
            MemorySubject::Result { function: id },
            &self.program.main.return_type,
            self.program.main.body.effects.bits(),
            MemoryEscape::Returned,
            MemoryOrigin {
                source: crate::memory_plan::source_origin(self.program.main.origin),
                expression: None,
            },
        )?;
        let body = self.walk_expr(
            &self.program.main.body,
            None,
            0,
            MemoryEscape::Returned,
            None,
        )?;
        self.functions.push(FunctionMemoryPlan {
            id,
            name: "main".into(),
            binding: None,
            source: crate::memory_plan::source_origin(self.program.main.origin),
            signature,
            parameter_entries,
            result_entry,
            body,
        });
        Ok(())
    }
    pub(super) fn walk_expr(
        &mut self,
        expression: &Expr,
        parent: Option<MemoryExpressionId>,
        child_index: u64,
        escape: MemoryEscape,
        loan_binding: Option<BindingId>,
    ) -> Result<MemoryExpressionId> {
        crate::stack::grow(|| {
            self.walk_expr_inner(expression, parent, child_index, escape, loan_binding)
        })
    }

    pub(super) fn walk_expr_inner(
        &mut self,
        expression: &Expr,
        parent: Option<MemoryExpressionId>,
        child_index: u64,
        escape: MemoryEscape,
        loan_binding: Option<BindingId>,
    ) -> Result<MemoryExpressionId> {
        let expression_id = self.next_expression()?;
        self.expression_parents.insert(expression_id, parent);
        let expression_entry = self.add_entry(
            MemorySubject::Expression {
                expression: expression_id,
                parent,
                child_index,
                kind: expression_kind(&expression.kind),
            },
            &expression.ty,
            expression.effects.bits(),
            escape,
            MemoryOrigin {
                source: crate::memory_plan::source_origin(expression.origin),
                expression: Some(expression_id),
            },
        )?;
        match &expression.kind {
            ExprKind::Hole => unreachable!("complete HIR cannot contain a hole"),
            ExprKind::LitI64(_)
            | ExprKind::LitF64(_)
            | ExprKind::LitBool(_)
            | ExprKind::LitUnit
            | ExprKind::EmptyList
            | ExprKind::LitStr(_)
            | ExprKind::LitBytes(_)
            | ExprKind::QuoteSymbol(_)
            | ExprKind::Load(_)
            | ExprKind::Move { .. }
            | ExprKind::Borrow { .. }
            | ExprKind::BorrowBytes { .. } => self.walk_leaf(
                expression,
                expression_id,
                expression_entry,
                escape,
                loan_binding,
            )?,
            ExprKind::Call { .. }
            | ExprKind::Operation { .. }
            | ExprKind::F64FromI64Exact(_)
            | ExprKind::F64FromI64Rounded(_)
            | ExprKind::I64FromF64Exact(_)
            | ExprKind::I64FromF64Trunc(_)
            | ExprKind::Do(_)
            | ExprKind::If { .. }
            | ExprKind::While { .. }
            | ExprKind::Loop { .. }
            | ExprKind::Return { .. }
            | ExprKind::Break { .. }
            | ExprKind::Continue { .. }
            | ExprKind::Trap { .. }
            | ExprKind::Exit { .. }
            | ExprKind::MatchUnreachable { .. } => {
                self.walk_control(expression, expression_id, expression_entry, escape)?
            }
            ExprKind::Let { .. }
            | ExprKind::MutableLocal { .. }
            | ExprKind::SetLocal { .. }
            | ExprKind::ProductValue { .. }
            | ExprKind::ProductField { .. }
            | ExprKind::WithProductField { .. }
            | ExprKind::EnumValue { .. }
            | ExprKind::EnumIsVariant { .. }
            | ExprKind::EnumField { .. }
            | ExprKind::EnumUnwrap { .. } => {
                self.walk_scopes(expression, expression_id, escape)?;
            }
        }
        Ok(expression_id)
    }

    pub(super) fn new(program: &'a hir::Program) -> Result<Self> {
        let mut function_ids = HashMap::new();
        function_ids
            .try_reserve(program.functions.len())
            .map_err(|_| Error::host("HIR memory-plan function index allocation failed"))?;
        for (index, function) in program.functions.iter().enumerate() {
            let raw = u64::try_from(index)
                .map_err(|_| Error::msg("HIR memory-plan function count exceeds u64"))?;
            if function_ids
                .insert(function.binding, MemoryFunctionId::new(raw))
                .is_some()
            {
                return Err(Error::msg(
                    "HIR memory-plan producer found duplicate function binding",
                ));
            }
        }
        let mut products_by_id = HashMap::new();
        products_by_id
            .try_reserve(program.products.len())
            .map_err(|_| Error::host("HIR memory-plan product index allocation failed"))?;
        for (index, product) in program.products.iter().enumerate() {
            if products_by_id.insert(product.id, index).is_some() {
                return Err(Error::msg(
                    "HIR memory-plan producer found duplicate product identity",
                ));
            }
        }
        let mut enums_by_id = HashMap::new();
        enums_by_id
            .try_reserve(program.enums.len())
            .map_err(|_| Error::host("HIR memory-plan enum index allocation failed"))?;
        for (index, enumeration) in program.enums.iter().enumerate() {
            if enums_by_id.insert(enumeration.id, index).is_some() {
                return Err(Error::msg(
                    "HIR memory-plan producer found duplicate enum identity",
                ));
            }
        }
        let mut producer = Self {
            program,
            type_planner: TypePlanner::new(program)?,
            function_ids,
            signatures: Vec::new(),
            functions: Vec::new(),
            entries: Vec::new(),
            expression_entries: HashMap::new(),
            child_entries: HashMap::new(),
            places_by_binding: HashMap::new(),
            products_by_id,
            enums_by_id,
            uses: Vec::new(),
            loans: Vec::new(),
            constants: Vec::new(),
            calls: Vec::new(),
            obligations: Vec::new(),
            destinations: Vec::new(),
            borrow_scopes: Vec::new(),
            current_function: MemoryFunctionId::new(0),
            next_expression: 0,
            next_place: 0,
            expression_parents: BTreeMap::new(),
            work: MemoryPlanWork::default(),
        };
        producer.build_signatures()?;
        Ok(producer)
    }
}

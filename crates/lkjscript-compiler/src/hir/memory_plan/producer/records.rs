use super::*;

impl<'a> Producer<'a> {
    pub(super) fn add_constant(
        &mut self,
        expression: MemoryExpressionId,
        expression_entry: MemoryEntryId,
        ty: &Type,
        hir_expression: &Expr,
        value: MemoryConstantValue,
        escape: MemoryEscape,
    ) -> Result<()> {
        self.charge_constants(1)?;
        self.constants
            .try_reserve(1)
            .map_err(|_| Error::host("HIR memory-plan constant allocation failed"))?;
        let id = MemoryConstantId::new(
            u64::try_from(self.constants.len())
                .map_err(|_| Error::msg("HIR memory-plan constant identity exceeds u64"))?,
        );
        self.constants.push(MemoryConstantPlan {
            id,
            function: self.current_function,
            expression,
            value,
        });
        let constant_entry = self.add_entry(
            MemorySubject::Constant {
                constant: id,
                expression,
            },
            ty,
            hir_expression.effects.bits(),
            escape,
            MemoryOrigin {
                source: crate::memory_plan::source_origin(hir_expression.origin),
                expression: Some(expression),
            },
        )?;
        if matches!(ty, Type::Bytes) {
            for entry_id in [expression_entry, constant_entry] {
                let entry = entry_id
                    .index()
                    .and_then(|index| self.entries.get_mut(index))
                    .ok_or_else(|| Error::msg("static bytes memory entry is missing"))?;
                entry.mode.multiplicity = MemoryMultiplicity::Copy;
                entry.mode.aliasing = MemoryAliasing::StaticShared;
                entry.mode.domain = MemoryDomain::Static;
                entry.mode.destruction = MemoryDestruction::Trivial;
                entry.mode.contention = MemoryContention::ImmutableShared;
                entry.root_projection = MemoryRootProjection::None;
                entry.copy_share = MemoryCopySharePlan::StaticIdentity;
                entry.drop_glue = None;
                entry.drop_path = None;
            }
        }
        Ok(())
    }
    pub(super) fn add_call(
        &mut self,
        expression: MemoryExpressionId,
        _expression_entry: MemoryEntryId,
        callee: &hir::BindingRef,
        args: &[Expr],
        hir_expression: &Expr,
        escape: MemoryEscape,
    ) -> Result<MemoryCallId> {
        let ExprKind::Call { instantiation, .. } = &hir_expression.kind else {
            return Err(Error::msg("HIR memory call record lost call expression"));
        };
        let mut resolved = self.resolved_call_signature(callee, instantiation.as_ref())?;
        if instantiation.is_some() && matches!(resolved.target, MemoryCallTarget::Direct(_)) {
            resolved.parameters = resolved
                .parameters
                .iter()
                .copied()
                .zip(args)
                .map(|(declared, argument)| {
                    if declared == MemoryParameterMode::Consume {
                        Ok(declared)
                    } else {
                        self.planned_parameter_mode(&argument.ty, false)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            resolved.result = self.planned_result_mode(&hir_expression.ty)?;
        }
        if args.len() != resolved.parameters.len() {
            return Err(Error::msg(
                "HIR direct call argument count does not match memory signature",
            ));
        }
        self.add_call_record(expression, hir_expression, resolved, escape)
    }
    pub(super) fn add_operation_call(
        &mut self,
        expression: MemoryExpressionId,
        _expression_entry: MemoryEntryId,
        operation: Operation,
        resolved_signature: &Type,
        hir_expression: &Expr,
        escape: MemoryEscape,
    ) -> Result<()> {
        let (parameters, result_ty) = callable_type(resolved_signature)?;
        let parameters = parameters
            .iter()
            .map(|parameter| operation_parameter_mode(operation, parameter))
            .collect();
        let resolved = ResolvedMemoryCall {
            target: MemoryCallTarget::Operation(operation.identity().as_u16()),
            witness_arguments: Vec::new(),
            parameters,
            result: result_mode(result_ty),
        };
        self.add_call_record(expression, hir_expression, resolved, escape)
            .map(|_| ())
    }
    pub(super) fn add_call_record(
        &mut self,
        expression: MemoryExpressionId,
        hir_expression: &Expr,
        resolved: ResolvedMemoryCall,
        escape: MemoryEscape,
    ) -> Result<MemoryCallId> {
        self.charge_calls(1)?;
        self.calls
            .try_reserve(1)
            .map_err(|_| Error::host("HIR memory-plan call allocation failed"))?;
        let id = MemoryCallId::new(
            u64::try_from(self.calls.len())
                .map_err(|_| Error::msg("HIR memory-plan call identity exceeds u64"))?,
        );
        self.calls.push(MemoryCallPlan {
            id,
            function: self.current_function,
            expression,
            target: resolved.target,
            witness_arguments: resolved.witness_arguments,
            borrow_scopes: vec![None; resolved.parameters.len()],
            parameters: resolved.parameters,
            result: resolved.result,
        });
        self.add_entry(
            MemorySubject::Call {
                call: id,
                expression,
            },
            &hir_expression.ty,
            hir_expression.effects.bits(),
            escape,
            MemoryOrigin {
                source: crate::memory_plan::source_origin(hir_expression.origin),
                expression: Some(expression),
            },
        )?;
        Ok(id)
    }
}

impl<'a> Producer<'a> {
    pub(super) fn add_obligation(
        &mut self,
        entry: MemoryEntryId,
        kind: MemoryObligationKind,
        drop_glue: Option<MemoryDropGlueId>,
        drop_path: Option<MemoryDropPathId>,
    ) -> Result<()> {
        self.charge_obligations(1)?;
        let id = MemoryObligationId::new(
            u64::try_from(self.obligations.len())
                .map_err(|_| Error::msg("HIR memory-plan obligation identity exceeds u64"))?,
        );
        self.obligations.push(MemoryObligation {
            id,
            function: self.current_function,
            entry,
            kind,
            drop_glue,
            drop_path,
            drop_class: (!matches!(kind, MemoryObligationKind::EndBorrow))
                .then_some(MemoryDropClass::Static),
        });
        Ok(())
    }
    pub(super) fn add_entry(
        &mut self,
        subject: MemorySubject,
        ty: &Type,
        effects: u16,
        escape: MemoryEscape,
        origin: MemoryOrigin,
    ) -> Result<MemoryEntryId> {
        self.charge_entries(1)?;
        self.entries
            .try_reserve(1)
            .map_err(|_| Error::host("HIR memory-plan entry allocation failed"))?;
        let id = MemoryEntryId::new(
            u64::try_from(self.entries.len())
                .map_err(|_| Error::msg("HIR memory-plan entry identity exceeds u64"))?,
        );
        let type_fact = self.type_planner.intern(ty)?;
        let fact = self.type_planner.fact(type_fact)?.clone();
        let (mode, execution, execution_cutover) = memory_mode(ty, &fact, effects, escape)?;
        let owns_glue = fact.mode != MemoryAggregateMode::Copy;
        let drop_path = owns_glue.then_some(fact.drop_path).flatten();
        let expression_index = match &subject {
            MemorySubject::Expression {
                expression,
                parent,
                child_index,
                ..
            } => Some((*expression, *parent, *child_index)),
            _ => None,
        };
        let place_index = match &subject {
            MemorySubject::Place {
                function,
                binding,
                place,
            } => Some(((*function, *binding), *place)),
            _ => None,
        };
        self.entries.push(MemoryPlanEntry {
            id,
            subject,
            ty: memory_type(ty),
            effects,
            mode,
            type_fact,
            root_projection: fact.root_projection,
            destination: None,
            copy_share: fact.copy_share,
            borrow_scope: None,
            drop_path,
            execution,
            execution_cutover,
            origin,
            drop_glue: owns_glue.then_some(fact.drop_glue).flatten(),
        });
        if let Some((expression, parent, child_index)) = expression_index {
            if self.expression_entries.insert(expression, id).is_some() {
                return Err(Error::msg(
                    "HIR memory-plan expression entry index is duplicated",
                ));
            }
            if let Some(parent) = parent {
                let children = self.child_entries.entry(parent).or_default();
                children
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR memory-plan child index allocation failed"))?;
                children.push((child_index, expression, drop_path));
            }
        }
        if let Some((key, place)) = place_index {
            if self.places_by_binding.insert(key, place).is_some() {
                return Err(Error::msg(
                    "HIR memory-plan place binding index is duplicated",
                ));
            }
        }
        Ok(id)
    }
    pub(super) fn next_expression(&mut self) -> Result<MemoryExpressionId> {
        self.charge_expressions(1)?;
        let id = MemoryExpressionId::new(self.next_expression);
        self.next_expression = self
            .next_expression
            .checked_add(1)
            .ok_or_else(|| Error::msg("HIR memory-plan expression identity overflow"))?;
        Ok(id)
    }
    pub(super) fn finish_loans(&mut self) -> Result<()> {
        let mut loads_by_binding: BTreeMap<(MemoryFunctionId, u64), Vec<MemoryExpressionId>> =
            BTreeMap::new();
        for usage in &self.uses {
            if usage.kind == MemoryUseKind::Load {
                let expressions = loads_by_binding
                    .entry((usage.function, usage.binding))
                    .or_default();
                expressions
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR memory-plan loan-use index allocation failed"))?;
                expressions.push(usage.expression);
            }
        }
        for expressions in loads_by_binding.values_mut() {
            expressions.sort_unstable();
        }
        for loan in &mut self.loans {
            let (semantic_uses, end_after) = if let Some(binding) = loan.binding {
                let expressions =
                    loads_by_binding
                        .get(&(loan.function, binding))
                        .ok_or_else(|| {
                            Error::msg("HIR memory-plan loan has no semantic reference use")
                        })?;
                let first =
                    expressions.partition_point(|expression| *expression <= loan.expression);
                let matching = &expressions[first..];
                let last_use = *matching.last().ok_or_else(|| {
                    Error::msg("HIR memory-plan loan has no semantic reference use")
                })?;
                let end_after = self
                    .expression_parents
                    .get(&last_use)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        Error::msg("HIR memory-plan reference use has no complete call expression")
                    })?;
                (
                    u64::try_from(matching.len())
                        .map_err(|_| Error::msg("HIR memory-plan loan use count exceeds u64"))?,
                    end_after,
                )
            } else {
                let parent = self
                    .expression_parents
                    .get(&loan.expression)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        Error::msg("temporary HIR loan has no enclosing call expression")
                    })?;
                (1, parent)
            };
            loan.semantic_uses = semantic_uses;
            loan.end_after = end_after;
        }
        Ok(())
    }
    pub(super) fn signature(&self, id: MemoryFunctionId) -> Result<&FunctionMemorySignature> {
        id.index()
            .and_then(|index| self.signatures.get(index))
            .filter(|signature| signature.function == id)
            .ok_or_else(|| Error::msg("HIR memory-plan function signature is missing"))
    }
    pub(super) fn binding_type(&self, binding: BindingId) -> Result<&Type> {
        self.program
            .binding(binding)
            .map(|binding| &binding.ty)
            .ok_or_else(|| Error::msg("HIR memory-plan references unknown binding"))
    }
    pub(super) fn charge_functions(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.functions, amount, "functions")
    }
    pub(super) fn charge_entries(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.entries, amount, "entries")
    }
    pub(super) fn charge_expressions(&mut self, amount: usize) -> Result<()> {
        let amount = u64::try_from(amount)
            .map_err(|_| Error::msg("HIR memory-plan expression charge exceeds u64"))?;
        self.work.expressions = self
            .work
            .expressions
            .checked_add(amount)
            .ok_or_else(|| Error::msg("HIR memory-plan expression work overflow"))?;
        Ok(())
    }
    pub(super) fn charge_uses(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.uses, amount, "uses")
    }
    pub(super) fn charge_loans(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.loans, amount, "loans")
    }
    pub(super) fn charge_constants(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.constants, amount, "constants")
    }
    pub(super) fn charge_calls(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.calls, amount, "calls")
    }
    pub(super) fn charge_obligations(&mut self, amount: usize) -> Result<()> {
        observe(&mut self.work.obligations, amount, "obligations")
    }
}

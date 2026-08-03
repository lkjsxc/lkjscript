impl<'a> Producer<'a> {
    fn add_constant(
        &mut self,
        expression: MemoryExpressionId,
        expression_entry: MemoryEntryId,
        ty: &Type,
        hir_expression: &Expr,
        value: MemoryConstantValue,
        escape: MemoryEscape,
    ) -> Result<()> {
        self.charge_constants(1)?;
        let id = MemoryConstantId::new(
            u32::try_from(self.constants.len())
                .map_err(|_| Error::msg("HIR memory-plan constant identity exceeds u32"))?,
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
                source: hir_expression.origin.raw(),
                expression: Some(expression),
            },
        )?;
        if matches!(ty, Type::Bytes) {
            for entry_id in [expression_entry, constant_entry] {
                let entry = self
                    .entries
                    .get_mut(entry_id.index().unwrap_or(usize::MAX))
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
    fn add_call(
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
        if instantiation.is_some()
            && matches!(resolved.target, MemoryCallTarget::Direct(_))
        {
            resolved.parameters = resolved.parameters.iter().copied().zip(args)
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
    fn add_operation_call(
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
    fn add_call_record(
        &mut self,
        expression: MemoryExpressionId,
        hir_expression: &Expr,
        resolved: ResolvedMemoryCall,
        escape: MemoryEscape,
    ) -> Result<MemoryCallId> {
        self.charge_calls(1)?;
        let id = MemoryCallId::new(
            u32::try_from(self.calls.len())
                .map_err(|_| Error::msg("HIR memory-plan call identity exceeds u32"))?,
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
                source: hir_expression.origin.raw(),
                expression: Some(expression),
            },
        )?;
        Ok(id)
    }
}

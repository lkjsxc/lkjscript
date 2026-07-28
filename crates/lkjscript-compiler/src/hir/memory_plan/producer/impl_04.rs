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
                entry.mode.storage = MemoryStorage::Static;
                entry.mode.destruction = MemoryDestruction::Trivial;
                entry.mode.contention = MemoryContention::ImmutableShared;
                entry.drop_glue = None;
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
    ) -> Result<()> {
        let (target, parameters, result) = match callee.storage {
            BindingStorage::Function => {
                let target = self
                    .function_ids
                    .get(&callee.binding)
                    .copied()
                    .ok_or_else(|| Error::msg("HIR direct call has no memory-signature target"))?;
                let signature = self.signature(target)?;
                (
                    MemoryCallTarget::Direct(target),
                    signature.parameters.clone(),
                    signature.result,
                )
            }
            BindingStorage::Local(_) => {
                let ty = self.binding_type(callee.binding)?;
                let (parameters, result_ty) = callable_type(ty)?;
                let modes: Vec<_> = parameters
                    .iter()
                    .map(|parameter| parameter_mode(parameter, false))
                    .collect();
                if modes.iter().any(|mode| *mode != MemoryParameterMode::Copy)
                    || result_mode(result_ty) != MemoryResultMode::Trivial
                {
                    return Err(Error::msg(
                        "affine or borrowed indirect call has no complete Current memory signature",
                    ));
                }
                (
                    MemoryCallTarget::Indirect(callee.binding.raw()),
                    modes,
                    result_mode(result_ty),
                )
            }
        };
        if args.len() != parameters.len() {
            return Err(Error::msg(
                "HIR direct call argument count does not match memory signature",
            ));
        }
        self.add_call_record(
            expression,
            hir_expression,
            target,
            parameters,
            result,
            escape,
        )
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
        self.add_call_record(
            expression,
            hir_expression,
            MemoryCallTarget::Operation(operation.identity().as_u16()),
            parameters,
            result_mode(result_ty),
            escape,
        )
    }
    fn add_call_record(
        &mut self,
        expression: MemoryExpressionId,
        hir_expression: &Expr,
        target: MemoryCallTarget,
        parameters: Vec<MemoryParameterMode>,
        result: MemoryResultMode,
        escape: MemoryEscape,
    ) -> Result<()> {
        self.charge_calls(1)?;
        let id = MemoryCallId::new(
            u32::try_from(self.calls.len())
                .map_err(|_| Error::msg("HIR memory-plan call identity exceeds u32"))?,
        );
        self.calls.push(MemoryCallPlan {
            id,
            function: self.current_function,
            expression,
            target,
            parameters,
            result,
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
        Ok(())
    }
    fn add_use(
        &mut self,
        expression: MemoryExpressionId,
        binding: BindingId,
        kind: MemoryUseKind,
    ) -> Result<()> {
        self.charge_uses(1)?;
        let id = MemoryUseId::new(
            u32::try_from(self.uses.len())
                .map_err(|_| Error::msg("HIR memory-plan use identity exceeds u32"))?,
        );
        self.uses.push(MemoryUse {
            id,
            function: self.current_function,
            expression,
            binding: binding.raw(),
            kind,
        });
        Ok(())
    }
}

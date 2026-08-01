use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_call(
        &mut self,
        callee: hir::BindingRef,
        args: &[Expr],
        instantiation: Option<&hir::GenericInstantiation>,
        ty: SsaType,
        expression_origin: hir::SourceId,
    ) -> Result<Option<ValueId>> {
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
        let witness_parameters = match &target {
            CallTarget::Direct(function) => self
                .function_witness_parameters
                .get(function)
                .cloned()
                .ok_or_else(|| Error::msg("SSA call target has no memory witness signature"))?,
            CallTarget::Indirect(_) => Vec::new(),
        };
        let consuming = match target {
            CallTarget::Direct(function) => self
                .function_parameter_consumption
                .get(&function)
                .cloned()
                .ok_or_else(|| Error::msg("SSA call target has no parameter ownership modes"))?,
            CallTarget::Indirect(_) => arguments
                .iter()
                .map(|argument| {
                    self.value_type(*argument).map(|ty| {
                        is_owned_value(self.structural, &ty)
                            && !self.structural.is_immutable(&ty)
                            && !matches!(ty, SsaType::Resource(_))
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        };
        if consuming.len() != arguments.len() {
            return Err(Error::msg(
                "SSA call parameter ownership modes do not match arity",
            ));
        }
        let value = self.append(
            ty,
            InstructionKind::Call {
                target,
                arguments,
                consuming,
                signature,
                instantiation: instantiation
                    .map(|instantiation| {
                        self.lower_instantiation(instantiation, &witness_parameters)
                    })
                    .transpose()?,
            },
            call_effects,
            expression_origin,
        )?;
        Ok(Some(value))
    }

    pub(in crate::ssa) fn lower_operation(
        &mut self,
        operation: Operation,
        resolved_signature: &Type,
        args: &[Expr],
        ty: SsaType,
        expression: &Expr,
    ) -> Result<Option<ValueId>> {
        if operation == Operation::Exit {
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
            return self.lower_short_circuit(operation, args, expression);
        }
        let Some(arguments) = self.lower_arguments(args)? else {
            return Ok(None);
        };
        let runtime = runtime_operation(operation)?;
        let signature = signature_from_type(resolved_signature, self.product_ids)?;
        let consumed_resource = arguments.first().copied();
        let result_ty = ty.clone();
        let result = self.append(
            ty,
            InstructionKind::Runtime {
                operation: runtime,
                arguments,
                signature,
            },
            effects(operation.effects()),
            expression.origin,
        )?;
        self.forget_consumed_ref_mut_arguments(args);
        if matches!(
            operation,
            Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
        ) {
            let [Expr {
                kind: ExprKind::Load(reference),
                ..
            }] = args
            else {
                return Err(Error::msg(
                    "resource close lowering requires one direct typed resource local",
                ));
            };
            let value = consumed_resource
                .ok_or_else(|| Error::msg("resource close lost its SSA operand"))?;
            self.record_explicit_close(reference.binding, value, expression.origin)?;
        }
        self.publish_structural_source(result_ty, result, expression.origin)
            .map(Some)
    }
}

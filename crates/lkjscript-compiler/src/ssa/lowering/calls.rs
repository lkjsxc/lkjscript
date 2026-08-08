use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn lower_call(
        &mut self,
        callee: hir::BindingRef,
        args: &[Expr],
        instantiation: Option<&hir::GenericInstantiation>,
        ty: SsaType,
        expression_origin: hir::Origin,
    ) -> Result<Option<ValueId>> {
        let parameter_modes = self.verified_call_parameter_modes()?;
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
                let declared = self
                    .function_parameter_modes
                    .get(&function)
                    .ok_or_else(|| {
                        Error::msg("SSA call target has no parameter ownership modes")
                    })?;
                if declared.len() != parameter_modes.len()
                    || (instantiation.is_none() && declared != &parameter_modes)
                {
                    return Err(Error::msg(
                        "SSA call parameter modes disagree with the verified callee",
                    ));
                }
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
        let Some(arguments) = self.lower_call_arguments(args, Some(&parameter_modes))? else {
            return Ok(None);
        };
        let signature = Signature::monomorphic(
            args.iter()
                .map(|argument| lower_type(&argument.ty, self.product_ids))
                .collect::<Result<Vec<_>>>()?,
            ty.clone(),
        );
        let witness_parameters = match &target {
            CallTarget::Direct(function) => self
                .function_witness_parameters
                .get(function)
                .cloned()
                .ok_or_else(|| Error::msg("SSA call target has no memory witness signature"))?,
            CallTarget::Indirect(_) => Vec::new(),
        };
        let consuming = parameter_modes
            .iter()
            .map(|mode| *mode == MemoryParameterMode::Consume)
            .collect::<Vec<_>>();
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
        if matches!(operation, Operation::And | Operation::Or) {
            return self.lower_short_circuit(operation, args, expression);
        }
        let parameter_modes = self.verified_call_parameter_modes()?;
        let Some(arguments) = self.lower_call_arguments(args, Some(&parameter_modes))? else {
            return Ok(None);
        };
        if operation == Operation::Exit {
            let Some(code) = arguments.first().copied() else {
                return Err(Error::msg("resolved exit has no code argument"));
            };
            self.terminate(Terminator::Exit { code })?;
            return Ok(None);
        }
        if operation == Operation::EqualValue {
            if let [Expr {
                ty: Type::Param(left),
                ..
            }, Expr {
                ty: Type::Param(right),
                ..
            }] = args
            {
                if left == right {
                    return self
                        .append(
                            SsaType::Bool,
                            InstructionKind::MemoryWitnessCompare {
                                parameter: left.clone(),
                                left: arguments[0],
                                right: arguments[1],
                            },
                            EffectSet::READS_MEMORY,
                            expression.origin,
                        )
                        .map(Some);
                }
            }
        }
        self.lower_concrete_operation(
            operation,
            resolved_signature,
            args,
            arguments,
            ty,
            expression,
        )
    }

    fn verified_call_parameter_modes(&self) -> Result<Vec<MemoryParameterMode>> {
        let expression = self
            .current_memory_expression
            .ok_or_else(|| Error::msg("SSA call has no HIR memory expression"))?;
        self.cleanup
            .call_parameter_modes
            .get(&expression.raw())
            .cloned()
            .ok_or_else(|| Error::msg("SSA call has no verified parameter modes"))
    }
}

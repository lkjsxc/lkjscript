struct ResolvedMemoryCall {
    target: MemoryCallTarget,
    witness_arguments: Vec<MemoryWitnessArgument>,
    parameters: Vec<MemoryParameterMode>,
    result: MemoryResultMode,
}

impl Producer<'_> {
    fn resolved_call_signature(
        &mut self,
        callee: &hir::BindingRef,
        instantiation: Option<&hir::GenericInstantiation>,
    ) -> Result<ResolvedMemoryCall> {
        match callee.storage {
            BindingStorage::Function => {
                let target = self
                    .function_ids
                    .get(&callee.binding)
                    .copied()
                    .ok_or_else(|| {
                        Error::msg("HIR direct call has no memory-signature target")
                    })?;
                let signature = self.signature(target)?.clone();
                let callee_ty = self.binding_type(callee.binding)?.clone();
                let witness_arguments = memory_witness_arguments(
                    &mut self.type_planner,
                    &callee_ty,
                    &signature.witness_parameters,
                    instantiation,
                )?;
                Ok(ResolvedMemoryCall {
                    target: MemoryCallTarget::Direct(target),
                    witness_arguments,
                    parameters: signature.parameters,
                    result: signature.result,
                })
            }
            BindingStorage::Local(_) => {
                let ty = self.binding_type(callee.binding)?;
                if instantiation.is_some() || matches!(ty, Type::Forall { .. }) {
                    return Err(Error::msg(
                        "HIR indirect generic call has no residual transport witness signature",
                    ));
                }
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
                Ok(ResolvedMemoryCall {
                    target: MemoryCallTarget::Indirect(callee.binding.raw()),
                    witness_arguments: Vec::new(),
                    parameters: modes,
                    result: result_mode(result_ty),
                })
            }
        }
    }
}

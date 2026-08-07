use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn install_dynamic_owner_parameters(
        &mut self,
        parameters: &[BindingId],
        source: hir::SourceId,
    ) -> Result<()> {
        let mut dynamic = Vec::new();
        dynamic
            .try_reserve(parameters.len())
            .map_err(|_| Error::host("SSA dynamic owner parameter allocation failed"))?;
        dynamic.extend(
            parameters
                .iter()
                .copied()
                .zip(&self.signature.parameters)
                .filter_map(|(binding, ty)| match ty {
                    SsaType::TypeParameter(parameter)
                        if self
                            .signature
                            .memory_witness_parameters
                            .iter()
                            .any(|requirement| {
                                requirement.parameter == parameter.as_str()
                                    && requirement.operations.contains(
                                        &lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
                                    )
                                    && requirement.operations.contains(
                                        &lkjscript_contracts::MemoryWitnessOperation::Dispose,
                                    )
                            }) =>
                    {
                        Some((binding, parameter.clone()))
                    }
                    _ => None,
                }),
        );
        for (binding, parameter) in dynamic {
            let original = self.env.get(&binding).copied().ok_or_else(|| {
                Error::msg("SSA dynamic owner parameter is absent from the entry environment")
            })?;
            let independent = self.append(
                SsaType::TypeParameter(parameter.clone()),
                InstructionKind::MemoryWitnessIndependentOwner {
                    parameter: parameter.clone(),
                    value: original,
                },
                EffectSet::ALLOCATES,
                source,
            )?;
            let _disposed = self.append(
                SsaType::Unit,
                InstructionKind::MemoryWitnessDispose {
                    parameter,
                    value: original,
                },
                EffectSet::PURE,
                source,
            )?;
            self.env.insert(binding, independent);
        }
        Ok(())
    }
}

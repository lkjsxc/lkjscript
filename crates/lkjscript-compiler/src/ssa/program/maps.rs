fn parameter_modes(
    function_ids: &HashMap<BindingId, FunctionId>,
    memory_plan: &HirMemoryPlan,
) -> HashMap<FunctionId, Vec<MemoryParameterMode>> {
    function_ids
        .values()
        .copied()
        .map(|id| {
            let modes = memory_plan
                .function(MemoryFunctionId::new(id.raw()))
                .map(|function| function.signature.parameters.clone())
                .unwrap_or_default();
            (id, modes)
        })
        .collect()
}

fn witness_parameters(
    memory_plan: &HirMemoryPlan,
) -> Result<HashMap<FunctionId, Vec<MemoryWitnessParameter>>> {
    memory_plan
        .functions
        .iter()
        .map(|function| {
            Ok((
                FunctionId::new(function.id.raw()),
                function
                    .signature
                    .witness_parameters
                    .iter()
                    .map(|parameter| MemoryWitnessParameter {
                        parameter: parameter.parameter.clone(),
                        operations: parameter
                            .operations
                            .iter()
                            .map(|operation| match operation {
                                crate::memory_plan::MemoryWitnessOperation::Transport => {
                                    lkjscript_contracts::MemoryWitnessOperation::Transport
                                }
                                crate::memory_plan::MemoryWitnessOperation::Compare => {
                                    lkjscript_contracts::MemoryWitnessOperation::Compare
                                }
                                crate::memory_plan::MemoryWitnessOperation::IndependentOwner => {
                                    lkjscript_contracts::MemoryWitnessOperation::IndependentOwner
                                }
                                crate::memory_plan::MemoryWitnessOperation::Dispose => {
                                    lkjscript_contracts::MemoryWitnessOperation::Dispose
                                }
                            })
                            .collect(),
                    })
                    .collect(),
            ))
        })
        .collect()
}

fn function_effects(
    program: &hir::Program,
    function_ids: &HashMap<BindingId, FunctionId>,
) -> HashMap<FunctionId, EffectSet> {
    program
        .functions
        .iter()
        .filter_map(|function| {
            function_ids
                .get(&function.binding)
                .copied()
                .map(|id| (id, effects(function.summary)))
        })
        .collect()
}
